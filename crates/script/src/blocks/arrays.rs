// Logics.dll's array blocks that edit rows and columns or search by a typed key (Set Row, Insert/Remove Column,
// Move Row, Sort Rows, Test Cell, Get Highest, Value Count, Remove Key Row, Remove Row If, Change Value If), and
// the typed reading and writing of cells the older array blocks share. Semantics: docs/gameplay_bbs.md, "Arrays".
// A column has a kind (1 Integer, 2 Float, 3 String, 4 Object, 5 Parameter); a pin or pout only meets a cell of
// the matching type, as the original's: a key of another type compares as zero (or the empty string), a cell is
// written only into a pout of its type.

use super::*;
use crate::Array;
use std::cmp::Ordering;

pub(crate) const INT: u32 = 1;
pub(crate) const FLOAT: u32 = 2;
pub(crate) const STRING: u32 = 3;
pub(crate) const OBJECT: u32 = 4;
pub(crate) const PARAMETER: u32 = 5;

/// The most columns an array may have (a column inserted beyond it is refused).
pub const MAX_COLUMNS: usize = 256;

// ---- pins ---------------------------------------------------------------------------------------------------------

/// Whether the input parameter `i` has a value to read (a source or an operation computing it).
fn has_source(rt: &Runtime, b: &Block, i: usize) -> bool {
    b.pins
        .get(i)
        .and_then(|&p| rt.graph.param(p))
        .is_some_and(|p| p.source.is_some() || p.op.is_some())
}

fn pin_value(rt: &mut Runtime, b: &Block, i: usize, host: &mut dyn Host) -> Option<Value> {
    if !has_source(rt, b, i) {
        return None;
    }
    let p = b.pins.get(i).copied()?;
    Some(rt.read(p, host))
}

/// An Integer pin by index; the block's default when the pin or its source is missing.
fn pin_int(rt: &mut Runtime, b: &Block, i: usize, default: i32, host: &mut dyn Host) -> i32 {
    pin_value(rt, b, i, host).map_or(default, |v| v.as_int())
}

fn pin_guid(rt: &Runtime, b: &Block, i: usize) -> Option<Guid> {
    b.pins
        .get(i)
        .and_then(|&p| rt.graph.param(p))
        .map(|p| p.guid)
}

// ---- typed keys and cells ---------------------------------------------------------------------------------------

/// A value to compare a column's cells with.
#[derive(Debug, Clone)]
pub(crate) enum Key {
    Int(i32),
    Float(f32),
    Text(String),
    Obj(Option<Id>),
}

/// The key a block reads from pin `i` for a column of `kind`: the pin's value when its type is the column's, else
/// zero or the empty string (the original compares against what it could not fill).
pub(crate) fn key_from(
    rt: &mut Runtime,
    b: &Block,
    i: usize,
    kind: u32,
    host: &mut dyn Host,
) -> Key {
    let Some(g) = pin_guid(rt, b, i) else {
        return Key::Int(0);
    };
    let v = pin_value(rt, b, i, host);
    match kind {
        FLOAT => Key::Float(match &v {
            Some(v) if g == types::FLOAT => v.as_float(),
            _ => 0.0,
        }),
        STRING => Key::Text(match &v {
            Some(v) if g == types::STRING => v.as_string(),
            _ => String::new(),
        }),
        OBJECT => Key::Obj(match &v {
            Some(v) if g == types::BEOBJECT => v.as_obj(),
            _ => None,
        }),
        _ => Key::Int(match &v {
            Some(v) if g == types::INT => v.as_int(),
            _ => 0,
        }),
    }
}

/// An object as the number the original compares and sorts by (none is 0; the ids follow creation order).
fn obj_num(o: Option<Id>) -> i64 {
    o.map_or(0, |i| {
        i64::try_from(i).unwrap_or(i64::MAX).saturating_add(1)
    })
}

/// The same as the 32-bit integer the original holds (an id stored from a number may be negative).
fn obj_int(o: Option<Id>) -> i32 {
    obj_num(o) as u32 as i32
}

fn op_holds(op: i32, o: Ordering) -> bool {
    match op {
        1 => o == Ordering::Equal,
        2 => o != Ordering::Equal,
        3 => o == Ordering::Less,
        4 => o != Ordering::Greater,
        5 => o == Ordering::Greater,
        6 => o != Ordering::Less,
        _ => false,
    }
}

fn float_holds(op: i32, a: f32, b: f32) -> bool {
    match op {
        1 => a == b,
        2 => a != b,
        3 => a < b,
        4 => a <= b,
        5 => a > b,
        6 => a >= b,
        _ => false,
    }
}

/// The column's test of a cell against a key (`op`: 1 ==, 2 !=, 3 <, 4 <=, 5 >, 6 >=). Parameter columns hold
/// nothing the runtime can compare: false.
pub(crate) fn cell_test(kind: u32, cell: &Value, op: i32, key: &Key) -> bool {
    match (kind, key) {
        (FLOAT, Key::Float(k)) => float_holds(op, cell.as_float(), *k),
        (STRING, Key::Text(k)) => op_holds(op, cell.as_string().as_bytes().cmp(k.as_bytes())),
        (OBJECT, Key::Obj(k)) => op_holds(op, obj_int(cell.as_obj()).cmp(&obj_int(*k))),
        (INT, Key::Int(k)) => op_holds(op, cell.as_int().cmp(k)),
        _ => false,
    }
}

fn column_kind(a: &Array, col: usize) -> Option<u32> {
    a.columns.get(col).map(|c| c.1)
}

fn test_row(a: &Array, row: usize, col: usize, op: i32, key: &Key) -> bool {
    match (
        column_kind(a, col),
        a.rows.get(row).and_then(|r| r.get(col)),
    ) {
        (Some(k), Some(cell)) => cell_test(k, cell, op, key),
        _ => false,
    }
}

/// The first row from `start` whose cell in `col` satisfies `op` against the key.
pub(crate) fn find_row(a: &Array, col: usize, op: i32, key: &Key, start: i32) -> Option<usize> {
    let start = usize::try_from(start).ok()?;
    if col >= a.columns.len() || start >= a.rows.len() {
        return None;
    }
    (start..a.rows.len()).find(|&r| test_row(a, r, col, op, key))
}

/// A cell as an output parameter takes it: only into a pout of the column's type; an object only into one of a
/// class it belongs to.
fn cell_to_pout(rt: &mut Runtime, pout: Id, kind: u32, cell: &Value) {
    let Some(g) = rt.graph.param(pout).map(|p| p.guid) else {
        return;
    };
    let v = match kind {
        INT if g == types::INT => Value::Int(cell.as_int()),
        FLOAT if g == types::FLOAT => Value::Float(cell.as_float()),
        STRING if g == types::STRING => Value::Str(cell.as_string()),
        OBJECT => {
            let Some(types::Kind::Object(cid)) = types::kind(g) else {
                return;
            };
            let o = cell.as_obj().filter(|&o| {
                rt.graph
                    .objects
                    .get(o)
                    .is_some_and(|x| types::class_derives(x.class, cid))
            });
            Value::Obj(o)
        }
        // a parameter cell holds a value of any type: it goes out as it is
        PARAMETER => cell.clone(),
        _ => return,
    };
    rt.write(pout, v);
}

/// One cell of an array into output `index` of the block.
pub(crate) fn cell_to_out(
    rt: &mut Runtime,
    b: &Block,
    array: usize,
    row: usize,
    col: usize,
    index: usize,
) {
    let Some(&p) = b.pouts.get(index) else { return };
    let Some(a) = rt.array(array) else { return };
    let (Some(kind), Some(cell)) = (
        column_kind(a, col),
        a.rows.get(row).and_then(|r| r.get(col)),
    ) else {
        return;
    };
    let cell = cell.clone();
    cell_to_pout(rt, p, kind, &cell);
}

/// A row's cells into the block's outputs from `first` on.
pub(crate) fn row_to_outs(rt: &mut Runtime, b: &Block, array: usize, row: usize, first: usize) {
    let n = rt.array(array).map_or(0, |a| a.columns.len());
    for col in 0..n {
        cell_to_out(rt, b, array, row, col, first.saturating_add(col));
    }
}

/// The four raw bytes of a value, as the original copies a parameter into a number, integer or object cell.
fn raw4(v: &Value) -> u32 {
    match v {
        Value::Int(i) => *i as u32,
        Value::Float(f) => f.to_bits(),
        Value::Bool(b) => u32::from(*b),
        Value::Obj(o) => obj_num(*o) as u32,
        Value::Raw(b) => match (b.first(), b.get(1), b.get(2), b.get(3)) {
            (Some(&a), Some(&b), Some(&c), Some(&d)) => u32::from_le_bytes([a, b, c, d]),
            _ => 0,
        },
        _ => 0,
    }
}

/// A parameter's text as the original writes it into a string cell.
fn text_of(rt: &Runtime, g: Guid, v: &Value) -> String {
    if types::derives(g, types::BOOL) {
        return if v.as_bool() { "TRUE" } else { "FALSE" }.to_string();
    }
    match v {
        Value::Obj(o) => o.map_or_else(|| "NULL".to_string(), |o| rt.graph.name(o).to_string()),
        v => v.as_string(),
    }
}

/// The cell a parameter gives a column of `kind` (Set Cell, Change Value If): a string column takes its text, a
/// number or object column the parameter's four bytes as they are. A parameter column takes the value itself.
pub(crate) fn cell_from(rt: &Runtime, kind: u32, g: Guid, v: &Value) -> Option<Value> {
    match kind {
        PARAMETER => Some(v.clone()),
        STRING => Some(Value::Str(text_of(rt, g, v))),
        FLOAT => Some(Value::Float(f32::from_bits(raw4(v)))),
        // another type's four bytes are an object id as they stand (one that names nothing is harmless)
        OBJECT => Some(Value::Obj(match v {
            Value::Obj(o) => *o,
            _ => (raw4(v) as usize).checked_sub(1),
        })),
        _ => Some(Value::Int(raw4(v) as i32)),
    }
}

fn default_cell(kind: u32) -> Value {
    match kind {
        FLOAT => Value::Float(0.0),
        STRING => Value::Str(String::new()),
        OBJECT => Value::Obj(None),
        PARAMETER => Value::None,
        _ => Value::Int(0),
    }
}

// ---- blocks ---------------------------------------------------------------------------------------------------------

/// The array a block acts on, if its target is one of the map's.
fn target_array(rt: &mut Runtime, b: &Block, host: &mut dyn Host) -> Option<usize> {
    let t = rt.target(b, host);
    array_of(rt, t)
}

pub(crate) fn set_row(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let Some(a) = target_array(rt, b, host) else {
        return Flow::Done;
    };
    let row = pin_int(rt, b, 0, 0, host);
    // setting 0 "Unique Key Column" (on unless saved off): a key another row already has is refused
    let unique = b.locals.is_empty() || local(rt, b, 0).as_bool();
    let key_col = rt.array(a).and_then(|x| x.key);
    if unique && let Some(k) = key_col {
        let kind = rt.array(a).and_then(|x| column_kind(x, k)).unwrap_or(INT);
        let key = key_from(rt, b, k.saturating_add(1), kind, host);
        let found = rt.array(a).and_then(|x| find_row(x, k, 1, &key, 0));
        if found.is_some_and(|f| i32::try_from(f).ok() != Some(row)) {
            input(rt, b, 0);
            out(rt, b, 1);
            return Flow::Done;
        }
    }
    if let Ok(r) = usize::try_from(row)
        && rt.array(a).is_some_and(|x| r < x.rows.len())
    {
        let n = rt.array(a).map_or(0, |x| x.columns.len());
        for col in 0..n {
            let i = col.saturating_add(1);
            let Some(kind) = rt.array(a).and_then(|x| column_kind(x, col)) else {
                continue;
            };
            let (Some(g), Some(v)) = (pin_guid(rt, b, i), pin_value(rt, b, i, host)) else {
                continue;
            };
            let cell = match kind {
                INT if g == types::INT => Value::Int(v.as_int()),
                FLOAT if g == types::FLOAT => Value::Float(v.as_float()),
                STRING if g == types::STRING => Value::Str(v.as_string()),
                OBJECT if g == types::BEOBJECT => Value::Obj(v.as_obj()),
                _ => continue,
            };
            rt.set_cell(a, r, col, cell);
        }
    }
    input(rt, b, 0);
    out(rt, b, 0);
    Flow::Done
}

pub(crate) fn insert_column(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    // the input is answered before the target is looked at: Out fires even for no array
    input(rt, b, 0);
    out(rt, b, 0);
    let Some(a) = target_array(rt, b, host) else {
        return Flow::Done;
    };
    let index = pin_int(rt, b, 0, -1, host);
    let name = pin_value(rt, b, 1, host)
        .map(|v| v.as_string())
        .unwrap_or_default();
    let kind = u32::try_from(pin_int(rt, b, 2, INT as i32, host)).unwrap_or(0);
    if !(INT..=PARAMETER).contains(&kind) {
        return Flow::Done;
    }
    let Some(arr) = rt.array_mut(a) else {
        return Flow::Done;
    };
    if arr.columns.len() >= MAX_COLUMNS {
        return Flow::Done;
    }
    let n = arr.columns.len();
    let at = usize::try_from(index).ok().filter(|&i| i <= n).unwrap_or(n);
    arr.columns.insert(at, (name, kind));
    for row in &mut arr.rows {
        let at = at.min(row.len());
        row.insert(at, default_cell(kind));
    }
    if arr.key.is_some_and(|k| k >= at) {
        arr.key = arr.key.map(|k| k.saturating_add(1));
    }
    Flow::Done
}

pub(crate) fn remove_column(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    input(rt, b, 0);
    out(rt, b, 0);
    let Some(a) = target_array(rt, b, host) else {
        return Flow::Done;
    };
    let index = pin_int(rt, b, 0, -1, host);
    let Some(arr) = rt.array_mut(a) else {
        return Flow::Done;
    };
    let Some(at) = usize::try_from(index)
        .ok()
        .filter(|&i| i < arr.columns.len())
    else {
        return Flow::Done;
    };
    arr.columns.remove(at);
    for row in &mut arr.rows {
        if at < row.len() {
            row.remove(at);
        }
    }
    arr.key = match arr.key {
        Some(k) if k == at => None,
        Some(k) if k > at => Some(k.saturating_sub(1)),
        k => k,
    };
    Flow::Done
}

pub(crate) fn move_row(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    // from the block's own pins (Src Row, Dest Row) and the array's move: no C reference, docs say so
    input(rt, b, 0);
    out(rt, b, 0);
    let Some(a) = target_array(rt, b, host) else {
        return Flow::Done;
    };
    let src = usize::try_from(pin_int(rt, b, 0, 0, host)).ok();
    let dst = usize::try_from(pin_int(rt, b, 1, 0, host)).ok();
    if let (Some(s), Some(d), Some(arr)) = (src, dst, rt.array_mut(a))
        && s < arr.rows.len()
        && d < arr.rows.len()
    {
        let row = arr.rows.remove(s);
        arr.rows.insert(d, row);
    }
    Flow::Done
}

/// The order the column puts two cells in, as the original's compare functions answer it (an integer's by a
/// wrapped difference, a float's by the sign of its difference, a string's byte by byte); positive when `y` is
/// above `x`.
fn column_compare(kind: u32, x: &Value, y: &Value) -> i32 {
    match kind {
        FLOAT => {
            let d = y.as_float() - x.as_float();
            if d == 0.0 {
                0
            } else if d > 0.0 {
                2
            } else {
                -2
            }
        }
        STRING => match y.as_string().as_bytes().cmp(x.as_string().as_bytes()) {
            Ordering::Less => -1,
            Ordering::Equal => 0,
            Ordering::Greater => 1,
        },
        PARAMETER => 0,
        OBJECT => (obj_num(y.as_obj()) as u32).wrapping_sub(obj_num(x.as_obj()) as u32) as i32,
        _ => (y.as_int() as u32).wrapping_sub(x.as_int() as u32) as i32,
    }
}

/// Sorts rows with the quicksort of the C library the original was built with (a median element swapped to the
/// front, partitioned, runs of eight or fewer by selection), so that rows with equal keys end up in the same
/// order as in the original. `cmp(a, b)` is positive when row `a` sorts after row `b`.
fn msvcrt_sort(order: &mut [usize], cmp: &dyn Fn(usize, usize) -> i32) {
    let at = |o: &[usize], i: usize| o.get(i).copied().unwrap_or(0);
    let mut stack: Vec<(usize, usize)> = Vec::new();
    if order.len() > 1 {
        stack.push((0, order.len().saturating_sub(1)));
    }
    while let Some((lo, hi)) = stack.pop() {
        let size = hi.saturating_sub(lo).saturating_add(1);
        if size <= 8 {
            let mut top = hi;
            while top > lo {
                let mut max = lo;
                for p in lo.saturating_add(1)..=top {
                    if cmp(at(order, p), at(order, max)) > 0 {
                        max = p;
                    }
                }
                order.swap(max.min(order.len().saturating_sub(1)), top);
                top = top.saturating_sub(1);
            }
            continue;
        }
        let mid = lo.saturating_add(size / 2);
        order.swap(mid, lo);
        let pivot = at(order, lo);
        let mut loguy = lo;
        let mut higuy = hi.saturating_add(1);
        loop {
            loop {
                loguy = loguy.saturating_add(1);
                if !(loguy <= hi && cmp(at(order, loguy), pivot) <= 0) {
                    break;
                }
            }
            loop {
                higuy = higuy.saturating_sub(1);
                if !(higuy > lo && cmp(at(order, higuy), pivot) >= 0) {
                    break;
                }
            }
            if higuy < loguy {
                break;
            }
            order.swap(loguy, higuy);
        }
        order.swap(lo, higuy);
        if lo.saturating_add(1) < higuy {
            stack.push((lo, higuy.saturating_sub(1)));
        }
        if loguy < hi {
            stack.push((loguy, hi));
        }
    }
}

pub(crate) fn sort_rows(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let Some(a) = target_array(rt, b, host) else {
        return Flow::Done;
    };
    let col = pin_int(rt, b, 0, 0, host);
    let ascending = pin_int(rt, b, 1, 1, host) != 0;
    if let Some(arr) = rt.array_mut(a)
        && let Some(col) = usize::try_from(col).ok().filter(|&c| c < arr.columns.len())
        && arr.rows.len() >= 2
        && let Some(kind) = column_kind(arr, col)
    {
        let none = Value::None;
        let cell = |r: usize| arr.rows.get(r).and_then(|x| x.get(col)).unwrap_or(&none);
        let cmp = |ra: usize, rb: usize| {
            let v = column_compare(kind, cell(rb), cell(ra));
            if ascending { v } else { v.wrapping_neg() }
        };
        let mut order: Vec<usize> = (0..arr.rows.len()).collect();
        msvcrt_sort(&mut order, &cmp);
        let sorted: Vec<Vec<Value>> = order
            .iter()
            .filter_map(|&r| arr.rows.get(r).cloned())
            .collect();
        arr.rows = sorted;
    }
    input(rt, b, 0);
    out(rt, b, 0);
    Flow::Done
}

pub(crate) fn test_cell(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let Some(a) = target_array(rt, b, host) else {
        return Flow::Done;
    };
    let row = pin_int(rt, b, 0, 0, host);
    let col = pin_int(rt, b, 1, 0, host);
    let op = pin_int(rt, b, 2, 1, host);
    let (Ok(row), Ok(col)) = (usize::try_from(row), usize::try_from(col)) else {
        out(rt, b, 1);
        return Flow::Done;
    };
    let kind = rt.array(a).and_then(|x| column_kind(x, col)).unwrap_or(INT);
    let key = key_from(rt, b, 3, kind, host);
    let hit = rt.array(a).is_some_and(|x| test_row(x, row, col, op, &key));
    // the input is not cleared here: it stays active, as the original's
    out(rt, b, usize::from(!hit));
    Flow::Done
}

pub(crate) fn get_highest(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let Some(a) = target_array(rt, b, host) else {
        return Flow::Done;
    };
    let col = usize::try_from(pin_int(rt, b, 0, 0, host)).ok();
    let mut row: Option<usize> = None;
    if let (Some(col), Some(arr)) = (col, rt.array(a))
        && let Some(kind) = column_kind(arr, col)
        && !arr.rows.is_empty()
    {
        let none = Value::None;
        let cell = |r: usize| arr.rows.get(r).and_then(|x| x.get(col)).unwrap_or(&none);
        let mut best = 0usize;
        for r in 1..arr.rows.len() {
            // the first of equal values stays
            if column_compare(kind, cell(best), cell(r)) > 0 {
                best = r;
            }
        }
        row = Some(best);
    }
    let shown = row.map_or(-1, |r| i32::try_from(r).unwrap_or(i32::MAX));
    set_pout(rt, b, "Row Number", 0, Value::Int(shown));
    input(rt, b, 0);
    match row {
        None => out(rt, b, 1),
        Some(r) => {
            row_to_outs(rt, b, a, r, 1);
            out(rt, b, 0);
        }
    }
    Flow::Done
}

pub(crate) fn value_count(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let Some(a) = target_array(rt, b, host) else {
        return Flow::Done;
    };
    let col = pin_int(rt, b, 0, 1, host);
    let kind = usize::try_from(col)
        .ok()
        .and_then(|c| rt.array(a).and_then(|x| column_kind(x, c)))
        .unwrap_or(INT);
    let key = key_from(rt, b, 2, kind, host);
    let op = pin_int(rt, b, 1, 1, host);
    let n = match (usize::try_from(col), rt.array(a)) {
        (Ok(col), Some(arr)) if col < arr.columns.len() => (0..arr.rows.len())
            .filter(|&r| test_row(arr, r, col, op, &key))
            .count(),
        _ => 0,
    };
    set_pout(
        rt,
        b,
        "Count",
        0,
        Value::Int(i32::try_from(n).unwrap_or(i32::MAX)),
    );
    input(rt, b, 0);
    out(rt, b, 0);
    Flow::Done
}

pub(crate) fn remove_key_row(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let Some(a) = target_array(rt, b, host) else {
        return Flow::Done;
    };
    if b.pins.is_empty() {
        return Flow::Done;
    }
    // an array without a key column: the original logs it and nothing happens (In stays)
    let Some(k) = rt.array(a).and_then(|x| x.key) else {
        return Flow::Done;
    };
    let kind = rt.array(a).and_then(|x| column_kind(x, k)).unwrap_or(INT);
    let key = key_from(rt, b, 0, kind, host);
    let row = rt.array(a).and_then(|x| find_row(x, k, 1, &key, 0));
    input(rt, b, 0);
    match row {
        Some(r) => {
            rt.remove_row(a, r);
            out(rt, b, 0);
        }
        None => out(rt, b, 1),
    }
    Flow::Done
}

pub(crate) fn remove_row_if(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let Some(a) = target_array(rt, b, host) else {
        return Flow::Done;
    };
    let col = pin_int(rt, b, 0, 0, host);
    let op = pin_int(rt, b, 1, 1, host);
    let kind = usize::try_from(col)
        .ok()
        .and_then(|c| rt.array(a).and_then(|x| column_kind(x, c)))
        .unwrap_or(INT);
    let key = key_from(rt, b, 2, kind, host);
    let mut removed = 0i32;
    if let Ok(col) = usize::try_from(col) {
        // each hit leaves the next candidate at the same index
        let mut r = 0usize;
        while let Some(found) = rt
            .array(a)
            .and_then(|x| find_row(x, col, op, &key, i32::try_from(r).unwrap_or(i32::MAX)))
        {
            rt.remove_row(a, found);
            removed = removed.saturating_add(1);
            r = found;
        }
    }
    set_pout(rt, b, "Number Removed", 0, Value::Int(removed));
    input(rt, b, 0);
    out(rt, b, usize::from(removed == 0));
    Flow::Done
}

pub(crate) fn change_value_if(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let Some(a) = target_array(rt, b, host) else {
        return Flow::Done;
    };
    let col = pin_int(rt, b, 0, 0, host);
    let op = pin_int(rt, b, 1, 1, host);
    let kind = usize::try_from(col)
        .ok()
        .and_then(|c| rt.array(a).and_then(|x| column_kind(x, c)))
        .unwrap_or(INT);
    let key = key_from(rt, b, 2, kind, host);
    let new = pin_guid(rt, b, 3).zip(pin_value(rt, b, 3, host));
    let mut changed = 0i32;
    if let Ok(col) = usize::try_from(col) {
        let mut r = 0usize;
        while let Some(found) = rt
            .array(a)
            .and_then(|x| find_row(x, col, op, &key, i32::try_from(r).unwrap_or(i32::MAX)))
        {
            if let Some(((g, v), kind)) = new.as_ref().zip(column_kind_of(rt, a, col))
                && let Some(cell) = cell_from(rt, kind, *g, v)
            {
                rt.set_cell(a, found, col, cell);
            }
            changed = changed.saturating_add(1);
            r = found.saturating_add(1);
        }
    }
    set_pout(rt, b, "Number Changed", 0, Value::Int(changed));
    input(rt, b, 0);
    out(rt, b, usize::from(changed == 0));
    Flow::Done
}

fn column_kind_of(rt: &Runtime, a: usize, col: usize) -> Option<u32> {
    rt.array(a).and_then(|x| column_kind(x, col))
}

/// Row Search: the first row from a start whose cell satisfies the operation against a typed reference.
pub(crate) fn row_search(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let Some(a) = target_array(rt, b, host) else {
        return Flow::Done;
    };
    let col = pin_int(rt, b, 0, 0, host);
    let first = pin_int(rt, b, 1, 0, host);
    let op = pin_int(rt, b, 2, 1, host);
    let kind = usize::try_from(col)
        .ok()
        .and_then(|c| column_kind_of(rt, a, c))
        .unwrap_or(INT);
    let key = key_from(rt, b, 3, kind, host);
    let found = usize::try_from(col)
        .ok()
        .and_then(|c| rt.array(a).and_then(|x| find_row(x, c, op, &key, first)));
    input(rt, b, 0);
    match found {
        Some(r) => {
            set_pout(
                rt,
                b,
                "First Row Found",
                0,
                Value::Int(i32::try_from(r).unwrap_or(i32::MAX)),
            );
            out(rt, b, 0);
        }
        None => out(rt, b, 1),
    }
    Flow::Done
}

/// Iterator If (Logics, `FUN_25499770`, C: `bb_iterator_if`): the rows whose Column compares (Operator) with the
/// Reference Value, one per loop; pOut 0 holds the row index the walk is at, the cells follow from pOut 1. Out
/// when no row is left (also with an empty array).
pub(crate) fn iterator_if(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let Some(a) = target_array(rt, b, host) else {
        return Flow::Done;
    };
    let start = if input(rt, b, 0) {
        0
    } else {
        input(rt, b, 1);
        pout_value(rt, b, 0)
            .map_or(0, |v| v.as_int())
            .saturating_add(1)
    };
    let n = i32::try_from(rt.rows(a).len()).unwrap_or(i32::MAX);
    if start >= n {
        out(rt, b, 0);
        return Flow::Done;
    }
    let col = pin_int(rt, b, 0, 0, host);
    let op = pin_int(rt, b, 1, 1, host);
    let col = usize::try_from(col).ok();
    let kind = col.and_then(|c| column_kind_of(rt, a, c)).unwrap_or(INT);
    let key = key_from(rt, b, 2, kind, host);
    let found = col.and_then(|c| rt.array(a).and_then(|x| find_row(x, c, op, &key, start)));
    match found {
        Some(r) => {
            row_to_outs(rt, b, a, r, 1);
            set_pout(
                rt,
                b,
                "Index",
                0,
                Value::Int(i32::try_from(r).unwrap_or(i32::MAX)),
            );
            out(rt, b, 1);
        }
        None => out(rt, b, 0),
    }
    Flow::Done
}

/// Shuffle Order (Logics): the rows of the target array put in a random order, drawn from the run's own seeded
/// generator (a Fisher-Yates pass, so a run and its replay shuffle alike).
pub(crate) fn shuffle_order(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        if let Some(a) = target_array(rt, b, host) {
            let n = rt.rows(a).len();
            for i in (1..n).rev() {
                let r = rt.random();
                let j = ((r * (i.saturating_add(1)) as f32) as usize).min(i);
                if let Some(arr) = rt.array_mut(a) {
                    arr.rows.swap(i, j);
                }
            }
        }
        out(rt, b, 0);
    }
    Flow::Done
}

pub(crate) const PROTOS: &[Proto] = &[
    Proto {
        guid: proto::ITERATOR_IF,
        name: "Iterator If",
        library: Library::Logics,
        pins: &["Column", "Operator", "Reference Value"],
        pouts: &["Index"],
        flags: flags::TARGET | flags::VAR_POUTS,
        standing: Standing::Full,
        layouts: &[],
        run: iterator_if,
        event: None,
    },
    decl(
        proto::SHUFFLE_ORDER,
        "Shuffle Order",
        &[],
        &[],
        flags::TARGET,
        shuffle_order,
    ),
    decl(
        proto::SET_ROW,
        "Set Row",
        &["Row Index"],
        &[],
        flags::TARGET | flags::VAR_PINS,
        set_row,
    ),
    decl(
        proto::INSERT_COLUMN,
        "Insert Column",
        &["Index", "Column Name", "Column Type", "Parameter Type"],
        &[],
        flags::TARGET,
        insert_column,
    ),
    decl(
        proto::REMOVE_COLUMN,
        "Remove Column",
        &["Index"],
        &[],
        flags::TARGET,
        remove_column,
    ),
    decl(
        proto::MOVE_ROW,
        "Move Row",
        &["Src Row", "Dest Row"],
        &[],
        flags::TARGET,
        move_row,
    ),
    decl(
        proto::SORT_ROWS,
        "Sort Rows",
        &["Sorting Column", "Ascending"],
        &[],
        flags::TARGET,
        sort_rows,
    ),
    decl(
        proto::TEST_CELL,
        "Test Cell",
        &["Row Index", "Column", "Operation", "Reference Value"],
        &[],
        flags::TARGET,
        test_cell,
    ),
    decl(
        proto::GET_HIGHEST,
        "Get Highest",
        &["Column"],
        &["Row Number"],
        flags::TARGET | flags::VAR_POUTS,
        get_highest,
    ),
    decl(
        proto::VALUE_COUNT,
        "Value Count",
        &["Column", "Operator", "Reference Value"],
        &["Count"],
        flags::TARGET,
        value_count,
    ),
    decl(
        proto::REMOVE_KEY_ROW,
        "Remove Key Row",
        &["Reference Value"],
        &[],
        flags::TARGET,
        remove_key_row,
    ),
    decl(
        proto::REMOVE_ROW_IF,
        "Remove Row If",
        &["Column", "Operation", "Reference Value"],
        &["Number Removed"],
        flags::TARGET,
        remove_row_if,
    ),
    decl(
        proto::CHANGE_VALUE_IF,
        "Change Value If",
        &["Column", "Operation", "Reference Value", "New Value"],
        &["Number Changed"],
        flags::TARGET,
        change_value_if,
    ),
];

const fn decl(
    guid: Guid,
    name: &'static str,
    pins: &'static [&'static str],
    pouts: &'static [&'static str],
    flags: u32,
    run: Run,
) -> Proto {
    Proto {
        guid,
        name,
        library: Library::Logics,
        pins,
        pouts,
        flags,
        standing: Standing::Full,
        layouts: &[],
        run,
        event: None,
    }
}

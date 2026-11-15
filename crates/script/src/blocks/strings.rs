// Logics.dll, strings and expressions: Create String, Scan String, Get SubString, Calculator (the C reference,
// src/bb/bb_logics.c, is the oracle for each; the oracle test compares them). Texts are bytes as far as the
// blocks care (the C's), kept as `String` by the runtime: a non-UTF-8 map string has already been read lossily.
// Hostile input: every text is capped at ops::MAX_STRING, indexes are checked, nothing panics.

use super::*;
use crate::calc;
use crate::ops::{self, MAX_STRING};

/// One argument of a printf-style format (the C hands snprintf one).
#[derive(Debug, Clone, Copy)]
pub enum Arg<'a> {
    Int(i32),
    Float(f64),
    Str(&'a str),
}

/// The longest part Create String formats (the C's buffer holds 1023 characters).
const PART: usize = 1023;

fn pad(body: String, left: bool, zero: bool, width: usize, sign_len: usize) -> String {
    let n = body.chars().count();
    if n >= width {
        return body;
    }
    let fill = width.saturating_sub(n);
    if left {
        format!("{body}{}", " ".repeat(fill))
    } else if zero {
        let (sign, digits) = body.split_at(sign_len.min(body.len()));
        format!("{sign}{}{digits}", "0".repeat(fill))
    } else {
        format!("{}{body}", " ".repeat(fill))
    }
}

/// `%e` of a finite number: C writes at least two exponent digits.
fn fmt_e(x: f64, prec: usize, upper: bool) -> String {
    let s = format!("{:.*e}", prec, x.abs());
    let (m, e) = s.split_once('e').unwrap_or((&s, "0"));
    let e: i32 = e.parse().unwrap_or(0);
    let r = format!(
        "{m}e{}{:02}",
        if e < 0 { '-' } else { '+' },
        e.unsigned_abs()
    );
    if upper { r.to_uppercase() } else { r }
}

fn fmt_g_prec(x: f64, prec: usize, alt: bool, upper: bool) -> String {
    let p = if prec == 0 { 1 } else { prec };
    if x == 0.0 {
        return if alt {
            format!("{:.*}", p.saturating_sub(1), 0.0)
        } else {
            "0".into()
        };
    }
    let probe = format!("{:.*e}", p.saturating_sub(1), x.abs());
    let exp: i32 = probe
        .split_once('e')
        .and_then(|(_, e)| e.parse().ok())
        .unwrap_or(0);
    let strip = |s: String| -> String {
        if !alt && s.contains('.') {
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        } else if alt && !s.contains('.') {
            format!("{s}.")
        } else {
            s
        }
    };
    let p_i = i32::try_from(p).unwrap_or(i32::MAX);
    if exp < -4 || exp >= p_i {
        let r = fmt_e(x.abs(), p.saturating_sub(1), upper);
        let (m, e) = r
            .split_once(if upper { 'E' } else { 'e' })
            .unwrap_or((&r, ""));
        format!(
            "{}{}{e}",
            strip(m.to_string()),
            if upper { 'E' } else { 'e' }
        )
    } else {
        let dec = usize::try_from(p_i.saturating_sub(1).saturating_sub(exp)).unwrap_or(0);
        strip(format!("{:.*}", dec, x.abs()))
    }
}

/// C's `snprintf(buf, 1024, fmt, arg)` for one argument: flags `-+ #0`, width, precision, the length modifiers
/// (ignored), the conversions d i u x X o c f F e E g G a A s and `%%`. A conversion after the first finds no
/// argument and writes nothing (undefined in C). The result is cut at 1023 bytes.
pub fn format_one(fmt: &str, arg: Arg<'_>) -> String {
    let bytes = fmt.as_bytes();
    let mut out = String::new();
    let mut i = 0usize;
    let mut used = false;
    while let Some(&c) = bytes.get(i) {
        if out.len() >= PART {
            break;
        }
        if c != b'%' {
            // copy a whole UTF-8 sequence
            let end = fmt
                .get(i..)
                .and_then(|r| r.char_indices().nth(1).map(|(n, _)| i.saturating_add(n)))
                .unwrap_or(bytes.len());
            out.push_str(fmt.get(i..end).unwrap_or(""));
            i = end;
            continue;
        }
        i = i.saturating_add(1);
        let (mut left, mut plus, mut space, mut alt, mut zero) =
            (false, false, false, false, false);
        while let Some(&f) = bytes.get(i) {
            match f {
                b'-' => left = true,
                b'+' => plus = true,
                b' ' => space = true,
                b'#' => alt = true,
                b'0' => zero = true,
                _ => break,
            }
            i = i.saturating_add(1);
        }
        let num = |i: &mut usize| -> Option<usize> {
            let start = *i;
            let mut n = 0usize;
            while let Some(d) = bytes.get(*i).filter(|d| d.is_ascii_digit()) {
                n = n
                    .saturating_mul(10)
                    .saturating_add(usize::from(d.saturating_sub(b'0')))
                    .min(PART);
                *i = i.saturating_add(1);
            }
            (*i > start).then_some(n)
        };
        let mut width = num(&mut i).unwrap_or(0);
        if bytes.get(i) == Some(&b'*') {
            i = i.saturating_add(1);
            width = 0;
        }
        let mut prec = None;
        if bytes.get(i) == Some(&b'.') {
            i = i.saturating_add(1);
            if bytes.get(i) == Some(&b'*') {
                i = i.saturating_add(1);
            } else {
                prec = Some(num(&mut i).unwrap_or(0));
            }
        }
        while matches!(
            bytes.get(i),
            Some(b'h' | b'l' | b'L' | b'q' | b'j' | b'z' | b't')
        ) {
            i = i.saturating_add(1);
        }
        let Some(&conv) = bytes.get(i) else { break };
        i = i.saturating_add(1);
        if conv == b'%' {
            out.push('%');
            continue;
        }
        if used {
            continue;
        }
        used = true;
        let as_f = match arg {
            Arg::Int(v) => f64::from(v),
            Arg::Float(v) => v,
            Arg::Str(_) => 0.0,
        };
        let as_i = match arg {
            Arg::Int(v) => v,
            Arg::Float(v) => v as i32,
            Arg::Str(_) => 0,
        };
        let part = match conv {
            b'd' | b'i' => {
                let mut digits = as_i.unsigned_abs().to_string();
                if let Some(p) = prec {
                    while digits.len() < p {
                        digits.insert(0, '0');
                    }
                    zero = false;
                }
                let sign = if as_i < 0 {
                    "-"
                } else if plus {
                    "+"
                } else if space {
                    " "
                } else {
                    ""
                };
                pad(format!("{sign}{digits}"), left, zero, width, sign.len())
            }
            b'u' | b'x' | b'X' | b'o' => {
                let v = as_i as u32;
                let mut digits = match conv {
                    b'u' => v.to_string(),
                    b'x' => format!("{v:x}"),
                    b'X' => format!("{v:X}"),
                    _ => format!("{v:o}"),
                };
                if let Some(p) = prec {
                    while digits.len() < p {
                        digits.insert(0, '0');
                    }
                    zero = false;
                }
                let prefix = match (alt, conv, v) {
                    (true, b'x', 1..) => "0x",
                    (true, b'X', 1..) => "0X",
                    (true, b'o', 1..) if !digits.starts_with('0') => "0",
                    _ => "",
                };
                pad(format!("{prefix}{digits}"), left, zero, width, prefix.len())
            }
            b'c' => pad(
                char::from(u8::try_from(as_i & 0xff).unwrap_or(b'?')).to_string(),
                left,
                false,
                width,
                0,
            ),
            b'f' | b'F' | b'e' | b'E' | b'g' | b'G' | b'a' | b'A' => {
                let sign = if as_f.is_sign_negative() && !as_f.is_nan() {
                    "-"
                } else if plus {
                    "+"
                } else if space {
                    " "
                } else {
                    ""
                };
                let upper = conv.is_ascii_uppercase();
                let body = if as_f.is_nan() {
                    zero = false;
                    if upper { "NAN" } else { "nan" }.to_string()
                } else if as_f.is_infinite() {
                    zero = false;
                    if upper { "INF" } else { "inf" }.to_string()
                } else {
                    let p = prec.unwrap_or(6);
                    match conv {
                        b'f' | b'F' => {
                            let mut s = format!("{:.*}", p, as_f.abs());
                            if alt && p == 0 {
                                s.push('.');
                            }
                            s
                        }
                        b'e' | b'E' => fmt_e(as_f, p, upper),
                        _ => fmt_g_prec(as_f, p, alt, upper),
                    }
                };
                pad(format!("{sign}{body}"), left, zero, width, sign.len())
            }
            b's' => {
                let text = match arg {
                    Arg::Str(s) => s.to_string(),
                    Arg::Int(v) => v.to_string(),
                    Arg::Float(v) => v.to_string(),
                };
                let text = match prec {
                    Some(p) => text.chars().take(p).collect(),
                    None => text,
                };
                pad(text, left, false, width, 0)
            }
            _ => String::new(),
        };
        out.push_str(&part);
    }
    // the C's buffer
    if out.len() > PART {
        let mut cut = PART;
        while !out.is_char_boundary(cut) {
            cut = cut.saturating_sub(1);
        }
        out.truncate(cut);
    }
    out
}

fn push_capped(s: &mut String, t: &str) {
    let room = MAX_STRING.saturating_sub(s.len());
    if t.len() <= room {
        s.push_str(t);
    } else {
        let mut cut = room;
        while !t.is_char_boundary(cut) {
            cut = cut.saturating_sub(1);
        }
        s.push_str(t.get(..cut).unwrap_or(""));
    }
}

/// The pin's type.
fn pin_guid(rt: &Runtime, b: &Block, i: usize) -> Guid {
    b.pins
        .get(i)
        .and_then(|&p| rt.graph.param(p))
        .map_or(types::NONE, |p| p.guid)
}

pub(crate) fn create_string(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    // Create: Text = the parts joined by the delimiter, each formatted by its local's printf format (empty: the
    // part's text; an Integer or Float source formatted as a number, any other as text, with `%s` unless the
    // format ends in `s`). Add Line: Text += "\n" + the parts (no newline to an empty Text). Clear: Text = ""
    // (a Create or Add Line active with it still runs).
    let delim = pin(rt, b, "Delimiter", 0, host).as_string();
    if active(rt, b, 2) {
        input(rt, b, 2);
        out(rt, b, 2);
        set_pout(rt, b, "Text", 0, Value::Str(String::new()));
        if !active(rt, b, 0) && !active(rt, b, 1) {
            return Flow::Done;
        }
    }
    let mut text = String::new();
    for i in 1..b.pins.len() {
        let Some(&p) = b.pins.get(i) else { break };
        let v = rt.read(p, host);
        let guid = pin_guid(rt, b, i);
        let fmt = match local(rt, b, i.saturating_sub(1)) {
            Value::Str(s) => s,
            _ => String::new(),
        };
        let part = if fmt.is_empty() {
            ops::value_text(rt, guid, &v)
        } else if guid == types::INT {
            format_one(&fmt, Arg::Int(v.as_int()))
        } else if guid == types::FLOAT {
            format_one(&fmt, Arg::Float(f64::from(v.as_float())))
        } else {
            let t = ops::value_text(rt, guid, &v);
            let t: String = t.chars().take(255).collect();
            if fmt.ends_with('s') {
                format_one(&fmt, Arg::Str(&t))
            } else {
                format_one("%s", Arg::Str(&t))
            }
        };
        if i != 1 {
            push_capped(&mut text, &delim);
        }
        push_capped(&mut text, &part);
    }
    if input(rt, b, 0) {
        out(rt, b, 0);
        set_pout(rt, b, "Text", 0, Value::Str(text));
    } else if input(rt, b, 1) {
        out(rt, b, 1);
        let prev = pout_value(rt, b, 0)
            .map(|v| v.as_string())
            .unwrap_or_default();
        let mut r = String::new();
        push_capped(&mut r, &prev);
        if b.locals.is_empty() || !prev.is_empty() {
            push_capped(&mut r, "\n");
        }
        push_capped(&mut r, &text);
        set_pout(rt, b, "Text", 0, Value::Str(r));
    }
    Flow::Done
}

/// The pieces of a text split at a delimiter, empty pieces skipped, the way Scan String walks it: (pieces, the
/// rest after the last delimiter). `limit` stops the walk after that many pieces (the outputs it has).
fn find_sub(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

pub(crate) fn scan_string(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    // Text split at Delimiter (a space when empty); pout 1... the pieces converted to their types (an empty piece
    // between delimiters is skipped; each piece is cut at 255 bytes), pout 0 their count. When the outputs run
    // out the count is not written (the original's behaviour).
    input(rt, b, 0);
    out(rt, b, 0);
    let text = pin(rt, b, "Text", 0, host).as_string();
    let delim = pin(rt, b, "Delimiter", 1, host).as_string();
    let delim = if delim.is_empty() {
        " ".to_string()
    } else {
        delim
    };
    let (text, dl) = (text.as_bytes(), delim.as_bytes());
    let mut count = 0usize;
    let mut cur = 0usize;
    let store = |rt: &mut Runtime, n: usize, piece: &[u8]| {
        let Some(&p) = b.pouts.get(n) else {
            return false;
        };
        if !piece.is_empty() {
            let piece = piece.get(..piece.len().min(255)).unwrap_or_default();
            let guid = rt.graph.param(p).map_or(types::NONE, |q| q.guid);
            let s = String::from_utf8_lossy(piece).into_owned();
            if let Some(v) = ops::text_to_value(rt, guid, s) {
                rt.write(p, v);
            }
        }
        true
    };
    while let Some(rest) = text.get(cur..) {
        let Some(at) = find_sub(rest, dl) else { break };
        let end = cur.saturating_add(at);
        if end != cur {
            count = count.saturating_add(1);
            if !store(rt, count, text.get(cur..end).unwrap_or_default()) {
                break;
            }
        }
        cur = end.saturating_add(dl.len());
    }
    if let Some(rest) = text.get(cur..).filter(|r| !r.is_empty()) {
        count = count.saturating_add(1);
        if !store(rt, count, rest) {
            return Flow::Done;
        }
    }
    set_pout(
        rt,
        b,
        "Elements Found",
        0,
        Value::Int(i32::try_from(count).unwrap_or(i32::MAX)),
    );
    Flow::Done
}

/// The bytes Get SubString returns for a string, a start and a length (0 = to the end), in the original's
/// arithmetic (32-bit, a start below 0 shortens the length).
pub fn substring(s: &[u8], start: i32, len: i32) -> Vec<u8> {
    let n = i32::try_from(s.len()).unwrap_or(i32::MAX);
    let (mut start, mut len) = (start, len);
    if len == 0 || n < len.wrapping_add(start) {
        len = n.wrapping_sub(start);
    }
    if start < 0 {
        len = len.wrapping_add(start);
        start = 0;
    }
    if len > 0 && start < n {
        let (a, l) = (
            usize::try_from(start).unwrap_or(0),
            usize::try_from(len).unwrap_or(0),
        );
        let end = a.saturating_add(l).min(s.len());
        s.get(a..end).unwrap_or_default().to_vec()
    } else {
        Vec::new()
    }
}

pub(crate) fn get_substring(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    input(rt, b, 0);
    out(rt, b, 0);
    let s = pin(rt, b, "String", 0, host).as_string();
    let start = pin(rt, b, "Start", 1, host).as_int();
    let len = pin(rt, b, "Length", 2, host).as_int();
    let part = substring(s.as_bytes(), start, len);
    set_pout(
        rt,
        b,
        "SubString",
        0,
        Value::Str(String::from_utf8_lossy(&part).into_owned()),
    );
    Flow::Done
}

pub(crate) fn calculator(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    // The expression over the variables a to z, which are the input parameters after it (a = pin 1). A syntax
    // error is logged, local 0 becomes "#####" and x stays as it was.
    input(rt, b, 0);
    out(rt, b, 0);
    let src = pin(rt, b, "expression", 0, host).as_string();
    // the variables the expression names, read once each
    let mut vars: Vec<Option<f32>> = vec![None; 26];
    for (k, slot) in vars.iter_mut().enumerate() {
        if let Some(&p) = b.pins.get(k.saturating_add(1)) {
            let v = rt.read(p, host);
            *slot = Some(match v {
                Value::Int(i) => i as f32,
                v => v.as_float(),
            });
        }
    }
    match calc::evaluate(src.as_bytes(), &|i| vars.get(i).copied().flatten()) {
        Some(x) => set_pout(rt, b, "x", 0, Value::Float(x)),
        None => {
            host.note("Syntax Error in CALCULATOR");
            if let Some(&l) = b.locals.first() {
                rt.write(l, Value::Str("#####".into()));
            }
        }
    }
    Flow::Done
}

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

pub(crate) const PROTOS: &[Proto] = &[
    decl(
        proto::CREATE_STRING,
        "Create String",
        &["Delimiter"],
        &["Text"],
        flags::VAR_PINS,
        create_string,
    ),
    decl(
        proto::SCAN_STRING,
        "Scan String",
        &["Text", "Delimiter"],
        &["Elements Found"],
        flags::VAR_POUTS,
        scan_string,
    ),
    decl(
        proto::GET_SUBSTRING,
        "Get SubString",
        &["String", "Start", "Length"],
        &["SubString"],
        0,
        get_substring,
    ),
    decl(
        proto::CALCULATOR,
        "Calculator",
        &["expression"],
        &["x"],
        flags::VAR_PINS,
        calculator,
    ),
];

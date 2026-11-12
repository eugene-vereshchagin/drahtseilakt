// Interface.dll: 2D text and fonts (docs/fonts.md; the findings the old C gave where it is silent are in
// docs/gameplay_bbs.md, "Text"). A block's text is a box in the original's 640x480 frame sent to the host
// (`Host::text_box`, keyed by the block); the engine draws and fits it. Fonts keep only size, weight and colour:
// the engine draws every font in its own face. 3D Text and Movie Player are named no-ops.

use super::*;
use crate::{Font, MAX_TEXT_CHARS, TextSpec, text_align};

/// The frame a script's 2D coordinates live in.
const FRAME: (f32, f32) = (640.0, 480.0);
/// Height in frame pixels of a font at scale 1: the glyph cell of the original's font texture (512 / 16).
const FONT_CELL: f32 = 32.0;
/// The most fonts a run keeps.
const MAX_FONTS: usize = 64;
/// How many pins a Text Display reads for its text (the first is pin 4).
const MAX_TEXT_PINS: usize = 8;

fn clip(text: &str) -> String {
    text.chars().take(MAX_TEXT_CHARS).collect()
}

/// The colour a pin holds; white when the pin has none.
fn color_of(v: &Value) -> [f32; 4] {
    match v {
        Value::Color(c) => *c,
        _ => [1.0; 4],
    }
}

impl Runtime {
    /// The font value (index + 1) of a font made by name: a second font of the same name replaces the first (a
    /// level restart makes its fonts again), and a run holds at most `MAX_FONTS`; 0 is "no font".
    fn make_font(&mut self, name: &str, bold: bool) -> i32 {
        let font = Font {
            name: name.to_string(),
            scale: [1.0; 2],
            color: [1.0; 4],
            bold,
        };
        let at = match self.fonts.iter().position(|f| f.name == name) {
            Some(i) => i,
            None if self.fonts.len() < MAX_FONTS => {
                self.fonts.push(font.clone());
                self.fonts.len() - 1
            }
            None => return 0,
        };
        if let Some(f) = self.fonts.get_mut(at) {
            *f = font;
        }
        i32::try_from(at + 1).unwrap_or(0)
    }

    fn font(&self, value: i32) -> Option<&Font> {
        self.fonts.get(usize::try_from(value).ok()?.checked_sub(1)?)
    }
}

/// TT CreateFontEx: pins Font Name, Font Texture (no texture, no font), ...; output Font. Out at once.
pub(crate) fn create_font_ex(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    if !input(rt, b, 0) {
        return Flow::Done;
    }
    out(rt, b, 0);
    let name = pin(rt, b, "Font Name", 0, host).as_string();
    let tex = pin(rt, b, "Font Texture", 1, host).as_obj();
    if name.is_empty() || tex.is_none() {
        return Flow::Done;
    }
    let i = rt.make_font(&name, false);
    set_pout(rt, b, "Font", 0, Value::Int(i));
    Flow::Done
}

/// Create System Font: pins Font Name, System Font Name, Font Weight (400), ...; output Font Created; Success.
pub(crate) fn create_system_font(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    if !input(rt, b, 0) {
        return Flow::Done;
    }
    let name = pin(rt, b, "Font Name", 0, host).as_string();
    let weight = pin(rt, b, "Font Weight", 2, host).as_int();
    let i = rt.make_font(&name, weight >= 600);
    if i == 0 {
        return Flow::Done; // the original fails without the Error output too
    }
    set_pout(rt, b, "Font Created", 0, Value::Int(i));
    out(rt, b, 0);
    Flow::Done
}

/// Set Font Properties: pins Font, Space, Scale (2), Italic Offset, Color (4), ...; a font that is not there
/// is left alone. Out first.
pub(crate) fn set_font_properties(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    if !input(rt, b, 0) {
        return Flow::Done;
    }
    out(rt, b, 0);
    let font = pin(rt, b, "Font", 0, host).as_int();
    let scale = pin(rt, b, "Scale", 2, host);
    let color = pin(rt, b, "Color", 4, host);
    let Some(f) = usize::try_from(font)
        .ok()
        .and_then(|i| i.checked_sub(1))
        .and_then(|i| rt.fonts.get_mut(i))
    else {
        return Flow::Done;
    };
    if let Value::Vec2(s) = scale {
        f.scale = s;
    }
    if let Value::Color(c) = color {
        f.color = c;
    }
    Flow::Done
}

/// 2D Text: while on, the text of pin Text on the target 2D entity (class 27 to 29), in the font of pin Font,
/// aligned by pin Alignment (Center 0, Left 1, Right 2, Top 4, Bottom 8), inside the margins; the entity's frame is
/// what Edit 2D Entity set (the whole frame when none). On -> Exit On; Off -> Exit Off and
/// the text is gone. Outputs: Line Count.
pub(crate) fn text_2d(rt: &mut Runtime, id: Id, b: &Block, _p: Guid, host: &mut dyn Host) -> Flow {
    if input(rt, b, 1) {
        host.text_box(id, None);
        out(rt, b, 1);
        return Flow::Done;
    }
    if input(rt, b, 0) {
        out(rt, b, 0);
    }
    let t = rt.target(b, host);
    let is_2d = t
        .and_then(|t| rt.graph.objects.get(t))
        .is_none_or(|o| (27..=29).contains(&o.class));
    if !is_2d {
        return Flow::Continue;
    }
    let text = clip(&pin(rt, b, "Text", 1, host).as_string());
    let font = pin(rt, b, "Font", 0, host).as_int();
    let align = pin(rt, b, "Alignment", 2, host).as_int();
    let m = match pin(rt, b, "Margins", 3, host) {
        Value::Rect(r) => r,
        _ => [2.0; 4],
    };
    let off = pin(rt, b, "Offset", 4, host).as_vec2();
    // the entity's frame as Edit 2D Entity left it: values up to 2 are fractions of the 640x480 frame, larger
    // ones frame pixels; what was not edited is the whole frame
    let (fx, fy, fw, fh) = {
        let e = t
            .and_then(|t| rt.ents2d.iter().find(|(k, _)| *k == t))
            .map(|(_, e)| *e)
            .unwrap_or_default();
        let px = |v: f32, whole: f32| if v.abs() <= 2.0 { v * whole } else { v };
        let [x, y] = e.pos.unwrap_or([0.0; 2]);
        let [w, h] = e.size.unwrap_or([FRAME.0, FRAME.1]);
        let (x, y) = (px(x, FRAME.0), px(y, FRAME.1));
        let (w, h) = (px(w, FRAME.0).max(0.0), px(h, FRAME.1).max(0.0));
        (x, y, w, h)
    };
    let (scale, color, bold) = rt
        .font(font)
        .map_or(([0.5; 2], [1.0; 4], false), |f| (f.scale, f.color, f.bold));
    let lines = text.split('\n').count();
    set_pout(
        rt,
        b,
        "Line Count",
        1,
        Value::Int(i32::try_from(lines).unwrap_or(i32::MAX)),
    );
    host.text_box(
        id,
        Some(TextSpec {
            text,
            x: fx + m[0] + off[0],
            y: fy + m[1] + off[1],
            w: fw - m[0] - m[2],
            h: fh - m[1] - m[3],
            size: FONT_CELL * scale[1],
            bold,
            color,
            align: u8::try_from(align & 15).unwrap_or(0),
        }),
    );
    Flow::Continue
}

/// Text Display (a CK sprite text): pins Offset (the sprite's top left in the frame), Color, Alignment (Center 1,
/// Left 2, Right 4, Top 8, Bottom 16, VCenter 32, HCenter 64), Size (points), then Text and any extra pins; the
/// text is every text pin's string followed by a space. The sprite is the local Sprite Size (320x32) large.
/// On -> Exit On and it stays; Off -> Exit Off and it is gone.
pub(crate) fn text_display(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _p: Guid,
    host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 1) {
        host.text_box(id, None);
        out(rt, b, 1);
        return Flow::Done;
    }
    if input(rt, b, 0) {
        out(rt, b, 0);
    }
    let off = pin(rt, b, "Offset", 0, host).as_vec2();
    let color = color_of(&pin(rt, b, "Color", 1, host));
    let align = pin(rt, b, "Alignment", 2, host).as_int();
    let size = pin(rt, b, "Size", 3, host).as_float();
    let mut text = String::new();
    for &p in b.pins.iter().skip(4).take(MAX_TEXT_PINS) {
        text.push_str(&rt.read(p, host).as_string());
        text.push(' ');
    }
    let sprite = match local(rt, b, 4) {
        Value::Vec2(s) if s[0] > 0.0 && s[1] > 0.0 => s,
        _ => [320.0, 32.0],
    };
    let mut a = 0;
    if align & 2 != 0 {
        a |= text_align::LEFT;
    }
    if align & 4 != 0 {
        a |= text_align::RIGHT;
    }
    if align & 8 != 0 {
        a |= text_align::TOP;
    }
    if align & 16 != 0 {
        a |= text_align::BOTTOM;
    }
    host.text_box(
        id,
        Some(TextSpec {
            text: clip(&text),
            x: off[0],
            y: off[1],
            w: sprite[0],
            h: sprite[1],
            // points to pixels at 96 dpi
            size: if size > 0.0 { size * 4.0 / 3.0 } else { 13.0 },
            bold: false,
            color,
            align: a,
        }),
    );
    Flow::Continue
}

/// A block that is known and draws nothing here (3D Text, Movie Player): inputs taken, said once.
pub(crate) fn named_no_op(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let mut any = false;
    for i in 0..b.inputs.len() {
        any |= input(rt, b, i);
    }
    if any {
        rt.report_once(
            proto,
            host,
            &format!("script block does nothing in the player: {}", b.name),
        );
    }
    Flow::Done
}

/// The box goes when the block is reset or its script is deactivated or deleted.
fn text_event(_rt: &mut Runtime, id: Id, _b: &Block, e: BlockEvent, host: &mut dyn Host) {
    if matches!(
        e,
        BlockEvent::Reset | BlockEvent::DeactivateScript | BlockEvent::Delete
    ) {
        host.text_box(id, None);
    }
}

const fn decl(
    guid: Guid,
    name: &'static str,
    library: Library,
    pins: &'static [&'static str],
    standing: Standing,
    run: Run,
    event: Option<OnEvent>,
) -> Proto {
    Proto {
        guid,
        name,
        library,
        pins,
        pouts: &[],
        flags: flags::TARGET,
        standing,
        layouts: &[],
        run,
        event,
    }
}

pub(super) const PROTOS: &[Proto] = &[
    decl(
        proto::TEXT_DISPLAY,
        "Text Display",
        Library::Interface,
        &["Offset", "Color", "Alignment", "Size", "Text"],
        Standing::Full,
        text_display,
        Some(text_event),
    ),
    decl(
        proto::TEXT_2D,
        "2D Text",
        Library::Interface,
        &["Font", "Text", "Alignment", "Margins", "Offset"],
        Standing::Full,
        text_2d,
        Some(text_event),
    ),
    decl(
        proto::SET_FONT_PROPERTIES,
        "Set Font Properties",
        Library::Interface,
        &["Font", "Space", "Scale", "Italic Offset", "Color"],
        Standing::Full,
        set_font_properties,
        None,
    ),
    decl(
        proto::CREATE_SYSTEM_FONT,
        "Create System Font",
        Library::Interface,
        &["Font Name", "System Font Name", "Font Weight"],
        Standing::Full,
        create_system_font,
        None,
    ),
    decl(
        proto::TEXT_3D,
        "3D Text",
        Library::Interface,
        &[],
        Standing::Partial,
        named_no_op,
        None,
    ),
    decl(
        proto::MOVIE_PLAYER,
        "Movie Player",
        Library::Interface,
        &[],
        Standing::Partial,
        named_no_op,
        None,
    ),
    decl(
        proto::TT_CREATE_FONT_EX,
        "TT CreateFontEx",
        Library::TtToolbox,
        &["Font Name", "Font Texture"],
        Standing::Full,
        create_font_ex,
        None,
    ),
];

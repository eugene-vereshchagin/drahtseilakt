// Sounds.dll (wave sounds) and the two TT blocks that drive a sound's volume. A wave sound is a map object
// (class 25) whose file the engine resolves; the blocks only send commands through the host, and the sound's
// "playing" state the Wave Player waits on comes back from the host's own clock (docs/gameplay_bbs.md, "Sounds").

use super::*;
use crate::exec::SoundCmd;

/// The wave sound a block acts on, and where its own pins start: the old layout of these blocks takes the sound
/// as its first pin (a Wave Sound parameter), the game's use the target.
fn sound_of(rt: &mut Runtime, b: &Block, host: &mut dyn Host) -> (Option<Id>, usize) {
    let first = b
        .pins
        .first()
        .and_then(|&p| rt.graph.param(p))
        .map(|p| p.guid);
    let (obj, k) = if first == Some(types::WAVE_SOUND) {
        let p = pin(rt, b, "", 0, host).as_obj();
        (p, 1)
    } else {
        (rt.target(b, host), 0)
    };
    // only a wave sound is one: anything else is ignored, as the original's block with no sound returns
    let obj = obj.filter(|&o| rt.graph.objects.get(o).is_some_and(|i| i.class == 25));
    (obj, k)
}

/// The file the map saved for a wave sound (its object's name when the map saved none).
fn file_of(rt: &Runtime, obj: Id) -> String {
    rt.graph
        .waves
        .iter()
        .find(|w| w.object == obj)
        .map(|w| w.file.clone())
        .filter(|f| !f.is_empty())
        .unwrap_or_else(|| rt.graph.name(obj).to_string())
}

fn float_pin(
    rt: &mut Runtime,
    b: &Block,
    name: &str,
    i: usize,
    default: f32,
    host: &mut dyn Host,
) -> f32 {
    if rt.pin(b, name, i).is_none() {
        return default;
    }
    let v = pin(rt, b, name, i, host).as_float();
    if v.is_finite() { v } else { default }
}

/// Wave Player: inputs Play, Stop, Pause/Resume; outputs Start Playing, End Playing, Paused/Resumed; pins Fade In,
/// Fade Out, Loop. Stays active while the sound plays; a looping Play returns at once.
pub(crate) fn wave_player(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let (Some(obj), k) = sound_of(rt, b, host) else {
        return Flow::Done;
    };
    let file = file_of(rt, obj);
    if input(rt, b, 0) {
        let fade_in = float_pin(rt, b, "Fade In", k, 0.0, host);
        let looping = pin(rt, b, "Loop", k.saturating_add(2), host).as_bool();
        host.sound(obj, &file, SoundCmd::Play { looping, fade_in });
        out(rt, b, 0);
        return if looping { Flow::Done } else { Flow::Continue };
    }
    if active(rt, b, 2) {
        input(rt, b, 2);
        out(rt, b, 2);
        if host.sound_playing(obj) {
            host.sound(obj, &file, SoundCmd::Pause);
            return Flow::Continue;
        }
        host.sound(obj, &file, SoundCmd::Resume);
        return Flow::Done;
    }
    if input(rt, b, 1) {
        let fade_out = float_pin(rt, b, "Fade Out", k.saturating_add(1), 0.0, host);
        host.sound(obj, &file, SoundCmd::Stop { fade_out });
        // the fade is not modelled: the sound ends at once, in the run's clock as in the audio
        out(rt, b, 1);
        return Flow::Done;
    }
    if !host.sound_playing(obj) {
        out(rt, b, 1);
        return Flow::Done;
    }
    Flow::Continue
}

/// A script that goes inactive, is reset or deleted silences its sounds.
pub(crate) fn wave_event(rt: &mut Runtime, _id: Id, b: &Block, e: BlockEvent, host: &mut dyn Host) {
    if matches!(
        e,
        BlockEvent::DeactivateScript | BlockEvent::Delete | BlockEvent::Reset
    ) {
        let (Some(obj), _) = sound_of(rt, b, host) else {
            return;
        };
        let file = file_of(rt, obj);
        host.sound(obj, &file, SoundCmd::Stop { fade_out: 0.0 });
    }
}

/// Volume Control, Pitch Control and Panning Control: one value to the sound (default 1, 1 and 0).
pub(crate) fn sound_control(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    input(rt, b, 0);
    out(rt, b, 0);
    let (Some(obj), k) = sound_of(rt, b, host) else {
        return Flow::Done;
    };
    let file = file_of(rt, obj);
    let (name, default) = match proto {
        proto::VOLUME_CONTROL => ("Gain", 1.0),
        proto::PITCH_CONTROL => ("Pitch", 1.0),
        _ => ("Pan", 0.0),
    };
    let v = float_pin(rt, b, name, k, default, host);
    let cmd = match proto {
        proto::VOLUME_CONTROL => SoundCmd::Gain(v),
        proto::PITCH_CONTROL => SoundCmd::Pitch(v),
        _ => SoundCmd::Pan(v),
    };
    host.sound(obj, &file, cmd);
    Flow::Done
}

/// Play Sound Instance: pins 2D, Object, Position, Direction, Min Delay, Volume. A one-shot copy of the sound; in
/// 3D (pin `2D` off) it sits at the object's position (plus the Position pin in the object's frame), else at the
/// Position pin as a world point.
pub(crate) fn play_sound_instance(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    input(rt, b, 0);
    out(rt, b, 0);
    let (Some(obj), _) = sound_of(rt, b, host) else {
        return Flow::Done;
    };
    let file = file_of(rt, obj);
    let volume = float_pin(rt, b, "Volume", 5, 1.0, host);
    let flat = rt
        .pin(b, "2D", 0)
        .is_none_or(|_| pin(rt, b, "2D", 0, host).as_bool());
    let at = if flat {
        None
    } else {
        let ent = pin(rt, b, "Object", 1, host).as_obj();
        let rel = pin(rt, b, "Position", 2, host).as_vec3();
        match rt.ent(ent).and_then(|e| host.matrix(e)) {
            Some(m) => Some(transform_point(&m, rel)),
            None => Some(rel),
        }
    };
    host.sound(obj, &file, SoundCmd::Instance { volume, at });
    Flow::Done
}

/// The perceptual curve of TT_LinearVolume and TT ProximityVolumeControl: 1 above 1, 0 at or below the cut, else
/// 50^(v-1). The linear block cuts at 0.01, the proximity block at 0 (and its 0 maps to 0.02, not silence).
fn volume_curve(v: f32, cut: f32) -> f32 {
    if v > 1.0 {
        1.0
    } else if v <= cut {
        0.0
    } else {
        (50.0f64.powf(f64::from(v)) * 0.02) as f32
    }
}

pub(crate) fn linear_volume(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    input(rt, b, 0);
    let v = float_pin(rt, b, "Vol. in Decibel", 0, 0.0, host);
    set_pout(
        rt,
        b,
        "Linear Volume",
        0,
        Value::Float(volume_curve(v, 0.01)),
    );
    out(rt, b, 0);
    Flow::Done
}

/// TT ProximityVolumeControl: pins Object 1, Object 2, Near, Far, Sound. Every tick the sound's gain follows how
/// near object 2 is to object 1 (squared distances, linear between Near and Far, then the curve); On starts it,
/// Off ends it after one more update.
pub(crate) fn proximity_volume(
    rt: &mut Runtime,
    id: Id,
    b: &Block,
    _proto: Guid,
    host: &mut dyn Host,
) -> Flow {
    let mut on = matches!(rt.scratch(id), Scratch::Flag(true));
    if input(rt, b, 1) {
        on = false;
    } else if input(rt, b, 0) {
        out(rt, b, 0);
        on = true;
    }
    rt.set_scratch(id, Scratch::Flag(on));
    let o1 = pin(rt, b, "Object 1", 0, host).as_obj();
    let o2 = pin(rt, b, "Object 2", 1, host).as_obj();
    let sound = pin(rt, b, "Sound", 4, host)
        .as_obj()
        .filter(|&o| rt.graph.objects.get(o).is_some_and(|i| i.class == 25));
    let (Some(m1), Some(m2), Some(sound)) = (
        rt.ent(o1).and_then(|e| host.matrix(e)),
        rt.ent(o2).and_then(|e| host.matrix(e)),
        sound,
    ) else {
        return Flow::Done;
    };
    // object 2's position in object 1's frame; only the distance matters: the frame's scale does, so invert it
    let d = [
        m2[3][0] - m1[3][0],
        m2[3][1] - m1[3][1],
        m2[3][2] - m1[3][2],
    ];
    let local = [0, 1, 2].map(|c| {
        let at = |row: &[f32; 4]| row.get(c).copied().unwrap_or(0.0);
        let axis = [at(&m1[0]), at(&m1[1]), at(&m1[2])];
        let s2 = axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2];
        if s2 > 1e-12 {
            (axis[0] * d[0] + axis[1] * d[1] + axis[2] * d[2]) / s2
        } else {
            0.0
        }
    });
    let d2 = local[0] * local[0] + local[1] * local[1] + local[2] * local[2];
    let near = float_pin(rt, b, "Near", 2, 0.0, host);
    let far = float_pin(rt, b, "Far", 3, 0.0, host);
    let (near2, far2) = (near * near, far * far);
    // NaN-preserving: a NaN distance counts as out of range, which `d2 >= far2` would not
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    let g = if !(d2 < far2) {
        0.0
    } else if d2 <= near2 {
        1.0
    } else {
        1.0 - (d2 - near2) / (far2 - near2)
    };
    let file = file_of(rt, sound);
    host.sound(sound, &file, SoundCmd::Gain(volume_curve(g, -1.0)));
    if on {
        Flow::Continue
    } else {
        out(rt, b, 1);
        Flow::Done
    }
}

pub(super) const PROTOS: &[Proto] = &[
    Proto {
        guid: proto::WAVE_PLAYER,
        name: "Wave Player",
        library: Library::Sounds,
        pins: &["Fade In", "Fade Out", "Loop"],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: wave_player,
        event: Some(wave_event),
    },
    Proto {
        guid: proto::VOLUME_CONTROL,
        name: "Volume Control",
        library: Library::Sounds,
        pins: &["Gain"],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: sound_control,
        event: None,
    },
    Proto {
        guid: proto::PITCH_CONTROL,
        name: "Pitch Control",
        library: Library::Sounds,
        pins: &["Pitch"],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: sound_control,
        event: None,
    },
    Proto {
        guid: proto::PANNING_CONTROL,
        name: "Panning Control",
        library: Library::Sounds,
        pins: &["Pan"],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: sound_control,
        event: None,
    },
    Proto {
        guid: proto::PLAY_SOUND_INSTANCE,
        name: "Play Sound Instance",
        library: Library::Sounds,
        pins: &[
            "2D",
            "Object",
            "Position",
            "Direction",
            "Min Delay",
            "Volume",
        ],
        pouts: &[],
        flags: flags::TARGET,
        standing: Standing::Full,
        layouts: &[],
        run: play_sound_instance,
        event: None,
    },
    Proto {
        guid: proto::TT_LINEAR_VOLUME,
        name: "TT_LinearVolume",
        library: Library::TtToolbox,
        pins: &["Vol. in Decibel"],
        pouts: &["Linear Volume"],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: linear_volume,
        event: None,
    },
    Proto {
        guid: proto::TT_PROXIMITY_VOLUME_CONTROL,
        name: "TT ProximityVolumeControl",
        library: Library::TtGravity,
        pins: &["Object 1", "Object 2", "Near", "Far", "Sound"],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: proximity_volume,
        event: None,
    },
];

// Materials.dll: materials and textures.

use super::*;

pub(crate) fn change_texture_video_format(
    rt: &mut Runtime,
    _id: Id,
    b: &Block,
    _proto: Guid,
    _host: &mut dyn Host,
) -> Flow {
    if input(rt, b, 0) {
        out(rt, b, 0);
    }
    Flow::Done
}

pub(super) const PROTOS: &[Proto] = &[
    Proto {
        guid: proto::CHANGE_TEXTURE_SIZE,
        name: "Change Texture Size",
        library: Library::Materials,
        pins: &[],
        pouts: &[],
        flags: 0,
        standing: Standing::Partial,
        layouts: &[],
        run: super::narratives::set_ic,
        event: None,
    },
    Proto {
        guid: proto::SET_TEXTURE,
        name: "Set Texture",
        library: Library::Materials,
        pins: &[],
        pouts: &[],
        flags: 0,
        standing: Standing::Partial,
        layouts: &[],
        run: super::narratives::set_ic,
        event: None,
    },
    Proto {
        guid: proto::SET_DIFFUSE,
        name: "Set Diffuse",
        library: Library::Materials,
        pins: &[],
        pouts: &[],
        flags: 0,
        standing: Standing::Partial,
        layouts: &[],
        run: super::narratives::set_ic,
        event: None,
    },
    Proto {
        guid: proto::SET_EMISSIVE,
        name: "Set Emissive",
        library: Library::Materials,
        pins: &[],
        pouts: &[],
        flags: 0,
        standing: Standing::Partial,
        layouts: &[],
        run: super::narratives::set_ic,
        event: None,
    },
    Proto {
        guid: proto::CHANGE_TEXTURE_VIDEO_FORMAT,
        name: "Change Texture Video Format",
        library: Library::Materials,
        pins: &[],
        pouts: &[],
        flags: 0,
        standing: Standing::Full,
        layouts: &[],
        run: change_texture_video_format,
        event: None,
    },
];

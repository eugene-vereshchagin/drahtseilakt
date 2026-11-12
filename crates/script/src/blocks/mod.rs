// The building blocks the runtime knows, by prototype GUID, and the parameter operations. Each block reads its
// active inputs (clearing them), its pins by name, acts through the host and activates its outputs. The list and
// each block's standing: docs/drahtseilakt/scripts.md. A block met that is not here is reported by the runtime.

use crate::exec::{Ent, Host, Runtime, Scratch};
use crate::{Block, Guid, Id, ObjKind, Value, types};

/// What a block returns: done this tick, stays active for the next, or not known to the runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Done,
    Continue,
    Unknown,
}

pub mod proto {
    use super::Guid;
    pub const OP: Guid = Guid(0x2d5d_6d01, 0x6a35_3eb0);
    pub const OBJECT_DELETE: Guid = Guid(0x7412_0ded, 0x7652_4673);
    pub const CHANGE_TEXTURE_VIDEO_FORMAT: Guid = Guid(0xf56d_92ef, 0x41eb_20c1);
    pub const ACTIVATE_SCRIPT: Guid = Guid(0x4c7e_7bc3, 0x0b69_3155);
    pub const DEACTIVATE_SCRIPT: Guid = Guid(0x1436_7c05, 0x635b_24f9);
    pub const EXECUTE_SCRIPT: Guid = Guid(0x706c_5a40, 0x5bb3_1a0b);
    pub const POINT_PARTICLE_SYSTEM: Guid = Guid(0x506b_40f7, 0x3085_2e46);
    pub const PLANAR_PARTICLE_SYSTEM: Guid = Guid(0x4995_7bfe, 0x0ff2_7ffc);
    pub const SPHERICAL_PARTICLE_SYSTEM: Guid = Guid(0x67c8_8f47, 0x8880_721e);
    pub const TT_SCALEABLE_PROXIMITY: Guid = Guid(0x2a2a_63ca, 0x0082_6175);
    pub const PROXIMITY: Guid = Guid(0x5321_cacb, 0xdcdc_5213);
    pub const GET_CELL: Guid = Guid(0x33b9_9f51, 0x07d9_5c45);
    pub const SET_CELL: Guid = Guid(0x30ed_1c6d, 0x4a3b_7067);
    pub const ADD_ROW: Guid = Guid(0x1c7e_5dc6, 0x3f64_23c2);
    pub const GET_KEY_ROW: Guid = Guid(0x4906_4205, 0x10e7_2f7a);
    pub const GET_ROW: Guid = Guid(0x33b7_7f41, 0x07b9_5c45);
    pub const CLEAR_ARRAY: Guid = Guid(0x35c9_352f, 0x7b1a_193b);
    pub const BINARY_SWITCH: Guid = Guid(0xeb50_6901, 0x984a_fccc);
    pub const WAIT_MESSAGE: Guid = Guid(0x4587_ffee, 0x4587_ffdd);
    pub const SEND_MESSAGE_TO_GROUP: Guid = Guid(0x5f90_6952, 0x6df1_1649);
    pub const SEND_MESSAGE: Guid = Guid(0xa20e_8d5b, 0xdf00_2150);
    pub const BROADCAST_MESSAGE: Guid = Guid(0x3d6c_4ae1, 0x72ae_2cd6);
    pub const SWITCH_ON_MESSAGE: Guid = Guid(0x1bb2_3f1d, 0x17ff_14b9);
    pub const GET_MESSAGE_DATA: Guid = Guid(0x4587_5fee, 0x4587_5fdd);
    pub const GROUP_ITERATOR: Guid = Guid(0x6050_252f, 0x3aa8_2d40);
    pub const COLLECTION_ITERATOR: Guid = Guid(0x4196_02ec, 0x7822_779e);
    pub const ITERATOR: Guid = Guid(0x198f_0af9, 0x0268_249f);
    pub const PHYSICALIZE: Guid = Guid(0x7522_370e, 0x37ec_15ec);
    pub const PHYSICS_IMPULSE: Guid = Guid(0x0c7e_39bb, 0x16db_20d5);
    pub const PHYSICS_WAKEUP: Guid = Guid(0x38b8_51b5, 0x72ca_74ac);
    pub const BOX_BOX_INTERSECTION: Guid = Guid(0x6415_4401, 0x76cf_37af);
    pub const SET_POSITION: Guid = Guid(0xe456_e78a, 0x4567_89aa);
    pub const SET_ORIENTATION: Guid = Guid(0x6258_74aa, 0xaa69_4132);
    pub const SET_WORLD_MATRIX: Guid = Guid(0xaa4a_a6f0, 0xddef_def4);
    pub const ROTATE: Guid = Guid(0xffff_ffee, 0xeeff_ffff);
    pub const RESTORE_IC: Guid = Guid(0x766e_4e44, 0x4fac_6d52);
    pub const TT_RESTORE_IC: Guid = Guid(0x7e0f_2f58, 0x232d_3109);
    pub const SET_IC: Guid = Guid(0x7267_1a24, 0x161a_2347);
    pub const KEEP_ACTIVE: Guid = Guid(0x7160_133a, 0x1f25_32fe);
    pub const HIDE: Guid = Guid(0x31d9_7d82, 0x78d5_4d98);
    pub const SHOW: Guid = Guid(0xa85a_213a, 0xef78_d52a);
    pub const SWITCH_ON_PARAMETER: Guid = Guid(0x4c42_aace, 0x1da4_5635);
    pub const DELAYER: Guid = Guid(0x15d4_72a5, 0x3bea_409f);
    pub const TIMER: Guid = Guid(0xa2a5_a63a, 0xe4e7_e8e5);
    pub const ACTIVATE_OBJECT: Guid = Guid(0x5f00_3fba, 0x5e57_4f60);
    pub const DEACTIVATE_OBJECT: Guid = Guid(0x160f_4b7d, 0x67de_224e);
    pub const OBJECT_COPY: Guid = Guid(0x3f6b_0ac7, 0x47d2_0f78);
    pub const TEST: Guid = Guid(0x17d6_6d26, 0x726b_7dec);
    pub const GET_CURRENT_CAMERA: Guid = Guid(0x368f_22b1, 0x2229_57e4);
    pub const SET_AS_ACTIVE_CAMERA: Guid = Guid(0x368f_0ab1, 0x2d89_57e4);
    pub const SET_CLIPPING_PLANES: Guid = Guid(0x6523_16a2, 0x01aa_09a9);
    pub const SET_FOV: Guid = Guid(0xaaaa_cccc, 0xaaaa_cccc);
    pub const TEXT_DISPLAY: Guid = Guid(0xf22d_010a, 0xf2cd_010a);
    pub const SET_FONT_PROPERTIES: Guid = Guid(0xdacf_bd61, 0x7a6e_65e7);
    pub const TEXT_2D: Guid = Guid(0x055b_29fe, 0x662d_5ca0);
    pub const TEXT_3D: Guid = Guid(0x1f5b_2fe7, 0x6d35_5175);
    pub const CREATE_SYSTEM_FONT: Guid = Guid(0x9363_34fc, 0xf243_684f);
    pub const MOVIE_PLAYER: Guid = Guid(0x778d_16d4, 0x1dd6_0060);
    pub const TT_CREATE_FONT_EX: Guid = Guid(0x260e_4eb0, 0x0e25_6b90);
    pub const NOP: Guid = Guid(0x3025_61c4, 0x0d28_2980);
    pub const WAVE_PLAYER: Guid = Guid(0x5bde_0e45, 0x2e21_07d5);
    pub const VOLUME_CONTROL: Guid = Guid(0x3361_3f21, 0x776b_185b);
    pub const PITCH_CONTROL: Guid = Guid(0x7b7c_731e, 0x175b_27a3);
    pub const PANNING_CONTROL: Guid = Guid(0x6a5f_17af, 0x6348_0864);
    pub const PLAY_SOUND_INSTANCE: Guid = Guid(0x0028_3a35, 0x38ef_6b48);
    pub const TT_LINEAR_VOLUME: Guid = Guid(0x09b3_35b3, 0x12d1_7cdc);
    pub const TT_PROXIMITY_VOLUME_CONTROL: Guid = Guid(0x3857_1b61, 0x64cd_2174);
    pub const IDENTITY: Guid = Guid(0x1515_1652, 0xaeef_ffd5);
    pub const PARAMETER_SELECTOR: Guid = Guid(0x63eb_2b0c, 0x27e2_767e);
    pub const KEY_EVENT: Guid = Guid(0x1af2_274b, 0x6b8c_1524);
    pub const KEY_WAITER: Guid = Guid(0x016d_010b, 0x017d_010b);
    pub const SWITCH_ON_KEY: Guid = Guid(0x1f49_612e, 0x6cd6_3de0);
    pub const MOUSE_WAITER: Guid = Guid(0x014d_010b, 0x015d_010b);
    pub const INPUT_STRING: Guid = Guid(0x693e_7e4b, 0x1001_04c4);
    pub const SHOW_MOUSE_CURSOR: Guid = Guid(0x16f6_368f, 0x506b_60fc);
    pub const MOUSE_CURSOR_SYSTEM: Guid = Guid(0x2483_576d, 0x57c6_6324);
    pub const MOUSE_CURSOR: Guid = Guid(0x140c_3eac, 0x6e50_2a81);
    pub const PER_SECOND: Guid = Guid(0x448e_54ce, 0x75a6_55c5);
    pub const SET_PHYSICS_FORCE: Guid = Guid(0x56e2_0c57, 0x0b92_6068);
    pub const CHANGE_TEXTURE_SIZE: Guid = Guid(0x7a7c_4d7b, 0x468d_0f33);
    pub const SET_TEXTURE: Guid = Guid(0xeb12_3eb5, 0x5be3_21be);
    pub const SET_DIFFUSE: Guid = Guid(0xe1e1_e1e1, 0x1e1e_1e1e);
    pub const SET_EMISSIVE: Guid = Guid(0xd1d1_d1d1, 0x3030_3030);
    pub const SET_SPECULAR: Guid = Guid(0x2020_2020, 0x3030_3030);
    pub const SET_AMBIENT: Guid = Guid(0xaaaa_aaaa, 0xaeae_1287);
    pub const SET_MATERIAL: Guid = Guid(0x3786_5d50, 0x2e28_5fab);
    pub const SET_BLEND_MODES: Guid = Guid(0x2f57_2a66, 0x6a9a_0088);
    pub const SET_MATERIAL_Z_BUFFER: Guid = Guid(0x1144_022f, 0x68fd_055c);
    pub const SET_ALPHA_TEST: Guid = Guid(0x1cb5_661e, 0x5b5d_3fda);
    pub const SET_CURRENT_SLOT: Guid = Guid(0xaaaa_213a, 0xeaa8_d52a);
    pub const GET_TEXTURE_INFO: Guid = Guid(0x250c_4d8d, 0x2889_60d9);
    pub const SET_3DSPRITE_MODE: Guid = Guid(0x0c5d_0457, 0xc64c_7000);
    pub const BLEND_TEXTURES: Guid = Guid(0x53ee_4292, 0x7d93_2bb1);
    pub const WRITE_IN_TEXTURE: Guid = Guid(0x1d57_024d, 0x7c06_4238);
    pub const IS_IN_GROUP: Guid = Guid(0x58a1_210b, 0x14e5_715c);
    pub const COUNTER: Guid = Guid(0x998f_000f, 0xf000_f899);
    pub const BINARY_MEMORY: Guid = Guid(0xd02d_67dd, 0x1021_1fdd);
    pub const GET_DELTA_TIME: Guid = Guid(0x47dc_3232, 0x64bf_203a);
    pub const CHRONO: Guid = Guid(0x5e2f_1788, 0x4f96_5a44);
    pub const WAIT_FOR_ALL: Guid = Guid(0xc044_a999, 0xfdfe_faf7);
    pub const STREAMING_EVENT: Guid = Guid(0x1f0b_52bf, 0x4c33_42dd);
    pub const SEQUENCER: Guid = Guid(0x4253_0844, 0x257b_6053);
    pub const ADD_TO_GROUP: Guid = Guid(0x0002_4125, 0x7854_20ab);
    pub const REMOVE_FROM_GROUP: Guid = Guid(0xd014_7412, 0x0258_520d);
    pub const GROUP_CLEAR: Guid = Guid(0x1d12_662e, 0x4268_7a3c);
    pub const BEZIER_PROGRESSION: Guid = Guid(0x6bb8_699a, 0x29fb_6a4b);
    pub const BEZIER_TRANSFORM: Guid = Guid(0x6d43_3e1b, 0x1919_54b4);
    pub const LINEAR_PROGRESSION: Guid = Guid(0xfff4_5680, 0xaa51_2a39);
    pub const INTERPOLATOR: Guid = Guid(0x3550_3950, 0x2dde_7a65);
    pub const GET_COMPONENT: Guid = Guid(0x2689_057b, 0x1a1a_446a);
    pub const SET_COMPONENT: Guid = Guid(0x6e80_0755, 0x57b6_4acb);
    pub const RANDOM: Guid = Guid(0x0c62_2386, 0x1c30_54f7);
    pub const RANDOM_SWITCH: Guid = Guid(0x79d7_2fde, 0x2e9d_0912);
    pub const SET_EULER_ORIENTATION: Guid = Guid(0x0c49_66d8, 0x6c0c_6d14);
    pub const SCALE: Guid = Guid(0x4123_6987, 0xa54a_87a6);
    pub const TT_TIMER: Guid = Guid(0x6ac6_7901, 0x7d2a_6059);
    pub const REMOVE_ROW: Guid = Guid(0x1fa5_7136, 0x1431_0857);
    pub const ROW_SEARCH: Guid = Guid(0x7886_3443, 0x45af_59b6);
    pub const GET_NEAREST_IN_GROUP: Guid = Guid(0x0852_07eb, 0x5849_50d8);
    pub const HAS_ATTRIBUTE: Guid = Guid(0x25b5_4079, 0x6ff9_0545);
    pub const SET_ATTRIBUTE: Guid = Guid(0x3730_40f2, 0x05e0_1b34);
    pub const REMOVE_ATTRIBUTE: Guid = Guid(0x6b63_40c4, 0x61e9_4a41);
    pub const OBJECTS_WITH_ATTRIBUTE_ITERATOR: Guid = Guid(0x6bc1_494c, 0x0c81_6ad3);
    pub const FILL_GROUP_BY_CLASS: Guid = Guid(0x4445_257b, 0x7001_6c57);
    pub const SET_ROW: Guid = Guid(0x62e8_7901, 0x2df0_07dd);
    pub const INSERT_COLUMN: Guid = Guid(0x05cb_7802, 0x04a6_4a58);
    pub const REMOVE_COLUMN: Guid = Guid(0x3f37_7888, 0x128a_7767);
    pub const MOVE_ROW: Guid = Guid(0x101a_62f4, 0x6e2d_25f6);
    pub const SORT_ROWS: Guid = Guid(0x6f62_3e68, 0x62bb_5a98);
    pub const TEST_CELL: Guid = Guid(0x4e6c_6da8, 0x6369_04fc);
    pub const GET_HIGHEST: Guid = Guid(0x46f7_1e13, 0x13d2_6f61);
    pub const VALUE_COUNT: Guid = Guid(0x5343_77de, 0x75fd_478a);
    pub const REMOVE_KEY_ROW: Guid = Guid(0x0f83_34ea, 0x279a_40cd);
    pub const REMOVE_ROW_IF: Guid = Guid(0x5786_5622, 0x662d_2fee);
    pub const CHANGE_VALUE_IF: Guid = Guid(0x0725_3edb, 0x4d12_37ed);
    pub const CREATE_STRING: Guid = Guid(0x4bcd_2f9d, 0x3826_52e2);
    pub const SCAN_STRING: Guid = Guid(0x4afa_2f11, 0x0348_72e2);
    pub const GET_SUBSTRING: Guid = Guid(0x10b1_6051, 0x2617_3b39);
    pub const CALCULATOR: Guid = Guid(0x4bc2_09b8, 0x5b36_79b0);
    pub const ITERATOR_IF: Guid = Guid(0x6bec_4be6, 0x12d6_4c7c);
    pub const SHUFFLE_ORDER: Guid = Guid(0x270f_177a, 0x4ca2_0023);
    pub const OBJECT_RENAME: Guid = Guid(0x6f07_4ca7, 0x4542_7431);
}

mod absent;
mod arrays;
mod attributes;
mod cameras;
mod collisions;
mod controllers;
pub(crate) mod environment;
mod interface;
mod logics;
mod materials;
mod narratives;
mod physics;
mod sounds;
pub mod strings;
mod transfo3d;
mod tt_particles;
mod tt_toolbox;
mod visuals;

/// The player's building-block libraries (the DLLs in its `BuildingBlocks/`), which the registry's modules
/// mirror, and the blocks found in maps but in none of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Library {
    Logics,
    Narratives,
    Materials,
    Transfo3d,
    Visuals,
    Collisions,
    Cameras,
    Controllers,
    Interface,
    Sounds,
    Lights,
    WorldEnvironments,
    Addons1,
    Physics,
    TtToolbox,
    TtGravity,
    TtParticleSystems,
    TtInterfaceManager,
    TtDatabaseManager,
    /// In none of the player's libraries: the original does nothing with such a block.
    NotInPlayer,
}

impl Library {
    /// The file the original loads the library from.
    pub fn dll(self) -> &'static str {
        match self {
            Library::Logics => "Logics.dll",
            Library::Narratives => "Narratives.dll",
            Library::Materials => "Materials.dll",
            Library::Transfo3d => "3DTransfo.dll",
            Library::Visuals => "Visuals.dll",
            Library::Collisions => "Collisions.dll",
            Library::Cameras => "Cameras.dll",
            Library::Controllers => "Controllers.dll",
            Library::Interface => "Interface.dll",
            Library::Sounds => "Sounds.dll",
            Library::Lights => "Lights.dll",
            Library::WorldEnvironments => "WorldEnvironments.dll",
            Library::Addons1 => "BuildingBlocksAddons1.dll",
            Library::Physics => "physics_RT.dll",
            Library::TtToolbox => "TT_Toolbox_RT.dll",
            Library::TtGravity => "TT_Gravity_RT.dll",
            Library::TtParticleSystems => "TT_ParticleSystems_RT.dll",
            Library::TtInterfaceManager => "TT_InterfaceManager_RT.dll",
            Library::TtDatabaseManager => "TT_DatabaseManager_RT.dll",
            Library::NotInPlayer => "-",
        }
    }
}

/// How far a declared block does what the original's does (docs/drahtseilakt/scripts.md, "Standing").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    Full,
    /// Runs, with a limited effect (the engine service it needs is not there yet).
    Partial,
    /// Known and does nothing, as in the original (a block absent from the player's libraries).
    NoOp,
}

/// Flags of a prototype's declaration: what its interface allows.
pub mod flags {
    /// The block may act on a target parameter of its own instead of the script's owner.
    pub const TARGET: u32 = 1;
    /// Inputs, input parameters or output parameters may be added by the map's author.
    pub const VAR_INPUTS: u32 = 2;
    pub const VAR_PINS: u32 = 4;
    pub const VAR_POUTS: u32 = 8;
    pub const VAR_OUTPUTS: u32 = 16;
}

/// What happens to a block besides its run (the original's behaviour callback messages, CKM_BEHAVIOR*).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockEvent {
    /// The block is put back to its start (its script is reset).
    Reset,
    /// The script holding the block becomes active, or inactive.
    ActivateScript,
    DeactivateScript,
    /// The block's script is deleted.
    Delete,
    /// The block is a fresh copy (Object Copy of its script's owner).
    Copy,
}

/// A block's run: reads its inputs and pins, acts, activates outputs. `proto` is the prototype it runs for
/// (one function may serve several prototypes).
pub type Run = fn(&mut Runtime, Id, &Block, Guid, &mut dyn Host) -> Flow;
pub type OnEvent = fn(&mut Runtime, Id, &Block, BlockEvent, &mut dyn Host);

/// A pin layout a prototype was saved with besides its current one (an older version of the block): chosen when
/// the block's first input parameter is not of `first_pin`'s type.
#[derive(Debug, Clone, Copy)]
pub struct Layout {
    pub first_pin_not: Guid,
    pub pins: &'static [&'static str],
}

/// One building-block prototype: what it is, its interface and its functions.
#[derive(Debug, Clone, Copy)]
pub struct Proto {
    pub guid: Guid,
    pub name: &'static str,
    pub library: Library,
    /// Input and output parameters by index, as the prototype declares them (a block in a file may have more,
    /// for a block with variable parameters, or carry its own names).
    pub pins: &'static [&'static str],
    pub pouts: &'static [&'static str],
    pub flags: u32,
    pub standing: Standing,
    /// Older pin layouts of the same prototype.
    pub layouts: &'static [Layout],
    pub run: Run,
    pub event: Option<OnEvent>,
}

impl Proto {
    /// The pin layout this block was saved with: an older one when its first pin's type says so.
    pub fn layout(&self, rt: &Runtime, b: &Block) -> &'static [&'static str] {
        let first = b
            .pins
            .first()
            .and_then(|&p| rt.graph.param(p))
            .map(|p| p.guid);
        for l in self.layouts {
            if first.is_some_and(|g| g != l.first_pin_not) {
                return l.pins;
            }
        }
        self.pins
    }
}

/// Every declared prototype, sorted by GUID for lookup.
pub fn registry() -> &'static [Proto] {
    static ALL: std::sync::OnceLock<Vec<Proto>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        let mut all: Vec<Proto> = [
            absent::PROTOS,
            arrays::PROTOS,
            attributes::PROTOS,
            cameras::PROTOS,
            collisions::PROTOS,
            controllers::PROTOS,
            environment::PROTOS,
            interface::PROTOS,
            logics::PROTOS,
            materials::PROTOS,
            narratives::PROTOS,
            physics::PROTOS,
            sounds::PROTOS,
            strings::PROTOS,
            transfo3d::PROTOS,
            tt_particles::PROTOS,
            tt_toolbox::PROTOS,
            visuals::PROTOS,
        ]
        .concat();
        all.sort_by_key(|p| (p.guid.0, p.guid.1));
        all
    })
}

/// The declaration of a prototype, if the runtime knows it.
pub fn find(guid: Guid) -> Option<&'static Proto> {
    let all = registry();
    all.binary_search_by_key(&(guid.0, guid.1), |p| (p.guid.0, p.guid.1))
        .ok()
        .and_then(|i| all.get(i))
}

/// Whether the runtime knows this prototype.
pub fn known(guid: Guid) -> bool {
    find(guid).is_some()
}

/// Runs one building block by its declaration; `Unknown` for a prototype not declared.
pub fn run(rt: &mut Runtime, id: Id, b: &Block, proto: Guid, host: &mut dyn Host) -> Flow {
    match find(proto) {
        Some(p) => (p.run)(rt, id, b, proto, host),
        None => Flow::Unknown,
    }
}

/// Tells a block of an event, if its prototype handles it.
pub fn event(rt: &mut Runtime, id: Id, b: &Block, e: BlockEvent, host: &mut dyn Host) {
    if let Some(f) = b.proto.and_then(find).and_then(|p| p.event) {
        f(rt, id, b, e, host);
    }
}

/// Reads and clears input `i`.
pub(crate) fn input(rt: &mut Runtime, b: &Block, i: usize) -> bool {
    let Some(&io) = b.inputs.get(i) else {
        return false;
    };
    let on = rt.io_active(io);
    if on {
        rt.set_io(io, false);
    }
    on
}

/// Whether input `i` is active, leaving it as it is (a block handles only some of its inputs; the others stay
/// active for its next run, as the original's).
fn active(rt: &Runtime, b: &Block, i: usize) -> bool {
    b.inputs.get(i).is_some_and(|&io| rt.io_active(io))
}

/// The value output parameter `i` holds now (a block keeping its count there), `None` without that output.
fn pout_value(rt: &Runtime, b: &Block, i: usize) -> Option<Value> {
    b.pouts.get(i).and_then(|&p| rt.values.get(p)).cloned()
}

fn out(rt: &mut Runtime, b: &Block, i: usize) {
    if let Some(&io) = b.outputs.get(i) {
        rt.set_io(io, true);
    }
}

fn pin(rt: &mut Runtime, b: &Block, name: &str, index: usize, host: &mut dyn Host) -> Value {
    match rt.pin(b, name, index) {
        Some(p) => rt.read(p, host),
        None => Value::None,
    }
}

fn set_pout(rt: &mut Runtime, b: &Block, name: &str, index: usize, v: Value) {
    if let Some(p) = rt.pout(b, name, index) {
        rt.write(p, v);
    }
}

fn local(rt: &Runtime, b: &Block, index: usize) -> Value {
    b.locals
        .get(index)
        .and_then(|&l| rt.values.get(l))
        .cloned()
        .unwrap_or_default()
}

/// The target as an engine entity.
fn target_ent(rt: &mut Runtime, b: &Block, host: &mut dyn Host) -> Option<Ent> {
    let t = rt.target(b, host);
    rt.ent(t)
}

/// A map array by object id: an index into the graph's arrays.
fn array_of(rt: &Runtime, id: Option<Id>) -> Option<usize> {
    match rt.graph.objects.get(id?)?.kind {
        ObjKind::Array(i) => Some(i),
        _ => None,
    }
}

type Mat = [[f32; 4]; 4];

fn mat_mul(a: &Mat, b: &Mat) -> Mat {
    b.map(|bc| {
        let mut out = [0.0f32; 4];
        for (ak, &bk) in a.iter().zip(bc.iter()) {
            for (o, &av) in out.iter_mut().zip(ak.iter()) {
                *o += av * bk;
            }
        }
        out
    })
}

fn transform_dir(m: &Mat, p: [f32; 3]) -> [f32; 3] {
    let mut out = [0.0f32; 3];
    for (col, &pk) in m.iter().take(3).zip(p.iter()) {
        for (o, &c) in out.iter_mut().zip(col.iter()) {
            *o += c * pk;
        }
    }
    out
}

fn transform_point(m: &Mat, p: [f32; 3]) -> [f32; 3] {
    let d = transform_dir(m, p);
    [d[0] + m[3][0], d[1] + m[3][1], d[2] + m[3][2]]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if l > 1e-12 {
        [v[0] / l, v[1] / l, v[2] / l]
    } else {
        v
    }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn scale_of(m: &Mat) -> [f32; 4] {
    m.map(|c| (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt())
}

/// Rotation by `angle` about `axis` (a unit vector), as a matrix.
fn rotation(axis: [f32; 3], angle: f32) -> Mat {
    let (s, c) = angle.sin_cos();
    let t = 1.0 - c;
    let [x, y, z] = axis;
    [
        [t * x * x + c, t * x * y + s * z, t * x * z - s * y, 0.0],
        [t * x * y - s * z, t * y * y + c, t * y * z + s * x, 0.0],
        [t * x * z + s * y, t * y * z - s * x, t * z * z + c, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ]
}

/// The referential's matrix, or the world's.
fn referential(
    rt: &mut Runtime,
    b: &Block,
    name: &str,
    index: usize,
    host: &mut dyn Host,
) -> Option<Mat> {
    let r = pin(rt, b, name, index, host).as_obj();
    let e = rt.ent(r)?;
    host.matrix(e)
}

fn distance_sq(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
}

fn ent_name(rt: &Runtime, e: Option<Id>) -> String {
    e.map(|i| rt.graph.name(i).to_string()).unwrap_or_default()
}

// The open format reads each accessor's bytes into the level's own arrays; a level.gltf whose meshes all name the same
// large accessor would turn one buffer into many copies of it. What is read stays within the buffer's size (found by
// review while fuzzing the open format; synthetic levels, no game data needed).

#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects
)]

use serde_json::json;

/// A level of `meshes` meshes, all on the same accessors of `vertices` vertices.
fn shared(meshes: usize, vertices: usize) -> (Vec<u8>, Vec<u8>) {
    let floats = vertices * 3 * 4;
    let uvs = vertices * 2 * 4;
    let bin = vec![0u8; floats * 2 + uvs + 12];
    let mesh = json!({"primitives": [{"attributes": {"POSITION": 0, "NORMAL": 1, "TEXCOORD_0": 2}, "indices": 3}]});
    let doc = json!({
        "asset": {"version": "2.0"},
        "scenes": [{"extras": {"drahtseilakt": {"format": 1}}}],
        "buffers": [{"uri": "level.bin", "byteLength": bin.len()}],
        "bufferViews": [
            {"buffer": 0, "byteOffset": 0, "byteLength": floats},
            {"buffer": 0, "byteOffset": floats, "byteLength": floats},
            {"buffer": 0, "byteOffset": floats * 2, "byteLength": uvs},
            {"buffer": 0, "byteOffset": floats * 2 + uvs, "byteLength": 12},
        ],
        "accessors": [
            {"bufferView": 0, "componentType": 5126, "type": "VEC3", "count": vertices},
            {"bufferView": 1, "componentType": 5126, "type": "VEC3", "count": vertices},
            {"bufferView": 2, "componentType": 5126, "type": "VEC2", "count": vertices},
            {"bufferView": 3, "componentType": 5125, "type": "SCALAR", "count": 3},
        ],
        "meshes": vec![mesh; meshes],
    });
    (serde_json::to_vec(&doc).unwrap(), bin)
}

#[test]
fn meshes_sharing_an_accessor_read_no_more_than_the_buffer() {
    let dir = std::env::temp_dir();
    // a few meshes on one accessor read fine (glTF allows sharing)
    let (gltf, bin) = shared(2, 1000);
    assert_eq!(
        level::open::import_bytes(&dir, &gltf, &bin)
            .unwrap()
            .meshes
            .len(),
        2
    );
    // ten thousand meshes on a 100 000-vertex accessor would be 32 GB of vertices out of a 3 MB buffer
    let (gltf, bin) = shared(10_000, 100_000);
    assert!(level::open::import_bytes(&dir, &gltf, &bin).is_err());
}

#![allow(clippy::expect_used)]

use super::*;

#[test]
fn fuzz_decode_world_map_snapshot() {
    crate::decoder_fuzz_support::codec(
        "decode_world_map_snapshot",
        |rng| {
            let origin = position(100, 200, -7);
            let mut value = tile(origin);
            value.ground_speed = rng.below(u64::from(MAX_GROUND_SPEED) + 1) as u16;
            WorldMapViewSnapshot {
                header: header(origin),
                tiles: vec![value],
            }
        },
        decode_world_map_snapshot,
        |v| encode_world_map_snapshot(v).expect("map"),
    );
}

#[test]
fn fuzz_decode_world_map_delta() {
    let view = header(position(100, 200, -7));
    crate::decoder_fuzz_support::codec(
        "decode_world_map_delta",
        |rng| WorldMapViewDelta {
            header: view,
            tiles: vec![tile(position(100 + rng.below(5) as i32, 200, -7))],
            cleared: vec![],
        },
        |b| decode_world_map_delta(&view, b),
        |v| encode_world_map_delta(&view, v).expect("delta"),
    );
}

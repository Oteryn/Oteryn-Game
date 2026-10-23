#![allow(clippy::expect_used)]

use super::*;

#[test]
fn fuzz_decode_use_intent() {
    crate::decoder_fuzz_support::codec(
        "decode_use_intent",
        |rng| WorldObjectTarget {
            placement: {
                let len = rng.below(64) as usize;
                rng.bytes(len)
            },
            expected_revision: rng.u64(),
        },
        decode_use_intent,
        |v| encode_use_intent(v).expect("target"),
    );
}

#[test]
fn fuzz_decode_use_result() {
    crate::decoder_fuzz_support::codec(
        "decode_use_result",
        |rng| {
            *rng.pick(&[
                UseDisposition::Committed,
                UseDisposition::Rejected,
                UseDisposition::BedWoken,
                UseDisposition::OfflineTraining,
            ])
        },
        decode_use_result,
        |v| encode_use_result(*v).expect("result"),
    );
}

#[test]
fn fuzz_decode_use_result_with_gate() {
    crate::decoder_fuzz_support::codec(
        "decode_use_result_with_gate",
        |rng| {
            (
                if rng.bool() {
                    UseDisposition::Committed
                } else {
                    UseDisposition::Rejected
                },
                None,
            )
        },
        decode_use_result_with_gate,
        |v| encode_use_result_with_gate(v.0, v.1).expect("result"),
    );
}

#[test]
fn fuzz_decode_world_object_overlay_delta() {
    crate::decoder_fuzz_support::codec(
        "decode_world_object_overlay_delta",
        |rng| WorldObjectOverlayEntry {
            content_generation: [1; 32],
            placement: {
                let len = rng.below(32) as usize;
                rng.bytes(len)
            },
            state: {
                let len = rng.below(32) as usize;
                rng.bytes(len)
            },
            revision: rng.u64(),
        },
        decode_world_object_overlay_delta,
        |v| encode_world_object_overlay_delta(v).expect("entry"),
    );
}

#[test]
fn fuzz_decode_world_object_overlay_snapshot() {
    crate::decoder_fuzz_support::codec(
        "decode_world_object_overlay_snapshot",
        |rng| {
            (0..rng.below(5))
                .map(|i| WorldObjectOverlayEntry {
                    content_generation: [1; 32],
                    placement: vec![i as u8],
                    state: vec![rng.u64() as u8],
                    revision: rng.u64(),
                })
                .collect::<Vec<_>>()
        },
        decode_world_object_overlay_snapshot,
        |v| encode_world_object_overlay_snapshot(v).expect("snapshot"),
    );
}

#[test]
fn fuzz_decode_use_intent_target() {
    let selections: &[&[u32]] = &[
        &[],
        &[4],
        &[4, 15],
        &[4, 15, 11],
        &[4, 15, 22],
        &[4, 15, 11, 22],
    ];
    for selected in selections {
        crate::decoder_fuzz_support::codec(
            "decode_use_intent_target",
            |rng| {
                let item = selected.contains(&4) && rng.bool();
                UseIntent {
                    target: if item {
                        UseTarget::Item(ItemHandle::new(rng.u64().max(1)).expect("handle"))
                    } else {
                        UseTarget::WorldObject(WorldObjectTarget {
                            placement: vec![1],
                            expected_revision: rng.u64(),
                        })
                    },
                    use_with: None,
                    use_with_map_item: None,
                    use_with_position: if item && selected.contains(&22) && rng.bool() {
                        Some(SpellTargetPosition {
                            x: rng.u64() as i32,
                            y: rng.u64() as i32,
                            floor: rng.u64() as i16,
                        })
                    } else {
                        None
                    },
                }
            },
            |b| decode_use_intent_target(selected, b),
            |v| encode_item_use_intent(v).expect("intent"),
        );
    }
}

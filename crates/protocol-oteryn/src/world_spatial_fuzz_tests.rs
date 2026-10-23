#![allow(clippy::expect_used)]

use super::*;

#[test]
fn fuzz_decode_step_intent() {
    crate::decoder_fuzz_support::codec(
        "decode_step_intent",
        |rng| {
            *rng.pick(&[
                StepDirection::North,
                StepDirection::East,
                StepDirection::South,
                StepDirection::West,
            ])
        },
        decode_step_intent,
        |v| encode_step_intent(*v),
    );
}

#[test]
fn fuzz_decode_step_result() {
    crate::decoder_fuzz_support::codec(
        "decode_step_result",
        |rng| {
            *rng.pick(&[
                StepDisposition::Moved,
                StepDisposition::Blocked,
                StepDisposition::Rejected,
            ])
        },
        decode_step_result,
        |v| encode_step_result(*v),
    );
}

#[test]
fn fuzz_decode_world_spatial() {
    crate::decoder_fuzz_support::codec(
        "decode_world_spatial",
        |rng| WorldSpatialObservation {
            content_generation: [0xab; 32],
            actor_position: ActorPosition {
                x: rng.u64() as i32,
                y: rng.u64() as i32,
                floor: rng.below(16) as i16,
            },
        },
        decode_world_spatial,
        encode_world_spatial,
    );
}

#[test]
fn fuzz_paced_step_decoders() {
    for paced in [false, true] {
        crate::decoder_fuzz_support::codec(
            "decode_step_result_paced",
            |rng| {
                if paced && rng.bool() {
                    StepDisposition::TooEarly
                } else {
                    StepDisposition::Moved
                }
            },
            |b| decode_step_result_paced(b, paced),
            |v| encode_step_result(*v),
        );
        crate::decoder_fuzz_support::codec(
            "decode_step_result_receipt",
            |rng| StepResult {
                disposition: StepDisposition::Moved,
                step_wait_ms: if paced {
                    (rng.below(2000) + 1) as u32 * 50
                } else {
                    0
                },
            },
            |b| decode_step_result_receipt(b, paced),
            |v| encode_step_result_paced(*v, paced).expect("receipt"),
        );
    }
}

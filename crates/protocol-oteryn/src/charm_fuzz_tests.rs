#![allow(clippy::expect_used)]

use super::*;

#[test]
fn fuzz_decode_charm_view() {
    crate::decoder_fuzz_support::codec(
        "decode_charm_view",
        |rng| CharmView {
            charms: vec![CharmState {
                charm: NonZeroU32::MIN,
                kind: if rng.bool() {
                    CharmKind::Major
                } else {
                    CharmKind::Minor
                },
                unlocked_stage: 1,
                assigned_race: rng.bool().then_some(NonZeroU32::MIN),
                next_stage_cost: (rng.below(100000) + 1) as u32,
                effect_active: rng.bool(),
            }],
            charm_points_available: rng.below(1000001) as u32,
            minor_charm_echoes_available: rng.below(1000001) as u32,
            assignment_slot_limit: None,
        },
        decode_charm_view,
        |v| encode_charm_view(v).expect("view"),
    );
}

#[test]
fn fuzz_decode_charm_unlock_stage_intent() {
    crate::decoder_fuzz_support::codec(
        "decode_charm_unlock_stage_intent",
        |rng| CharmUnlockStageIntent {
            charm: NonZeroU32::new((rng.below(32) + 1) as u32).expect("charm"),
            expected_stage: rng.below(3) as u8,
        },
        decode_charm_unlock_stage_intent,
        |v| encode_charm_unlock_stage_intent(v).expect("intent"),
    );
}

#[test]
fn fuzz_decode_charm_assign_intent() {
    crate::decoder_fuzz_support::codec(
        "decode_charm_assign_intent",
        |rng| CharmAssignIntent {
            charm: NonZeroU32::new((rng.below(32) + 1) as u32).expect("charm"),
            race: NonZeroU32::new((rng.below(1024) + 1) as u32).expect("race"),
        },
        decode_charm_assign_intent,
        |v| encode_charm_assign_intent(v).expect("intent"),
    );
}

#[test]
fn fuzz_decode_charm_unlock_stage_result() {
    crate::decoder_fuzz_support::codec(
        "decode_charm_unlock_stage_result",
        |rng| {
            *rng.pick(&[
                CharmUnlockDisposition::Unlocked,
                CharmUnlockDisposition::StageMismatch,
                CharmUnlockDisposition::Rejected,
            ])
        },
        decode_charm_unlock_stage_result,
        |v| encode_charm_unlock_stage_result(*v),
    );
}

#[test]
fn fuzz_decode_charm_assign_result() {
    crate::decoder_fuzz_support::codec(
        "decode_charm_assign_result",
        |rng| {
            *rng.pick(&[
                CharmAssignDisposition::Assigned,
                CharmAssignDisposition::Rejected,
                CharmAssignDisposition::AssignmentSlotsFull,
            ])
        },
        decode_charm_assign_result,
        |v| encode_charm_assign_result(*v),
    );
}

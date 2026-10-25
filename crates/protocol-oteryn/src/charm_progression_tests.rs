#![allow(clippy::expect_used)]
use super::*;
use crate::charm::{CharmKind, CharmState, MAX_CHARMS};

fn index(value: u32) -> NonZeroU32 {
    NonZeroU32::new(value).expect("non-zero")
}

fn actions() -> [CharmProgressionAction; 4] {
    [
        CharmProgressionAction::Query,
        CharmProgressionAction::Unlock {
            charm: index(MAX_CHARM),
            expected_stage: MAX_CHARM_STAGE - 1,
        },
        CharmProgressionAction::Assign {
            charm: index(1),
            race: index(MAX_BESTIARY_RACE),
        },
        CharmProgressionAction::Remove { charm: index(7) },
    ]
}

#[test]
fn charm_progression_request_round_trips_and_refuses_contradictions() {
    for action in actions() {
        let bytes = encode_charm_progression_request(&action).expect("request");
        assert!(bytes.len() <= MAX_REQUEST_BYTES);
        assert_eq!(decode_charm_progression_request(&bytes), Ok(action));
        // A prefix may be another valid action (proto3 omits a zero field), never this one.
        for end in 0..bytes.len() {
            assert_ne!(decode_charm_progression_request(&bytes[..end]), Ok(action));
        }
        let mut duplicate = bytes.clone();
        duplicate.extend_from_slice(&bytes[..2]);
        assert!(decode_charm_progression_request(&duplicate).is_err());
        let mut unknown = bytes;
        unknown.extend_from_slice(&[0x28, 1]);
        assert!(decode_charm_progression_request(&unknown).is_err());
    }
    for refused in [
        &[][..],
        &[0x08, 0x05],
        &[0x08, 0x01, 0x10, 0x01],
        &[0x08, 0x02],
        &[0x08, 0x02, 0x10, 0x01, 0x18, 0x01],
        &[0x08, 0x02, 0x10, 0x01, 0x20, MAX_CHARM_STAGE],
        &[0x08, 0x03, 0x10, 0x01],
        &[0x08, 0x03, 0x10, 0x01, 0x18, 0x01, 0x20, 0x01],
        &[0x08, 0x04, 0x10, 0x01, 0x18, 0x01],
        &[0x08, 0x04, 0x10, 0x21],
    ] {
        assert!(
            decode_charm_progression_request(refused).is_err(),
            "{refused:?}"
        );
    }
    assert_eq!(
        encode_charm_progression_request(&CharmProgressionAction::Remove {
            charm: index(MAX_CHARM + 1)
        }),
        Err(CyclopediaWireError::LimitExceeded)
    );
    assert_eq!(
        decode_charm_progression_request(&[0; MAX_REQUEST_BYTES + 1]),
        Err(CyclopediaWireError::LimitExceeded)
    );
}

fn largest_views() -> (Vec<BestiaryRaceProgress>, CharmView) {
    let bestiary = (1..=crate::bestiary::MAX_BESTIARY_VIEW_ENTRIES as u32)
        .map(|race| BestiaryRaceProgress {
            race: index(race),
            kill_count: crate::bestiary::MAX_BESTIARY_KILL_THRESHOLD,
            kill_thresholds: [
                crate::bestiary::MAX_BESTIARY_KILL_THRESHOLD - 2,
                crate::bestiary::MAX_BESTIARY_KILL_THRESHOLD - 1,
                crate::bestiary::MAX_BESTIARY_KILL_THRESHOLD,
            ],
        })
        .collect();
    let charms = CharmView {
        charms: (1..=MAX_CHARMS as u32)
            .map(|charm| CharmState {
                charm: index(charm),
                kind: CharmKind::Minor,
                unlocked_stage: 2,
                assigned_race: Some(index(MAX_BESTIARY_RACE)),
                next_stage_cost: crate::charm::MAX_CHARM_STAGE_COST,
                effect_active: true,
            })
            .collect(),
        charm_points_available: crate::charm::MAX_CHARM_BALANCE,
        minor_charm_echoes_available: crate::charm::MAX_CHARM_BALANCE,
        assignment_slot_limit: Some(index(MAX_CHARMS as u32)),
    };
    (bestiary, charms)
}

#[test]
fn charm_progression_reply_round_trips_every_outcome_within_its_bound() {
    let (bestiary, charms) = largest_views();
    let mut outcomes = vec![CharmProgressionOutcome::Query];
    for code in 1..=9 {
        for action in 2..=4 {
            if let Ok(outcome) = CharmProgressionOutcome::from_codes(action, code) {
                outcomes.push(outcome);
            }
        }
    }
    assert_eq!(outcomes.len(), 1 + 6 + 9 + 6);
    for outcome in outcomes {
        let removed = outcome == CharmProgressionOutcome::Remove(CharmRemoveDisposition::Removed);
        let reply = CharmProgressionReply {
            outcome,
            fee_gold: if removed { i64::MAX as u64 } else { 0 },
            bestiary: bestiary.clone(),
            charms: charms.clone(),
        };
        let bytes = encode_charm_progression_reply(&reply).expect("reply");
        assert!(bytes.len() <= MAX_REPLY_BYTES, "{}", bytes.len());
        assert_eq!(decode_charm_progression_reply(&bytes), Ok(reply.clone()));
        let wrong_fee = CharmProgressionReply {
            fee_gold: if removed { 0 } else { 100 },
            ..reply
        };
        assert_eq!(
            encode_charm_progression_reply(&wrong_fee),
            Err(CyclopediaWireError::Malformed)
        );
    }
}

#[test]
fn charm_progression_reply_refuses_malformed_payloads() {
    let reply = CharmProgressionReply {
        outcome: CharmProgressionOutcome::Remove(CharmRemoveDisposition::Removed),
        fee_gold: 800,
        bestiary: Vec::new(),
        charms: CharmView::default(),
    };
    let bytes = encode_charm_progression_reply(&reply).expect("reply");
    assert_eq!(decode_charm_progression_reply(&bytes), Ok(reply));
    for end in 0..bytes.len() {
        assert!(decode_charm_progression_reply(&bytes[..end]).is_err());
    }
    for refused in [
        &[0x08, 0x01, 0x10, 0x02, 0x22, 0x00, 0x2a, 0x00][..],
        &[0x08, 0x02, 0x10, 0x07, 0x22, 0x00, 0x2a, 0x00],
        &[0x08, 0x04, 0x10, 0x01, 0x18, 0x00, 0x22, 0x00, 0x2a, 0x00],
        &[0x08, 0x01, 0x10, 0x01, 0x22, 0x00],
        &[0x08, 0x01, 0x10, 0x01, 0x22, 0x00, 0x2a, 0x00, 0x30, 0x01],
    ] {
        assert!(
            decode_charm_progression_reply(refused).is_err(),
            "{refused:?}"
        );
    }
    assert_eq!(
        decode_charm_progression_reply(&[0; MAX_REPLY_BYTES + 1]),
        Err(CyclopediaWireError::LimitExceeded)
    );
}

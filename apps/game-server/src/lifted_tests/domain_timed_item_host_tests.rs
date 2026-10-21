// Tests lifted from `crate::domain::timed_item_host` (crate oteryn-gameplay-domain): they name higher-layer
// game-server modules, so they run here against the re-exported path.
#![allow(clippy::expect_used, clippy::panic)]
#[allow(unused_imports)]
mod parent_imports {
    pub use crate::domain::timed_item::{
        ActorTimedLanes, AmbiguousLookup, LaneStep, LaneWrite, LaneWriteOutcome,
        TIMEDITEM0B_RL_04_LIVE_ITEMS_PER_ACTOR, TimedItemError, TimedItemLane, TimedValues,
        ambiguous_bound_exceeded,
    };
    pub use std::collections::{BTreeMap, BTreeSet};
}
use crate::content::{
    ReferenceItemField, ReferenceItemSemantics, ReferenceItemTarget, ReferenceTemporalMode,
    ReferenceTransformKind,
};
use crate::domain::timed_item_host::*;
#[allow(unused_imports)]
use parent_imports::*;
type TimedItemDefinition = crate::domain::timed_item_host::TimedItemDefinition<ReferenceItemTarget>;
type HostWrite = crate::domain::timed_item_host::HostWrite<ReferenceItemTarget>;
type HostedItem = crate::domain::timed_item_host::HostedItem<ReferenceItemTarget>;
type TimedItemHost = crate::domain::timed_item_host::TimedItemHost<ReferenceItemTarget>;
use crate::content::{ReferenceItemCharges, ReferenceItemTemporal, ReferenceMilliseconds};
use crate::domain::timed_item::{
    ExpireReason, LaneWriteKind, TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS,
    TIMEDITEM0B_RL_02_AMBIGUOUS_BOUND_MS,
};

const RING_MS: u64 = 1_200_000;

fn target(key: &str) -> ReferenceItemTarget {
    ReferenceItemTarget::new(key, "rev-1").expect("target")
}

fn ring_semantics(decay: Option<&str>) -> ReferenceItemSemantics {
    ReferenceItemSemantics {
        temporal: ReferenceItemField::Known(ReferenceItemTemporal {
            consumption_mode: ReferenceItemField::Known(
                ReferenceTemporalMode::AuthoritativeActiveTimeBudget,
            ),
            duration: ReferenceItemField::Known(ReferenceMilliseconds(RING_MS)),
            stop_duration: ReferenceItemField::Known(true),
            decay_target: decay.map_or(ReferenceItemField::NotApplicable, |key| {
                ReferenceItemField::Known(target(key))
            }),
        }),
        ..ReferenceItemSemantics::default()
    }
}

fn ring() -> TimedItemDefinition {
    resolve_timed_definition(
        &target("oteryn:item.tibia.i3052"),
        &ring_semantics(Some("oteryn:item.tibia.i3999")),
    )
    .expect("resolve")
    .expect("timed")
}

fn item(seed: u8) -> HostedItem {
    HostedItem {
        item_instance_id: [seed; 16],
        definition: ring(),
        row: None,
    }
}

fn charged(seed: u8, charges: u32) -> HostedItem {
    let semantics = ReferenceItemSemantics {
        charges: ReferenceItemField::Known(ReferenceItemCharges {
            count: ReferenceItemField::Known(charges),
        }),
        ..ReferenceItemSemantics::default()
    };
    HostedItem {
        item_instance_id: [seed; 16],
        definition: resolve_timed_definition(&target("oteryn:item.tibia.i28552"), &semantics)
            .expect("resolve")
            .expect("timed"),
        row: None,
    }
}

fn host() -> TimedItemHost {
    TimedItemHost::new(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS).expect("host")
}

fn untimed(_: &ReferenceItemTarget) -> Option<TimedItemDefinition> {
    None
}

const SPARE_MS: u64 = 600_000;

/// Every decay target resolves as a timed spare ring of `SPARE_MS`.
fn spare(target: &ReferenceItemTarget) -> Option<TimedItemDefinition> {
    Some(TimedItemDefinition {
        definition: target.clone(),
        full: TimedValues::new(None, Some(SPARE_MS)).expect("v"),
        decay_target: None,
    })
}

fn values(remaining_ms: u64) -> TimedValues {
    TimedValues::new(None, Some(remaining_ms)).expect("v")
}

/// A host whose ring (row at revision 3, 1 s left) has its expiry in flight at 1 s, with a
/// second ring live beside it.
fn expiring() -> TimedItemHost {
    let mut host = host();
    host.host_live(
        vec![
            HostedItem {
                row: Some((3, Some(values(1_000)))),
                ..item(1)
            },
            item(2),
        ],
        0,
    )
    .expect("login");
    host.advance(1_000);
    let expiry = host.due_writes(1_000, tx, spare);
    assert_eq!(expiry.len(), 1);
    assert!(matches!(expiry[0].write.kind, LaneWriteKind::Expire { .. }));
    host
}

fn commit(host: &mut TimedItemHost, seed: u8) {
    host.on_outcome(&[seed; 16], LaneWriteOutcome::Committed, 0)
        .expect("committed");
}

fn write_for(writes: &[HostWrite], seed: u8) -> LaneWrite {
    writes
        .iter()
        .find(|write| write.item_instance_id == [seed; 16])
        .expect("a write for the item")
        .write
}

fn tx() -> [u8; 16] {
    [7; 16]
}

#[test]
fn the_resolver_reads_the_active_time_budget_charges_and_decay_target() {
    let ring = ring();
    assert_eq!(ring.full.remaining_ms(), Some(RING_MS));
    assert_eq!(ring.full.charges(), None);
    assert_eq!(ring.decay_target, Some(target("oteryn:item.tibia.i3999")));
    // No timed field, a durable deadline (RT-1c) and a conflict.
    let plain = target("oteryn:item.tibia.i2854");
    assert_eq!(
        resolve_timed_definition(&plain, &ReferenceItemSemantics::default()),
        Ok(None)
    );
    let mut deadline = ring_semantics(None);
    if let ReferenceItemField::Known(temporal) = &mut deadline.temporal {
        temporal.consumption_mode =
            ReferenceItemField::Known(ReferenceTemporalMode::DurableAbsoluteDeadline);
    }
    assert_eq!(resolve_timed_definition(&plain, &deadline), Ok(None));
    let conflict = ReferenceItemSemantics {
        charges: ReferenceItemField::Conflict,
        ..ReferenceItemSemantics::default()
    };
    assert_eq!(
        resolve_timed_definition(&plain, &conflict),
        Err(TimedHostError::InvalidDefinition)
    );
    // A charges count of 0 is not a timed definition's full value.
    assert_eq!(
        resolve_timed_definition(
            &plain,
            &ReferenceItemSemantics {
                charges: ReferenceItemField::Known(ReferenceItemCharges {
                    count: ReferenceItemField::Known(0),
                }),
                ..ReferenceItemSemantics::default()
            }
        ),
        Err(TimedHostError::InvalidDefinition)
    );
}

fn decay_transform(key: &str) -> ReferenceItemField<crate::content::ReferenceItemUseTransform> {
    ReferenceItemField::Known(crate::content::ReferenceItemUseTransform {
        targets: (1..=10)
            .map(|wire| {
                let kind = ReferenceTransformKind::from_wire(wire).expect("kind");
                crate::content::ReferenceTransformTarget {
                    kind,
                    target: if kind == ReferenceTransformKind::Decay {
                        ReferenceItemField::Known(target(key))
                    } else {
                        ReferenceItemField::NotApplicable
                    },
                }
            })
            .collect(),
    })
}

#[test]
fn the_decay_transform_is_the_expiry_target_of_time_and_charges() {
    let plain = target("oteryn:item.tibia.i28552");
    // A charge-only item with `transform {trigger: decay}` transforms at expiry.
    let charge_only = ReferenceItemSemantics {
        charges: ReferenceItemField::Known(ReferenceItemCharges {
            count: ReferenceItemField::Known(3),
        }),
        use_transform: decay_transform("oteryn:item.tibia.i28553"),
        ..ReferenceItemSemantics::default()
    };
    let resolved = resolve_timed_definition(&plain, &charge_only)
        .expect("resolve")
        .expect("timed");
    assert_eq!(
        resolved.decay_target,
        Some(target("oteryn:item.tibia.i28553"))
    );
    // The same target from both sources resolves; different targets conflict.
    let same = ReferenceItemSemantics {
        use_transform: decay_transform("oteryn:item.tibia.i3999"),
        ..ring_semantics(Some("oteryn:item.tibia.i3999"))
    };
    assert_eq!(
        resolve_timed_definition(&plain, &same)
            .expect("resolve")
            .expect("timed")
            .decay_target,
        Some(target("oteryn:item.tibia.i3999"))
    );
    let different = ReferenceItemSemantics {
        use_transform: decay_transform("oteryn:item.tibia.i3998"),
        ..ring_semantics(Some("oteryn:item.tibia.i3999"))
    };
    assert_eq!(
        resolve_timed_definition(&plain, &different),
        Err(TimedHostError::InvalidDefinition)
    );
    let conflict = ReferenceItemSemantics {
        use_transform: ReferenceItemField::Conflict,
        ..charge_only
    };
    assert_eq!(
        resolve_timed_definition(&plain, &conflict),
        Err(TimedHostError::InvalidDefinition)
    );
}

#[test]
fn stale_fences_on_one_lane_stop_every_lane_of_the_actor() {
    let mut host = host();
    host.host_live(vec![item(1), item(2)], 0).expect("login");
    host.bind_exercise(charged(3, 2), 0).expect("exercise");
    host.advance(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS);
    assert!(host.spend_exercise_charge());
    let now = TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS;
    assert_eq!(host.due_writes(now, tx, untimed).len(), 3);
    let stored = TimedValues::new(None, Some(RING_MS)).expect("v");
    host.on_unexpected_revision(&[1; 16], false, 3, stored, true)
        .expect("stale");
    // No lane writes again: every lane is released and the exercise binding with it.
    host.advance(RING_MS);
    assert!(host.due_writes(now + RING_MS, tx, untimed).is_empty());
    assert_eq!(host.live_count(), 0);
    assert!(host.all_empty());
    assert!(!host.spend_exercise_charge());
}

#[test]
fn stale_fences_stop_a_committed_expiry_from_continuing_as_its_timed_target() {
    let mut host = expiring();
    host.advance(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS);
    let now = 1_000 + TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS;
    assert_eq!(host.due_writes(now, tx, spare).len(), 1);
    commit(&mut host, 1);
    // The other lane finds the fences stale before the expired lane is processed.
    host.on_unexpected_revision(&[2; 16], false, 3, values(RING_MS), true)
        .expect("stale");
    assert!(host.due_writes(now, tx, spare).is_empty());
    assert_eq!(host.live_count(), 0);
    assert!(host.all_empty());
}

#[test]
fn a_stop_before_the_expiry_commits_stops_the_continuing_target() {
    let mut host = expiring();
    host.stop_all();
    let stops = host.due_writes(1_000, tx, spare);
    assert_eq!(write_for(&stops, 2).expected_revision, 0);
    commit(&mut host, 2);
    commit(&mut host, 1);
    // The drain waits for the committed expiry to be continued or released.
    assert!(!host.all_empty());
    // The target continues stopped at its full values: nothing to write, and it is released.
    assert!(host.due_writes(1_000, tx, spare).is_empty());
    assert!(!host.all_empty());
    assert!(host.due_writes(1_000, tx, spare).is_empty());
    assert!(host.all_empty());
    assert_eq!(host.live_count(), 0);
}

#[test]
fn a_stop_after_the_expiry_commits_stops_the_target_at_its_debited_time() {
    let mut host = expiring();
    commit(&mut host, 1);
    // §8: the target is live from the commit; the clock stops with the stop.
    host.advance(500);
    host.stop_all();
    host.advance(700);
    assert!(!host.all_empty());
    // The ring beside it stops too; the target continues stopping at revision 4.
    let first = host.due_writes(2_200, tx, spare);
    assert_eq!(first.len(), 1);
    commit(&mut host, 2);
    let stop = host.due_writes(2_200, tx, spare);
    assert_eq!(
        write_for(&stop, 1),
        LaneWrite {
            kind: LaneWriteKind::Checkpoint {
                values: values(SPARE_MS - 500)
            },
            expected_revision: 4,
            transaction_id: tx(),
        }
    );
    assert_eq!(
        stop[0].definition.definition,
        target("oteryn:item.tibia.i3999")
    );
    commit(&mut host, 1);
    assert!(host.due_writes(2_200, tx, spare).is_empty());
    assert!(host.all_empty());
    assert_eq!(host.live_count(), 0);
    // A reload hosts the target from its row.
    host.host_live(
        vec![HostedItem {
            item_instance_id: [1; 16],
            definition: spare(&target("oteryn:item.tibia.i3999")).expect("timed"),
            row: Some((5, Some(values(SPARE_MS - 500)))),
        }],
        3_000,
    )
    .expect("reload");
    assert_eq!(host.live_count(), 1);
}

#[test]
fn a_delayed_continuation_debits_the_time_since_the_commit() {
    let mut host = expiring();
    commit(&mut host, 1);
    host.advance(300);
    assert!(host.due_writes(1_300, tx, spare).is_empty());
    host.stop_all();
    let stops = host.due_writes(1_300, tx, spare);
    assert_eq!(
        write_for(&stops, 1).kind,
        LaneWriteKind::Checkpoint {
            values: values(SPARE_MS - 300)
        }
    );

    // Time past the target's full duration expires it at once, at revision 4.
    let mut host = expiring();
    commit(&mut host, 1);
    host.advance(SPARE_MS);
    assert!(
        host.due_writes(1_000 + SPARE_MS, tx, spare)
            .iter()
            .all(|write| write.item_instance_id != [1; 16])
    );
    let expiry = host.due_writes(1_000 + SPARE_MS, tx, spare);
    assert_eq!(
        write_for(&expiry, 1),
        LaneWrite {
            kind: LaneWriteKind::Expire {
                reason: ExpireReason::TimeExhausted
            },
            expected_revision: 4,
            transaction_id: tx(),
        }
    );
}

#[test]
fn rl_01_host_cadence_at_max_and_max_plus_one() {
    assert!(TimedItemHost::new(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS).is_ok());
    assert_eq!(
        TimedItemHost::new(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS + 1),
        Err(TimedHostError::Lane(TimedItemError::InvalidValues))
    );
    // An item hosted at login checkpoints within 60 s, so a crash returns at most that.
    let mut host = host();
    host.host_live(vec![item(1)], 0).expect("login");
    host.advance(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS - 1);
    assert!(
        host.due_writes(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS - 1, tx, untimed)
            .is_empty()
    );
    host.advance(1);
    let writes = host.due_writes(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS, tx, untimed);
    assert_eq!(writes.len(), 1);
    assert_eq!(writes[0].write.expected_revision, 0);
    assert!(matches!(
        writes[0].write.kind,
        LaneWriteKind::Checkpoint { values } if values.remaining_ms()
            == Some(RING_MS - TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS)
    ));
}

#[test]
fn rl_02_ambiguous_outcome_at_max_and_max_plus_one() {
    for (elapsed, held) in [
        (TIMEDITEM0B_RL_02_AMBIGUOUS_BOUND_MS, false),
        (TIMEDITEM0B_RL_02_AMBIGUOUS_BOUND_MS + 1, true),
    ] {
        let mut host = host();
        host.host_live(vec![item(1)], 0).expect("login");
        host.advance(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS);
        let now = TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS;
        assert_eq!(host.due_writes(now, tx, untimed).len(), 1);
        host.on_outcome(&[1; 16], LaneWriteOutcome::Ambiguous, now)
            .expect("ambiguous");
        host.on_lookup(&[1; 16], None, now + elapsed)
            .expect("lookup");
        if held {
            // Held: the item is inactive and nothing more is written.
            host.advance(1);
            assert!(host.due_writes(now + elapsed, tx, untimed).is_empty());
            assert!(!host.all_empty());
        } else {
            // Still open: the record is found and the lane runs on.
            host.on_lookup(
                &[1; 16],
                Some(AmbiguousLookup::FoundSameTransaction),
                now + elapsed,
            )
            .expect("found");
            assert!(host.all_empty());
        }
    }
}

#[test]
fn rl_04_live_items_at_max_and_max_plus_one() {
    let mut host = host();
    let slots: Vec<_> = (1..=10).map(item).collect();
    host.host_live(slots, 0).expect("ten slots");
    host.bind_exercise(charged(11, 500), 0).expect("exercise");
    assert_eq!(host.live_count(), TIMEDITEM0B_RL_04_LIVE_ITEMS_PER_ACTOR);
    // A 12th is refused before any lane opens, at login and at the exercise binding.
    let mut full = host.clone();
    assert_eq!(
        full.host_live(vec![item(12)], 0),
        Err(TimedHostError::LiveItemLimit)
    );
    let mut fresh = TimedItemHost::new(1_000).expect("host");
    assert_eq!(
        fresh.host_live((1..=12).map(item).collect(), 0),
        Err(TimedHostError::LiveItemLimit)
    );
    assert_eq!(fresh.live_count(), 0);
    // A second exercise binding is refused; spent rows are never live.
    let mut one = TimedItemHost::new(1_000).expect("host");
    one.bind_exercise(charged(1, 5), 0).expect("exercise");
    assert_eq!(
        one.bind_exercise(charged(2, 5), 0),
        Err(TimedHostError::ExerciseBinding)
    );
    let spent = HostedItem {
        row: Some((3, None)),
        ..item(3)
    };
    one.host_live(vec![spent], 0).expect("skipped");
    assert_eq!(one.live_count(), 1);
}

#[test]
fn rl_05_a_write_in_flight_is_never_sent_twice() {
    let mut host = host();
    host.host_live(vec![item(1)], 0).expect("login");
    host.advance(TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS);
    let now = TIMEDITEM0B_RL_01_CHECKPOINT_INTERVAL_MS;
    let checkpoint = host.due_writes(now, tx, untimed);
    assert_eq!(checkpoint.len(), 1);
    // The clock runs out while the checkpoint is in flight: the expiry waits for it.
    host.advance(RING_MS);
    assert!(host.due_writes(now + RING_MS, tx, untimed).is_empty());
    host.on_outcome(&[1; 16], LaneWriteOutcome::Committed, now + RING_MS)
        .expect("committed");
    let expiry = host.due_writes(now + RING_MS, tx, untimed);
    assert_eq!(expiry.len(), 1);
    assert_eq!(expiry[0].write.expected_revision, 1);
    assert_eq!(
        expiry[0].write.kind,
        LaneWriteKind::Expire {
            reason: ExpireReason::TimeExhausted
        }
    );
    // A known-not-committed outcome hands the same write out again, once.
    host.on_outcome(&[1; 16], LaneWriteOutcome::NotCommitted, now + RING_MS)
        .expect("retry");
    let retry = host.due_writes(now + RING_MS, tx, untimed);
    assert_eq!(retry.len(), 1);
    assert_eq!(retry[0].write, expiry[0].write);
    assert!(host.due_writes(now + RING_MS, tx, untimed).is_empty());
    host.on_outcome(&[1; 16], LaneWriteOutcome::Committed, now + RING_MS)
        .expect("expired");
    // The expired lane is released.
    assert!(host.due_writes(now + RING_MS, tx, untimed).is_empty());
    assert_eq!(host.live_count(), 0);
}

#[test]
fn an_expiry_into_a_timed_target_keeps_the_item_live_at_the_next_revision() {
    const SPARE_MS: u64 = 600_000;
    let spare = |target: &ReferenceItemTarget| {
        (target == &self::target("oteryn:item.tibia.i3999")).then(|| TimedItemDefinition {
            definition: target.clone(),
            full: TimedValues::new(None, Some(SPARE_MS)).expect("v"),
            decay_target: None,
        })
    };
    let mut host = host();
    host.host_live(
        vec![HostedItem {
            row: Some((3, Some(TimedValues::new(None, Some(1_000)).expect("v")))),
            ..item(1)
        }],
        0,
    )
    .expect("login");
    host.advance(1_000);
    let expiry = host.due_writes(1_000, tx, spare);
    assert_eq!(expiry[0].write.expected_revision, 3);
    host.on_outcome(&[1; 16], LaneWriteOutcome::Committed, 1_000)
        .expect("expired");
    // §8: the same item stays live as the target, at revision 4 from the target's full values.
    assert!(host.due_writes(1_000, tx, spare).is_empty());
    assert_eq!(host.live_count(), 1);
    host.advance(SPARE_MS - 1);
    host.stop_all();
    let stop = host.due_writes(SPARE_MS, tx, spare);
    assert_eq!(
        stop[0].definition.definition,
        target("oteryn:item.tibia.i3999")
    );
    assert_eq!(stop[0].definition.decay_target, None);
    assert_eq!(
        stop[0].write.kind,
        LaneWriteKind::Checkpoint {
            values: TimedValues::new(None, Some(1)).expect("v")
        }
    );
    assert_eq!(stop[0].write.expected_revision, 4);

    // A target that is not timed leaves a spent row: the lane is released.
    let mut host = self::host();
    host.host_live(
        vec![HostedItem {
            row: Some((3, Some(TimedValues::new(None, Some(1_000)).expect("v")))),
            ..item(1)
        }],
        0,
    )
    .expect("login");
    host.advance(1_000);
    assert_eq!(host.due_writes(1_000, tx, untimed).len(), 1);
    host.on_outcome(&[1; 16], LaneWriteOutcome::Committed, 1_000)
        .expect("expired");
    assert!(host.due_writes(1_000, tx, untimed).is_empty());
    assert_eq!(host.live_count(), 0);
}

#[test]
fn the_exercise_binding_spends_charges_and_expires() {
    let mut host = host();
    assert!(!host.spend_exercise_charge());
    host.bind_exercise(charged(1, 2), 0).expect("exercise");
    assert!(host.spend_exercise_charge());
    assert!(host.spend_exercise_charge());
    assert!(!host.spend_exercise_charge());
    let expiry = host.due_writes(0, tx, untimed);
    assert_eq!(
        expiry[0].write.kind,
        LaneWriteKind::Expire {
            reason: ExpireReason::ChargesExhausted
        }
    );
    assert_eq!(expiry[0].definition.decay_target, None);
    host.on_outcome(&[1; 16], LaneWriteOutcome::Committed, 0)
        .expect("expired");
    assert!(host.due_writes(0, tx, untimed).is_empty());
    // The binding is released with its lane.
    host.bind_exercise(charged(2, 2), 0).expect("rebind");
    assert!(host.spend_exercise_charge());
    host.unbind_exercise().expect("unbind");
    assert_eq!(host.due_writes(1, tx, untimed).len(), 1);
}

#[test]
fn logout_waits_for_every_lane_and_stores_the_live_values() {
    let mut host = host();
    host.host_live(
        vec![
            item(1),
            HostedItem {
                row: Some((4, Some(TimedValues::new(None, Some(5_000)).expect("v")))),
                ..item(2)
            },
        ],
        0,
    )
    .expect("login");
    host.advance(1_000);
    host.stop_all();
    assert!(!host.all_empty());
    let stops = host.due_writes(1_000, tx, untimed);
    assert_eq!(stops.len(), 2);
    assert_eq!(
        stops[1].write.kind,
        LaneWriteKind::Checkpoint {
            values: TimedValues::new(None, Some(4_000)).expect("v")
        }
    );
    assert_eq!(stops[1].write.expected_revision, 4);
    host.on_outcome(&[1; 16], LaneWriteOutcome::Committed, 1_000)
        .expect("one");
    assert!(!host.all_empty());
    host.on_outcome(&[2; 16], LaneWriteOutcome::Committed, 1_000)
        .expect("two");
    // The next pass completes both stops and releases the lanes.
    assert!(host.due_writes(1_000, tx, untimed).is_empty());
    assert!(host.all_empty());
    assert_eq!(host.live_count(), 0);
}

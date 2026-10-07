#![allow(
    clippy::expect_used,
    clippy::panic,
    reason = "test fixtures fail loudly on an impossible setup"
)]

use std::cell::{Cell, RefCell};

use oteryn_simulation_determinism::ExactI64;

use super::*;
use crate::combat::{
    COMBAT01_LOOT_PLAN_ENTRIES_MAX, LootDefinitionRef, LootSelectionAlgorithm, LootTableDefinition,
    LootTableEntry,
};
use crate::foundation::CharacterId;
use crate::foundation::{ChannelId, CombatDeathFixture, ScopeOwnershipGeneration, WorldId};

fn id(seed: u8) -> [u8; 16] {
    [
        seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
    ]
}

fn session(seed: u8) -> GameSessionId {
    GameSessionId::decode(&id(seed)).expect("session")
}

/// A loot table whose every one of `drops` entries always drops: `drops` planned MINTs.
fn row(drops: usize) -> Arc<CreatureRewardRow> {
    let item = |key: String| LootDefinitionRef::new("Item", key, "definition-r1");
    Arc::new(CreatureRewardRow {
        xp_amount: ExactI64::new(5),
        corpse_item: item("oteryn:item.corpse".into()),
        loot_table_ref: LootDefinitionRef::new("Loot", "oteryn:loot.test", "definition-r1"),
        loot_table: LootTableDefinition {
            algorithm: LootSelectionAlgorithm::IndependentBernoulliPpm,
            entries: (0..drops)
                .map(|index| LootTableEntry {
                    item: item(format!("oteryn:item.drop-{index}")),
                    min_count: 1,
                    max_count: 1,
                    probability_ppm: Some(1_000_000),
                })
                .collect(),
        },
        race: None,
    })
}

/// One projected death on its own Channel (`channel` keys it), principal `session_seed` at
/// lease generation `lease`.
fn pending(channel: u8, session_seed: u8, lease: u64, drops: usize) -> PendingKillSettlement {
    let mut fixture = CombatDeathFixture::new(
        WorldId::decode(&id(1)).expect("world"),
        ChannelId::decode(&id(channel)).expect("channel"),
        ScopeOwnershipGeneration::new(1).expect("generation"),
    )
    .expect("fixture");
    fixture
        .strike(
            "fixture:kill-reward-tests.strike",
            CombatDeathFixture::HEALTH,
        )
        .expect("lethal strike");
    fixture.project_death().expect("projected death");
    let actor = fixture.actor();
    let principal = CapturedRewardPrincipal {
        character: CharacterId::decode(&id(41)).expect("character"),
        lease_generation: lease,
        session: session(session_seed),
        actor,
    };
    let facts = capture_projected_death_facts(
        &mut fixture.borrow_combat_death(),
        actor,
        principal,
        None,
        1_000,
    )
    .expect("facts");
    PendingKillSettlement {
        facts,
        creature: "fixture:rat".into(),
        row: row(drops),
        ground: DeathGroundContext {
            map_revision: "map-1".into(),
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
            native_room_placement_context: b"room".to_vec(),
        },
        no_progression_logged: false,
    }
}

fn taken(outcome: TakeOutcome) -> TakenKillSettlement {
    match outcome {
        TakeOutcome::Taken(taken) => *taken,
        TakeOutcome::Empty => panic!("expected an entry, found none"),
        TakeOutcome::CapacityWait => panic!("expected an entry, found capacity_wait"),
    }
}

#[test]
fn a_death_is_queued_once() {
    let queue = KillSettlementQueue::default();
    assert_eq!(queue.append(pending(2, 50, 3, 0)), AppendOutcome::Queued);
    assert_eq!(queue.append(pending(2, 50, 3, 0)), AppendOutcome::Duplicate);
    let in_flight = taken(queue.take(session(50), 3));
    // In flight still deduplicates.
    assert_eq!(queue.append(pending(2, 50, 3, 0)), AppendOutcome::Duplicate);
    in_flight.finish();
    assert_eq!(queue.counts(), (0, 0, 0));
}

#[test]
fn killrw_rl_01_bounds_queued_plus_in_flight() {
    let queue = KillSettlementQueue::default();
    let in_flight = {
        assert_eq!(queue.append(pending(2, 50, 3, 0)), AppendOutcome::Queued);
        taken(queue.take(session(50), 3))
    };
    for channel in 3..(2 + KILLRW_RL_01_UNSETTLED_ENTRIES_PER_CHANNEL_MAX as u8) {
        assert_eq!(
            queue.append(pending(channel, 50, 3, 0)),
            AppendOutcome::Queued
        );
    }
    assert_eq!(queue.counts(), (63, 1, 0));
    // The 65th unsettled death is refused while one is in flight.
    assert_eq!(queue.append(pending(200, 50, 3, 0)), AppendOutcome::Full);
    in_flight.finish();
    assert_eq!(queue.append(pending(200, 50, 3, 0)), AppendOutcome::Queued);
}

#[test]
fn take_reserves_planned_loot_mints_and_waits_for_capacity() {
    let queue = KillSettlementQueue::default();
    for channel in 2..7 {
        queue.append(pending(channel, 50, 3, COMBAT01_LOOT_PLAN_ENTRIES_MAX));
    }
    let held: Vec<_> = (0..4).map(|_| taken(queue.take(session(50), 3))).collect();
    assert_eq!(held[3].inflight_before(), 48);
    assert_eq!(queue.counts(), (1, 4, 64));
    assert!(matches!(
        queue.take(session(50), 3),
        TakeOutcome::CapacityWait
    ));
    // Finishing one gives its reservation back.
    let mut held = held.into_iter();
    held.next().expect("held").finish();
    assert_eq!(queue.counts(), (1, 3, 48));
    let fifth = taken(queue.take(session(50), 3));
    assert_eq!(fifth.inflight_before(), 48);
}

#[test]
fn a_dropped_or_requeued_entry_returns_to_the_front_and_frees_its_reservation() {
    let queue = KillSettlementQueue::default();
    queue.append(pending(2, 50, 3, 2));
    queue.append(pending(3, 50, 3, 1));
    let first = taken(queue.take(session(50), 3));
    let death = first.entry().expect("entry").facts.death;
    assert_eq!(queue.counts(), (1, 1, 2));
    first.requeue();
    assert_eq!(queue.counts(), (2, 0, 0));
    let again = taken(queue.take(session(50), 3));
    assert_eq!(again.entry().expect("entry").facts.death, death);
    drop(again);
    assert_eq!(queue.counts(), (2, 0, 0));
}

#[test]
fn take_matches_session_and_lease_generation() {
    let queue = KillSettlementQueue::default();
    queue.append(pending(2, 50, 3, 0));
    queue.append(pending(3, 51, 3, 0));
    assert!(queue.has_session(session(51)));
    assert!(!queue.has_session(session(52)));
    assert!(matches!(queue.take(session(50), 4), TakeOutcome::Empty));
    assert!(matches!(queue.take(session(52), 3), TakeOutcome::Empty));
    let other = taken(queue.take(session(51), 3));
    assert_eq!(
        other.entry().expect("entry").facts.principal.session,
        session(51)
    );
}

/// Asserts no `attack` guard is held while it settles, and replays scripted reports.
struct ScriptedSettle<'a> {
    attack: &'a AsyncMutex<ChannelAttackStates>,
    reports: RefCell<VecDeque<SettleReport>>,
    calls: Cell<usize>,
}

impl KillSettle for ScriptedSettle<'_> {
    async fn settle(&self, entry: &PendingKillSettlement, _: usize) -> SettleReport {
        assert!(
            self.attack.try_lock().is_ok(),
            "a channel guard is held across the settle"
        );
        assert_eq!(entry.creature, "fixture:rat");
        self.calls.set(self.calls.get() + 1);
        self.reports
            .borrow_mut()
            .pop_front()
            .expect("scripted report")
    }
}

fn report(verdict: SettleVerdict, no_progression: bool) -> SettleReport {
    SettleReport {
        verdict,
        no_progression,
    }
}

fn run(test: impl Future<Output = ()>) {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("runtime")
        .block_on(test);
}

#[test]
fn the_drain_settles_with_no_guard_held_and_stops_on_an_unknown_outcome() {
    run(async {
        let attack = AsyncMutex::new(ChannelAttackStates::default());
        for channel in 2..5 {
            attack
                .lock()
                .await
                .kills()
                .append(pending(channel, 50, 3, 1));
        }
        attack.lock().await.kills().append(pending(5, 51, 3, 1));
        let settler = ScriptedSettle {
            attack: &attack,
            reports: RefCell::new(VecDeque::from([
                report(SettleVerdict::Final, true),
                report(SettleVerdict::Retry, true),
            ])),
            calls: Cell::new(0),
        };
        assert_eq!(
            drain_session_kills(&attack, session(50), 3, &settler).await,
            DrainOutcome::Deferred
        );
        assert_eq!(settler.calls.get(), 2);
        // One settled; the unknown one is requeued with its reservation given back.
        assert_eq!(attack.lock().await.kills().counts(), (3, 0, 0));
        // The requeued entry keeps its logged flag, so `no_progression_binding` is logged once.
        let requeued = taken(attack.lock().await.kills().take(session(50), 3));
        assert!(requeued.entry().expect("entry").no_progression_logged);
        requeued.requeue();

        settler.reports.borrow_mut().extend([
            report(SettleVerdict::Final, true),
            report(SettleVerdict::Final, false),
        ]);
        assert_eq!(
            drain_session_kills(&attack, session(50), 3, &settler).await,
            DrainOutcome::Drained
        );
        assert_eq!(settler.calls.get(), 4);
        // The other session's entry is untouched.
        assert_eq!(attack.lock().await.kills().counts(), (1, 0, 0));
    });
}

#[test]
fn the_drain_defers_on_capacity_wait() {
    run(async {
        let attack = AsyncMutex::new(ChannelAttackStates::default());
        let blocker = {
            let states = attack.lock().await;
            for channel in 2..7 {
                states
                    .kills()
                    .append(pending(channel, 50, 3, COMBAT01_LOOT_PLAN_ENTRIES_MAX));
            }
            (0..4)
                .map(|_| taken(states.kills().take(session(50), 3)))
                .collect::<Vec<_>>()
        };
        let settler = ScriptedSettle {
            attack: &attack,
            reports: RefCell::new(VecDeque::new()),
            calls: Cell::new(0),
        };
        assert_eq!(
            drain_session_kills(&attack, session(50), 3, &settler).await,
            DrainOutcome::Deferred
        );
        assert_eq!(settler.calls.get(), 0);
        drop(blocker);
    });
}

#[test]
fn an_unknown_reward_table_row_is_no_loot_binding() {
    let mut table = CreatureRewardTable::default();
    assert_eq!(
        table.row("fixture:rat").err(),
        Some(crate::content::creature_reward::NoSettlementReason::NoLootBinding)
    );
    table.insert(
        "fixture:rat",
        Err(crate::content::creature_reward::NoSettlementReason::LootTableMissing),
    );
    assert_eq!(
        table.row("fixture:rat").err().map(|reason| reason.as_str()),
        Some("loot_table_missing")
    );
}

fn final_settles(attack: &AsyncMutex<ChannelAttackStates>, count: usize) -> ScriptedSettle<'_> {
    ScriptedSettle {
        attack,
        reports: RefCell::new(
            (0..count)
                .map(|_| report(SettleVerdict::Final, false))
                .collect(),
        ),
        calls: Cell::new(0),
    }
}

fn mark(outcome: KillRelease) -> KillReleaseMark {
    match outcome {
        KillRelease::Send(Some(mark)) => mark,
        other => panic!("expected a committing mark, found {other:?}"),
    }
}

#[test]
fn the_seal_finds_an_append_after_the_last_take() {
    let queue = KillSettlementQueue::default();
    let principal = (session(50), 3);
    // An append after the drain's last take and before the seal.
    assert_eq!(queue.append(pending(2, 50, 3, 0)), AppendOutcome::Queued);
    assert_eq!(queue.seal(principal), Seal::Queued);
    assert_eq!(queue.parked(principal), None);
    // An in-flight settle of the principal also holds the seal back.
    let in_flight = taken(queue.take(session(50), 3));
    assert_eq!(queue.seal(principal), Seal::InFlight);
    in_flight.finish();
    // Another principal's entry does not.
    assert_eq!(queue.append(pending(3, 51, 3, 0)), AppendOutcome::Queued);
    assert_eq!(queue.append(pending(4, 50, 4, 0)), AppendOutcome::Queued);
    assert_eq!(queue.seal(principal), Seal::Sealed);
    assert_eq!(
        queue.parked(principal),
        Some((ReleasePhase::Sealed, vec![]))
    );
}

#[test]
fn the_handshake_settles_every_own_entry_before_it_marks_committing() {
    run(async {
        let attack = AsyncMutex::new(ChannelAttackStates::default());
        for channel in 2..4 {
            attack
                .lock()
                .await
                .kills()
                .append(pending(channel, 50, 3, 1));
        }
        let settler = final_settles(&attack, 2);
        let mark = mark(release_handshake(&attack, session(50), 3, &settler).await);
        assert_eq!(settler.calls.get(), 2);
        let kills = attack.lock().await;
        assert_eq!(kills.kills().counts(), (0, 0, 0));
        assert_eq!(
            kills.kills().parked((session(50), 3)),
            Some((ReleasePhase::Committing, vec![]))
        );
        mark.settle(kills.kills(), ReleaseEnd::Committed);
        mark.forget(kills.kills());
        assert_eq!(kills.kills().parked((session(50), 3)), None);
    });
}

#[test]
fn a_deferred_drain_marks_nothing() {
    run(async {
        let attack = AsyncMutex::new(ChannelAttackStates::default());
        attack.lock().await.kills().append(pending(2, 50, 3, 1));
        let settler = ScriptedSettle {
            attack: &attack,
            reports: RefCell::new(VecDeque::from([report(SettleVerdict::Retry, false)])),
            calls: Cell::new(0),
        };
        assert_eq!(
            release_handshake(&attack, session(50), 3, &settler).await,
            KillRelease::Deferred
        );
        let kills = attack.lock().await;
        assert_eq!(kills.kills().counts(), (1, 0, 0));
        assert_eq!(kills.kills().parked((session(50), 3)), None);
    });
}

/// Another drain owns the principal's only entry: the release defers once instead of
/// spinning on the queue until that settle ends.
#[test]
fn an_entry_another_drain_settles_defers_the_release() {
    run(async {
        let attack = AsyncMutex::new(ChannelAttackStates::default());
        attack.lock().await.kills().append(pending(2, 50, 3, 0));
        let other = taken(attack.lock().await.kills().take(session(50), 3));
        let settler = final_settles(&attack, 0);
        assert_eq!(
            release_handshake(&attack, session(50), 3, &settler).await,
            KillRelease::Deferred
        );
        assert_eq!(settler.calls.get(), 0);
        assert_eq!(attack.lock().await.kills().parked((session(50), 3)), None);
        // Once the other drain finishes, the retry seals and commits.
        other.finish();
        let mark = mark(release_handshake(&attack, session(50), 3, &settler).await);
        mark.settle(attack.lock().await.kills(), ReleaseEnd::Committed);
    });
}

/// One kill parked on each side of the step 4 critical section.
#[test]
fn a_park_before_the_commit_point_aborts_and_one_after_it_is_committing() {
    run(async {
        let attack = AsyncMutex::new(ChannelAttackStates::default());
        let principal = (session(50), 3);
        // Sealed by an earlier pass; a kill whose D132 winner is the releasing session is
        // parked before the commit point, with its winner unchanged.
        assert_eq!(attack.lock().await.kills().seal(principal), Seal::Sealed);
        assert_eq!(
            attack.lock().await.kills().append(pending(2, 50, 3, 1)),
            AppendOutcome::Parked(ReleasePhase::Sealed)
        );
        assert_eq!(attack.lock().await.kills().counts(), (0, 0, 0));
        // The commit point aborts, the parked entry returns to the drain and is settled for
        // its winner before the mark moves to `committing`.
        let settler = final_settles(&attack, 1);
        let mark = mark(release_handshake(&attack, session(50), 3, &settler).await);
        assert_eq!(settler.calls.get(), 1);
        // After the commit point a park is in phase `committing`.
        assert_eq!(
            attack.lock().await.kills().append(pending(3, 50, 3, 1)),
            AppendOutcome::Parked(ReleasePhase::Committing)
        );
        let kills = attack.lock().await;
        assert_eq!(
            kills.kills().parked(principal),
            Some((ReleasePhase::Committing, vec![ReleasePhase::Committing]))
        );
        // A committing park does not abort a second commit point.
        assert!(kills.kills().commit_point(principal));
        // Committed: the late kill is `principal_gone`, credited to no one, and nothing for
        // the released identity is left queued or parked.
        mark.settle(kills.kills(), ReleaseEnd::Committed);
        assert_eq!(kills.kills().counts(), (0, 0, 0));
        assert_eq!(
            kills.kills().parked(principal),
            Some((ReleasePhase::Committing, vec![]))
        );
        assert!(!kills.kills().has_session(session(50)));
    });
}

#[test]
fn a_retryable_release_failure_requeues_the_parked_entries() {
    run(async {
        let attack = AsyncMutex::new(ChannelAttackStates::default());
        let settler = final_settles(&attack, 2);
        let mark = mark(release_handshake(&attack, session(50), 3, &settler).await);
        attack.lock().await.kills().append(pending(2, 50, 3, 1));
        mark.settle(attack.lock().await.kills(), ReleaseEnd::Retryable);
        assert_eq!(attack.lock().await.kills().parked((session(50), 3)), None);
        assert_eq!(attack.lock().await.kills().counts(), (1, 0, 0));
        // The resumed session settles it, and its next kill enqueues for it.
        assert_eq!(
            drain_session_kills(&attack, session(50), 3, &settler).await,
            DrainOutcome::Drained
        );
        assert_eq!(settler.calls.get(), 1);
        assert_eq!(
            attack.lock().await.kills().append(pending(3, 50, 3, 1)),
            AppendOutcome::Queued
        );
    });
}

#[test]
fn an_unknown_release_keeps_the_mark_until_the_retry_reconciles() {
    run(async {
        for (end, queued) in [(ReleaseEnd::Retryable, 1), (ReleaseEnd::Committed, 0)] {
            let attack = AsyncMutex::new(ChannelAttackStates::default());
            let principal = (session(50), 3);
            let settler = final_settles(&attack, 0);
            let first = mark(release_handshake(&attack, session(50), 3, &settler).await);
            attack.lock().await.kills().append(pending(2, 50, 3, 1));
            first.settle(attack.lock().await.kills(), ReleaseEnd::Unknown);
            assert_eq!(
                attack.lock().await.kills().parked(principal),
                Some((ReleasePhase::Committing, vec![ReleasePhase::Committing]))
            );
            // The next attempt runs the handshake again; the committing park does not abort.
            let retry = mark(release_handshake(&attack, session(50), 3, &settler).await);
            assert_eq!(settler.calls.get(), 0);
            // The retried transaction reads the row: non-terminal requeues, terminal frees.
            retry.settle(attack.lock().await.kills(), end);
            assert_eq!(attack.lock().await.kills().counts(), (queued, 0, 0));
        }
    });
}

#[test]
fn concurrent_releases_share_the_mark_until_the_last_settles() {
    run(async {
        for (end, queued) in [(ReleaseEnd::Retryable, 2), (ReleaseEnd::Committed, 0)] {
            let attack = AsyncMutex::new(ChannelAttackStates::default());
            let principal = (session(50), 3);
            let settler = final_settles(&attack, 0);
            // A grace expiry and a mismatch release of one lost epoch hold the same mark.
            let expiry = mark(release_handshake(&attack, session(50), 3, &settler).await);
            let mismatch = mark(release_handshake(&attack, session(50), 3, &settler).await);
            assert_eq!(attack.lock().await.kills().holders(principal), Some(2));
            attack.lock().await.kills().append(pending(2, 50, 3, 1));
            // One reconciles as lifted while the other is still in flight: the mark stays.
            mismatch.settle(attack.lock().await.kills(), ReleaseEnd::Retryable);
            assert_eq!(attack.lock().await.kills().holders(principal), Some(1));
            assert_eq!(
                attack.lock().await.kills().append(pending(3, 50, 3, 1)),
                AppendOutcome::Parked(ReleasePhase::Committing)
            );
            assert!(!attack.lock().await.kills().has_session(session(50)));
            expiry.settle(attack.lock().await.kills(), end);
            assert_eq!(attack.lock().await.kills().counts(), (queued, 0, 0));
        }
    });
}

#[test]
fn a_committed_release_is_not_undone_by_a_retryable_one() {
    run(async {
        let attack = AsyncMutex::new(ChannelAttackStates::default());
        let principal = (session(50), 3);
        let settler = final_settles(&attack, 0);
        let expiry = mark(release_handshake(&attack, session(50), 3, &settler).await);
        let mismatch = mark(release_handshake(&attack, session(50), 3, &settler).await);
        expiry.settle(attack.lock().await.kills(), ReleaseEnd::Committed);
        attack.lock().await.kills().append(pending(2, 50, 3, 1));
        mismatch.settle(attack.lock().await.kills(), ReleaseEnd::Retryable);
        assert_eq!(attack.lock().await.kills().counts(), (0, 0, 0));
        assert_eq!(
            attack.lock().await.kills().parked(principal),
            Some((ReleasePhase::Committing, vec![ReleasePhase::Committing]))
        );
        expiry.forget(attack.lock().await.kills());
        assert_eq!(attack.lock().await.kills().parked(principal), None);
    });
}

#[test]
fn a_resumed_epoch_clears_a_mark_an_unknown_outcome_left() {
    run(async {
        let attack = AsyncMutex::new(ChannelAttackStates::default());
        let principal = (session(50), 3);
        let settler = final_settles(&attack, 1);
        let lost = mark(release_handshake(&attack, session(50), 3, &settler).await);
        attack.lock().await.kills().append(pending(2, 50, 3, 1));
        lost.settle(attack.lock().await.kills(), ReleaseEnd::Unknown);
        assert_eq!(attack.lock().await.kills().holders(principal), Some(0));
        // Only parked: a drain of the live session would not see it.
        assert!(!attack.lock().await.kills().has_session(session(50)));
        attack.lock().await.kills().release_orphaned(session(50));
        assert_eq!(attack.lock().await.kills().parked(principal), None);
        assert!(attack.lock().await.kills().has_session(session(50)));
        assert_eq!(
            drain_session_kills(&attack, session(50), 3, &settler).await,
            DrainOutcome::Drained
        );
        assert_eq!(
            attack.lock().await.kills().append(pending(3, 50, 3, 1)),
            AppendOutcome::Queued
        );
    });
}

#[test]
fn a_held_mark_is_not_an_orphan() {
    run(async {
        let attack = AsyncMutex::new(ChannelAttackStates::default());
        let principal = (session(50), 3);
        let settler = final_settles(&attack, 0);
        let _held = mark(release_handshake(&attack, session(50), 3, &settler).await);
        attack.lock().await.kills().append(pending(2, 50, 3, 1));
        attack.lock().await.kills().release_orphaned(session(50));
        assert_eq!(
            attack.lock().await.kills().parked(principal),
            Some((ReleasePhase::Committing, vec![ReleasePhase::Committing]))
        );
    });
}

#[test]
fn an_entry_whose_principal_has_no_live_session_is_freed() {
    let queue = KillSettlementQueue::default();
    queue.append(pending(3, 50, 3, 0));
    queue.append(pending(4, 51, 2, 0));
    assert_eq!(queue.seal((session(50), 2)), Seal::Sealed);
    queue.append(pending(5, 50, 2, 0));
    // Lease generation 3 is live: generation 2's parked entry and its mark are gone.
    queue.discard_gone(session(50), Some(3));
    assert_eq!(queue.counts(), (2, 0, 0));
    assert_eq!(queue.parked((session(50), 2)), None);
    // So is a queued entry of generation 2.
    assert_eq!(queue.append(pending(2, 50, 2, 0)), AppendOutcome::Queued);
    queue.discard_gone(session(50), Some(3));
    assert_eq!(queue.counts(), (2, 0, 0));
    // The session is over: nothing of it is left; the other session's entry stays.
    queue.discard_gone(session(50), None);
    assert_eq!(queue.counts(), (1, 0, 0));
    assert!(!queue.has_session(session(50)));
}

#[test]
fn parked_entries_count_toward_killrw_rl_01() {
    let queue = KillSettlementQueue::default();
    assert_eq!(queue.seal((session(50), 3)), Seal::Sealed);
    for channel in 2..(2 + KILLRW_RL_01_UNSETTLED_ENTRIES_PER_CHANNEL_MAX as u8) {
        assert_eq!(
            queue.append(pending(channel, 50, 3, 0)),
            AppendOutcome::Parked(ReleasePhase::Sealed)
        );
    }
    assert_eq!(queue.append(pending(200, 51, 3, 0)), AppendOutcome::Full);
    // A parked death is deduplicated like a queued one.
    assert_eq!(queue.append(pending(2, 50, 3, 0)), AppendOutcome::Duplicate);
}

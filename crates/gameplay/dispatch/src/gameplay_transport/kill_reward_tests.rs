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
        damage_shares: Vec::new(),
        activity_observed_at: None,
        activity_at_death: Some(Vec::new()),
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
        target: actor,
        no_progression_logged: false,
    }
}

fn pending_with_ordinary_surprise_bag() -> PendingKillSettlement {
    let mut pending = pending(2, 50, 3, 0);
    let row = Arc::make_mut(&mut pending.row);
    row.loot_table_ref =
        LootDefinitionRef::new("Loot", "oteryn:loot.creature.rat", "definition-r1");
    row.loot_table.entries.push(LootTableEntry {
        item: LootDefinitionRef::new("Item", "oteryn:item.tibia.i6570", "definition-r1"),
        min_count: 1,
        max_count: 1,
        probability_ppm: Some(1_000_000),
    });
    pending
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
fn production_kill_settlement_does_not_reserve_an_all_year_surprise_bag_mint() {
    let queue = KillSettlementQueue::default();
    assert_eq!(
        queue.append(pending_with_ordinary_surprise_bag()),
        AppendOutcome::Queued
    );

    let in_flight = taken(queue.take(session(50), 3));

    assert_eq!(in_flight.loot_mints, 0);
    in_flight.finish();
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
    assert!(matches!(queue.seal(principal), Seal::Sealed(_)));
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
        assert!(matches!(
            attack.lock().await.kills().seal(principal),
            Seal::Sealed(_)
        ));
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
        assert!(kills.kills().commit_point(mark));
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

/// Codex 4202610727: a hold whose mark another holder's abort cleared acts on nothing, even
/// under the principal's next mark.
#[test]
fn a_hold_of_a_cleared_mark_does_not_touch_the_next_mark() {
    let queue = KillSettlementQueue::default();
    let principal = (session(50), 3);
    let sealed = |seal| match seal {
        Seal::Sealed(hold) => hold,
        other => panic!("expected a seal, found {other:?}"),
    };
    // A grace expiry and a mismatch release seal the same mark.
    let first = sealed(queue.seal(principal));
    let second = sealed(queue.seal(principal));
    assert_eq!(queue.holders(principal), Some(2));
    // A kill parks before the commit point: the first commit point aborts and clears the mark.
    assert_eq!(
        queue.append(pending(2, 50, 3, 1)),
        AppendOutcome::Parked(ReleasePhase::Sealed)
    );
    assert!(!queue.commit_point(first));
    assert_eq!(queue.parked(principal), None);
    // The first release drains the entry and seals again: a new mark of the principal.
    taken(queue.take(session(50), 3)).finish();
    let replacement = sealed(queue.seal(principal));
    assert_eq!(queue.holders(principal), Some(1));
    // The second release's hold commits nothing and settles nothing on the new mark.
    assert!(!queue.commit_point(second));
    second.settle(&queue, ReleaseEnd::Retryable);
    second.settle(&queue, ReleaseEnd::Committed);
    second.reconcile(&queue, ReleaseEnd::Retryable);
    second.forget(&queue);
    assert_eq!(
        queue.parked(principal),
        Some((ReleasePhase::Sealed, vec![]))
    );
    assert_eq!(queue.hold_slots(), vec![replacement.slot]);
    // The replacement goes through, and at quiescence no hold is counted.
    assert!(queue.commit_point(replacement));
    replacement.settle(&queue, ReleaseEnd::Committed);
    replacement.forget(&queue);
    assert_eq!(queue.hold_slots(), Vec::<u64>::new());
    assert_eq!(queue.parked(principal), None);
}

/// Every hold counted on a mark is a live hold: settles and stale holds leave none behind.
#[test]
fn the_holder_count_matches_the_live_holds_at_quiescence() {
    run(async {
        for end in [
            ReleaseEnd::Retryable,
            ReleaseEnd::Unknown,
            ReleaseEnd::Committed,
        ] {
            let attack = AsyncMutex::new(ChannelAttackStates::default());
            let settler = final_settles(&attack, 0);
            let expiry = mark(release_handshake(&attack, session(50), 3, &settler).await);
            let mismatch = mark(release_handshake(&attack, session(50), 3, &settler).await);
            assert_eq!(attack.lock().await.kills().hold_slots().len(), 2);
            expiry.settle(attack.lock().await.kills(), end);
            // A hold settled twice is not counted off twice.
            expiry.settle(attack.lock().await.kills(), end);
            assert_eq!(attack.lock().await.kills().hold_slots().len(), 1);
            mismatch.settle(attack.lock().await.kills(), end);
            assert_eq!(attack.lock().await.kills().hold_slots(), Vec::<u64>::new());
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
    assert!(matches!(queue.seal((session(50), 2)), Seal::Sealed(_)));
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
    assert!(matches!(queue.seal((session(50), 3)), Seal::Sealed(_)));
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

/// LOOT-LIVE-1: a pinned Channel runtime with one creature at (101, 100, 7) that a 20-point lab
/// hit has killed, its death projected as `record_creature_kill` leaves it.
fn runtime_with_killed_creature() -> (ChannelRuntimeV1, ExactActorRef) {
    let world = WorldId::decode(&id(1)).expect("world");
    let pin =
        oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin::from_activation(
            world,
            1,
            [1; 32],
            [2; 32],
            [3; 32],
            [4; 32],
            (100, 100, 7),
        );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&id(0x61)).expect("channel"),
        crate::foundation::NodeId::decode(&id(0x62)).expect("node"),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        pin,
    )
    .expect("runtime");
    let creature = runtime
        .admit_pinned_test_creature(crate::foundation::MovementLocalPosition {
            x: 101,
            y: 100,
            floor: 7,
        })
        .expect("creature");
    runtime
        .commit_monster_lab_damage(creature, b"loot-live-1", 20)
        .expect("lethal hit");
    let mut death = runtime.borrow_combat_death();
    let receipt = death.committed_lethal_receipt(creature).expect("receipt");
    death
        .project_committed_lethal(receipt)
        .expect("projected death");
    (runtime, creature)
}

/// A settled entry whose killed actor is `creature`: the runtime's projected corpse.
fn entry_for(creature: ExactActorRef) -> PendingKillSettlement {
    let mut entry = pending(2, 50, 3, 1);
    entry.target = creature;
    entry
}

fn mint(seed: u8) -> crate::durability::item_mint::CommittedItemMint {
    crate::durability::item_mint::CommittedItemMint {
        transaction_id: id(seed),
        event_id: id(seed.wrapping_add(1)),
        item_instance_id: id(seed.wrapping_add(2)),
        occurred_at_unix_ms: 1_000,
        envelope_sha256: [seed; 32],
    }
}

fn committed(corpse: u8) -> Result<CommittedCorpseLoot, CombatDeathRewardLootError> {
    Ok(CommittedCorpseLoot {
        corpse: mint(corpse),
        entries: vec![mint(corpse.wrapping_add(10))],
    })
}

#[test]
fn a_committed_corpse_binds_to_the_killed_actors_projection() {
    let (mut runtime, creature) = runtime_with_killed_creature();
    let entry = entry_for(creature);
    assert!(runtime.visible_corpses().is_empty());
    assert_eq!(
        bind_settled_corpse(&mut runtime, &entry, &committed(20)),
        CorpseBind::Bound
    );
    let corpse = runtime
        .bound_corpse(creature.corpse_identity())
        .expect("a visible corpse");
    assert_eq!(corpse.item.item_instance_id, mint(20).item_instance_id);
    assert_eq!(corpse.item.family, "Item");
    assert_eq!(corpse.item.production_key, "oteryn:item.corpse");
    assert_eq!(corpse.item.revision_ref, "definition-r1");
    assert_eq!(
        corpse.position,
        crate::foundation::MovementLocalPosition {
            x: 101,
            y: 100,
            floor: 7
        }
    );
    assert_eq!(runtime.visible_corpses().len(), 1);
}

#[test]
fn a_replayed_settle_binds_the_same_corpse_once() {
    let (mut runtime, creature) = runtime_with_killed_creature();
    let entry = entry_for(creature);
    assert_eq!(
        bind_settled_corpse(&mut runtime, &entry, &committed(20)),
        CorpseBind::Bound
    );
    let before = runtime.visible_corpses();
    // A Retry settle replays the same committed corpse MINT (`AlreadyCommitted`).
    assert_eq!(
        bind_settled_corpse(&mut runtime, &entry, &committed(20)),
        CorpseBind::Bound
    );
    assert_eq!(runtime.visible_corpses(), before);
}

#[test]
fn another_item_never_rebinds_a_bound_corpse() {
    let (mut runtime, creature) = runtime_with_killed_creature();
    let entry = entry_for(creature);
    assert_eq!(
        bind_settled_corpse(&mut runtime, &entry, &committed(20)),
        CorpseBind::Bound
    );
    assert_eq!(
        bind_settled_corpse(&mut runtime, &entry, &committed(40)),
        CorpseBind::Refused(CarrierError::CorpseProjectionConflict)
    );
    let corpse = runtime
        .bound_corpse(creature.corpse_identity())
        .expect("a visible corpse");
    assert_eq!(corpse.item.item_instance_id, mint(20).item_instance_id);
}

#[test]
fn no_committed_corpse_binds_nothing() {
    let (mut runtime, creature) = runtime_with_killed_creature();
    let entry = entry_for(creature);
    let refused = Err(CombatDeathRewardLootError::Corpse(
        ItemMintError::CapacityExceeded,
    ));
    assert_eq!(
        bind_settled_corpse(&mut runtime, &entry, &refused),
        CorpseBind::NoCorpse
    );
    assert!(runtime.visible_corpses().is_empty());
}

#[test]
fn a_corpse_committed_before_a_refused_loot_entry_still_binds() {
    let (mut runtime, creature) = runtime_with_killed_creature();
    let entry = entry_for(creature);
    // A later entry's MINT was refused finally; the corpse it went into is committed (D52).
    let partial = Err(CombatDeathRewardLootError::Mint(
        ItemMintError::AuthorityRejected,
        mint(20),
    ));
    assert_eq!(
        bind_settled_corpse(&mut runtime, &entry, &partial),
        CorpseBind::Bound
    );
    let corpse = runtime
        .bound_corpse(creature.corpse_identity())
        .expect("a visible corpse");
    assert_eq!(corpse.item.item_instance_id, mint(20).item_instance_id);
    // A Retry settle that later commits every entry replays the same corpse.
    assert_eq!(
        bind_settled_corpse(&mut runtime, &entry, &committed(20)),
        CorpseBind::Bound
    );
    assert_eq!(runtime.visible_corpses().len(), 1);
}

#[test]
fn a_corpse_of_another_channel_generation_is_refused() {
    let (mut runtime, _) = runtime_with_killed_creature();
    // The entry's actor belongs to another Channel's scope generation (D52).
    let foreign = pending(2, 50, 3, 1);
    assert!(matches!(
        bind_settled_corpse(&mut runtime, &foreign, &committed(20)),
        CorpseBind::Refused(_)
    ));
    assert!(runtime.visible_corpses().is_empty());
}

#[test]
fn a_live_actor_without_a_projected_death_is_refused() {
    let (mut runtime, _) = runtime_with_killed_creature();
    let living = runtime
        .admit_pinned_test_creature(crate::foundation::MovementLocalPosition {
            x: 102,
            y: 100,
            floor: 7,
        })
        .expect("creature");
    let mut entry = pending(2, 50, 3, 1);
    entry.target = living;
    assert_eq!(
        bind_settled_corpse(&mut runtime, &entry, &committed(20)),
        CorpseBind::Refused(CarrierError::CommittedLethalUnavailable)
    );
    assert!(runtime.visible_corpses().is_empty());
}

/// Kill another creature of `runtime` standing at `position` and project its death.
fn kill_creature_at(
    runtime: &mut ChannelRuntimeV1,
    position: crate::foundation::MovementLocalPosition,
    occurrence: &[u8],
) -> ExactActorRef {
    let creature = runtime
        .admit_pinned_test_creature(position)
        .expect("creature");
    runtime
        .commit_monster_lab_damage(creature, occurrence, 20)
        .expect("lethal hit");
    let mut death = runtime.borrow_combat_death();
    let receipt = death.committed_lethal_receipt(creature).expect("receipt");
    death
        .project_committed_lethal(receipt)
        .expect("projected death");
    creature
}

const SECOND_CREATURE: crate::foundation::MovementLocalPosition =
    crate::foundation::MovementLocalPosition {
        x: 102,
        y: 100,
        floor: 7,
    };

/// Codex 4248192466: an accepted kill pins its corpse projection against eviction until its
/// settle is final; a Retry keeps the pin, and a final settle releases it after the bind.
#[test]
fn an_accepted_kill_pins_its_corpse_until_its_settle_is_final() {
    let (mut runtime, creature) = runtime_with_killed_creature();
    let queue = KillSettlementQueue::default();
    assert!(!runtime.corpse_projection_pinned(creature));
    assert_eq!(
        accept_kill(&mut runtime, &queue, entry_for(creature)),
        AppendOutcome::Queued
    );
    assert!(runtime.corpse_projection_pinned(creature));
    // A replayed append of the same death keeps its one pin.
    assert_eq!(
        accept_kill(&mut runtime, &queue, entry_for(creature)),
        AppendOutcome::Duplicate
    );
    assert!(runtime.corpse_projection_pinned(creature));
    let in_flight = taken(queue.take(session(50), 3));
    let entry = in_flight.entry().expect("entry");
    release_settled_corpse(&mut runtime, entry, SettleVerdict::Retry);
    assert!(runtime.corpse_projection_pinned(creature));
    assert_eq!(
        bind_settled_corpse(&mut runtime, entry, &committed(20)),
        CorpseBind::Bound
    );
    release_settled_corpse(&mut runtime, entry, SettleVerdict::Final);
    assert!(!runtime.corpse_projection_pinned(creature));
    in_flight.finish();
    // Only the protection from eviction is released: the bound corpse stays visible.
    assert!(runtime.retains_corpse_projection(creature));
    assert_eq!(runtime.visible_corpses().len(), 1);
}

/// Codex 4248192466: a kill parked under a `releasing` mark is accepted and pins its corpse.
#[test]
fn a_parked_kill_pins_its_corpse() {
    let (mut runtime, creature) = runtime_with_killed_creature();
    let queue = KillSettlementQueue::default();
    assert!(matches!(queue.seal((session(50), 3)), Seal::Sealed(_)));
    assert_eq!(
        accept_kill(&mut runtime, &queue, entry_for(creature)),
        AppendOutcome::Parked(ReleasePhase::Sealed)
    );
    assert!(runtime.corpse_projection_pinned(creature));
}

/// Codex 4248192466: a kill the full queue refuses pins nothing and leaves the accepted kills'
/// pins in place, so its projection can evict only unpinned ones.
#[test]
fn a_kill_the_full_queue_refuses_pins_nothing() {
    let (mut runtime, first) = runtime_with_killed_creature();
    let queue = KillSettlementQueue::default();
    assert_eq!(
        accept_kill(&mut runtime, &queue, entry_for(first)),
        AppendOutcome::Queued
    );
    for channel in 3..(2 + KILLRW_RL_01_UNSETTLED_ENTRIES_PER_CHANNEL_MAX as u8) {
        assert_eq!(
            queue.append(pending(channel, 50, 3, 0)),
            AppendOutcome::Queued
        );
    }
    let second = kill_creature_at(&mut runtime, SECOND_CREATURE, b"loot-live-1-second");
    let mut late = pending(200, 51, 3, 0);
    late.target = second;
    assert_eq!(accept_kill(&mut runtime, &queue, late), AppendOutcome::Full);
    assert!(runtime.corpse_projection_pinned(first));
    assert!(!runtime.corpse_projection_pinned(second));
}

/// Codex 4248192466: an entry freed without a settle (`principal_gone`) loses its pin at the
/// next accepted kill, so the pins stay those of held entries and never leak.
#[test]
fn the_pin_of_an_entry_freed_without_a_settle_is_released() {
    let (mut runtime, first) = runtime_with_killed_creature();
    let queue = KillSettlementQueue::default();
    assert_eq!(
        accept_kill(&mut runtime, &queue, entry_for(first)),
        AppendOutcome::Queued
    );
    queue.discard_gone(session(50), None);
    let second = kill_creature_at(&mut runtime, SECOND_CREATURE, b"loot-live-1-second");
    let mut next = pending(3, 51, 3, 0);
    next.target = second;
    assert_eq!(
        accept_kill(&mut runtime, &queue, next),
        AppendOutcome::Queued
    );
    assert!(!runtime.corpse_projection_pinned(first));
    assert!(runtime.corpse_projection_pinned(second));
}

/// Codex 4248192466: an entry requeued after its final settle released its pin (a cancelled
/// drain) regains the pin at the next accepted kill.
#[test]
fn a_requeued_entry_regains_its_pin_at_the_next_accepted_kill() {
    let (mut runtime, first) = runtime_with_killed_creature();
    let queue = KillSettlementQueue::default();
    assert_eq!(
        accept_kill(&mut runtime, &queue, entry_for(first)),
        AppendOutcome::Queued
    );
    let in_flight = taken(queue.take(session(50), 3));
    release_settled_corpse(
        &mut runtime,
        in_flight.entry().expect("entry"),
        SettleVerdict::Final,
    );
    assert!(!runtime.corpse_projection_pinned(first));
    in_flight.requeue();
    let second = kill_creature_at(&mut runtime, SECOND_CREATURE, b"loot-live-1-second");
    let mut next = pending(3, 51, 3, 0);
    next.target = second;
    assert_eq!(
        accept_kill(&mut runtime, &queue, next),
        AppendOutcome::Queued
    );
    assert!(runtime.corpse_projection_pinned(first));
    assert!(runtime.corpse_projection_pinned(second));
}

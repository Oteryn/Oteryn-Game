//! ARCH-KILL-REWARD-LOGOUT-1 §1.3: the Channel's unsettled creature kills. A lethal arm (auto
//! attack, committed cast, fired spell timer) projects the death, captures its facts under the
//! runtime lock and appends one entry; the drain settles the principal session's entries after
//! every channel guard is released (§1.2).
//!
//! Lock order: `runtime → spell_states → attack`, then the queue's own mutex, which is never held
//! across an await. No channel guard is held while a revision slot is acquired or any durable
//! call runs, nor across a terminal release transaction.
//!
//! §1.3 release handshake: a terminal release drains its session's entries, seals its principal
//! identity `(GameSessionId, lease generation)` as `releasing` once the queue holds none, and
//! moves the mark to `committing` just before it sends the transaction. An append for a marked
//! principal is parked under the mark with the mark's phase at that moment.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard};

use tokio::sync::Mutex as AsyncMutex;

use crate::combat::{
    CapturedRewardPrincipal, CombatDeathRewardBestiaryError, CombatDeathRewardLootError,
    CombatDeathRewardXpError, CreatureDeathBestiaryInput, CreatureDeathRewardAdmissionError,
    CreatureDeathRewardInput, CreatureDeathRewardWithBestiaryOutcome, DeathGroundContext,
    DurabilitySession, ProjectedCreatureDeathFacts, RewardPrincipal, capture_projected_death_facts,
    check_inflight_loot_mint_capacity, loot_plan_seed, plan_creature_loot,
    settle_creature_death_rewards_with_bestiary,
};
use crate::content::creature_reward::{CreatureRewardRow, CreatureRewardTable};
use crate::durability::bestiary_progress::BestiaryProgressError;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::item_mint::ItemMintError;
use crate::foundation::runtime_actor_spell_types::CombatBatchReceipt;
use crate::foundation::{
    ChannelRuntimeV1, CreatureDeathOccurrenceKey, ExactActorRef, GameSessionId,
};

use super::attack::ChannelAttackStates;
use super::{ComposedFreshAdmission, operator_event};

/// KILLRW-RL-01: queued plus in-flight unsettled kills of one Channel.
pub(crate) const KILLRW_RL_01_UNSETTLED_ENTRIES_PER_CHANNEL_MAX: usize = 64;

/// One captured kill awaiting its settle.
#[derive(Debug)]
pub(crate) struct PendingKillSettlement {
    pub(crate) facts: ProjectedCreatureDeathFacts,
    pub(crate) creature: String,
    pub(crate) row: Arc<CreatureRewardRow>,
    pub(crate) ground: DeathGroundContext,
    /// §1.5: `no_progression_binding` is logged once per death.
    no_progression_logged: bool,
}

/// A principal identity: the captured session and its Character lease generation.
type Principal = (GameSessionId, u64);

fn principal_of(entry: &PendingKillSettlement) -> Principal {
    (
        entry.facts.principal.session,
        entry.facts.principal.lease_generation,
    )
}

/// The phase of a `releasing` mark (§1.3 steps 2 and 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReleasePhase {
    Sealed,
    Committing,
}

#[derive(Debug)]
struct ReleasingMark {
    principal: Principal,
    phase: ReleasePhase,
    /// Releases holding the mark between their seal and their step 5. A grace expiry and a
    /// mismatch release of one lost epoch may hold it together; one ending retryable leaves it to
    /// the other. An unknown outcome gives up its hold and keeps the mark.
    holders: usize,
    /// A holder's transaction ended the session.
    committed: bool,
    /// Entries appended for the marked principal, each with the phase at its park.
    parked: Vec<(PendingKillSettlement, ReleasePhase)>,
}

#[derive(Debug, Default)]
struct KillQueueState {
    queued: VecDeque<PendingKillSettlement>,
    in_flight: Vec<(CreatureDeathOccurrenceKey, Principal)>,
    reserved_loot_mints: usize,
    releasing: Vec<ReleasingMark>,
}

impl KillQueueState {
    fn holds(&self, death: CreatureDeathOccurrenceKey) -> bool {
        self.in_flight.iter().any(|(held, _)| *held == death)
            || self.queued.iter().any(|e| e.facts.death == death)
            || self
                .releasing
                .iter()
                .any(|mark| mark.parked.iter().any(|(e, _)| e.facts.death == death))
    }

    fn entries(&self) -> usize {
        self.queued.len()
            + self.in_flight.len()
            + self
                .releasing
                .iter()
                .map(|mark| mark.parked.len())
                .sum::<usize>()
    }

    fn mark(&mut self, principal: Principal) -> Option<&mut ReleasingMark> {
        self.releasing
            .iter_mut()
            .find(|mark| mark.principal == principal)
    }

    /// Remove `principal`'s mark and give back its parked entries.
    fn unmark(&mut self, principal: Principal) -> Vec<PendingKillSettlement> {
        let Some(index) = self
            .releasing
            .iter()
            .position(|mark| mark.principal == principal)
        else {
            return Vec::new();
        };
        let mark = self.releasing.swap_remove(index);
        mark.parked.into_iter().map(|(entry, _)| entry).collect()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AppendOutcome {
    Queued,
    /// The principal is `releasing`: the entry is parked under its mark in this phase.
    Parked(ReleasePhase),
    /// The death is already queued or in flight: nothing changed.
    Duplicate,
    /// KILLRW-RL-01 is reached: the death settles nothing.
    Full,
}

pub(crate) enum TakeOutcome {
    Empty,
    /// The first entry's loot plan does not fit the in-flight MINT ceiling: it stays queued and
    /// the pass stops.
    CapacityWait,
    Taken(Box<TakenKillSettlement>),
}

/// The Channel's kill queue, stored behind the `attack` mutex.
#[derive(Debug, Default, Clone)]
pub(crate) struct KillSettlementQueue(Arc<Mutex<KillQueueState>>);

impl KillSettlementQueue {
    fn state(&self) -> MutexGuard<'_, KillQueueState> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(crate) fn append(&self, entry: PendingKillSettlement) -> AppendOutcome {
        let mut state = self.state();
        if state.holds(entry.facts.death) {
            return AppendOutcome::Duplicate;
        }
        if state.entries() >= KILLRW_RL_01_UNSETTLED_ENTRIES_PER_CHANNEL_MAX {
            return AppendOutcome::Full;
        }
        if let Some(mark) = state.mark(principal_of(&entry)) {
            let phase = mark.phase;
            mark.parked.push((entry, phase));
            return AppendOutcome::Parked(phase);
        }
        state.queued.push_back(entry);
        AppendOutcome::Queued
    }

    /// §1.3 step 2, one critical section: when the queue holds no queued or in-flight entry of
    /// `principal`, mark it `releasing` (phase `sealed`). A mark kept from an unknown outcome
    /// keeps its phase and parked entries.
    pub(crate) fn seal(&self, principal: Principal) -> bool {
        let mut state = self.state();
        let pending = state.queued.iter().any(|e| principal_of(e) == principal)
            || state.in_flight.iter().any(|(_, held)| *held == principal);
        if pending {
            return false;
        }
        if let Some(mark) = state.mark(principal) {
            mark.holders += 1;
        } else {
            state.releasing.push(ReleasingMark {
                principal,
                phase: ReleasePhase::Sealed,
                holders: 1,
                committed: false,
                parked: Vec::new(),
            });
        }
        true
    }

    /// §1.3 step 4, one critical section just before the transaction is sent: an entry parked
    /// in phase `sealed` aborts the release (the mark is cleared and the parked entries are
    /// queued, `false`); otherwise the mark moves to `committing` (`true`). No holder has sent
    /// its transaction while a `sealed` park exists, so an abort strands none of them.
    pub(crate) fn commit_point(&self, principal: Principal) -> bool {
        let mut state = self.state();
        let Some(mark) = state.mark(principal) else {
            return false;
        };
        if mark
            .parked
            .iter()
            .all(|(_, phase)| *phase == ReleasePhase::Committing)
        {
            mark.phase = ReleasePhase::Committing;
            return true;
        }
        let parked = state.unmark(principal);
        state.queued.extend(parked);
        false
    }

    /// §1.3 step 5, Committed: the session ended at the phase change, so every parked entry is
    /// a kill after it ended and is freed with `principal_gone`. The mark stays until
    /// [`KillSettlementQueue::forget_released`]. `held`: the caller gives up its hold.
    pub(crate) fn release_committed(&self, principal: Principal, held: bool) {
        let parked = {
            let mut state = self.state();
            state
                .mark(principal)
                .map(|mark| {
                    mark.holders = mark.holders.saturating_sub(usize::from(held));
                    mark.committed = true;
                    std::mem::take(&mut mark.parked)
                })
                .unwrap_or_default()
        };
        for (entry, _) in parked {
            refuse("principal_gone", &entry.creature);
        }
    }

    /// The released actor slot is removed: drop the mark, freeing any late park.
    pub(crate) fn forget_released(&self, principal: Principal) {
        let parked = self.state().unmark(principal);
        for entry in parked {
            refuse("principal_gone", &entry.creature);
        }
    }

    /// §1.3 step 5, retryable failure: once no other release holds the mark and none ended the
    /// session, clear it and queue the parked entries for the resumed session. `held`: the
    /// caller gives up its hold.
    pub(crate) fn release_failed(&self, principal: Principal, held: bool) {
        let mut state = self.state();
        let Some(mark) = state.mark(principal) else {
            return;
        };
        mark.holders = mark.holders.saturating_sub(usize::from(held));
        if mark.holders == 0 && !mark.committed {
            let parked = state.unmark(principal);
            state.queued.extend(parked);
        }
    }

    /// §1.3 step 5, unknown outcome: give up the hold; the mark and its parked entries stay for
    /// the retry.
    pub(crate) fn release_unknown(&self, principal: Principal) {
        if let Some(mark) = self.state().mark(principal) {
            mark.holders = mark.holders.saturating_sub(1);
        }
    }

    /// The lost epoch was resumed, so no release of `session` committed: a mark that an unknown
    /// outcome left with no holder is cleared as retryable, and its parked entries are queued.
    pub(crate) fn release_orphaned(&self, session: GameSessionId) {
        let mut state = self.state();
        let orphaned: Vec<Principal> = state
            .releasing
            .iter()
            .filter(|mark| mark.principal.0 == session && mark.holders == 0 && !mark.committed)
            .map(|mark| mark.principal)
            .collect();
        for principal in orphaned {
            let parked = state.unmark(principal);
            state.queued.extend(parked);
        }
    }

    /// Free `session`'s queued and parked entries whose principal has no live session: the
    /// session is over (`live == None`), or the entry's lease generation is not the live one.
    /// Their marks go with them.
    pub(crate) fn discard_gone(&self, session: GameSessionId, live: Option<u64>) {
        let gone_principal = |(owner, lease): Principal| owner == session && Some(lease) != live;
        let gone = {
            let mut state = self.state();
            let (mut gone, kept) = std::mem::take(&mut state.queued)
                .into_iter()
                .partition::<Vec<_>, _>(|e| gone_principal(principal_of(e)));
            state.queued = kept.into();
            let (ended, marks) = std::mem::take(&mut state.releasing)
                .into_iter()
                .partition::<Vec<_>, _>(|mark| gone_principal(mark.principal));
            state.releasing = marks;
            gone.extend(
                ended
                    .into_iter()
                    .flat_map(|mark| mark.parked.into_iter().map(|(entry, _)| entry)),
            );
            gone
        };
        for entry in gone {
            refuse("principal_gone", &entry.creature);
        }
    }

    pub(crate) fn has_session(&self, session: GameSessionId) -> bool {
        self.state()
            .queued
            .iter()
            .any(|e| e.facts.principal.session == session)
    }

    /// One critical section: the first entry of `session` at `lease_generation` reserves its
    /// planned loot MINTs and moves in flight. A plan error reserves nothing; the settle then
    /// reports it.
    pub(crate) fn take(&self, session: GameSessionId, lease_generation: u64) -> TakeOutcome {
        let mut state = self.state();
        let Some(index) = state.queued.iter().position(|e| {
            e.facts.principal.session == session
                && e.facts.principal.lease_generation == lease_generation
        }) else {
            return TakeOutcome::Empty;
        };
        let entry = &state.queued[index];
        let loot_mints = plan_creature_loot(
            loot_plan_seed(entry.facts.death),
            &entry.row.loot_table_ref,
            &entry.row.loot_table,
        )
        .map_or(0, |plan| plan.entries.len());
        let inflight_before = state.reserved_loot_mints;
        if check_inflight_loot_mint_capacity(inflight_before, loot_mints).is_err() {
            return TakeOutcome::CapacityWait;
        }
        let Some(entry) = state.queued.remove(index) else {
            return TakeOutcome::Empty;
        };
        state.reserved_loot_mints = inflight_before + loot_mints;
        state
            .in_flight
            .push((entry.facts.death, principal_of(&entry)));
        TakeOutcome::Taken(Box::new(TakenKillSettlement {
            queue: self.clone(),
            death: entry.facts.death,
            entry: Some(entry),
            loot_mints,
            inflight_before,
        }))
    }

    /// The number of releases holding `principal`'s mark.
    #[cfg(test)]
    pub(crate) fn holders(&self, principal: Principal) -> Option<usize> {
        self.state().mark(principal).map(|mark| mark.holders)
    }

    /// The parked entries' phases under `principal`'s mark, and the mark's phase.
    #[cfg(test)]
    pub(crate) fn parked(&self, principal: Principal) -> Option<(ReleasePhase, Vec<ReleasePhase>)> {
        let mut state = self.state();
        state.mark(principal).map(|mark| {
            (
                mark.phase,
                mark.parked.iter().map(|(_, phase)| *phase).collect(),
            )
        })
    }

    #[cfg(test)]
    pub(crate) fn counts(&self) -> (usize, usize, usize) {
        let state = self.state();
        (
            state.queued.len(),
            state.in_flight.len(),
            state.reserved_loot_mints,
        )
    }
}

/// An in-flight entry. Dropping it gives back its MINT reservation and requeues the entry at
/// the front unless [`TakenKillSettlement::finish`] consumed it.
pub(crate) struct TakenKillSettlement {
    queue: KillSettlementQueue,
    death: CreatureDeathOccurrenceKey,
    entry: Option<PendingKillSettlement>,
    loot_mints: usize,
    inflight_before: usize,
}

impl TakenKillSettlement {
    pub(crate) fn entry(&self) -> Option<&PendingKillSettlement> {
        self.entry.as_ref()
    }

    pub(crate) const fn inflight_before(&self) -> usize {
        self.inflight_before
    }

    /// The settle reached a final result: the entry leaves the queue.
    pub(crate) fn finish(mut self) {
        self.entry = None;
    }

    /// The settle's outcome is unknown: the entry returns to the front of the queue.
    pub(crate) fn requeue(self) {}

    /// The creature key the first time the entry's missing progression binding is reported.
    fn mark_no_progression_logged(&mut self) -> Option<String> {
        let entry = self.entry.as_mut()?;
        (!std::mem::replace(&mut entry.no_progression_logged, true)).then(|| entry.creature.clone())
    }
}

impl Drop for TakenKillSettlement {
    fn drop(&mut self) {
        let mut state = self.queue.state();
        state.reserved_loot_mints = state.reserved_loot_mints.saturating_sub(self.loot_mints);
        state.in_flight.retain(|(death, _)| *death != self.death);
        if let Some(entry) = self.entry.take() {
            state.queued.push_front(entry);
        }
    }
}

/// Where a lethal arm appends: the Channel's queue and the active generation's reward rows.
#[derive(Clone, Copy)]
pub(crate) struct KillSink<'a> {
    pub(crate) queue: &'a KillSettlementQueue,
    pub(crate) rewards: &'a CreatureRewardTable,
}

fn refuse(reason: &str, creature: &str) {
    operator_event(&format!(
        "kill_reward_refused reason={reason} creature={creature}"
    ));
}

/// Project `target`'s committed death and append its settle (§1.1–§1.3). `lethal` is the
/// attacker whose committed hit was lethal; it is the principal only when no contributor was
/// tracked. Runs under the runtime lock and never awaits.
pub(crate) fn record_creature_kill(
    runtime: &mut ChannelRuntimeV1,
    sink: KillSink<'_>,
    target: ExactActorRef,
    lethal: CapturedRewardPrincipal,
    now_ms: u64,
) {
    let creature = runtime
        .companion_snapshot_including_dead(target)
        .map(|snapshot| snapshot.state.policy.definition_key.clone());
    let pin = runtime.content_pin();
    let hex = |bytes: [u8; 32]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let ground = DeathGroundContext {
        map_revision: format!("sha256:{}", hex(pin.map_revision_digest())),
        content_revision: format!("sha256:{}", hex(pin.server_artifact_digest())),
        ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
        sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
        native_room_placement_context: pin.frame_binding_digest().to_vec(),
    };
    let mut owner = runtime.borrow_combat_death();
    if crate::combat::project_fixed_one_creature_death(&mut owner, target).is_err() {
        return;
    }
    let Ok(creature) = creature else {
        refuse("no_loot_binding", "unknown");
        return;
    };
    let Ok(top) = owner.top_damage_contributor(target) else {
        return;
    };
    if top.is_gone() {
        refuse("principal_gone", &creature);
        return;
    }
    let (principal, last_damage_at_ms, untracked) = match top.present() {
        Some(winner) => (
            CapturedRewardPrincipal {
                character: winner.character,
                lease_generation: winner.lease_generation,
                session: winner.session,
                actor: winner.actor,
            },
            winner.last_damage_at_ms,
            false,
        ),
        None => (lethal, None, true),
    };
    let row = match sink.rewards.row(&creature) {
        Ok(row) => row,
        Err(reason) => {
            refuse(reason.as_str(), &creature);
            return;
        }
    };
    let Ok(mut facts) =
        capture_projected_death_facts(&mut owner, target, principal, last_damage_at_ms, now_ms)
    else {
        return;
    };
    if untracked {
        // The lethal hit is the untracked principal's own clocked damage.
        facts.principal_last_damage_at_ms = Some(facts.death_at_ms);
    }
    let entry = PendingKillSettlement {
        facts,
        creature,
        row,
        ground,
        no_progression_logged: false,
    };
    let creature = entry.creature.clone();
    if sink.queue.append(entry) == AppendOutcome::Full {
        refuse("queue_full", &creature);
    }
}

/// §1.4: every lethal effect of a committed or replayed spell batch.
pub(crate) fn record_batch_kills(
    runtime: &mut ChannelRuntimeV1,
    sink: KillSink<'_>,
    receipt: &CombatBatchReceipt,
    now_ms: u64,
) {
    let session = receipt.command.game_session_id();
    for effect in &receipt.effects {
        if effect.health.is_none_or(|health| health.health_after > 0) {
            continue;
        }
        let Ok(lease) = runtime.bound_attacker_lease(receipt.caster, session) else {
            continue;
        };
        let lethal = CapturedRewardPrincipal {
            character: lease.character_id(),
            lease_generation: lease.generation(),
            session,
            actor: receipt.caster,
        };
        record_creature_kill(runtime, sink, effect.target, lethal, now_ms);
    }
}

/// Whether a settle reached its final result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettleVerdict {
    Final,
    /// The outcome is unknown: the entry is requeued and the pass stops.
    Retry,
}

/// One settle's result as the drain needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SettleReport {
    pub(crate) verdict: SettleVerdict,
    /// §1.5: XP and Bestiary found no progression binding.
    pub(crate) no_progression: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DrainOutcome {
    Drained,
    /// An entry's outcome is unknown or its MINTs wait for capacity; it stays queued.
    Deferred,
}

/// The durable half of a drain. Called with no channel guard held.
pub(crate) trait KillSettle {
    async fn settle(&self, entry: &PendingKillSettlement, inflight_before: usize) -> SettleReport;
}

/// Settle `session`'s entries at `lease_generation` one at a time. The `attack` guard is held
/// only to take and to finish or requeue an entry, never across `settler`.
pub(crate) async fn drain_session_kills(
    attack: &AsyncMutex<ChannelAttackStates>,
    session: GameSessionId,
    lease_generation: u64,
    settler: &impl KillSettle,
) -> DrainOutcome {
    loop {
        let taken = attack.lock().await.kills().take(session, lease_generation);
        let mut taken = match taken {
            TakeOutcome::Empty => return DrainOutcome::Drained,
            TakeOutcome::CapacityWait => return DrainOutcome::Deferred,
            TakeOutcome::Taken(taken) => taken,
        };
        let Some(entry) = taken.entry() else {
            return DrainOutcome::Drained;
        };
        let report = settler.settle(entry, taken.inflight_before()).await;
        if report.no_progression
            && let Some(creature) = taken.mark_no_progression_logged()
        {
            refuse("no_progression_binding", &creature);
        }
        let _attack = attack.lock().await;
        match report.verdict {
            SettleVerdict::Final => taken.finish(),
            SettleVerdict::Retry => {
                taken.requeue();
                return DrainOutcome::Deferred;
            }
        }
    }
}

/// The end of a release handshake's steps 1–4 (§1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum KillRelease {
    /// Send the transaction, then report its outcome with [`KillReleaseMark::settle`]. No mark
    /// is held when the session is already terminal.
    Send(Option<KillReleaseMark>),
    /// Step 1 met an unknown outcome or a `capacity_wait`: nothing is marked and the
    /// transaction must not be sent.
    Deferred,
}

/// The terminal release transaction's outcome as step 5 reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReleaseEnd {
    /// `session_state = 3`.
    Committed,
    /// The session did not end.
    Retryable,
    /// The outcome is unknown: the mark and the parked entries stay for the next attempt.
    Unknown,
}

/// A principal marked `committing` by [`release_handshake`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct KillReleaseMark {
    principal: Principal,
}

impl KillReleaseMark {
    /// §1.3 step 5 of the release holding the mark, in one `attack` critical section. Called
    /// once per [`release_handshake`] that returned the mark.
    pub(crate) fn settle(self, queue: &KillSettlementQueue, end: ReleaseEnd) {
        match end {
            ReleaseEnd::Committed => queue.release_committed(self.principal, true),
            ReleaseEnd::Retryable => queue.release_failed(self.principal, true),
            ReleaseEnd::Unknown => queue.release_unknown(self.principal),
        }
    }

    /// Step 5 from a durable reconcile after the hold was given up to an unknown outcome.
    pub(crate) fn reconcile(self, queue: &KillSettlementQueue, end: ReleaseEnd) {
        match end {
            ReleaseEnd::Committed => queue.release_committed(self.principal, false),
            ReleaseEnd::Retryable => queue.release_failed(self.principal, false),
            ReleaseEnd::Unknown => {}
        }
    }

    /// The released actor slot is removed.
    pub(crate) fn forget(self, queue: &KillSettlementQueue) {
        queue.forget_released(self.principal);
    }
}

/// §1.3 steps 1–4: drain, seal, then move the mark to `committing`, returning to the drain
/// while the seal finds an entry or a park in phase `sealed` aborts the commit point. Each
/// return needs a new kill for this principal, so the loop advances with the game. No guard is
/// held across a settle or on return.
pub(crate) async fn release_handshake(
    attack: &AsyncMutex<ChannelAttackStates>,
    session: GameSessionId,
    lease_generation: u64,
    settler: &impl KillSettle,
) -> KillRelease {
    let principal = (session, lease_generation);
    loop {
        if drain_session_kills(attack, session, lease_generation, settler).await
            == DrainOutcome::Deferred
        {
            return KillRelease::Deferred;
        }
        if !attack.lock().await.kills().seal(principal) {
            tokio::task::yield_now().await;
            continue;
        }
        if attack.lock().await.kills().commit_point(principal) {
            return KillRelease::Send(Some(KillReleaseMark { principal }));
        }
    }
}

/// An unknown durable outcome: the write may or may not have committed, or capacity freed later
/// may admit it.
fn loot_unknown(error: &CombatDeathRewardLootError) -> bool {
    matches!(
        error,
        CombatDeathRewardLootError::Capacity(_)
            | CombatDeathRewardLootError::Corpse(
                ItemMintError::Unavailable(_) | ItemMintError::CapacityExceeded
            )
            | CombatDeathRewardLootError::Mint(ItemMintError::Unavailable(_))
    )
}

fn classify(
    creature: &str,
    outcome: &Result<CreatureDeathRewardWithBestiaryOutcome, CreatureDeathRewardAdmissionError>,
) -> SettleReport {
    let report = |verdict, no_progression| SettleReport {
        verdict,
        no_progression,
    };
    let outcome = match outcome {
        Ok(outcome) => outcome,
        Err(CreatureDeathRewardAdmissionError::PrincipalMismatch) => {
            refuse("principal_gone", creature);
            return report(SettleVerdict::Final, false);
        }
        Err(CreatureDeathRewardAdmissionError::Limit(error)) => {
            refuse(&format!("limit:{error}"), creature);
            return report(SettleVerdict::Final, false);
        }
    };
    let rewards = &outcome.rewards;
    let no_progression = matches!(
        rewards.xp,
        Err(CombatDeathRewardXpError::NoProgressionBinding)
    );
    let retry = rewards.loot.as_ref().is_err_and(loot_unknown)
        || matches!(
            rewards.xp,
            Err(CombatDeathRewardXpError::Progression(
                CharacterProgressionError::Unavailable(_)
            ))
        )
        || matches!(
            outcome.bestiary,
            Err(CombatDeathRewardBestiaryError::Progress(
                BestiaryProgressError::Unavailable(_)
            ))
        );
    if retry {
        return report(SettleVerdict::Retry, no_progression);
    }
    if let Err(error) = &rewards.loot {
        operator_event(&format!(
            "kill_reward_loot_failed creature={creature} reason={error:?}"
        ));
    }
    if let Err(error) = &rewards.xp
        && !no_progression
    {
        operator_event(&format!(
            "kill_reward_xp_failed creature={creature} reason={error:?}"
        ));
    }
    if let Err(error) = &outcome.bestiary
        && !matches!(error, CombatDeathRewardBestiaryError::NoProgressionBinding)
    {
        operator_event(&format!(
            "kill_reward_bestiary_failed creature={creature} reason={error:?}"
        ));
    }
    report(SettleVerdict::Final, no_progression)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// The active generation's reward rows. Empty until the loot pin is composed, so every kill
    /// refuses `no_loot_binding`.
    pub(super) fn reward_table(&self) -> &CreatureRewardTable {
        static EMPTY: std::sync::LazyLock<CreatureRewardTable> =
            std::sync::LazyLock::new(CreatureRewardTable::default);
        &EMPTY
    }

    /// §1.4: record the kills of committed or replayed spell batches, under the caller's
    /// `runtime` and `spell_states` guards; `attack` is locked after them.
    pub(super) async fn record_spell_kills<'r>(
        &self,
        runtime: &mut ChannelRuntimeV1,
        receipts: impl Iterator<Item = &'r CombatBatchReceipt>,
    ) {
        let attack = self.attack.lock().await;
        let sink = KillSink {
            queue: attack.kills(),
            rewards: self.reward_table(),
        };
        let now_ms = self.owner_now().get() / 1_000;
        for receipt in receipts {
            record_batch_kills(runtime, sink, receipt, now_ms);
        }
    }

    /// Settle `session`'s queued kills (§1.3), after every channel guard is released. An entry
    /// whose principal has no live session is freed with `principal_gone`.
    pub(super) async fn drain_kill_rewards(&self, session: GameSessionId) -> DrainOutcome {
        if !self.attack.lock().await.kills().has_session(session) {
            return DrainOutcome::Drained;
        }
        let fence = match self.current_quest_fence(session).await {
            Ok(Some(fence)) => fence,
            Ok(None) => {
                self.attack.lock().await.kills().discard_gone(session, None);
                return DrainOutcome::Drained;
            }
            Err(()) => return DrainOutcome::Deferred,
        };
        self.attack
            .lock()
            .await
            .kills()
            .discard_gone(session, Some(fence.character_lease_generation));
        drain_session_kills(
            &self.attack,
            session,
            fence.character_lease_generation,
            &DurableKillSettle {
                admission: self,
                fence,
            },
        )
        .await
    }

    /// §1.3 release handshake of `session`, run after its monk save and before its terminal
    /// release transaction.
    pub(super) async fn seal_kill_rewards(&self, session: GameSessionId) -> KillRelease {
        let fence = match self.current_quest_fence(session).await {
            Ok(Some(fence)) => fence,
            // Already terminal: the transaction only reconciles, and nothing settles any more.
            Ok(None) => {
                self.attack.lock().await.kills().discard_gone(session, None);
                return KillRelease::Send(None);
            }
            Err(()) => return KillRelease::Deferred,
        };
        let lease_generation = fence.character_lease_generation;
        self.attack
            .lock()
            .await
            .kills()
            .discard_gone(session, Some(lease_generation));
        release_handshake(
            &self.attack,
            session,
            lease_generation,
            &DurableKillSettle {
                admission: self,
                fence,
            },
        )
        .await
    }

    /// §1.3 step 5 for a release that sent its transaction.
    pub(super) async fn settle_kill_release(&self, mark: Option<KillReleaseMark>, end: ReleaseEnd) {
        if let Some(mark) = mark {
            mark.settle(self.attack.lock().await.kills(), end);
        }
    }

    /// Step 5 of a durable reconcile after an unknown outcome gave up the hold.
    pub(super) async fn reconcile_kill_release(
        &self,
        mark: Option<KillReleaseMark>,
        end: ReleaseEnd,
    ) {
        if let Some(mark) = mark {
            mark.reconcile(self.attack.lock().await.kills(), end);
        }
    }

    /// The lost epoch was resumed: clear `session`'s marks left by unknown outcomes.
    pub(super) async fn release_orphaned_kills(&self, session: GameSessionId) {
        self.attack.lock().await.kills().release_orphaned(session);
    }

    /// The released actor slot is removed: its mark goes.
    pub(super) async fn forget_kill_release(&self, mark: Option<KillReleaseMark>) {
        if let Some(mark) = mark {
            mark.forget(self.attack.lock().await.kills());
        }
    }
}

struct DurableKillSettle<'s, 'a, 'f, 'g> {
    admission: &'s ComposedFreshAdmission<'a, 'f, 'g>,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
}

impl KillSettle for DurableKillSettle<'_, '_, '_, '_> {
    async fn settle(&self, entry: &PendingKillSettlement, inflight_before: usize) -> SettleReport {
        let admission = self.admission;
        let mut slot = admission
            .revision_sequencer
            .acquire(self.fence.character_id)
            .await;
        let Ok(expected_character_revision) =
            slot.cursor(admission.root, admission.character).await
        else {
            return SettleReport {
                verdict: SettleVerdict::Retry,
                no_progression: false,
            };
        };
        let fence = crate::durability::character_progression::CurrentCharacterGameplayFence {
            expected_character_revision,
            ..self.fence
        };
        let input = CreatureDeathRewardInput::<1> {
            corpse_item: entry.row.corpse_item.clone(),
            loot_table_ref: entry.row.loot_table_ref.clone(),
            loot_table: entry.row.loot_table.clone(),
            ground: entry.ground.clone(),
            inflight_loot_mints_before_this_death: inflight_before,
            reward_principals: vec![RewardPrincipal {
                gameplay_fence: fence,
            }],
            xp_amount: entry.row.xp_amount,
            progression: super::player_death_progression().cloned(),
        };
        let session = DurabilitySession {
            root: admission.root,
            authority: admission.character,
            node: admission.holder,
        };
        let outcome = settle_creature_death_rewards_with_bestiary(
            entry.facts,
            &session,
            &mut slot,
            input,
            CreatureDeathBestiaryInput {
                race: entry.row.race.clone(),
            },
        )
        .await;
        classify(&entry.creature, &outcome)
    }
}

#[cfg(test)]
#[path = "kill_reward_tests.rs"]
mod tests;

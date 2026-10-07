//! ARCH-KILL-REWARD-LOGOUT-1 §1.3: the Channel's unsettled creature kills. A lethal arm (auto
//! attack, committed cast, fired spell timer) projects the death, captures its facts under the
//! runtime lock and appends one entry; the drain settles the principal session's entries after
//! every channel guard is released (§1.2).
//!
//! Lock order: `runtime → spell_states → attack`, then the queue's own mutex, which is never held
//! across an await. No channel guard is held while a revision slot is acquired or any durable
//! call runs.

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

#[derive(Debug, Default)]
struct KillQueueState {
    queued: VecDeque<PendingKillSettlement>,
    in_flight: Vec<CreatureDeathOccurrenceKey>,
    reserved_loot_mints: usize,
}

impl KillQueueState {
    fn holds(&self, death: CreatureDeathOccurrenceKey) -> bool {
        self.in_flight.contains(&death) || self.queued.iter().any(|e| e.facts.death == death)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AppendOutcome {
    Queued,
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
        if state.queued.len() + state.in_flight.len()
            >= KILLRW_RL_01_UNSETTLED_ENTRIES_PER_CHANNEL_MAX
        {
            return AppendOutcome::Full;
        }
        state.queued.push_back(entry);
        AppendOutcome::Queued
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
        state.in_flight.push(entry.facts.death);
        TakeOutcome::Taken(Box::new(TakenKillSettlement {
            queue: self.clone(),
            death: entry.facts.death,
            entry: Some(entry),
            loot_mints,
            inflight_before,
        }))
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
        state.in_flight.retain(|death| *death != self.death);
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

    /// Settle `session`'s queued kills (§1.3), after every channel guard is released.
    pub(super) async fn drain_kill_rewards(&self, session: GameSessionId) -> DrainOutcome {
        if !self.attack.lock().await.kills().has_session(session) {
            return DrainOutcome::Drained;
        }
        let fence = match self.current_quest_fence(session).await {
            Ok(Some(fence)) => fence,
            Ok(None) => return DrainOutcome::Drained,
            Err(()) => return DrainOutcome::Deferred,
        };
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

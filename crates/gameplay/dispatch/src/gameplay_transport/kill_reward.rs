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
    CapturedRewardPrincipal, CombatBestiaryOutcome, CombatDeathRewardBestiaryError,
    CombatDeathRewardLootError, CombatDeathRewardXpError, CommittedCorpseLoot,
    CreatureDeathBestiaryInput, CreatureDeathRewardAdmissionError, CreatureDeathRewardInput,
    CreatureDeathRewardWithBestiaryOutcome, DeathGroundContext, DurabilitySession,
    ProjectedCreatureDeathFacts, RewardPrincipal, animus_mastery_experience,
    capture_projected_death_facts, check_inflight_loot_mint_capacity, loot_plan_seed,
    plan_creature_loot, settle_creature_death_rewards_with_bestiary,
    settle_creature_death_rewards_with_party_experience,
};
use crate::content::character_progression_content::CHARACTER_EXPERIENCE_TABLE_LEVELS;
use crate::content::creature_reward::{CreatureRewardRow, CreatureRewardTable};
use crate::durability::bestiary_progress::{BestiaryKillOutcome, BestiaryProgressError};
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::cyclopedia_tracker::TrackerKind;
use crate::durability::item_mint::ItemMintError;
use crate::foundation::runtime_actor_spell_types::CombatBatchReceipt;
use {
    crate::foundation::{
        ChannelRuntimeV1, CreatureDeathOccurrenceKey, ExactActorRef, GameSessionId,
    },
    oteryn_gameplay_domain::domain::carrier_values::CarrierError,
};

use super::attack::ChannelAttackStates;
use super::{ComposedFreshAdmission, operator_event};

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "unit fixtures construct valid proficiency shapes"
)]
#[path = "proficiency_kill_tests.rs"]
mod proficiency_kill_tests;

fn apply_hunting_stamina_to_loot(
    loot_table: &mut crate::combat::LootTableDefinition,
    stamina_minutes: Option<u16>,
) {
    if stamina_minutes
        .is_some_and(|minutes| !oteryn_gameplay_characters::stamina::hunting_loot_allowed(minutes))
    {
        loot_table.entries.clear();
    }
}

/// A completed Bestiary entry makes its Charm reward available immediately. Reconciled
/// receipts also mark the title projection, so recovery cannot lose Beastly/class unlocks.
fn bestiary_completion_affects_titles(outcome: &BestiaryKillOutcome) -> bool {
    match outcome {
        BestiaryKillOutcome::Committed(kill) | BestiaryKillOutcome::AlreadyCommitted(kill) => {
            kill.kill_count_before < kill.final_kill_threshold
                && kill.kill_count_after >= kill.final_kill_threshold
        }
        BestiaryKillOutcome::Saturated { .. } => false,
    }
}

/// KILLRW-RL-01: queued plus in-flight unsettled kills of one Channel.
pub const KILLRW_RL_01_UNSETTLED_ENTRIES_PER_CHANNEL_MAX: usize = 64;

/// One captured kill awaiting its settle.
#[derive(Debug, Clone)]
pub struct PendingKillSettlement {
    pub facts: ProjectedCreatureDeathFacts,
    pub creature: String,
    pub row: Option<Arc<CreatureRewardRow>>,
    pub bosstiary_only: Option<crate::domain::bosstiary::BosstiaryBoss>,
    pub ground: DeathGroundContext,
    /// The killed actor whose committed corpse projection the minted corpse
    /// Item binds to (MAP-ITEM-REF-1 Part B, D607).
    pub target: ExactActorRef,
    pub activity_observed_at: Option<oteryn_simulation_determinism::SemanticTimeMicros>,
    pub activity_at_death:
        Option<Vec<oteryn_gameplay_social_rules::shared_experience::SharedExperienceActivity>>,
    pub damage_shares: Vec<crate::foundation::RewardDamageShare>,
    pub(super) forge: Option<ForgeDeath>,
    /// §1.5: `no_progression_binding` is logged once per death.
    no_progression_logged: bool,
}

impl PendingKillSettlement {
    pub(super) fn for_bosstiary(
        facts: ProjectedCreatureDeathFacts,
        boss: crate::domain::bosstiary::BosstiaryBoss,
        ground: DeathGroundContext,
        target: ExactActorRef,
        observed_at: Option<oteryn_simulation_determinism::SemanticTimeMicros>,
    ) -> Self {
        Self {
            facts,
            creature: boss.key.clone(),
            row: None,
            bosstiary_only: Some(boss),
            ground,
            target,
            activity_observed_at: observed_at,
            activity_at_death: None,
            damage_shares: Vec::new(),
            forge: None,
            no_progression_logged: false,
        }
    }
}

/// Immutable classification and bounded recipient positions captured with the lethal event.
#[derive(Debug, Clone)]
pub(super) struct ForgeDeath {
    pub kind: crate::domain::forge_creature::ForgeCreature,
    pub roll: u8,
    pub origin: crate::foundation::MovementLocalPosition,
    pub recipients: Vec<(
        ExactActorRef,
        GameSessionId,
        crate::foundation::MovementLocalPosition,
    )>,
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
pub enum ReleasePhase {
    Sealed,
    Committing,
}

#[derive(Debug)]
struct ReleasingMark {
    /// This mark's identity: a mark cleared and created again for the principal is another
    /// mark, and a [`KillReleaseMark`] of the first one acts on nothing.
    id: u64,
    principal: Principal,
    phase: ReleasePhase,
    /// The hold slot of each release between its seal and its step 5. A grace expiry and a
    /// mismatch release of one lost epoch may hold it together; one ending retryable leaves it to
    /// the other. An unknown outcome gives up its hold and keeps the mark.
    holders: Vec<u64>,
    /// A holder's transaction ended the session.
    committed: bool,
    /// Entries appended for the marked principal, each with the phase at its park.
    parked: Vec<(PendingKillSettlement, ReleasePhase)>,
}

#[derive(Debug, Default)]
struct KillQueueState {
    // Equipment-owner projections, independent of a held attack target or swing-cache epoch.
    proficiency_weapons: Vec<(ExactActorRef, GameSessionId, u64, Option<String>)>,
    proficiency_deaths: Vec<FrozenProficiencyDeath>,
    /// One frozen contribution per retained exact corpse projection; pruned on each death.
    prey_loot: Vec<(ExactActorRef, u8)>,
    queued: VecDeque<PendingKillSettlement>,
    /// Each in-flight entry's death, principal and killed actor.
    in_flight: Vec<(CreatureDeathOccurrenceKey, Principal, ExactActorRef)>,
    reserved_loot_mints: usize,
    /// W22c: MINTs an in-flight settle reserved on top of its take (a Wealth Duplex roll).
    extra_loot_mints: Vec<(CreatureDeathOccurrenceKey, usize)>,
    releasing: Vec<ReleasingMark>,
    /// The last mark id or hold slot given out.
    last_id: u64,
}

impl KillQueueState {
    fn holds(&self, death: CreatureDeathOccurrenceKey) -> bool {
        self.in_flight.iter().any(|(held, _, _)| *held == death)
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

    fn next_id(&mut self) -> u64 {
        self.last_id += 1;
        self.last_id
    }

    /// The mark `hold` was sealed under, if it still exists.
    fn held_mark(&mut self, hold: KillReleaseMark) -> Option<&mut ReleasingMark> {
        let mark = self
            .releasing
            .iter_mut()
            .find(|mark| mark.id == hold.mark && mark.principal == hold.principal);
        if mark.is_none() {
            stale_hold(hold);
        }
        mark
    }

    /// Remove mark `id` and give back its parked entries.
    fn unmark(&mut self, id: u64) -> Vec<PendingKillSettlement> {
        let Some(index) = self.releasing.iter().position(|mark| mark.id == id) else {
            return Vec::new();
        };
        let mark = self.releasing.swap_remove(index);
        mark.parked.into_iter().map(|(entry, _)| entry).collect()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum AppendOutcome {
    Queued,
    /// The principal is `releasing`: the entry is parked under its mark in this phase.
    Parked(ReleasePhase),
    /// The death is already queued or in flight: nothing changed.
    Duplicate,
    /// KILLRW-RL-01 is reached: the death settles nothing.
    Full,
}

pub enum TakeOutcome {
    Empty,
    /// The first entry's loot plan does not fit the in-flight MINT ceiling: it stays queued and
    /// the pass stops.
    CapacityWait,
    Taken(Box<TakenKillSettlement>),
}

/// The Channel's kill queue, stored behind the `attack` mutex.
#[derive(Debug, Default, Clone)]
pub struct KillSettlementQueue(Arc<Mutex<KillQueueState>>);

#[derive(Debug, Clone)]
struct FrozenProficiencyContributor {
    actor: ExactActorRef,
    session: GameSessionId,
    last_damage_at_ms: u64,
    weapon: String,
}

#[derive(Debug, Clone)]
struct FrozenProficiencyDeath {
    target: ExactActorRef,
    occurrence: [u8; 16],
    contributors: Vec<FrozenProficiencyContributor>,
    reward_principal_gone: bool,
}

impl KillSettlementQueue {
    /// Actor-end saves drain deaths to which this session contributed, including another
    /// principal's queue. An in-flight settle must finish before the contributor can save.
    pub(super) fn proficiency_principals(&self, session: GameSessionId) -> Vec<GameSessionId> {
        let state = self.state();
        let mut principals: std::collections::BTreeSet<_> = state
            .queued
            .iter()
            .chain(
                state
                    .releasing
                    .iter()
                    .flat_map(|mark| mark.parked.iter().map(|(entry, _)| entry)),
            )
            .filter(|entry| {
                entry.facts.principal.session == session
                    || entry
                        .damage_shares
                        .iter()
                        .any(|share| share.session == Some(session))
            })
            .map(|entry| entry.facts.principal.session)
            .collect();
        for (_, principal, target) in &state.in_flight {
            if principal.0 == session
                || state.proficiency_deaths.iter().any(|death| {
                    death.target == *target
                        && death
                            .contributors
                            .iter()
                            .any(|contributor| contributor.session == session)
                })
            {
                principals.insert(principal.0);
            }
        }
        principals.into_iter().collect()
    }

    pub(super) fn prune_proficiency(&self, runtime: &ChannelRuntimeV1) {
        let mut state = self.state();
        state.proficiency_weapons.retain(|(actor, session, _, _)| {
            runtime.player_control_facts(*actor, *session).is_ok()
        });
        state
            .proficiency_deaths
            .retain(|death| runtime.retains_corpse_projection(death.target));
    }
    pub(super) fn proficiency_equipment(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        revision: u64,
        weapon: Option<&str>,
    ) {
        let mut state = self.state();
        if state
            .proficiency_weapons
            .iter()
            .any(|(a, s, r, _)| *a == actor && *s == session && *r > revision)
        {
            return;
        }
        state
            .proficiency_weapons
            .retain(|(a, s, _, _)| *a != actor || *s != session);
        state
            .proficiency_weapons
            .push((actor, session, revision, weapon.map(str::to_owned)));
    }

    fn freeze_proficiency(
        &self,
        runtime: &ChannelRuntimeV1,
        target: ExactActorRef,
        occurrence: [u8; 16],
        shares: &[crate::foundation::RewardDamageShare],
        reward_principal_gone: bool,
    ) {
        if !runtime
            .companion_snapshot_including_dead(target)
            .is_ok_and(|snapshot| {
                snapshot.state.master.is_none() && !snapshot.state.policy.is_familiar
            })
        {
            return;
        }
        let mut state = self.state();
        state.proficiency_weapons.retain(|(actor, session, _, _)| {
            runtime.player_control_facts(*actor, *session).is_ok()
        });
        state
            .proficiency_deaths
            .retain(|death| runtime.retains_corpse_projection(death.target));
        if state
            .proficiency_deaths
            .iter()
            .any(|death| death.occurrence == occurrence)
        {
            return;
        }
        let contributors = shares
            .iter()
            .filter_map(|share| {
                let session = share.session?;
                let lease = share.lease_generation?;
                let (actor, _, _, weapon) = state
                    .proficiency_weapons
                    .iter()
                    .find(|(_, s, _, _)| *s == session)?;
                let weapon = weapon.as_ref()?;
                let bound = runtime.bound_attacker_lease(*actor, session).ok()?;
                if bound.character_id() != share.character || bound.generation() != lease {
                    return None;
                }
                Some(FrozenProficiencyContributor {
                    actor: *actor,
                    session,
                    last_damage_at_ms: share.last_damage_at_ms?,
                    weapon: weapon.clone(),
                })
            })
            .collect();
        state.proficiency_deaths.push(FrozenProficiencyDeath {
            target,
            occurrence,
            contributors,
            reward_principal_gone,
        });
    }
    fn state(&self) -> MutexGuard<'_, KillQueueState> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(super) fn blocks_prey(&self, character: &[u8; 16]) -> bool {
        let state = self.state();
        let affects = |entry: &PendingKillSettlement| {
            entry.facts.principal.character.as_bytes() == character
                || entry
                    .damage_shares
                    .iter()
                    .any(|share| share.character.as_bytes() == character)
                || entry
                    .activity_at_death
                    .as_ref()
                    .is_none_or(|activity| activity.iter().any(|cell| cell.character == *character))
        };
        !state.in_flight.is_empty()
            || state.queued.iter().any(affects)
            || state
                .releasing
                .iter()
                .any(|mark| mark.parked.iter().any(|(entry, _)| affects(entry)))
    }

    fn prey_at_death(&self, runtime: &ChannelRuntimeV1, target: ExactActorRef, percent: u8) -> u8 {
        let mut state = self.state();
        // Retention is bounded by the carrier's existing pinned + unpinned corpse ceilings.
        // Once evicted, that exact death can no longer be projected or replayed by this runtime.
        state
            .prey_loot
            .retain(|(actor, _)| runtime.retains_corpse_projection(*actor));
        if let Some((_, frozen)) = state.prey_loot.iter().find(|(actor, _)| *actor == target) {
            return *frozen;
        }
        if !runtime.retains_corpse_projection(target) {
            return 0;
        }
        state.prey_loot.push((target, percent));
        percent
    }

    pub fn append(&self, entry: PendingKillSettlement) -> AppendOutcome {
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
    /// `principal`, mark it `releasing` (phase `sealed`) and give the caller a hold slot on the
    /// mark. A mark kept from an unknown outcome keeps its phase and parked entries.
    pub fn seal(&self, principal: Principal) -> Seal {
        let mut state = self.state();
        if state
            .in_flight
            .iter()
            .any(|(_, held, _)| *held == principal)
        {
            return Seal::InFlight;
        }
        if state.queued.iter().any(|e| principal_of(e) == principal) {
            return Seal::Queued;
        }
        let slot = state.next_id();
        let mark = if let Some(mark) = state.mark(principal) {
            mark.holders.push(slot);
            mark.id
        } else {
            let id = state.next_id();
            state.releasing.push(ReleasingMark {
                id,
                principal,
                phase: ReleasePhase::Sealed,
                holders: vec![slot],
                committed: false,
                parked: Vec::new(),
            });
            id
        };
        Seal::Sealed(KillReleaseMark {
            principal,
            mark,
            slot,
        })
    }

    /// §1.3 step 4, one critical section just before the transaction is sent: an entry parked
    /// in phase `sealed` aborts the release (the mark is cleared and the parked entries are
    /// queued, `false`); otherwise the mark moves to `committing` (`true`). No holder has sent
    /// its transaction while a `sealed` park exists, so an abort strands none of them. A hold
    /// whose mark another holder's abort cleared commits nothing, even under a newer mark of
    /// the principal.
    pub fn commit_point(&self, hold: KillReleaseMark) -> bool {
        let mut state = self.state();
        let Some(mark) = state
            .releasing
            .iter_mut()
            .find(|mark| mark.id == hold.mark && mark.holders.contains(&hold.slot))
        else {
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
        let parked = state.unmark(hold.mark);
        state.queued.extend(parked);
        false
    }

    /// §1.3 step 5, Committed: the session ended at the phase change, so every parked entry is
    /// a kill after it ended and is freed with `principal_gone`. The mark stays until
    /// [`KillSettlementQueue::forget_released`]. `held`: the caller gives up its hold.
    fn release_committed(&self, hold: KillReleaseMark, held: bool) {
        let parked = {
            let mut state = self.state();
            state
                .held_mark(hold)
                .map(|mark| {
                    if held {
                        give_back(mark, hold);
                    }
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
    fn forget_released(&self, hold: KillReleaseMark) {
        let parked = self.state().unmark(hold.mark);
        for entry in parked {
            refuse("principal_gone", &entry.creature);
        }
    }

    /// §1.3 step 5, retryable failure: once no other release holds the mark and none ended the
    /// session, clear it and queue the parked entries for the resumed session. `held`: the
    /// caller gives up its hold.
    fn release_failed(&self, hold: KillReleaseMark, held: bool) {
        let mut state = self.state();
        let Some(mark) = state.held_mark(hold) else {
            return;
        };
        if held {
            give_back(mark, hold);
        }
        if mark.holders.is_empty() && !mark.committed {
            let parked = state.unmark(hold.mark);
            state.queued.extend(parked);
        }
    }

    /// §1.3 step 5, unknown outcome: give up the hold; the mark and its parked entries stay for
    /// the retry.
    fn release_unknown(&self, hold: KillReleaseMark) {
        if let Some(mark) = self.state().held_mark(hold) {
            give_back(mark, hold);
        }
    }

    /// The lost epoch was resumed, so no release of `session` committed: a mark that an unknown
    /// outcome left with no holder is cleared as retryable, and its parked entries are queued.
    pub fn release_orphaned(&self, session: GameSessionId) {
        let mut state = self.state();
        let orphaned: Vec<u64> = state
            .releasing
            .iter()
            .filter(|mark| {
                mark.principal.0 == session && mark.holders.is_empty() && !mark.committed
            })
            .map(|mark| mark.id)
            .collect();
        for id in orphaned {
            let parked = state.unmark(id);
            state.queued.extend(parked);
        }
    }

    /// Free `session`'s queued and parked entries whose principal has no live session: the
    /// session is over (`live == None`), or the entry's lease generation is not the live one.
    /// Their marks go with them.
    pub fn discard_gone(&self, session: GameSessionId, live: Option<u64>) {
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

    /// The killed actor of every queued, parked or in-flight entry: the corpse projections the
    /// queue still binds to (Codex 4248192466).
    fn held_targets(&self) -> Vec<ExactActorRef> {
        let state = self.state();
        state
            .in_flight
            .iter()
            .map(|(_, _, target)| *target)
            .chain(state.queued.iter().map(|e| e.target))
            .chain(
                state
                    .releasing
                    .iter()
                    .flat_map(|mark| mark.parked.iter().map(|(e, _)| e.target)),
            )
            .collect()
    }

    pub fn has_session(&self, session: GameSessionId) -> bool {
        self.state()
            .queued
            .iter()
            .any(|e| e.facts.principal.session == session)
    }

    /// One critical section: the first entry of `session` at `lease_generation` reserves its
    /// planned loot MINTs and moves in flight. A plan error reserves nothing; the settle then
    /// reports it.
    pub fn take(&self, session: GameSessionId, lease_generation: u64) -> TakeOutcome {
        let mut state = self.state();
        let Some(index) = state.queued.iter().position(|e| {
            e.facts.principal.session == session
                && e.facts.principal.lease_generation == lease_generation
        }) else {
            return TakeOutcome::Empty;
        };
        let entry = &state.queued[index];
        let loot_mints = entry.row.as_ref().map_or(0, |row| {
            plan_creature_loot(
                loot_plan_seed(entry.facts.death),
                &row.loot_table_ref,
                &row.loot_table,
            )
            // The flattened plan counts descendants as well as direct corpse entries.
            .map_or(0, |plan| plan.entries.len())
                + if entry.creature == "oteryn:creature.the_corruptor" {
                    3
                } else {
                    0
                }
        });
        // A plan that cannot fit even an idle scope must reach settle's terminal
        // capacity refusal, rather than wait forever for impossible headroom.
        let loot_mints = if check_inflight_loot_mint_capacity(0, loot_mints).is_ok() {
            loot_mints
        } else {
            0
        };
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
            .push((entry.facts.death, principal_of(&entry), entry.target));
        TakeOutcome::Taken(Box::new(TakenKillSettlement {
            queue: self.clone(),
            death: entry.facts.death,
            entry: Some(entry),
            loot_mints,
            inflight_before,
        }))
    }

    /// W22c: reserve `extra` more loot MINTs for the in-flight `death`. `false` reserves
    /// nothing: the death is not in flight or the scope has no room for them now.
    pub fn reserve_extra_loot_mints(
        &self,
        death: CreatureDeathOccurrenceKey,
        extra: usize,
    ) -> bool {
        let mut state = self.state();
        if !state.in_flight.iter().any(|(held, _, _)| *held == death)
            || check_inflight_loot_mint_capacity(state.reserved_loot_mints, extra).is_err()
        {
            return false;
        }
        state.reserved_loot_mints += extra;
        state.extra_loot_mints.push((death, extra));
        true
    }

    /// The number of releases holding `principal`'s mark.
    #[cfg(test)]
    pub fn holders(&self, principal: Principal) -> Option<usize> {
        self.state().mark(principal).map(|mark| mark.holders.len())
    }

    /// Every hold slot still counted on any mark.
    #[cfg(test)]
    pub fn hold_slots(&self) -> Vec<u64> {
        let state = self.state();
        let mut slots: Vec<u64> = state
            .releasing
            .iter()
            .flat_map(|mark| mark.holders.iter().copied())
            .collect();
        slots.sort_unstable();
        slots
    }

    /// The parked entries' phases under `principal`'s mark, and the mark's phase.
    #[cfg(test)]
    pub fn parked(&self, principal: Principal) -> Option<(ReleasePhase, Vec<ReleasePhase>)> {
        let mut state = self.state();
        state.mark(principal).map(|mark| {
            (
                mark.phase,
                mark.parked.iter().map(|(_, phase)| *phase).collect(),
            )
        })
    }

    #[cfg(test)]
    pub fn counts(&self) -> (usize, usize, usize) {
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
pub struct TakenKillSettlement {
    queue: KillSettlementQueue,
    death: CreatureDeathOccurrenceKey,
    entry: Option<PendingKillSettlement>,
    loot_mints: usize,
    inflight_before: usize,
}

impl TakenKillSettlement {
    pub fn entry(&self) -> Option<&PendingKillSettlement> {
        self.entry.as_ref()
    }

    pub const fn inflight_before(&self) -> usize {
        self.inflight_before
    }

    /// The settle reached a final result: the entry leaves the queue.
    pub fn finish(mut self) {
        self.entry = None;
    }

    /// The settle's outcome is unknown: the entry returns to the front of the queue.
    pub fn requeue(self) {}

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
        let death = self.death;
        let extra: usize = state
            .extra_loot_mints
            .iter()
            .filter(|(held, _)| *held == death)
            .map(|(_, extra)| extra)
            .sum();
        state.extra_loot_mints.retain(|(held, _)| *held != death);
        state.reserved_loot_mints = state.reserved_loot_mints.saturating_sub(extra);
        state.in_flight.retain(|(death, _, _)| *death != self.death);
        if let Some(entry) = self.entry.take() {
            state.queued.push_front(entry);
        }
    }
}

/// Where a lethal arm appends: the Channel's queue and the active generation's reward rows.
#[derive(Clone, Copy)]
pub struct KillSink<'a> {
    pub prey: Option<&'a super::actor_spell::ChannelSpellStates>,
    pub activity: &'a [oteryn_gameplay_social_rules::shared_experience::SharedExperienceActivity],
    pub queue: &'a KillSettlementQueue,
    pub rewards: &'a CreatureRewardTable,
    /// The imported Charm catalogue the loot owner's Gut Charm resolves in (W22b).
    pub charms: Option<&'a crate::content::charm_source::CanonicalCharmCatalogue>,
}

/// Give back `hold`'s slot on its mark; a slot already given back is not counted again.
fn give_back(mark: &mut ReleasingMark, hold: KillReleaseMark) {
    if let Some(index) = mark.holders.iter().position(|slot| *slot == hold.slot) {
        mark.holders.swap_remove(index);
    } else {
        stale_hold(hold);
    }
}

/// A hold acted on a mark it no longer holds: the mark was cleared (an abort, a resumed epoch,
/// a gone principal) or the slot was given back. Nothing changes.
fn stale_hold(hold: KillReleaseMark) {
    debug_assert!(hold.mark != 0, "a hold always names a mark");
    operator_event(&format!(
        "kill_release_stale_hold mark={} slot={}",
        hold.mark, hold.slot
    ));
}

fn refuse(reason: &str, creature: &str) {
    operator_event(&format!(
        "kill_reward_refused reason={reason} creature={creature}"
    ));
}

/// Project `target`'s committed death and append its settle (§1.1–§1.3). `lethal` is the
/// attacker whose committed hit was lethal; it is the principal only when no contributor was
/// tracked. Runs under the runtime lock and never awaits.
pub fn record_creature_kill(
    runtime: &mut ChannelRuntimeV1,
    sink: KillSink<'_>,
    target: ExactActorRef,
    lethal: CapturedRewardPrincipal,
    now_ms: u64,
    observed_at: Option<oteryn_simulation_determinism::SemanticTimeMicros>,
) {
    let snapshot = runtime.companion_snapshot_including_dead(target);
    let forge_kind = snapshot
        .as_ref()
        .ok()
        .filter(|s| s.state.master.is_none())
        .and_then(|s| s.forge.zip(s.state.policy.forge_difficulty));
    let creature = snapshot.map(|s| s.state.policy.definition_key.clone());
    let pin = runtime.content_pin();
    let hex = |bytes: [u8; 32]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let ground = DeathGroundContext {
        boss_spawn: runtime
            .boss_spawns()
            .iter()
            .find(|s| s.actor == Some(target))
            .map(|s| (s.key.clone(), s.cycle, s.policy)),
        map_revision: format!("sha256:{}", hex(pin.map_revision_digest())),
        content_revision: format!("sha256:{}", hex(pin.server_artifact_digest())),
        ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
        sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
        native_room_placement_context: pin.frame_binding_digest().to_vec(),
    };
    let daily_boost = runtime.daily_boost().cloned();
    // Freeze the principal's Prey boost on the death turn, before any command can reroll it.
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
        // The lethal session carries proficiency through the queue; XP keeps its refusal.
        refuse("principal_gone", &creature);
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
    let reward = sink.rewards.row(&creature);
    let Ok(mut facts) =
        capture_projected_death_facts(&mut owner, target, principal, last_damage_at_ms, now_ms)
    else {
        return;
    };
    if untracked {
        // The lethal hit is the untracked principal's own clocked damage.
        facts.principal_last_damage_at_ms = Some(facts.death_at_ms);
    }
    let mut row = match reward {
        Ok(row) => row,
        Err(reason) => {
            let Some(boss) = sink.rewards.boss(&creature) else {
                refuse(reason.as_str(), &creature);
                return;
            };
            let entry = PendingKillSettlement::for_bosstiary(
                facts,
                boss.clone(),
                ground,
                target,
                observed_at,
            );
            accept_kill(runtime, sink.queue, entry);
            return;
        }
    };
    if let Some(choice) = daily_boost {
        let boosted_creature = creature == choice.creature;
        let boosted_boss = creature == choice.boss;
        if boosted_creature {
            let Ok(base) = u64::try_from(row.xp_amount.get()) else {
                return;
            };
            let Ok(amount) = crate::content::daily_boost::apply_daily_boost(
                base,
                &creature,
                &choice.creature,
                (2, 1),
            )
            .and_then(|v| {
                i64::try_from(v).map_err(|_| crate::content::daily_boost::DailyBoostError::Overflow)
            }) else {
                refuse("daily_boost_xp_overflow", &creature);
                return;
            };
            Arc::make_mut(&mut row).xp_amount =
                oteryn_simulation_determinism::ExactI64::new(amount);
        }
        if boosted_creature || boosted_boss {
            let row = Arc::make_mut(&mut row);
            if row.loot_table.algorithm
                == crate::combat::LootSelectionAlgorithm::IndependentBernoulliPpm
            {
                // Canary's default boss bonus is +250%: two additional rolls and a
                // deterministic third using the Reference inclusive 0..=100 draw.
                let extra_rolls = if boosted_boss {
                    use sha2::{Digest, Sha256};
                    let mut hash = Sha256::new();
                    hash.update(b"oteryn:daily-boost:boss-roll:v1");
                    hash.update(facts.occurrence);
                    let digest = hash.finalize();
                    let draw = u64::from_be_bytes([
                        digest[0], digest[1], digest[2], digest[3], digest[4], digest[5],
                        digest[6], digest[7],
                    ]);
                    2 + u8::from(draw % 101 < 50)
                } else {
                    1
                };
                let boosted_entries = if boosted_boss {
                    // Boss bonus rerolls equipment only, never gold, creature products or
                    // other consumables. Stable partition keeps replay ordinals deterministic.
                    row.loot_table.entries.sort_by_key(|entry| {
                        !sink
                            .rewards
                            .is_daily_boss_equipment(&entry.item.production_key)
                    });
                    row.loot_table
                        .entries
                        .iter()
                        .take_while(|entry| {
                            sink.rewards
                                .is_daily_boss_equipment(&entry.item.production_key)
                        })
                        .count()
                } else {
                    row.loot_table.entries.len()
                };
                row.loot_table.algorithm = crate::combat::LootSelectionAlgorithm::DailyBoosted {
                    extra_rolls,
                    boosted_entries,
                };
            }
        }
    }
    if let Some((crate::domain::forge_creature::ForgeCreature::Fiendish, _difficulty)) = forge_kind
    {
        let Some(item) = sink.rewards.forge_sliver() else {
            refuse("forge_sliver_inadmissible", &creature);
            return;
        };
        let count = u32::from(crate::domain::forge_creature::slivers(facts.occurrence[14]));
        if count > 0 {
            Arc::make_mut(&mut row)
                .loot_table
                .entries
                .push(crate::combat::LootTableEntry {
                    item: item.clone(),
                    min_count: count,
                    max_count: count,
                    probability_ppm: Some(1_000_000),
                    details: Default::default(),
                });
        }
    }
    if let Some((kind, _)) = forge_kind {
        let Some(amount) = kind.experience(row.xp_amount.get()) else {
            refuse("forge_xp_overflow", &creature);
            return;
        };
        Arc::make_mut(&mut row).xp_amount = oteryn_simulation_determinism::ExactI64::new(amount);
    }
    let Ok(damage_shares) = owner.reward_damage_shares(target) else {
        refuse("damage_shares_unavailable", &creature);
        return;
    };
    sink.queue.freeze_proficiency(
        runtime,
        target,
        facts.occurrence,
        &damage_shares,
        top.is_gone(),
    );
    if let Some(effects) =
        super::hazard::effects(runtime, principal.actor, principal.session, target)
    {
        let Ok(base) = u64::try_from(row.xp_amount.get()) else {
            return;
        };
        let Ok(amount) = effects.experience(base).and_then(|value| {
            i64::try_from(value).map_err(|_| {
                oteryn_gameplay_encounters::bosses::hazard_primal::HazardError::Overflow
            })
        }) else {
            return;
        };
        Arc::make_mut(&mut row).xp_amount = oteryn_simulation_determinism::ExactI64::new(amount);
        let entropy = u64::from_be_bytes(facts.occurrence[..8].try_into().unwrap_or_default());
        let draw = crate::domain::uniform_draw::uniform_draw(entropy, 0, 100) as u32;
        for _ in 0..effects.extra_loot_rolls(draw) {
            let extra = row.loot_table.entries.clone();
            Arc::make_mut(&mut row).loot_table.entries.extend(extra);
        }
    }
    let percent = sink.prey.map_or(0, |states| {
        states.prey_percent(
            runtime,
            principal.actor,
            principal.session,
            &creature,
            4,
            now_ms * 1000,
        )
    });
    let percent = sink.queue.prey_at_death(runtime, target, percent);
    // One independent trigger plus two <=7-entry rolls stay within PREY-0 R4's
    // 29 draws; a forge sliver adds two more, still below the existing 32 ceiling.
    let base = sink
        .rewards
        .row(&creature)
        .ok()
        .filter(|base| base.loot_table.entries.len() <= 7);
    if percent > 0
        && let Some(base) = base
    {
        use sha2::Digest as _;
        let roll = sha2::Sha256::digest(
            [
                b"oteryn.prey.loot.v1".as_slice(),
                facts.occurrence.as_slice(),
            ]
            .concat(),
        );
        let value = u64::from_be_bytes(roll[..8].try_into().unwrap_or([0; 8]));
        let draw = ((u128::from(value) * 100) >> 64) as u8;
        if draw < percent {
            Arc::make_mut(&mut row)
                .loot_table
                .entries
                .extend(base.loot_table.entries.iter().cloned());
        }
    }
    let activity_at_death = match super::party_xp::capture_activity_at_death(
        runtime,
        principal.character,
        &damage_shares,
        sink.activity,
    ) {
        Ok(cells) => cells,
        Err(_) => {
            refuse("activity_census_unavailable", &creature);
            return;
        }
    };
    let forge = forge_kind.and_then(|(kind, _difficulty)| {
        let origin = runtime.read_actor_position(target).ok()?.position();
        let census = runtime.positioned_actor_census().ok()?;
        let mut involved = std::collections::BTreeSet::new();
        involved.insert(*principal.character.as_bytes());
        involved.extend(damage_shares.iter().map(|d| *d.character.as_bytes()));
        for (actor, _, session) in &census {
            let Some(session) = session else { continue };
            if runtime
                .bound_attacker_lease(*actor, *session)
                .is_ok_and(|lease| involved.contains(lease.character_id().as_bytes()))
                && let Ok(Some(party)) = runtime.cached_world_party(*actor, *session)
                && party.is_well_formed()
            {
                involved.extend(party.members.iter().map(|m| m.character));
            }
        }
        let recipients: Vec<_> = census
            .into_iter()
            .filter_map(|(actor, at, session)| {
                let session = session?;
                let lease = runtime.bound_attacker_lease(actor, session).ok()?;
                involved
                    .contains(lease.character_id().as_bytes())
                    .then_some((actor, session, at.position()))
            })
            .take(51)
            .collect();
        if recipients.len() > 50 {
            return None;
        }
        Some(ForgeDeath {
            kind,
            roll: facts.occurrence[15],
            origin,
            recipients,
        })
    });
    // W22b: the loot owner's Gut Charm on this race widens every creature-product roll, the
    // extra Prey, daily boost and hazard rolls included.
    let gut = sink.prey.zip(sink.charms).map_or(0, |(states, charms)| {
        super::attack::charm_combat::gut_bonus_hundredths(
            states,
            runtime,
            charms,
            principal.actor,
            principal.session,
            &creature,
            &oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
                runtime.content_pin().server_artifact_digest(),
            ),
            oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(facts.occurrence),
        )
    });
    if gut > 0 {
        apply_gut_to_loot(&mut Arc::make_mut(&mut row).loot_table.entries, gut);
    }
    if top.is_gone() {
        // A proficiency-only entry reserves no loot MINT capacity.
        Arc::make_mut(&mut row).loot_table.entries.clear();
    }
    let entry = PendingKillSettlement {
        forge,
        activity_observed_at: observed_at.filter(|at| at.get() / 1000 == facts.death_at_ms),
        activity_at_death,
        damage_shares,
        facts,
        creature,
        row: Some(row),
        bosstiary_only: None,
        ground,
        target,
        no_progression_logged: false,
    };
    accept_kill(runtime, sink.queue, entry);
}

/// Append `entry` to `queue`. Once the queue accepts it (queued or parked), pin its killed
/// actor's corpse projection, so no later death evicts it before the settle binds the committed
/// corpse (Codex 4248192466). A duplicate keeps the pin its first append took; a kill the full
/// queue refuses pins nothing. Runs under the runtime lock and the `attack` guard.
fn accept_kill(
    runtime: &mut ChannelRuntimeV1,
    queue: &KillSettlementQueue,
    entry: PendingKillSettlement,
) -> AppendOutcome {
    let target = entry.target;
    let creature = entry.creature.clone();
    let outcome = queue.append(entry);
    match &outcome {
        AppendOutcome::Queued | AppendOutcome::Parked(_) => {
            // The pins become those of the held entries, at most KILLRW-RL-01, so this one
            // fits: an entry freed without a settle (`principal_gone`) loses its pin, and one
            // requeued after its final settle released it (a cancelled drain) regains it.
            let held = queue.held_targets();
            runtime.retain_corpse_projection_pins(&held);
            for actor in held {
                if let Err(error) = runtime.pin_corpse_projection(actor)
                    && actor == target
                {
                    operator_event(&format!(
                        "kill_reward_corpse_unpinned creature={creature} reason={error:?}"
                    ));
                }
            }
        }
        AppendOutcome::Duplicate => {}
        AppendOutcome::Full => refuse("queue_full", &creature),
    }
    outcome
}

/// Codex 4248192466: a final settle releases the pin its acceptance took, after its corpse bind.
/// A Retry requeues the entry, which keeps the pin.
fn release_settled_corpse(
    runtime: &mut ChannelRuntimeV1,
    entry: &PendingKillSettlement,
    verdict: SettleVerdict,
) {
    if verdict == SettleVerdict::Final {
        runtime.unpin_corpse_projection(entry.target);
    }
}

/// §1.4: every lethal effect of a committed or replayed spell batch.
pub fn record_batch_kills(
    runtime: &mut ChannelRuntimeV1,
    sink: KillSink<'_>,
    receipt: &CombatBatchReceipt,
    now_ms: u64,
) {
    let observed_at = runtime.committed_spell_batch_time(receipt).ok().flatten();
    let now_ms = observed_at.map_or(now_ms, |at| at.get() / 1_000);
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
        record_creature_kill(runtime, sink, effect.target, lethal, now_ms, observed_at);
    }
}

/// Whether a settle reached its final result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettleVerdict {
    Final,
    /// The outcome is unknown: the entry is requeued and the pass stops.
    Retry,
}

/// One settle's result as the drain needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SettleReport {
    pub verdict: SettleVerdict,
    /// §1.5: XP and Bestiary found no progression binding.
    pub no_progression: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrainOutcome {
    Drained,
    /// An entry's outcome is unknown or its MINTs wait for capacity; it stays queued.
    Deferred,
}

/// The durable half of a drain. Called with no channel guard held.
pub trait KillSettle {
    async fn settle(&self, entry: &PendingKillSettlement, inflight_before: usize) -> SettleReport;
}

/// Settle `session`'s entries at `lease_generation` one at a time. The `attack` guard is held
/// only to take and to finish or requeue an entry, never across `settler`.
pub async fn drain_session_kills(
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

/// The result of [`KillSettlementQueue::seal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Seal {
    /// The principal is marked `releasing`, and the caller holds the mark.
    Sealed(KillReleaseMark),
    /// An entry of the principal was appended after the drain's last take.
    Queued,
    /// Another drain is settling an entry of the principal.
    InFlight,
}

/// The end of a release handshake's steps 1–4 (§1.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KillRelease {
    /// Send the transaction, then report its outcome with [`KillReleaseMark::settle`]. No mark
    /// is held when the session is already terminal.
    Send(Option<KillReleaseMark>),
    /// Step 1 met an unknown outcome or a `capacity_wait`: nothing is marked and the
    /// transaction must not be sent.
    Deferred,
}

/// The terminal release transaction's outcome as step 5 reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseEnd {
    /// `session_state = 3`.
    Committed,
    /// The session did not end.
    Retryable,
    /// The outcome is unknown: the mark and the parked entries stay for the next attempt.
    Unknown,
}

/// One release's hold on a principal's mark: the mark's identity and the release's own slot,
/// from [`KillSettlementQueue::seal`]. Every later step presents it, and acts only on that mark
/// and slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KillReleaseMark {
    principal: Principal,
    mark: u64,
    slot: u64,
}

impl KillReleaseMark {
    /// §1.3 step 5 of the release holding the mark, in one `attack` critical section. Called
    /// once per [`release_handshake`] that returned the mark.
    pub fn settle(self, queue: &KillSettlementQueue, end: ReleaseEnd) {
        match end {
            ReleaseEnd::Committed => queue.release_committed(self, true),
            ReleaseEnd::Retryable => queue.release_failed(self, true),
            ReleaseEnd::Unknown => queue.release_unknown(self),
        }
    }

    /// Step 5 from a durable reconcile after the hold was given up to an unknown outcome.
    pub fn reconcile(self, queue: &KillSettlementQueue, end: ReleaseEnd) {
        match end {
            ReleaseEnd::Committed => queue.release_committed(self, false),
            ReleaseEnd::Retryable => queue.release_failed(self, false),
            ReleaseEnd::Unknown => {}
        }
    }

    /// The released actor slot is removed.
    pub fn forget(self, queue: &KillSettlementQueue) {
        queue.forget_released(self);
    }
}

/// §1.3 steps 1–4: drain, seal, then move the mark to `committing`, returning to the drain
/// while the seal finds a queued entry or a park in phase `sealed` aborts the commit point.
/// Each return needs a new kill for this principal, so the loop advances with the game. An
/// entry another drain is settling defers the release, which the caller retries behind its
/// backoff instead of waiting on that settle. No guard is held across a settle or on return.
pub async fn release_handshake(
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
        let hold = match attack.lock().await.kills().seal(principal) {
            Seal::Sealed(hold) => hold,
            Seal::Queued => continue,
            Seal::InFlight => return KillRelease::Deferred,
        };
        if attack.lock().await.kills().commit_point(hold) {
            return KillRelease::Send(Some(hold));
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
            | CombatDeathRewardLootError::Mint(ItemMintError::Unavailable(_), _)
    )
}
/// The result of [`bind_settled_corpse`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CorpseBind {
    /// The projected corpse names the committed corpse Item (or already did).
    Bound,
    /// The settle committed no corpse (its plan, capacity or corpse MINT was refused), so there
    /// is nothing to bind.
    NoCorpse,
    /// The Channel refused the binding; the durable Items are unchanged.
    Refused(CarrierError),
}
/// MAP-ITEM-REF-1 Part B (D607): after the settle commits the corpse Ground MINT, bind that Item
/// to the killed actor's committed corpse projection, so domain 1 shows the corpse and USE and
/// command 9 reach its contents. Runtime memory only: it writes nothing durable, and a replayed
/// settle carries the same committed Item, which binds idempotently. The projection is per
/// generation (D52), so a corpse of another world, Channel or scope generation is refused.
pub fn bind_settled_corpse(
    runtime: &mut ChannelRuntimeV1,
    entry: &PendingKillSettlement,
    loot: &Result<CommittedCorpseLoot, CombatDeathRewardLootError>,
) -> CorpseBind {
    // A loot entry's refused MINT still leaves the corpse committed (D52).
    let committed = match loot {
        Ok(loot) => &loot.corpse,
        Err(CombatDeathRewardLootError::Mint(_, corpse)) => corpse,
        Err(_) => return CorpseBind::NoCorpse,
    };
    let Some(row) = &entry.row else {
        return CorpseBind::NoCorpse;
    };
    let corpse = &row.corpse_item;
    match runtime.bind_corpse_item(
        entry.target,
        committed.item_instance_id,
        &corpse.family,
        &corpse.production_key,
        &corpse.revision_ref,
    ) {
        Ok(()) => CorpseBind::Bound,
        Err(error) => CorpseBind::Refused(error),
    }
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
    if retry
        || matches!(
            outcome.bosstiary,
            Err(CharacterProgressionError::Unavailable(_))
        )
    {
        return report(SettleVerdict::Retry, no_progression);
    }
    if let Err(error) = &rewards.loot {
        operator_event(&format!(
            "kill_reward_loot_failed creature={creature} reason={error:?}"
        ));
    }
    if let Err(error) = &rewards.xp
        && !no_progression
        && !matches!(error, CombatDeathRewardXpError::NoExperienceAward)
    {
        operator_event(&format!(
            "kill_reward_xp_failed creature={creature} reason={error:?}"
        ));
    }
    if let Err(error) = &outcome.bosstiary {
        operator_event(&format!(
            "kill_reward_bosstiary_failed creature={creature} reason={error:?}"
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
    /// The active generation's reward rows; empty without an active native gameplay pin, so
    /// every kill then refuses `no_loot_binding`.
    pub(super) fn reward_table(&self) -> &CreatureRewardTable {
        static EMPTY: std::sync::LazyLock<CreatureRewardTable> =
            std::sync::LazyLock::new(CreatureRewardTable::default);
        self.active_generation
            .and_then(|generation| generation.native_gameplay())
            .map_or(&EMPTY, |gameplay| gameplay.reward_table())
    }
    /// §1.4: record the kills of committed or replayed spell batches, under the caller's
    /// `runtime` and `spell_states` guards; `attack` is locked after them.
    pub(super) async fn record_spell_kills<'r>(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &super::actor_spell::ChannelSpellStates,
        receipts: impl Iterator<Item = &'r CombatBatchReceipt>,
    ) {
        let attack = self.states.attack.lock().await;
        self.record_spell_kills_locked(&attack, states, runtime, receipts);
    }
    /// [`Self::record_spell_kills`] under an attack-state guard the caller already holds, so a
    /// writer can take the guard before it releases its cancellation-safe attempt.
    pub(super) fn record_spell_kills_locked<'r>(
        &self,
        attack: &ChannelAttackStates,
        states: &super::actor_spell::ChannelSpellStates,
        runtime: &mut ChannelRuntimeV1,
        receipts: impl Iterator<Item = &'r CombatBatchReceipt>,
    ) {
        let activity = attack.shared_experience_cells();
        let sink = KillSink {
            prey: Some(states),
            activity: &activity,
            queue: attack.kills(),
            rewards: self.reward_table(),
            charms: Some(self.imported_charms),
        };
        let now_ms = self.owner_now().get() / 1_000;
        for receipt in receipts {
            record_batch_kills(runtime, sink, receipt, now_ms);
        }
    }
    /// Settle `session`'s queued kills (§1.3), after every channel guard is released. An entry
    /// whose principal has no live session is freed with `principal_gone`.
    pub(super) async fn drain_kill_rewards(&self, session: GameSessionId) -> DrainOutcome {
        if !self.states.attack.lock().await.kills().has_session(session) {
            return DrainOutcome::Drained;
        }
        let fence = match self.current_quest_fence(session).await {
            Ok(Some(fence)) => fence,
            Ok(None) => {
                self.states
                    .attack
                    .lock()
                    .await
                    .kills()
                    .discard_gone(session, None);
                return DrainOutcome::Drained;
            }
            Err(()) => return DrainOutcome::Deferred,
        };
        self.states
            .attack
            .lock()
            .await
            .kills()
            .discard_gone(session, Some(fence.character_lease_generation));
        drain_session_kills(
            &self.states.attack,
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
                self.states
                    .attack
                    .lock()
                    .await
                    .kills()
                    .discard_gone(session, None);
                return KillRelease::Send(None);
            }
            Err(()) => return KillRelease::Deferred,
        };
        let lease_generation = fence.character_lease_generation;
        self.states
            .attack
            .lock()
            .await
            .kills()
            .discard_gone(session, Some(lease_generation));
        release_handshake(
            &self.states.attack,
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
            mark.settle(self.states.attack.lock().await.kills(), end);
        }
    }
    /// Step 5 of a durable reconcile after an unknown outcome gave up the hold.
    pub(super) async fn reconcile_kill_release(
        &self,
        mark: Option<KillReleaseMark>,
        end: ReleaseEnd,
    ) {
        if let Some(mark) = mark {
            mark.reconcile(self.states.attack.lock().await.kills(), end);
        }
    }
    /// The lost epoch was resumed: clear `session`'s marks left by unknown outcomes.
    pub(super) async fn release_orphaned_kills(&self, session: GameSessionId) {
        self.states
            .attack
            .lock()
            .await
            .kills()
            .release_orphaned(session);
    }
    /// The released actor slot is removed: its mark goes.
    pub(super) async fn forget_kill_release(&self, mark: Option<KillReleaseMark>) {
        if let Some(mark) = mark {
            mark.forget(self.states.attack.lock().await.kills());
        }
    }
}
struct DurableKillSettle<'s, 'a, 'f, 'g> {
    admission: &'s ComposedFreshAdmission<'a, 'f, 'g>,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
}
impl KillSettle for DurableKillSettle<'_, '_, '_, '_> {
    async fn settle(&self, entry: &PendingKillSettlement, inflight_before: usize) -> SettleReport {
        Box::pin(self.settle_durably(entry, inflight_before)).await
    }
}
impl DurableKillSettle<'_, '_, '_, '_> {
    /// W22c: the corpse owner's live Wealth Duplex rolls the creature's base loot once more,
    /// without Gut (Crystal ff7ede59 `ondroploot_wealth_duplex.lua:61-63`, `gut = false`). The
    /// extra MINTs are reserved first; `false` waits for scope room. A roll that cannot fit even
    /// an idle scope is left out, so the kill still settles.
    async fn wealth_duplex(
        &self,
        entry: &PendingKillSettlement,
        row: &CreatureRewardRow,
        base: &CreatureRewardRow,
        loot_table: &mut oteryn_gameplay_items::loot_plan::LootTableDefinition,
        inflight_before: usize,
    ) -> bool {
        let entropy = u64::from_be_bytes(
            entry.facts.death.world_id().as_bytes()[8..]
                .try_into()
                .unwrap_or_default(),
        ) ^ entry.facts.death.actor_local_generation();
        let draw =
            u8::try_from(crate::domain::uniform_draw::uniform_draw(entropy, 0, 100)).unwrap_or(100);
        let rolls = oteryn_gameplay_items::loot_plan::wealth_duplex_extra_rolls(1, draw);
        let mut extended = loot_table.clone();
        for _ in 0..rolls {
            let mut extra = base.loot_table.entries.clone();
            apply_gut_to_loot(&mut extra, 0);
            extended.entries.extend(extra);
        }
        let seed = loot_plan_seed(entry.facts.death);
        let count = |table| {
            plan_creature_loot(seed, &row.loot_table_ref, table)
                .map_or(0, |plan| plan.entries.len())
        };
        let (before, after) = (count(loot_table), count(&extended));
        let extra = after.saturating_sub(before);
        if check_inflight_loot_mint_capacity(0, after).is_err() {
            return true;
        }
        if extra > 0
            && (check_inflight_loot_mint_capacity(inflight_before, after).is_err()
                || !self
                    .admission
                    .states
                    .attack
                    .lock()
                    .await
                    .kills()
                    .reserve_extra_loot_mints(entry.facts.death, extra))
        {
            return false;
        }
        *loot_table = extended;
        true
    }
    /// The durable settle of one entry. Its future holds the principal's progression binding,
    /// whose policy carries the whole `CHARACTER_EXPERIENCE_TABLE_LEVELS` threshold table by
    /// value, across the reward writes, so [`KillSettle::settle`] keeps it on the heap: inline,
    /// it would grow every future that drains kills by several copies of that table.
    async fn settle_durably(
        &self,
        entry: &PendingKillSettlement,
        inflight_before: usize,
    ) -> SettleReport {
        let admission = self.admission;
        if let Some(boss) = &entry.bosstiary_only {
            let Some(_permit) = admission.spell_lane_permit().await else {
                return SettleReport {
                    verdict: SettleVerdict::Retry,
                    no_progression: false,
                };
            };
            let mut slot = admission
                .revision_sequencer
                .acquire(self.fence.character_id)
                .await;
            let session = DurabilitySession {
                root: admission.root,
                authority: admission.character,
                node: admission.holder,
            };
            let principal = RewardPrincipal {
                gameplay_fence: self.fence,
            };
            let result = crate::combat::settle_bosstiary(
                &session,
                &mut slot,
                &entry.facts,
                &principal,
                Some(boss),
            )
            .await;
            if matches!(result, Err(CharacterProgressionError::Unavailable(_))) {
                return SettleReport {
                    verdict: SettleVerdict::Retry,
                    no_progression: false,
                };
            }
            if matches!(result, Ok(true)) {
                admission.states.cyclopedia_tracker_refresh.mark(
                    self.fence.character_id,
                    TrackerKind::Bosstiary,
                    &boss.key,
                );
            }
            if let Err(error) = result {
                operator_event(&format!(
                    "kill_reward_bosstiary_failed creature={} reason={error:?}",
                    entry.creature
                ));
            }
            return SettleReport {
                verdict: SettleVerdict::Final,
                no_progression: false,
            };
        }
        let proficiency_only = admission
            .states
            .attack
            .lock()
            .await
            .kills()
            .state()
            .proficiency_deaths
            .iter()
            .any(|death| death.occurrence == entry.facts.occurrence && death.reward_principal_gone);
        if proficiency_only {
            admission.credit_weapon_proficiency_kill(entry).await;
            release_settled_corpse(
                &mut *admission.runtime.lock().await,
                entry,
                SettleVerdict::Final,
            );
            return SettleReport {
                verdict: SettleVerdict::Final,
                no_progression: false,
            };
        }
        // Existing fenced equipment is authoritative. Read it before the spell lane and
        // revision slot; an unavailable snapshot fails closed rather than inventing ownership.
        let talisman_equipped =
            super::equipment_move::read_current_equipment(admission, self.fence.game_session_id)
                .await
                .is_some_and(|equipment| {
                    equipment.items.iter().any(|item| {
                        item.slot == 10
                            && item.definition.production_key
                                == oteryn_gameplay_quests::quest::task_board::TALISMAN_ITEM
                    })
                });
        let mut entry = entry.clone();
        let Some(mut row) = entry.row.take() else {
            return SettleReport {
                verdict: SettleVerdict::Final,
                no_progression: false,
            };
        };
        let masteries = admission
            .root
            .read_animus_masteries(admission.character, self.fence.character_id)
            .await
            .unwrap_or_default();
        let mastered = masteries.iter().any(|key| key == &entry.creature);
        let base_xp = u64::try_from(row.xp_amount.get()).unwrap_or(0);
        let animus_xp = animus_mastery_experience(base_xp, masteries.len(), mastered);
        // W05d: the scheduled event experience rate (Crystal `player.lua:428-429`) applies to the
        // kill's experience before the party split, so solo and party kills both carry it.
        let event_rates = oteryn_gameplay_foundation::scheduled_events::current_rates();
        let event_xp = oteryn_gameplay_foundation::scheduled_events::scale_gain(
            animus_xp,
            event_rates.experience,
        );
        Arc::make_mut(&mut row).xp_amount = oteryn_simulation_determinism::ExactI64::new(
            i64::try_from(event_xp).unwrap_or(i64::MAX),
        );
        entry.row = Some(Arc::clone(&row));
        // Lock order (SPELL-LOCK-2 §1.2): the lane before the revision slot; the corpse loot
        // mint is a key-33 writer. `None` retries with no side effect while the lane is fenced.
        let Some(permit) = admission.spell_lane_permit().await else {
            return SettleReport {
                verdict: SettleVerdict::Retry,
                no_progression: false,
            };
        };
        let party = if admission
            .progression_binding(self.fence.game_session_id)
            .is_some()
        {
            match admission.settle_party_experience(&permit, &entry).await {
                Ok(party) => Some(party),
                Err(error) => {
                    operator_event(&format!("party_xp_settle_deferred reason={error:?}"));
                    Some(super::party_kill_settlement::SettledPartyXp {
                        xp: Err(CombatDeathRewardXpError::Progression(error)),
                        slots: Vec::new(),
                        leveled: Vec::new(),
                    })
                }
            }
        } else {
            None
        };
        let (party_xp, mut other_slots, mut leveled) = party
            .map_or((None, Vec::new(), Vec::new()), |p| {
                (Some(p.xp), p.slots, p.leveled)
            });
        let mut slot = if let Some(index) = other_slots
            .iter()
            .position(|s| s.character_id() == self.fence.character_id)
        {
            other_slots.remove(index)
        } else {
            admission
                .revision_sequencer
                .acquire(self.fence.character_id)
                .await
        };
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
        let Ok(mut loot_table) = admission.tutorial_corruptor_table(fence, &entry).await else {
            return SettleReport {
                verdict: SettleVerdict::Retry,
                no_progression: false,
            };
        };
        if let Some(race) = &row.race
            && let Some(tracks) = admission.task_board_tracks(self.fence.game_session_id)
        {
            let bonus = oteryn_gameplay_quests::quest::task_board::active_talisman_bonus(
                &tracks,
                race.key(),
                talisman_equipped,
                2,
            );
            apply_talisman_loot_bonus(&mut loot_table, bonus);
        }
        // The captured reward principal is Global's highest-damage loot owner. Read its live
        // session value only after the current fence has been requalified; party members' stamina
        // affects their XP, not ownership of this corpse's loot.
        let live_stamina = admission
            .states
            .spell_states
            .lock()
            .await
            .live_stamina(entry.facts.principal.actor, entry.facts.principal.session);
        apply_hunting_stamina_to_loot(&mut loot_table, live_stamina);
        // W05d: the scheduled event loot rate scales each drop chance (Crystal
        // `functions.lua:68-70`), capped at certainty.
        for loot in &mut loot_table.entries {
            if let Some(probability) = &mut loot.probability_ppm {
                *probability = u32::try_from(
                    oteryn_gameplay_foundation::scheduled_events::scale_gain(
                        u64::from(*probability),
                        event_rates.loot,
                    )
                    .min(1_000_000),
                )
                .unwrap_or(1_000_000);
            }
        }
        if !loot_table.entries.is_empty() {
            let Ok(active) = admission
                .root
                .wealth_duplex_active(admission.character, self.fence.character_id)
                .await
            else {
                return SettleReport {
                    verdict: SettleVerdict::Retry,
                    no_progression: false,
                };
            };
            if active
                && let Ok(base) = admission.reward_table().row(&entry.creature)
                && !self
                    .wealth_duplex(&entry, &row, &base, &mut loot_table, inflight_before)
                    .await
            {
                return SettleReport {
                    verdict: SettleVerdict::Retry,
                    no_progression: false,
                };
            }
        }
        let input = CreatureDeathRewardInput::<CHARACTER_EXPERIENCE_TABLE_LEVELS> {
            corpse_item: row.corpse_item.clone(),
            loot_table_ref: row.loot_table_ref.clone(),
            loot_table,
            ground: entry.ground.clone(),
            inflight_loot_mints_before_this_death: inflight_before,
            reward_principals: vec![RewardPrincipal {
                gameplay_fence: fence,
            }],
            xp_amount: row.xp_amount,
            // ARCH-PROGRESSION-SOURCE-0 §1.4: the principal's own session binding; an unbound
            // session settles corpse and loot only (`no_progression_binding`).
            progression: admission
                .progression_binding(self.fence.game_session_id)
                .map(|binding| (*binding).clone()),
        };
        let session = DurabilitySession {
            root: admission.root,
            authority: admission.character,
            node: admission.holder,
        };
        let bestiary = CreatureDeathBestiaryInput {
            race: row.race.clone(),
            boss: row.boss.clone(),
        };
        let outcome = if let Some(xp) = party_xp {
            settle_creature_death_rewards_with_party_experience(
                &permit,
                entry.facts,
                &session,
                &mut slot,
                input,
                bestiary,
                xp,
            )
            .await
        } else {
            settle_creature_death_rewards_with_bestiary(
                &permit,
                entry.facts,
                &session,
                &mut slot,
                input,
                bestiary,
            )
            .await
        };
        // The runtime lock is first in the lock order, so the lane permit and the revision slot
        // are released before it is taken.
        drop(slot);
        drop(other_slots);
        drop(permit);
        if let Ok(outcome) = &outcome
            && let Ok(xp) = &outcome.rewards.xp
        {
            leveled.extend(super::level_up_live::leveled_character(xp));
        }
        // P12a: completed Bestiary entries can unlock Beastly or a class title even when
        // the race has no individual kill-count title (PR #2782 Q2).
        let titled = row.race.as_ref().is_some_and(|race| {
            oteryn_protocol_oteryn::character_info::TRACKED_CREATURES.contains(&race.key())
        }) || row.boss.is_some()
            || outcome.as_ref().is_ok_and(|settled| {
                matches!(&settled.bestiary, Ok(CombatBestiaryOutcome::Recorded(kill))
                    if bestiary_completion_affects_titles(kill))
            });
        admission.states.title_refresh.mark(
            leveled
                .iter()
                .copied()
                .chain(titled.then_some(fence.character_id)),
        );
        admission.states.level_refresh.mark(leveled);
        if let Ok(outcome) = &outcome
            && let Ok(xp) = &outcome.rewards.xp
        {
            admission
                .states
                .soul_kills
                .mark(super::soul_kill_live::soul_hook(xp));
        }
        // P22d: a committed Bestiary or Bosstiary kill refreshes the tracker it is on.
        if let Ok(outcome) = &outcome {
            let marks = &admission.states.cyclopedia_tracker_refresh;
            if let Ok(CombatBestiaryOutcome::Recorded(BestiaryKillOutcome::Committed(kill))) =
                &outcome.bestiary
            {
                marks.mark(kill.character_id, TrackerKind::Bestiary, &kill.race_key);
            }
            if matches!(outcome.bosstiary, Ok(true))
                && let Some(boss) = &row.boss
            {
                marks.mark(self.fence.character_id, TrackerKind::Bosstiary, &boss.key);
            }
        }
        // TASKBOARD-1 §2.4: the settled kill credits the principal's active Task Board task.
        if outcome.is_ok()
            && let Some(race) = &row.race
        {
            admission
                .credit_task_board_kill(self.fence, race.key(), entry.facts.occurrence)
                .await;
        }
        if outcome.is_ok() {
            admission.credit_weapon_proficiency_kill(&entry).await;
        }
        let mut runtime = admission.runtime.lock().await;
        if let Ok(outcome) = &outcome
            && let CorpseBind::Refused(error) =
                bind_settled_corpse(&mut runtime, &entry, &outcome.rewards.loot)
        {
            operator_event(&format!(
                "kill_reward_corpse_unbound creature={} reason={error:?}",
                entry.creature
            ));
        }
        let report = classify(&entry.creature, &outcome);
        release_settled_corpse(&mut runtime, &entry, report.verdict);
        report
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn credit_weapon_proficiency_kill(&self, entry: &PendingKillSettlement) {
        let contributors = self
            .states
            .attack
            .lock()
            .await
            .kills()
            .state()
            .proficiency_deaths
            .iter()
            .find(|death| death.occurrence == entry.facts.occurrence)
            .map(|death| death.contributors.clone())
            .unwrap_or_default();
        let points = proficiency_points(self, entry);
        let mut checkpoints = Vec::new();
        {
            let runtime = self.runtime.lock().await;
            if runtime.owner_fence().is_err() {
                return;
            }
            let Ok(mut sessions) = self.states.proficiencies.lock() else {
                return;
            };
            for contributor in &contributors {
                // A transition fence blocks new attacks, not the acknowledgement of death
                // facts frozen under the contributor's bound lease. The exact actor/session
                // must still exist; the checkpoint independently resolves current DB fences.
                let in_channel = runtime
                    .player_control_facts(contributor.actor, contributor.session)
                    .is_ok();
                let Some(owner) = sessions.get_mut(&contributor.session) else {
                    continue;
                };
                use sha2::{Digest, Sha256};
                let roll = Sha256::new()
                    .chain_update(b"oteryn:daily-proficiency:v2\0")
                    .chain_update(entry.facts.occurrence)
                    .chain_update(contributor.session.as_bytes())
                    .finalize();
                let roll = u16::from_be_bytes([roll[0], roll[1]]) % 100;
                let streak = self
                    .states
                    .resting_areas
                    .streak(contributor.session, contributor.actor)
                    .unwrap_or(0);
                let points = daily_proficiency_points(points, streak, roll as u8);
                if credit_proficiency(
                    owner,
                    entry.facts.occurrence,
                    entry.facts.death_at_ms,
                    points,
                    in_channel,
                    contributor.last_damage_at_ms,
                    &contributor.weapon,
                )
                .is_some()
                    && owner.checkpoint_due(self.owner_now().get(), false)
                {
                    checkpoints.push((contributor.actor, contributor.session));
                }
            }
        }
        for (actor, session) in checkpoints {
            let _ = self.save_weapon_proficiency(actor, session, false).await;
        }
    }
}

/// The death/session binding makes retrying a kill unable to reroll the bonus.
fn daily_proficiency_points(points: Option<u32>, streak: u32, roll: u8) -> Option<u32> {
    let points = points?;
    if roll < oteryn_gameplay_progression::daily_reward::proficiency_bonus_chance(streak) {
        points.checked_mul(2)
    } else {
        Some(points)
    }
}

fn proficiency_points(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    entry: &PendingKillSettlement,
) -> Option<u32> {
    use crate::domain::weapon_proficiency::{
        BossCategory, CreatureDifficulty, CreatureInfluence, boss_points, creature_points,
    };
    let native = admission.active_generation?.native_gameplay()?;
    let profile = native
        .creature_profiles()
        .records
        .iter()
        .find(|row| row.profile.target.key == entry.creature)?;
    let oteryn_gameplay_content::content::ProjectV2AuthoringProfileData::Creature(creature) =
        &profile.profile.data
    else {
        return None;
    };
    if let Some(boss) = &creature.bosstiary {
        return Some(boss_points(match boss.category.as_str() {
            "bane" => BossCategory::Bane,
            "archfoe" => BossCategory::Archfoe,
            "nemesis" => BossCategory::Nemesis,
            _ => return None,
        }));
    }
    let difficulty = match creature.bestiary.as_ref()?.difficulty.as_str() {
        "harmless" => CreatureDifficulty::Harmless,
        "trivial" => CreatureDifficulty::Trivial,
        "easy" => CreatureDifficulty::Easy,
        "medium" => CreatureDifficulty::Medium,
        "hard" => CreatureDifficulty::Hard,
        "challenging" => CreatureDifficulty::Challenging,
        _ => return None,
    };
    let influence = match entry.forge.as_ref().map(|forge| forge.kind) {
        Some(crate::domain::forge_creature::ForgeCreature::Fiendish) => CreatureInfluence::Fiendish,
        Some(crate::domain::forge_creature::ForgeCreature::Influenced(stacks)) => {
            CreatureInfluence::Stacks(stacks)
        }
        None => CreatureInfluence::Stacks(0),
    };
    creature_points(difficulty, influence).ok()
}

#[allow(
    clippy::too_many_arguments,
    reason = "the frozen death and contributor facts are independently validated"
)]
fn credit_proficiency(
    owner: &mut super::weapon_proficiency::WeaponProficiencyDispatch,
    occurrence: [u8; 16],
    death_at_ms: u64,
    points: Option<u32>,
    in_channel: bool,
    last_damage_at_ms: u64,
    weapon: &str,
) -> Option<u32> {
    owner.dispatch_kill(super::weapon_proficiency::KillDispatch {
        death_occurrence: occurrence,
        death_at_ms,
        creature_is_summon: false,
        points,
        contributor: crate::domain::weapon_proficiency::ProficiencyKillContributor {
            character_in_channel: in_channel,
            last_damage_at_ms,
            equipped_weapon_item_key: Some(weapon),
            weapon_has_promoted_proficiency: true,
        },
    })
}

/// Spoils increases every independent loot chance by its hundredths-of-percent bonus. The
/// multiplication rounds up, matching Global's combat bonus rounding, and guaranteed drops cap
/// at the planner's one-million ppm scale.
fn apply_gut_to_loot(entries: &mut [crate::combat::LootTableEntry], gut_bonus_hundredths: u32) {
    for entry in entries {
        if entry.details.creature_product {
            entry.details.gut_bonus_hundredths = gut_bonus_hundredths;
        }
        apply_gut_to_loot(&mut entry.details.children, gut_bonus_hundredths);
    }
}

fn apply_talisman_loot_bonus(
    table: &mut oteryn_gameplay_items::loot_plan::LootTableDefinition,
    bonus_hundredths: i64,
) {
    let Ok(bonus) = u64::try_from(bonus_hundredths) else {
        return;
    };
    if bonus == 0 {
        return;
    }
    for entry in &mut table.entries {
        if let Some(chance) = entry.probability_ppm {
            let scaled = u64::from(chance)
                .saturating_mul(10_000_u64.saturating_add(bonus))
                .saturating_add(9_999)
                / 10_000;
            entry.probability_ppm = Some(u32::try_from(scaled.min(1_000_000)).unwrap_or(1_000_000));
        }
    }
}
#[cfg(test)]
#[path = "kill_reward_tests.rs"]
mod tests;

//! CREATURE-AI-0 §4.4/§4.6: consume native attack/defence schedules as AI proposals.
//! This prepares the real Ability input, never commits effects or grants actor authority.
//! The owner must independently resolve both actors, content, range/LOS and protections.
//! Melee remains ATTACK-1's AutoAttack; caller facts prepare MONSTER-SUMMON-1 requests.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::ability::{AbilityIntent, AbilityOccurrence, AiAbilityAdapter, RevisionSet};
use crate::content::{
    ProjectV2AbilityAuthoring, ProjectV2AbilityDetails, ProjectV2AbilityKind,
    ProjectV2AbilitySchedule, ProjectV2BehaviorAuthoring, ProjectV2DefinitionRef, ProjectV2Family,
    ProjectV2Magnitude,
};
use crate::foundation::ExactActorRef;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

use super::{CREATURE_THINK_INTERVAL_MILLIS, ThinkOccurrence};

pub const MAX_ATTACK_ENTRIES: usize = 16;
pub const MAX_DEFENCE_ENTRIES: usize = 8;
pub const MAX_SUMMON_ENTRIES: usize = 8;
pub const MAX_MONSTER_SUMMONS: u32 = 16;

/// The same owner snapshot used by attack/defence and monster summon preparation.
#[derive(Clone, Copy)]
pub struct ProfileScheduleInput<'a> {
    pub occurrence: ThinkOccurrence,
    pub behavior: &'a ProjectV2BehaviorAuthoring,
    pub abilities: &'a BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
    pub target: Option<AttackTarget>,
    pub revisions: &'a RevisionSet,
    pub root: &'a GameplayDecisionRoot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedSummonCount {
    pub creature: ProjectV2DefinitionRef,
    pub count: u32,
}

/// Caller-resolved facts, not a grant of authority or a reconstruction of live state.
/// Every native entry has an explicit count, including zero, in authored entry order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonsterSummonFacts {
    pub occurrence: ThinkOccurrence,
    pub is_summon: bool,
    pub target_with_path: Option<ExactActorRef>,
    pub total_count: u32,
    pub entry_counts: Vec<ObservedSummonCount>,
}

/// Prepared §8.1 request only. The owner rechecks count, path and free-tile placement,
/// creates/links the actor atomically, or refuses without reserving any count here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileSummonProposal {
    pub occurrence: ThinkOccurrence,
    pub entry_index: usize,
    pub revisions: RevisionSet,
    pub creature: ProjectV2DefinitionRef,
    pub target_with_path: ExactActorRef,
    pub maximum_total: u32,
    pub maximum_of_creature: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleList {
    Attack,
    Defence,
}

impl ScheduleList {
    const fn purpose(self) -> &'static str {
        match self {
            Self::Attack => "AI_ATTACK",
            Self::Defence => "AI_DEFENCE",
        }
    }

    const fn atom(self) -> &'static str {
        match self {
            Self::Attack => "attack",
            Self::Defence => "defence",
        }
    }
}

/// Target facts read by the owner, not coordinates or an actor reconstructed from content IDs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttackTarget {
    pub actor: ExactActorRef,
    /// None means the target is not on the issuer's floor and is not attackable here.
    pub same_floor_distance: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileAbilityProposal {
    pub issuer: ExactActorRef,
    pub target: ExactActorRef,
    pub ability: ProjectV2DefinitionRef,
    pub list: ScheduleList,
    pub entry_index: usize,
    pub magnitude: Option<ProjectV2Magnitude>,
    pub range_tiles: u16,
    pub occurrence: AbilityOccurrence,
    pub intent: AbilityIntent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulePlan {
    pub proposals: Vec<ProfileAbilityProposal>,
    pub summon_proposals: Vec<ProfileSummonProposal>,
    /// Due evaluation uses these ticks; reset happens after evaluating the whole list.
    pub evaluated_attack_ticks_ms: u64,
    pub evaluated_defence_ticks_ms: u64,
    pub attack_ticks_after_ms: u64,
    pub defence_ticks_after_ms: u64,
    pub melee_delegated: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleError {
    ActorMismatch,
    OccurrenceSuperseded,
    OccurrenceConflict,
    TooManyEntries,
    InvalidSchedule,
    MissingAbilityProfile,
    InvalidDefenceMelee,
    SummonClockDependency,
    InvalidSummonFacts,
    TickOverflow,
    InvalidProposal,
}

struct PreparedThink {
    sequence: u64,
    snapshot: [u8; 32],
    root: GameplayDecisionRoot,
    summon_facts: Option<MonsterSummonFacts>,
    result: Result<SchedulePlan, ScheduleError>,
}

/// One active creature's ephemeral schedule state. It confers no Channel owner authority.
pub struct ProfileScheduleState {
    actor: ExactActorRef,
    attack_ticks_ms: u64,
    defence_ticks_ms: u64,
    last: Option<PreparedThink>,
}

impl ProfileScheduleState {
    pub fn new(actor: ExactActorRef) -> Self {
        Self {
            actor,
            attack_ticks_ms: 0,
            defence_ticks_ms: 0,
            last: None,
        }
    }

    /// Retry returns the first result and never advances clocks or redraws. A conflicting
    /// target, revision or native profile snapshot of the same occurrence is refused.
    pub fn prepare(
        &mut self,
        occurrence: ThinkOccurrence,
        behavior: &ProjectV2BehaviorAuthoring,
        abilities: &BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
        target: Option<AttackTarget>,
        revisions: &RevisionSet,
        root: &GameplayDecisionRoot,
    ) -> Result<SchedulePlan, ScheduleError> {
        self.prepare_with_summons(
            ProfileScheduleInput {
                occurrence,
                behavior,
                abilities,
                target,
                revisions,
                root,
            },
            None,
        )
    }

    /// Supplying §8.1 facts enables the shared defence/summon clock. Legacy callers
    /// without those facts still fail closed on a profile with summons.
    pub fn prepare_with_summons(
        &mut self,
        input: ProfileScheduleInput<'_>,
        summon_facts: Option<&MonsterSummonFacts>,
    ) -> Result<SchedulePlan, ScheduleError> {
        let ProfileScheduleInput {
            occurrence,
            behavior,
            abilities,
            target,
            revisions,
            root,
        } = input;
        if occurrence.actor != self.actor {
            return Err(ScheduleError::ActorMismatch);
        }
        if let Some(previous) = &self.last
            && occurrence.sequence < previous.sequence
        {
            return Err(ScheduleError::OccurrenceSuperseded);
        }
        // Bound the actual scan before hashing or retaining any supplied content.
        if behavior.attacks.len() > MAX_ATTACK_ENTRIES
            || behavior.defenses.len() > MAX_DEFENCE_ENTRIES
            || behavior
                .summons
                .as_ref()
                .is_some_and(|s| s.entries.len() > MAX_SUMMON_ENTRIES)
            || summon_facts.is_some_and(|f| f.entry_counts.len() > MAX_SUMMON_ENTRIES)
        {
            return Err(ScheduleError::TooManyEntries);
        }
        let snapshot = snapshot_hash(behavior, abilities, target, revisions)?;
        if let Some(previous) = &self.last
            && occurrence.sequence == previous.sequence
        {
            return if snapshot == previous.snapshot
                && root == &previous.root
                && summon_facts == previous.summon_facts.as_ref()
            {
                previous.result.clone()
            } else {
                Err(ScheduleError::OccurrenceConflict)
            };
        }
        let result = self.plan(input, summon_facts);
        if let Ok(plan) = &result {
            self.attack_ticks_ms = plan.attack_ticks_after_ms;
            self.defence_ticks_ms = plan.defence_ticks_after_ms;
        }
        self.last = Some(PreparedThink {
            sequence: occurrence.sequence,
            snapshot,
            root: root.clone(),
            summon_facts: summon_facts.cloned(),
            result: result.clone(),
        });
        result
    }

    fn plan(
        &self,
        input: ProfileScheduleInput<'_>,
        summon_facts: Option<&MonsterSummonFacts>,
    ) -> Result<SchedulePlan, ScheduleError> {
        let ProfileScheduleInput {
            occurrence,
            behavior,
            abilities,
            target,
            revisions,
            root,
        } = input;
        if behavior.summons.is_some() && summon_facts.is_none() {
            return Err(ScheduleError::SummonClockDependency);
        }
        if let Some(facts) = summon_facts {
            validate_summon_facts(input, facts)?;
        }
        let attack_ticks = self
            .attack_ticks_ms
            .checked_add(CREATURE_THINK_INTERVAL_MILLIS)
            .ok_or(ScheduleError::TickOverflow)?;
        let defence_ticks = self
            .defence_ticks_ms
            .checked_add(CREATURE_THINK_INTERVAL_MILLIS)
            .ok_or(ScheduleError::TickOverflow)?;
        let mut plan = SchedulePlan {
            proposals: Vec::new(),
            summon_proposals: Vec::new(),
            evaluated_attack_ticks_ms: attack_ticks,
            evaluated_defence_ticks_ms: defence_ticks,
            attack_ticks_after_ms: 0,
            defence_ticks_after_ms: 0,
            melee_delegated: 0,
        };
        for (list, entries, ticks) in [
            (ScheduleList::Attack, &behavior.attacks, attack_ticks),
            (ScheduleList::Defence, &behavior.defenses, defence_ticks),
        ] {
            let mut waits = false;
            for (index, entry) in entries.iter().enumerate() {
                let details = validate_entry(entry, abilities)?;
                if details.kind == ProjectV2AbilityKind::Melee {
                    if list == ScheduleList::Defence {
                        return Err(ScheduleError::InvalidDefenceMelee);
                    }
                    plan.melee_delegated += 1;
                    continue;
                }
                if ticks < entry.interval_ms {
                    waits = true;
                    continue;
                }
                if ticks % entry.interval_ms >= CREATURE_THINK_INTERVAL_MILLIS {
                    continue;
                }
                let range = entry.range_tiles.unwrap_or(details.range_tiles);
                let recipient = match list {
                    ScheduleList::Defence => self.actor,
                    ScheduleList::Attack => {
                        if !behavior.targeting.hostile || !behavior.targeting.can_target {
                            continue;
                        }
                        let Some(target) = target else {
                            continue;
                        };
                        let Some(distance) = target.same_floor_distance else {
                            continue;
                        };
                        // Native source range 0 is uncapped, not an invented default radius.
                        if range != 0 && distance > range {
                            continue;
                        }
                        target.actor
                    }
                };
                let decision = deterministic_decision_u64(
                    root,
                    decision_occurrence(occurrence),
                    list.purpose(),
                    index as u64,
                )
                .map_err(|_| ScheduleError::InvalidProposal)?;
                if decision % 1_000_000 >= u64::from(entry.chance_ppm) {
                    continue;
                }
                let issuer = actor_atom(self.actor);
                let target_atom = actor_atom(recipient);
                let id = format!(
                    "ai-profile:{}:{}:{}:{index}",
                    hex(&self.actor.placement_identity()),
                    occurrence.sequence,
                    list.atom()
                );
                plan.proposals.push(ProfileAbilityProposal {
                    issuer: self.actor,
                    target: recipient,
                    ability: entry.ability.clone(),
                    list,
                    entry_index: index,
                    magnitude: entry.magnitude,
                    range_tiles: range,
                    occurrence: AbilityOccurrence::new(&id, revisions.clone())
                        .map_err(|_| ScheduleError::InvalidProposal)?,
                    intent: AiAbilityAdapter::normalize(&issuer, &[&target_atom])
                        .map_err(|_| ScheduleError::InvalidProposal)?,
                });
            }
            if waits {
                match list {
                    ScheduleList::Attack => plan.attack_ticks_after_ms = ticks,
                    ScheduleList::Defence => plan.defence_ticks_after_ms = ticks,
                }
            }
        }
        if let Some(facts) = summon_facts {
            prepare_summons(input, facts, defence_ticks, &mut plan)?;
        }
        Ok(plan)
    }
}

fn validate_summon_facts(
    input: ProfileScheduleInput<'_>,
    facts: &MonsterSummonFacts,
) -> Result<(), ScheduleError> {
    let summons = input
        .behavior
        .summons
        .as_ref()
        .ok_or(ScheduleError::InvalidSummonFacts)?;
    if facts.occurrence != input.occurrence
        || facts.total_count > MAX_MONSTER_SUMMONS
        || summons.max_summons == 0
        || summons.max_summons > MAX_MONSTER_SUMMONS
        || summons.entries.is_empty()
        || facts.entry_counts.len() != summons.entries.len()
        || facts.target_with_path.is_some_and(|actor| {
            input
                .target
                .is_none_or(|target| target.actor != actor || target.same_floor_distance.is_none())
                || !input.behavior.targeting.hostile
                || !input.behavior.targeting.can_target
        })
    {
        return Err(ScheduleError::InvalidSummonFacts);
    }
    let mut observed = BTreeMap::new();
    for (entry, count) in summons.entries.iter().zip(&facts.entry_counts) {
        if entry.creature.family != ProjectV2Family::Creature
            || entry.creature != count.creature
            || entry.interval_ms == 0
            || entry.chance_ppm > 1_000_000
            || entry.count == 0
            || entry.count > MAX_MONSTER_SUMMONS
            || count.count > facts.total_count
        {
            return Err(ScheduleError::InvalidSummonFacts);
        }
        if observed
            .insert(&entry.creature, count.count)
            .is_some_and(|previous| previous != count.count)
        {
            return Err(ScheduleError::InvalidSummonFacts);
        }
    }
    if observed.values().sum::<u32>() > facts.total_count {
        return Err(ScheduleError::InvalidSummonFacts);
    }
    Ok(())
}

fn prepare_summons(
    input: ProfileScheduleInput<'_>,
    facts: &MonsterSummonFacts,
    ticks: u64,
    plan: &mut SchedulePlan,
) -> Result<(), ScheduleError> {
    let summons = input
        .behavior
        .summons
        .as_ref()
        .ok_or(ScheduleError::InvalidSummonFacts)?;
    if facts.is_summon || facts.total_count >= summons.max_summons {
        return Ok(());
    }
    let Some(target_with_path) = facts.target_with_path else {
        return Ok(());
    };
    for (index, (entry, count)) in summons.entries.iter().zip(&facts.entry_counts).enumerate() {
        if ticks < entry.interval_ms {
            plan.defence_ticks_after_ms = ticks;
            continue;
        }
        if ticks % entry.interval_ms >= CREATURE_THINK_INTERVAL_MILLIS || count.count >= entry.count
        {
            continue;
        }
        let decision = deterministic_decision_u64(
            input.root,
            decision_occurrence(input.occurrence),
            "AI_SUMMON",
            index as u64,
        )
        .map_err(|_| ScheduleError::InvalidProposal)?;
        if decision % 1_000_000 >= u64::from(entry.chance_ppm) {
            continue;
        }
        plan.summon_proposals.push(ProfileSummonProposal {
            occurrence: input.occurrence,
            entry_index: index,
            revisions: input.revisions.clone(),
            creature: entry.creature.clone(),
            target_with_path,
            maximum_total: summons.max_summons,
            maximum_of_creature: entry.count,
        });
    }
    Ok(())
}

pub(crate) fn validate_entry<'a>(
    entry: &ProjectV2AbilitySchedule,
    abilities: &'a BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
) -> Result<&'a ProjectV2AbilityDetails, ScheduleError> {
    if entry.ability.family != ProjectV2Family::Ability
        || entry.interval_ms == 0
        || entry.chance_ppm > 1_000_000
        || entry
            .magnitude
            .is_some_and(|value| value.minimum > value.maximum)
    {
        return Err(ScheduleError::InvalidSchedule);
    }
    abilities
        .get(&entry.ability)
        .and_then(|ability| ability.details.as_deref())
        .ok_or(ScheduleError::MissingAbilityProfile)
}

pub(crate) fn decision_occurrence(occurrence: ThinkOccurrence) -> DecisionOccurrenceId {
    let digest = Sha256::new()
        .chain_update(b"oteryn:ai-profile-think:v1")
        .chain_update(occurrence.actor.placement_identity())
        .chain_update(occurrence.sequence.to_be_bytes())
        .finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&digest[..16]);
    DecisionOccurrenceId::from_bytes(bytes)
}

fn actor_atom(actor: ExactActorRef) -> String {
    format!("actor:{}", hex(&actor.placement_identity()))
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn snapshot_hash(
    behavior: &ProjectV2BehaviorAuthoring,
    abilities: &BTreeMap<ProjectV2DefinitionRef, ProjectV2AbilityAuthoring>,
    target: Option<AttackTarget>,
    revisions: &RevisionSet,
) -> Result<[u8; 32], ScheduleError> {
    let mut hash = Sha256::new();
    hash.update(serde_json::to_vec(behavior).map_err(|_| ScheduleError::InvalidSchedule)?);
    for entry in behavior.attacks.iter().chain(&behavior.defenses) {
        hash.update(
            serde_json::to_vec(&entry.ability).map_err(|_| ScheduleError::InvalidSchedule)?,
        );
        if let Some(ability) = abilities.get(&entry.ability) {
            hash.update(serde_json::to_vec(ability).map_err(|_| ScheduleError::InvalidSchedule)?);
        }
    }
    hash.update(format!("{target:?}{revisions:?}"));
    Ok(hash.finalize().into())
}

#[cfg(test)]
#[path = "profile_schedule_tests.rs"]
mod tests;

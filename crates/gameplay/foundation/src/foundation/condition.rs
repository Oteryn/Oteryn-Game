//! COND-1a (`CONDITIONS0-ACTOR-CONDITIONS-V1` §3, §3.1, §3.4, §6.1): the per-actor
//! `ConditionInstance` store, its transitions, admission and the tick schedule. Dispel and
//! the mana shield split (§3.3) and the channel handover (§6.2) follow in COND-1b.
//! Cleanse (§5) prepares one deterministic removal and retains its conflict-key immunity in this
//! same store. Actor composition must commit and replay the retained plan under its own fence.
//!
//! The store is pure runtime-actor-local state owned by the Channel runtime. It never commits
//! damage: a due damage tick comes out as a typed [`ConditionTick`] that the owner turns into one
//! Effect Plan (COND-1b). Values come from content through [`ConditionDefinition`]; code owns only
//! the families, conflict keys and policies. Every draw is one `oteryn_simulation_determinism`
//! decision bound to the caller's `(GameplayDecisionRoot, DecisionOccurrenceId)`.

type ConditionApplicationSource<'a, S> = dyn Fn(&ConditionProvenance<S>) -> bool + 'a;

use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

fn valid_atom(value: &str) -> bool {
    !value.is_empty()
        && value.is_ascii()
        && value.len() <= 4_096
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b':' | b'.' | b'_' | b'-' | b'/')
        })
}

#[path = "condition_spell.rs"]
mod spell;
pub use spell::{SkillAdjustments, SpellSpeedError};
#[path = "damage_schedule.rs"]
mod damage_schedule;
use damage_schedule::DamageCursor;
pub use damage_schedule::{DamageSchedule, DamageSegment};

/// `COND0-RL-01`: instances per actor (§3.5: 29 exhaustive keys and headroom for three).
pub const COND0_RL_01_INSTANCES_PER_ACTOR: usize = 32;
/// `COND0-RL-02`: minimum tick interval, in milliseconds.
pub const COND0_RL_02_MIN_TICK_INTERVAL_MS: u32 = 1_000;
/// `COND0-RL-03`: damage ticks per actor per simulation tick. The store also counts regeneration
/// ticks against it, so a catch-up of any tick kind is bounded per pass (`RUN_EACH_BOUNDED`).
pub const COND0_RL_03_DAMAGE_TICKS_PER_SIM_TICK: usize = 4;
/// ITEM-USE-0 §6.1: the food time cap (a parity value, not a resource limit).
pub const FOOD_REGENERATION_CAP_MS: u32 = 1_200_000;
/// §3: paralysis never takes speed below this.
pub const PARALYSIS_SPEED_FLOOR: i64 = 40;

pub const COND_SPEED_DRAW: &str = "oteryn.condition.speed_draw.v1";
pub const COND_DOT_TOTAL_DRAW: &str = "oteryn.condition.dot_total_draw.v1";
pub const COND_CLEANSE_PICK: &str = "oteryn.condition.cleanse_pick.v1";
/// CONDITIONS-0 §5: immunity to the removed conflict key.
pub const CLEANSE_IMMUNITY_MS: u32 = 11_000;

const MICROS_PER_MS: u64 = 1_000;

use oteryn_gameplay_domain::domain::condition::attributes::AttributeModifiers;
use oteryn_gameplay_domain::domain::condition::{
    ConditionGate, ConditionRefusal, ConditionType, ConditionValues, ConflictKey, DotElement,
    DotSequenceStep, MAX_DOT_SEQUENCE_STEPS, SpeedRange, StatusKind,
};
#[cfg(test)]
use oteryn_gameplay_domain::domain::condition::{ExactSpeedRatio, RationalSpeedRange};

/// An item appearance retains the exact activated Item identity and generation.
/// These historical selection bytes do not grant an Outfit unlock or ownership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporaryItemAppearance {
    definition_key: String,
    revision_ref: String,
    artifact_digest: [u8; 32],
}
impl TemporaryItemAppearance {
    pub fn definition_key(&self) -> &str {
        &self.definition_key
    }
    pub fn revision_ref(&self) -> &str {
        &self.revision_ref
    }
    pub fn artifact_digest(&self) -> [u8; 32] {
        self.artifact_digest
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporaryDisplayedAppearance<'a> {
    Outfit(&'a crate::domain::appearance::AppearanceSelection<String>),
    Item(&'a TemporaryItemAppearance),
}

/// A versioned `ConditionDefinition` (GAME-ABILITY-01), built by the content loader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionDefinition {
    key: String,
    revision: u32,
    values: ConditionValues,
    damage_schedule: Option<DamageSchedule>,
    appearance_selection: Option<crate::domain::appearance::AppearanceSelection<String>>,
    item_appearance: Option<TemporaryItemAppearance>,
}

impl ConditionDefinition {
    /// `None` for an invalid key, a tick interval under `COND0-RL-02`, a zero duration or
    /// amount, or an inverted damage range.
    pub fn new(key: &str, revision: u32, values: ConditionValues) -> Option<Self> {
        let interval_ok = |ms: u32| ms >= COND0_RL_02_MIN_TICK_INTERVAL_MS;
        let valid = valid_atom(key)
            && match values {
                ConditionValues::DamageSchedule { .. } => false,
                ConditionValues::TimedStatus { duration_ms, .. }
                | ConditionValues::Gate { duration_ms, .. } => duration_ms > 0,
                ConditionValues::SourceAttributes {
                    modifiers,
                    duration_ms,
                } => modifiers.valid() && duration_ms > 0,
                ConditionValues::RationalSpeed {
                    range, duration_ms, ..
                } => range.valid() && duration_ms > 0,
                ConditionValues::Speed { duration_ms, .. }
                | ConditionValues::ManaShield { duration_ms }
                | ConditionValues::Light { duration_ms, .. }
                | ConditionValues::SpellLight { duration_ms, .. }
                | ConditionValues::Invisible { duration_ms }
                | ConditionValues::ItemOutfit { duration_ms }
                | ConditionValues::Outfit { duration_ms, .. } => duration_ms > 0,
                ConditionValues::SpellSkills {
                    duration_ms,
                    sub_id,
                    magic_level,
                    fist,
                    melee,
                    distance,
                    shielding,
                } => {
                    duration_ms > 0
                        && sub_id <= 3
                        && [magic_level, fist, melee, distance, shielding]
                            .iter()
                            .all(|v| (-65535..=65535).contains(v))
                }
                ConditionValues::Attributes {
                    duration_ms,
                    critical_chance_percent,
                    incoming_reduction_percent,
                    ..
                } => {
                    duration_ms > 0
                        && critical_chance_percent <= 100
                        && incoming_reduction_percent <= 100
                }
                ConditionValues::SpellRegeneration {
                    duration_ms,
                    health_gain,
                    health_interval_ms,
                    mana_gain,
                    mana_interval_ms,
                    sub_id,
                } => {
                    duration_ms > 0
                        && sub_id <= 1
                        && (health_gain > 0 || mana_gain > 0)
                        && interval_ok(health_interval_ms)
                        && interval_ok(mana_interval_ms)
                }
                ConditionValues::DamageOverTime {
                    total_min,
                    total_max,
                    per_tick,
                    interval_ms,
                    ..
                }
                | ConditionValues::Agony {
                    total_min,
                    total_max,
                    per_tick,
                    interval_ms,
                    ..
                } => {
                    total_min > 0
                        && total_min <= total_max
                        && per_tick > 0
                        && interval_ok(interval_ms)
                }
                ConditionValues::DamageSequence { steps, len, .. } => {
                    let count = usize::from(len);
                    count > 0
                        && count <= MAX_DOT_SEQUENCE_STEPS
                        && steps[..count].iter().all(|s| {
                            s.amount > 0 && s.repetitions > 0 && interval_ok(s.interval_ms)
                        })
                        && steps[count..]
                            .iter()
                            .all(|s| *s == DotSequenceStep::default())
                        && sequence_total(&steps[..count]).is_some()
                        && sequence_duration(&steps[..count]).is_some()
                }
                ConditionValues::FoodRegeneration {
                    added_ms,
                    interval_ms,
                } => added_ms > 0 && interval_ok(interval_ms),
                ConditionValues::Recovery {
                    duration_ms,
                    interval_ms,
                } => duration_ms > 0 && interval_ok(interval_ms),
            };
        valid.then(|| Self {
            key: key.to_owned(),
            revision,
            values,
            damage_schedule: None,
            appearance_selection: None,
            item_appearance: None,
        })
    }

    pub fn new_damage_schedule(
        key: &str,
        revision: u32,
        element: DotElement,
        schedule: DamageSchedule,
    ) -> Option<Self> {
        (valid_atom(key) && revision > 0 && schedule.validate()).then(|| Self {
            key: key.to_owned(),
            revision,
            values: ConditionValues::DamageSchedule { element },
            damage_schedule: Some(schedule),
            appearance_selection: None,
            item_appearance: None,
        })
    }

    /// The caller resolves and qualifies this appearance against the active content pin.
    pub fn with_appearance(
        mut self,
        selection: crate::domain::appearance::AppearanceSelection<String>,
    ) -> Option<Self> {
        if !matches!(self.values, ConditionValues::Outfit { .. }) || selection.outfit_key.is_empty()
        {
            return None;
        }
        self.appearance_selection = Some(selection);
        Some(self)
    }

    /// The owner resolves this exact production Item in the independently
    /// active artifact before constructing and before applying this definition.
    pub fn with_item_appearance(
        mut self,
        key: &str,
        revision: &str,
        digest: [u8; 32],
    ) -> Option<Self> {
        if !matches!(self.values, ConditionValues::ItemOutfit { .. })
            || !valid_atom(key)
            || !valid_atom(revision)
            || digest == [0; 32]
        {
            return None;
        }
        self.item_appearance = Some(TemporaryItemAppearance {
            definition_key: key.into(),
            revision_ref: revision.into(),
            artifact_digest: digest,
        });
        Some(self)
    }
    pub fn source_appearance_selection(
        &self,
    ) -> Option<&crate::domain::appearance::AppearanceSelection<String>> {
        self.appearance_selection.as_ref()
    }
    pub fn source_item_appearance(&self) -> Option<&TemporaryItemAppearance> {
        self.item_appearance.as_ref()
    }
    pub fn source_damage_schedule(&self) -> Option<&DamageSchedule> {
        self.damage_schedule.as_ref()
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub const fn revision(&self) -> u32 {
        self.revision
    }

    pub const fn values(&self) -> ConditionValues {
        self.values
    }

    pub fn is_negative(&self) -> bool {
        match self.values {
            ConditionValues::SourceAttributes { modifiers, .. } => modifiers.is_negative(),
            _ => self.condition_type().is_negative(),
        }
    }

    pub const fn condition_type(&self) -> ConditionType {
        match self.values {
            ConditionValues::TimedStatus { kind, .. } => ConditionType::Status(kind),
            ConditionValues::SourceAttributes { .. } => ConditionType::Attributes,
            ConditionValues::Speed {
                paralysis: true, ..
            }
            | ConditionValues::RationalSpeed {
                paralysis: true, ..
            } => ConditionType::Paralysis,
            ConditionValues::Speed { .. } | ConditionValues::RationalSpeed { .. } => {
                ConditionType::Haste
            }
            ConditionValues::DamageOverTime { element, .. }
            | ConditionValues::DamageSchedule { element } => ConditionType::DamageOverTime(element),
            ConditionValues::DamageSequence { element, .. } => {
                ConditionType::DamageOverTime(element)
            }
            ConditionValues::FoodRegeneration { .. } => ConditionType::FoodRegeneration,
            ConditionValues::Recovery { .. }
            | ConditionValues::SpellRegeneration { sub_id: 0, .. } => ConditionType::Recovery,
            ConditionValues::SpellRegeneration { sub_id, .. } => {
                ConditionType::SpellRegeneration(sub_id)
            }
            ConditionValues::ManaShield { .. } => ConditionType::ManaShield,
            ConditionValues::Light { .. } | ConditionValues::SpellLight { .. } => {
                ConditionType::Light
            }
            ConditionValues::Attributes { .. } => ConditionType::Attributes,
            ConditionValues::SpellSkills { sub_id, .. } => ConditionType::SpellSkills(sub_id),
            ConditionValues::Invisible { .. } => ConditionType::Invisible,
            ConditionValues::Outfit { .. } | ConditionValues::ItemOutfit { .. } => {
                ConditionType::Outfit
            }
            ConditionValues::Gate { gate, .. } => ConditionType::Gate(gate),
            ConditionValues::Agony { .. } => ConditionType::Agony,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionSourceKind {
    Creature,
    Player,
    /// A field on the ground: its damage over time always replaces (§3.1).
    Field,
    /// The actor's own use of an item or spell (food, potion, self spell).
    SelfUse,
}

/// §3.2: provenance frozen at application. `S` is the owner's source actor identity; the owner
/// looks it up again at each tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionProvenance<S> {
    pub source: Option<S>,
    pub source_kind: ConditionSourceKind,
    pub definition_key: String,
    pub definition_revision: u32,
    /// Historical application lineage, never refreshed from a tick command.
    pub lineage: Option<ConditionLineage>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionLineage {
    pub source_character: [u8; 16],
    pub source_session: [u8; 16],
    pub source_lease_generation: u64,
    pub source_actor_placement: [u8; 16],
    pub source_scope_generation: u64,
    pub source_content_digest: [u8; 32],
    pub source_catalog_digest: [u8; 32],
    /// Real original creation command, not a manufactured periodic command.
    pub source_creation_command: u64,
}

/// One live instance of a definition on one actor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionInstance<S> {
    definition: ConditionDefinition,
    provenance: ConditionProvenance<S>,
    sequence: u32,
    /// Semantic time (µs) at which a timed instance ends; `None` for damage over time, which ends
    /// when its total is used up.
    ends_at: Option<u64>,
    /// The next tick's semantic time (µs), for ticking families.
    next_tick_at: Option<u64>,
    /// The application time of a non-`delayed` damage over time whose first tick is not dealt yet
    /// (§3.1); it runs through `take_due` under the same budget as every other tick.
    immediate_due: Option<u64>,
    started_at: u64,
    /// The remaining damage total of a damage over time.
    remaining_total: u32,
    scheduled_damage: Option<DamageCursor>,
    sequence_step: u8,
    sequence_repetition: u32,
    /// The speed delta fixed at application.
    speed_delta: i64,
    speed_base: u16,
    mana_shield_capacity: u32,
    mana_shield_remaining: u32,
}

impl<S> ConditionInstance<S> {
    pub const fn definition(&self) -> &ConditionDefinition {
        &self.definition
    }

    pub const fn provenance(&self) -> &ConditionProvenance<S> {
        &self.provenance
    }

    pub const fn started_at(&self) -> u64 {
        self.started_at
    }

    pub const fn sequence(&self) -> u32 {
        self.sequence
    }

    pub const fn application_base_speed(&self) -> u16 {
        self.speed_base
    }

    pub const fn speed_delta(&self) -> i64 {
        self.speed_delta
    }

    /// Remaining lifetime of the actual instance, for a source-qualified native
    /// snapshot. `-1` represents the owner's infinite instance; expired is zero.
    pub fn remaining_duration_ms(&self, now: u64) -> Option<i32> {
        match self.ends_at {
            None => Some(-1),
            Some(end) => i32::try_from(end.saturating_sub(now) / MICROS_PER_MS).ok(),
        }
    }

    pub const fn remaining_total(&self) -> u32 {
        self.remaining_total
    }

    /// `(remaining, capacity)` of a mana shield (§3.3).
    pub const fn mana_shield(&self) -> (u32, u32) {
        (self.mana_shield_remaining, self.mana_shield_capacity)
    }

    /// §3: the light level decays linearly over the duration.
    pub fn light_level(&self, now: u64) -> u8 {
        let (ConditionValues::Light { level, .. }, Some(ends_at)) =
            (self.definition.values, self.ends_at)
        else {
            return 0;
        };
        let total = ends_at.saturating_sub(self.started_at).max(1);
        let left = ends_at.saturating_sub(now);
        u8::try_from(u64::from(level) * left / total).unwrap_or(level)
    }

    fn remaining_at(&self, now: u64) -> u64 {
        self.ends_at.map_or(0, |end| end.saturating_sub(now))
    }
}

/// The owner facts read with an application, in the same work item.
#[derive(Clone, Copy)]
pub struct ApplicationFacts<'a> {
    pub now: u64,
    /// The target's base speed, for a `SPEED` definition.
    pub base_speed: u16,
    /// The mana shield capacity from the content formula, at most maximum mana (§3.3).
    pub mana_shield_capacity: u32,
    /// §6.1 PvE re-entry protection: the target, or the applying player, is in its window.
    pub target_reentry_protected: bool,
    pub source_reentry_protected: bool,
    pub target_is_player: bool,
    pub decision_root: &'a GameplayDecisionRoot,
    pub occurrence: DecisionOccurrenceId,
}

/// A committed application. A damage over time that is not `delayed` has its first tick due at
/// once: the owner's `take_due` pass in the same simulation tick deals it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Applied {
    pub sequence: u32,
    pub replaced: bool,
}

/// A due tick, in the §3.4 order `(due, actor, sequence)`. It carries the instance's frozen
/// provenance (§3.2), so the owner can build its Effect Plan even when this was the last tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionTick<S> {
    pub due: u64,
    pub sequence: u32,
    pub kind: TickKind,
    pub provenance: ConditionProvenance<S>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickKind {
    /// One damage occurrence (§3.2); `refused` in a protection zone, where it is still used up.
    Damage {
        element: DotElement,
        amount: u32,
        refused: bool,
    },
    /// One §3.5 `agony` occurrence: undefined element, not absorbed by the mana shield;
    /// `refused` in a protection zone, where it is still used up.
    Agony { amount: u32, refused: bool },
    /// A regeneration tick (FOOD-REGEN-1, Recovery); `suppressed` regenerates nothing.
    Regeneration { key: ConflictKey, suppressed: bool },
    SpellRegeneration {
        health_gain: u32,
        mana_gain: u32,
        suppressed: bool,
    },
}

/// The owner facts read with a tick pass, in the same work item.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TickFacts {
    pub in_protection_zone: bool,
    /// The element of a field the actor stands on: ticks of that element are not used up.
    pub standing_on_field: Option<DotElement>,
}

/// Frozen removal chosen before commit. The actor owner retains this exact plan for replay;
/// it must never reselect from changed conditions for the same occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CleansePlan {
    sequence: u32,
    key: ConflictKey,
    immune_until: u64,
}

impl CleansePlan {
    #[cfg(any(test, feature = "test-support"))]
    pub const fn conflict_key(self) -> ConflictKey {
        self.key
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CleanseImmunity {
    key: ConflictKey,
    until: u64,
}

/// The per-actor store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConditionStore<S> {
    /// Sorted by sequence.
    instances: Vec<ConditionInstance<S>>,
    /// At most one immunity per closed conflict key; actor movement retains this same store.
    cleanse_immunities: Vec<CleanseImmunity>,
    next_sequence: u32,
    /// The simulation tick (its semantic time) of the current pass and the ticks it has dealt.
    pass_at: u64,
    pass_ticks: usize,
    /// ATTACK-0 §4 (as amended by #2106): the semantic time of the latest `pacified`
    /// application, kept past its expiry so the attack owner clears a target held across it.
    pacified_applied_at: Option<u64>,
}

impl<S> Default for ConditionStore<S> {
    fn default() -> Self {
        Self {
            instances: Vec::new(),
            cleanse_immunities: Vec::new(),
            next_sequence: 0,
            pass_at: 0,
            pass_ticks: 0,
            pacified_applied_at: None,
        }
    }
}

impl<S: Clone> ConditionStore<S> {
    /// The real application owner supplies historical field receipt lineage.
    /// Primitive lineage data is evidence only; it never grants tick authority.
    pub fn apply_with_lineage(
        &mut self,
        definition: &ConditionDefinition,
        source: Option<S>,
        source_kind: ConditionSourceKind,
        immunities: &[ConditionType],
        facts: &ApplicationFacts<'_>,
        lineage: &ConditionLineage,
    ) -> Result<Applied, ConditionRefusal> {
        if lineage.source_lease_generation == 0
            || lineage.source_scope_generation == 0
            || lineage.source_creation_command == 0
            || lineage.source_character == [0; 16]
            || lineage.source_session == [0; 16]
            || lineage.source_actor_placement == [0; 16]
            || lineage.source_content_digest == [0; 32]
            || lineage.source_catalog_digest == [0; 32]
        {
            return Err(ConditionRefusal::ArithmeticBounds);
        }
        let mut staged = self.clone();
        let applied = staged.apply(definition, source, source_kind, immunities, facts)?;
        let instance = staged
            .instances
            .iter_mut()
            .find(|i| i.sequence == applied.sequence)
            .ok_or(ConditionRefusal::ArithmeticBounds)?;
        instance.provenance.lineage = Some(lineage.clone());
        *self = staged;
        Ok(applied)
    }
    /// §6.2 fresh admission: no instances (food time is restored by COND-DUR-1).
    pub fn new() -> Self {
        Self::default()
    }

    /// Pure current-time view; expired status never remains effective before cleanup.
    pub fn has_status(&self, kind: StatusKind, now: u64) -> bool {
        self.get(ConflictKey::Status(kind))
            .is_some_and(|instance| instance.ends_at.is_some_and(|end| now < end))
    }

    pub fn instances(&self) -> &[ConditionInstance<S>] {
        &self.instances
    }

    pub fn get(&self, key: ConflictKey) -> Option<&ConditionInstance<S>> {
        self.instances
            .iter()
            .find(|instance| instance.definition.condition_type().conflict_key() == key)
    }

    pub fn effective_attributes(&self, now: u64) -> AttributeModifiers {
        self.get(ConflictKey::Attributes)
            .and_then(|instance| {
                if instance.ends_at.is_some_and(|end| now >= end) {
                    return None;
                }
                match instance.definition.values {
                    ConditionValues::SourceAttributes { modifiers, .. } => Some(modifiers),
                    _ => None,
                }
            })
            .unwrap_or_default()
    }

    /// Exact application base and native fixed speed delta; expired instances have no effect,
    /// even before a tick/cleanup pass. This does not invent a saved Character speed fact.
    pub fn effective_speed(&self, now: u64) -> Option<(u16, i64)> {
        let instance = self.get(ConflictKey::Speed)?;
        if instance.ends_at.is_none_or(|end| now >= end) {
            return None;
        }
        Some((
            instance.speed_base,
            i64::from(instance.speed_base) + instance.speed_delta,
        ))
    }

    /// The effective `SPEED` delta, 0 without one.
    pub fn speed_delta(&self) -> i64 {
        self.get(ConflictKey::Speed)
            .map_or(0, |instance| instance.speed_delta)
    }

    /// A timed speed contribution ends at its exact deadline, even before owner cleanup.
    pub fn active_speed_delta(&self, now: u64) -> i64 {
        self.get(ConflictKey::Speed)
            .filter(|instance| instance.ends_at.is_none_or(|end| now < end))
            .map_or(0, |instance| instance.speed_delta)
    }

    /// Owner-local expiry without executing or discarding any scheduled occurrence.
    /// DOT, food and recovery remain exclusively under `take_due`, including overdue ticks.
    pub fn expire_non_ticking(&mut self, now: u64) -> bool {
        let before = (self.instances.len(), self.cleanse_immunities.len());
        self.instances.retain(|instance| {
            !matches!(
                instance.definition.values,
                ConditionValues::TimedStatus { .. }
                    | ConditionValues::Gate { .. }
                    | ConditionValues::SourceAttributes { .. }
                    | ConditionValues::SpellLight { .. }
                    | ConditionValues::SpellSkills { .. }
                    | ConditionValues::Invisible { .. }
                    | ConditionValues::Outfit { .. }
                    | ConditionValues::ItemOutfit { .. }
                    | ConditionValues::Attributes { .. }
                    | ConditionValues::RationalSpeed { .. }
                    | ConditionValues::Speed { .. }
                    | ConditionValues::Light { .. }
                    | ConditionValues::ManaShield { .. }
            ) || instance.ends_at.is_none_or(|end| now < end)
        });
        self.cleanse_immunities
            .retain(|immunity| now < immunity.until);
        before != (self.instances.len(), self.cleanse_immunities.len())
    }

    /// One application at admission (§3, §3.1, §6.1). The provenance is frozen here (§3.2): the
    /// definition key and revision are always those of `definition`, never a caller's copy.
    pub fn apply(
        &mut self,
        definition: &ConditionDefinition,
        source: Option<S>,
        source_kind: ConditionSourceKind,
        immunities: &[ConditionType],
        facts: &ApplicationFacts<'_>,
    ) -> Result<Applied, ConditionRefusal> {
        let now = facts.now;
        if matches!(definition.values, ConditionValues::ItemOutfit { .. })
            && definition.item_appearance.is_none()
        {
            return Err(ConditionRefusal::ArithmeticBounds);
        }
        if now < self.pass_at || self.instances.iter().any(|v| v.started_at > now) {
            return Err(ConditionRefusal::ArithmeticBounds);
        }
        // Bound this definition's real duration/next interval before mutation.
        let bound_ms = match definition.values {
            ConditionValues::DamageSchedule { .. } => 0, // Source cursor application checks its first interval below; later advances use checked_add.
            ConditionValues::TimedStatus { duration_ms, .. }
            | ConditionValues::Gate { duration_ms, .. }
            | ConditionValues::SourceAttributes { duration_ms, .. }
            | ConditionValues::RationalSpeed { duration_ms, .. } => duration_ms,
            ConditionValues::DamageSequence { steps, len, .. } => {
                sequence_duration(&steps[..usize::from(len)])
                    .ok_or(ConditionRefusal::ArithmeticBounds)?
            }
            ConditionValues::Speed { duration_ms, .. }
            | ConditionValues::Recovery { duration_ms, .. }
            | ConditionValues::SpellRegeneration { duration_ms, .. }
            | ConditionValues::ManaShield { duration_ms }
            | ConditionValues::Light { duration_ms, .. }
            | ConditionValues::SpellLight { duration_ms, .. }
            | ConditionValues::Attributes { duration_ms, .. }
            | ConditionValues::SpellSkills { duration_ms, .. }
            | ConditionValues::Invisible { duration_ms }
            | ConditionValues::ItemOutfit { duration_ms }
            | ConditionValues::Outfit { duration_ms, .. } => duration_ms,
            ConditionValues::DamageOverTime { interval_ms, .. }
            | ConditionValues::Agony { interval_ms, .. } => interval_ms,
            ConditionValues::FoodRegeneration { .. } => FOOD_REGENERATION_CAP_MS,
        };
        now.checked_add(u64::from(bound_ms) * MICROS_PER_MS)
            .ok_or(ConditionRefusal::ArithmeticBounds)?;
        let condition_type = definition.condition_type();
        let key = condition_type.conflict_key();
        // §3.5: neither creature nor Cleanse immunity refuses `agony`.
        if condition_type != ConditionType::Agony && immunities.contains(&condition_type)
            || matches!(condition_type, ConditionType::SpellSkills(_))
                && immunities.contains(&ConditionType::Attributes)
            || matches!(condition_type, ConditionType::SpellRegeneration(_))
                && immunities.contains(&ConditionType::Recovery)
            || condition_type != ConditionType::Agony
                && self.cleanse_immunity_remaining(key, now) > 0
        {
            return Err(ConditionRefusal::Immune);
        }
        let pve_source_blocked = source_kind == ConditionSourceKind::Player
            && facts.source_reentry_protected
            && !facts.target_is_player;
        let pve_target_blocked =
            source_kind == ConditionSourceKind::Creature && facts.target_reentry_protected;
        if pve_source_blocked || pve_target_blocked {
            return Err(ConditionRefusal::ReentryProtected);
        }
        let current = self
            .instances
            .iter()
            .position(|instance| instance.definition.condition_type().conflict_key() == key);
        if current.is_none() && self.instances.len() >= COND0_RL_01_INSTANCES_PER_ACTOR {
            return Err(ConditionRefusal::InstanceLimit);
        }
        let existing = current.map(|index| &self.instances[index]);
        let ms = |value: u32| u64::from(value) * MICROS_PER_MS;
        // §3.1: a replacement keeps the current tick timing.
        let keep_timing = |interval_ms: u32| {
            existing
                .and_then(|existing| existing.next_tick_at)
                .map_or_else(
                    || {
                        now.checked_add(ms(interval_ms))
                            .ok_or(ConditionRefusal::TimeOverflow)
                    },
                    Ok,
                )
        };
        let next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(ConditionRefusal::SequenceExhausted)?;
        let mut instance = ConditionInstance {
            definition: definition.clone(),
            provenance: ConditionProvenance {
                source,
                source_kind,
                definition_key: definition.key.clone(),
                definition_revision: definition.revision,
                lineage: None,
            },
            sequence: self.next_sequence,
            ends_at: None,
            next_tick_at: None,
            immediate_due: None,
            started_at: now,
            remaining_total: 0,
            scheduled_damage: None,
            sequence_step: 0,
            sequence_repetition: 0,
            speed_delta: 0,
            speed_base: facts.base_speed,
            mana_shield_capacity: 0,
            mana_shield_remaining: 0,
        };
        match definition.values {
            ConditionValues::DamageSchedule { .. } => {
                let spec = definition
                    .damage_schedule
                    .as_ref()
                    .ok_or(ConditionRefusal::DrawFailed)?;
                let cursor = spec.compile(facts)?;
                if let Some(existing) = existing
                    && instance.provenance.source_kind != ConditionSourceKind::Field
                    && spec.comparison_total(&cursor) <= existing.remaining_total
                {
                    return Err(ConditionRefusal::KeptCurrent);
                }
                let first = cursor.current().ok_or(ConditionRefusal::DrawFailed)?;
                instance.remaining_total = cursor.remaining;
                instance.next_tick_at = Some(keep_timing(first.interval_ms)?);
                instance.immediate_due = (!spec.delayed()).then_some(now);
                instance.scheduled_damage = Some(cursor);
            }

            ConditionValues::TimedStatus {
                kind: _,
                duration_ms,
            } => {
                if existing.is_some_and(|old| ms(duration_ms) < old.remaining_at(now)) {
                    return Err(ConditionRefusal::KeptCurrent);
                }
                instance.ends_at = Some(
                    now.checked_add(ms(duration_ms))
                        .ok_or(ConditionRefusal::DrawFailed)?,
                );
            }
            ConditionValues::SourceAttributes { duration_ms, .. } => {
                let end = now
                    .checked_add(ms(duration_ms))
                    .ok_or(ConditionRefusal::DrawFailed)?;
                if existing
                    .and_then(|i| i.ends_at)
                    .is_some_and(|old| old > end)
                {
                    return Err(ConditionRefusal::KeptCurrent);
                }
                instance.ends_at = Some(end);
            }
            ConditionValues::RationalSpeed {
                paralysis,
                range,
                duration_ms,
            } => {
                instance.speed_delta =
                    if paralysis && i64::from(facts.base_speed) < PARALYSIS_SPEED_FLOOR {
                        0
                    } else {
                        let (low, high) = range.endpoints(facts.base_speed)?;
                        draw_speed_endpoints(paralysis, low, high, facts)?
                    };
                instance.ends_at = Some(
                    now.checked_add(ms(duration_ms))
                        .ok_or(ConditionRefusal::DrawFailed)?,
                );
            }

            ConditionValues::Speed {
                paralysis,
                range,
                duration_ms,
            } => {
                instance.speed_delta = draw_speed_delta(paralysis, range, facts)?;
                instance.ends_at = Some(
                    now.checked_add(ms(duration_ms))
                        .ok_or(ConditionRefusal::TimeOverflow)?,
                );
            }
            ConditionValues::DamageOverTime {
                total_min,
                total_max,
                interval_ms,
                delayed,
                ..
            } => {
                if let Some(existing) = existing {
                    let midpoint = (u64::from(total_min) + u64::from(total_max)) / 2;
                    let is_field = instance.provenance.source_kind == ConditionSourceKind::Field;
                    if !is_field && midpoint <= u64::from(existing.remaining_total) {
                        return Err(ConditionRefusal::KeptCurrent);
                    }
                }
                instance.remaining_total =
                    draw_in_range(total_min, total_max, facts, COND_DOT_TOTAL_DRAW)?;
                instance.next_tick_at = Some(keep_timing(interval_ms)?);
                instance.immediate_due = (!delayed).then_some(now);
            }
            ConditionValues::DamageSequence {
                steps,
                len,
                delayed,
                ..
            } => {
                let total = sequence_total(&steps[..usize::from(len)])
                    .ok_or(ConditionRefusal::ArithmeticBounds)?;
                if existing.is_some_and(|old| {
                    source_kind != ConditionSourceKind::Field && total <= old.remaining_total
                }) {
                    return Err(ConditionRefusal::KeptCurrent);
                }
                instance.remaining_total = total;
                instance.next_tick_at = Some(keep_timing(steps[0].interval_ms)?);
                instance.immediate_due = (!delayed).then_some(now);
            }
            ConditionValues::Agony {
                total_min,
                total_max,
                interval_ms,
                delayed,
                ..
            } => {
                // §3.5: replace when the new total is at least the remaining one; an undrawn
                // total counts as the midpoint (§3.1). The replacement follows §3.1.
                if let Some(existing) = existing {
                    let midpoint = (u64::from(total_min) + u64::from(total_max)) / 2;
                    if midpoint < u64::from(existing.remaining_total) {
                        return Err(ConditionRefusal::KeptCurrent);
                    }
                }
                instance.remaining_total =
                    draw_in_range(total_min, total_max, facts, COND_DOT_TOTAL_DRAW)?;
                instance.next_tick_at = Some(keep_timing(interval_ms)?);
                instance.immediate_due = (!delayed).then_some(now);
            }
            ConditionValues::Gate { duration_ms, .. } => {
                // §3.5: refresh the duration, also when the new one is shorter.
                instance.ends_at = Some(
                    now.checked_add(ms(duration_ms))
                        .ok_or(ConditionRefusal::TimeOverflow)?,
                );
            }
            ConditionValues::FoodRegeneration {
                added_ms,
                interval_ms,
            } => {
                let remaining = existing.map_or(0, |existing| existing.remaining_at(now));
                let total = remaining
                    .checked_add(ms(added_ms))
                    .ok_or(ConditionRefusal::TimeOverflow)?;
                if total >= ms(FOOD_REGENERATION_CAP_MS) {
                    return Err(ConditionRefusal::Full);
                }
                instance.ends_at = Some(
                    now.checked_add(total)
                        .ok_or(ConditionRefusal::TimeOverflow)?,
                );
                instance.next_tick_at = Some(keep_timing(interval_ms)?);
            }
            ConditionValues::Recovery {
                duration_ms,
                interval_ms,
            } => {
                instance.ends_at = Some(
                    now.checked_add(ms(duration_ms))
                        .ok_or(ConditionRefusal::TimeOverflow)?,
                );
                instance.next_tick_at = Some(keep_timing(interval_ms)?);
            }
            ConditionValues::ManaShield { duration_ms } => {
                instance.ends_at = Some(
                    now.checked_add(ms(duration_ms))
                        .ok_or(ConditionRefusal::TimeOverflow)?,
                );
                instance.mana_shield_capacity = facts.mana_shield_capacity;
                instance.mana_shield_remaining = facts.mana_shield_capacity;
            }
            ConditionValues::SpellRegeneration {
                duration_ms,
                health_interval_ms,
                mana_interval_ms,
                ..
            } => {
                instance.ends_at = Some(now + ms(duration_ms));
                instance.next_tick_at = Some(now + ms(health_interval_ms.min(mana_interval_ms)));
            }
            ConditionValues::Attributes { duration_ms, .. }
            | ConditionValues::SpellSkills { duration_ms, .. }
            | ConditionValues::Invisible { duration_ms } => {
                if existing
                    .is_some_and(|v| v.ends_at.is_none() || ms(duration_ms) < v.remaining_at(now))
                {
                    return Err(ConditionRefusal::KeptCurrent);
                }
                instance.ends_at = Some(now + ms(duration_ms));
            }
            ConditionValues::ItemOutfit { duration_ms }
            | ConditionValues::Outfit { duration_ms, .. } => {
                if existing
                    .is_some_and(|v| v.ends_at.is_none() || ms(duration_ms) < v.remaining_at(now))
                {
                    return Err(ConditionRefusal::KeptCurrent);
                }
                instance.ends_at = Some(now + ms(duration_ms));
            }
            ConditionValues::Light { duration_ms, .. }
            | ConditionValues::SpellLight { duration_ms, .. } => {
                if existing.is_some_and(|existing| ms(duration_ms) < existing.remaining_at(now)) {
                    return Err(ConditionRefusal::KeptCurrent);
                }
                instance.ends_at = Some(
                    now.checked_add(ms(duration_ms))
                        .ok_or(ConditionRefusal::TimeOverflow)?,
                );
            }
        }
        let sequence = instance.sequence;
        self.next_sequence = next_sequence;
        let replaced = current.is_some();
        if let Some(index) = current {
            self.instances.remove(index);
        }
        self.instances.push(instance);
        if matches!(
            definition.values,
            ConditionValues::Gate {
                gate: ConditionGate::Pacified,
                ..
            }
        ) {
            self.pacified_applied_at = Some(now);
        }
        Ok(Applied { sequence, replaced })
    }

    /// Remaining immunity on the removed conflict key. CONDITIONS-0 §5 deliberately binds
    /// the key; haste and paralysis therefore share this temporary immunity.
    pub fn cleanse_immunity_remaining(&self, key: ConflictKey, now: u64) -> u64 {
        self.cleanse_immunities
            .iter()
            .find(|immunity| immunity.key == key)
            .map_or(0, |immunity| immunity.until.saturating_sub(now))
    }

    /// Chooses one eligible negative instance without changing the store. Drowning and
    /// already expired conditions are excluded; selection is stable in instance sequence order.
    pub fn prepare_cleanse(
        &self,
        facts: &ApplicationFacts<'_>,
    ) -> Result<Option<CleansePlan>, ConditionRefusal> {
        let mut candidates: Vec<_> = self
            .instances
            .iter()
            .filter(|instance| {
                let kind = instance.definition.condition_type();
                instance.definition.is_negative()
                    && kind != ConditionType::DamageOverTime(DotElement::Drown)
                    && instance.ends_at.is_none_or(|end| end > facts.now)
                    && (!matches!(kind, ConditionType::DamageOverTime(_))
                        || instance.remaining_total > 0)
            })
            .collect();
        candidates.sort_by_key(|instance| instance.sequence);
        let index = match candidates.len() {
            0 => return Ok(None),
            1 => 0,
            count => {
                let draw = deterministic_decision_u64(
                    facts.decision_root,
                    facts.occurrence,
                    COND_CLEANSE_PICK,
                    0,
                )
                .map_err(|_| ConditionRefusal::DrawFailed)?;
                let count = u64::try_from(count).map_err(|_| ConditionRefusal::DrawFailed)?;
                usize::try_from(draw % count).map_err(|_| ConditionRefusal::DrawFailed)?
            }
        };
        let instance = candidates[index];
        let immune_until = facts
            .now
            .checked_add(u64::from(CLEANSE_IMMUNITY_MS) * MICROS_PER_MS)
            .ok_or(ConditionRefusal::TimeOverflow)?;
        Ok(Some(CleansePlan {
            sequence: instance.sequence,
            key: instance.definition.condition_type().conflict_key(),
            immune_until,
        }))
    }

    /// Applies the retained plan once. A removed or replaced instance is never substituted
    /// with another candidate; replay neither removes another condition nor extends immunity.
    pub fn commit_cleanse(&mut self, plan: CleansePlan) -> bool {
        let Some(index) = self.instances.iter().position(|instance| {
            instance.sequence == plan.sequence
                && instance.definition.condition_type().conflict_key() == plan.key
        }) else {
            return false;
        };
        self.instances.remove(index);
        self.cleanse_immunities
            .retain(|immunity| immunity.key != plan.key);
        self.cleanse_immunities.push(CleanseImmunity {
            key: plan.key,
            until: plan.immune_until,
        });
        true
    }

    /// §3.4: the ticks due at `now`, in `(due, sequence)` order. One simulation tick (`now`)
    /// deals at most `COND0-RL-03` ticks per actor, across every pass at that `now`; the rest stay
    /// due, in order, for the next simulation tick (`RUN_EACH_BOUNDED`), and none is dropped.
    /// Instances whose time is over and whose ticks are all dealt end here.
    pub fn take_due(&mut self, now: u64, facts: TickFacts) -> Vec<ConditionTick<S>> {
        self.take_due_matching(now, facts, true, None)
    }
    /// Only a just-applied field's immediate tick; retained overdue conditions are untouched.
    pub fn take_due_source_application(
        &mut self,
        now: u64,
        facts: TickFacts,
        source: &S,
    ) -> Vec<ConditionTick<S>>
    where
        S: PartialEq,
    {
        let matches =
            |provenance: &ConditionProvenance<S>| provenance.source.as_ref() == Some(source);
        self.take_due_matching(now, facts, true, Some(&matches))
    }
    /// Independent qualified regeneration must not drain unknown combat ticks.
    pub fn take_due_non_damage(&mut self, now: u64, facts: TickFacts) -> Vec<ConditionTick<S>> {
        self.take_due_matching(now, facts, false, None)
    }
    fn take_due_matching(
        &mut self,
        now: u64,
        facts: TickFacts,
        damage: bool,
        application_source: Option<&ConditionApplicationSource<'_, S>>,
    ) -> Vec<ConditionTick<S>> {
        self.cleanse_immunities
            .retain(|immunity| immunity.until > now);
        if self.pass_at != now {
            self.pass_at = now;
            self.pass_ticks = 0;
        }
        let mut ticks = Vec::new();
        while self.pass_ticks < COND0_RL_03_DAMAGE_TICKS_PER_SIM_TICK {
            let next = self
                .instances
                .iter()
                .enumerate()
                .filter_map(|(index, instance)| {
                    if application_source.is_some_and(|matches| {
                        !matches(&instance.provenance) || instance.immediate_due != Some(now)
                    }) {
                        return None;
                    }
                    if !damage
                        && matches!(
                            instance.definition.condition_type(),
                            ConditionType::DamageOverTime(_) | ConditionType::Agony
                        )
                    {
                        return None;
                    }
                    let scheduled = instance
                        .next_tick_at
                        .filter(|&due| instance.ends_at.is_none_or(|end| due <= end));
                    let due = match (instance.immediate_due, scheduled) {
                        (Some(a), Some(b)) => a.min(b),
                        (a, b) => a.or(b)?,
                    };
                    (due <= now).then_some((due, instance.sequence, index))
                })
                .min();
            let Some((due, _, index)) = next else { break };
            let instance = &mut self.instances[index];
            let kind = match instance.definition.values {
                ConditionValues::DamageSchedule { element } => {
                    // A malformed internal cursor must not emit a fabricated tick or remove
                    // a condition. Preserve earlier valid ticks and this pass's consumed budget.
                    let Some(cursor) = instance.scheduled_damage.as_mut() else {
                        return ticks;
                    };
                    let Some(segment) = cursor.current() else {
                        return ticks;
                    };
                    let immediate = instance.immediate_due == Some(due);
                    if immediate {
                        instance.immediate_due = None;
                    }
                    // As in the sequence path, the application tick is consumed even on the
                    // condition's own field; only later field-standing ticks retain a segment.
                    if immediate
                        || facts.standing_on_field != Some(element)
                        || facts.in_protection_zone
                    {
                        cursor.consume();
                    }
                    instance.remaining_total = cursor.remaining;
                    // The next source segment's interval begins after the current tick. A
                    // standing field reuses this same segment rather than consuming it.
                    instance.next_tick_at = cursor.current().and_then(|next| {
                        due.checked_add(u64::from(next.interval_ms) * MICROS_PER_MS)
                    });
                    TickKind::Damage {
                        element,
                        amount: segment.amount,
                        refused: facts.in_protection_zone,
                    }
                }

                ConditionValues::DamageSequence {
                    element,
                    steps,
                    len,
                    ..
                } => {
                    let step = steps[usize::from(instance.sequence_step)];
                    let amount = step.amount.min(instance.remaining_total);
                    let immediate = instance.immediate_due == Some(due);
                    if immediate {
                        instance.immediate_due = None;
                    }
                    // Source getNextDamage pops the application tick even on
                    // its own field; only later field-standing ticks retain it.
                    if immediate
                        || facts.standing_on_field != Some(element)
                        || facts.in_protection_zone
                    {
                        instance.remaining_total -= amount;
                        instance.sequence_repetition += 1;
                        if instance.sequence_repetition >= step.repetitions {
                            instance.sequence_repetition = 0;
                            instance.sequence_step += 1;
                        }
                    }
                    instance.next_tick_at = if instance.sequence_step < len {
                        Some(
                            due + u64::from(steps[usize::from(instance.sequence_step)].interval_ms)
                                * MICROS_PER_MS,
                        )
                    } else {
                        None
                    };
                    TickKind::Damage {
                        element,
                        amount,
                        refused: facts.in_protection_zone,
                    }
                }
                ConditionValues::DamageOverTime {
                    element,
                    per_tick,
                    interval_ms,
                    ..
                } => {
                    if instance.immediate_due == Some(due) {
                        instance.immediate_due = None;
                    } else {
                        instance.next_tick_at =
                            due.checked_add(u64::from(interval_ms) * MICROS_PER_MS);
                    }
                    let amount = per_tick.min(instance.remaining_total);
                    if facts.standing_on_field != Some(element) || facts.in_protection_zone {
                        instance.remaining_total -= amount;
                    }
                    TickKind::Damage {
                        element,
                        amount,
                        refused: facts.in_protection_zone,
                    }
                }
                ConditionValues::Agony {
                    per_tick,
                    interval_ms,
                    ..
                } => {
                    if instance.immediate_due == Some(due) {
                        instance.immediate_due = None;
                    } else {
                        instance.next_tick_at =
                            due.checked_add(u64::from(interval_ms) * MICROS_PER_MS);
                    }
                    // No element, so no field keeps it: every occurrence is used up.
                    let amount = per_tick.min(instance.remaining_total);
                    instance.remaining_total -= amount;
                    TickKind::Agony {
                        amount,
                        refused: facts.in_protection_zone,
                    }
                }
                ConditionValues::FoodRegeneration { interval_ms, .. }
                | ConditionValues::Recovery { interval_ms, .. } => {
                    let key = instance.definition.condition_type().conflict_key();
                    instance.next_tick_at = due.checked_add(u64::from(interval_ms) * MICROS_PER_MS);
                    TickKind::Regeneration {
                        key,
                        suppressed: facts.in_protection_zone
                            && key == ConflictKey::FoodRegeneration,
                    }
                }
                ConditionValues::SpellRegeneration {
                    health_gain,
                    health_interval_ms,
                    mana_gain,
                    mana_interval_ms,
                    ..
                } => {
                    let elapsed = due - instance.started_at;
                    let h = u64::from(health_interval_ms) * MICROS_PER_MS;
                    let m = u64::from(mana_interval_ms) * MICROS_PER_MS;
                    let next_h = (elapsed / h + 1)
                        .checked_mul(h)
                        .and_then(|v| instance.started_at.checked_add(v));
                    let next_m = (elapsed / m + 1)
                        .checked_mul(m)
                        .and_then(|v| instance.started_at.checked_add(v));
                    instance.next_tick_at = match (next_h, next_m) {
                        (Some(a), Some(b)) => Some(a.min(b)),
                        (a, b) => a.or(b),
                    };
                    TickKind::SpellRegeneration {
                        health_gain: if elapsed.is_multiple_of(h) {
                            health_gain
                        } else {
                            0
                        },
                        mana_gain: if elapsed.is_multiple_of(m) {
                            mana_gain
                        } else {
                            0
                        },
                        suppressed: facts.in_protection_zone,
                    }
                }
                // Speed, mana shield and light never schedule a tick.
                _ => {
                    instance.next_tick_at = None;
                    continue;
                }
            };
            self.pass_ticks += 1;
            ticks.push(ConditionTick {
                due,
                sequence: instance.sequence,
                kind,
                provenance: instance.provenance.clone(),
            });
        }
        self.instances.retain(|instance| match instance.ends_at {
            None => instance.remaining_total > 0,
            Some(end) => end > now || instance.next_tick_at.is_some_and(|due| due <= end),
        });
        ticks
    }

    /// §3.5: the gate families and `agony` end only by their duration, death or the §6.3
    /// lifecycle; no dispel names them.
    pub fn dispel_key(&mut self, key: ConflictKey) -> usize {
        if matches!(key, ConflictKey::Gate(_) | ConflictKey::Agony) {
            return 0;
        }
        let before = self.instances.len();
        self.instances
            .retain(|instance| instance.definition.condition_type().conflict_key() != key);
        before - self.instances.len()
    }
    pub fn cleanse_negative(&mut self) -> usize {
        let before = self.instances.len();
        self.instances
            .retain(|instance| !instance.definition.is_negative());
        before - self.instances.len()
    }

    /// §6.1 death: every instance ends, and so does the pacified application a later target
    /// would otherwise be measured against.
    pub fn clear_on_death(&mut self) {
        self.instances.clear();
        self.cleanse_immunities.clear();
        self.pacified_applied_at = None;
    }
}

/// §3.5 gates. Once the action is identified they run before every other cast and attack
/// admission check (protection zone included), before cost and cooldown; a refusal spends nothing.
impl<S: Clone> ConditionStore<S> {
    /// §3.5: the same current-time view for a gate family.
    pub fn has_gate(&self, gate: ConditionGate, now: u64) -> bool {
        self.get(ConflictKey::Gate(gate))
            .is_some_and(|instance| instance.ends_at.is_some_and(|end| now < end))
    }

    /// §3.5 spell cast admission: `muted` refuses any spell cast; `pacified` and `powerless`
    /// refuse one whose authored profile is `targeting.aggressive`. The first refusal in the
    /// order `muted`, `pacified`, `powerless` answers alone.
    pub fn spell_cast_gate(&self, aggressive: bool, now: u64) -> Result<(), ConditionGate> {
        if self.has_gate(ConditionGate::Muted, now) {
            return Err(ConditionGate::Muted);
        }
        self.rune_gate(aggressive, now)
    }

    /// §3.5 rune admission: `pacified` and `powerless` refuse an aggressive rune, in that
    /// order; `muted` never gates a rune.
    pub fn rune_gate(&self, aggressive: bool, now: u64) -> Result<(), ConditionGate> {
        if !aggressive {
            return Ok(());
        }
        [ConditionGate::Pacified, ConditionGate::Powerless]
            .into_iter()
            .find(|&gate| self.has_gate(gate, now))
            .map_or(Ok(()), Err)
    }

    /// §3.5 attack target admission: `pacified` refuses setting an attack target.
    pub fn attack_target_gate(&self, now: u64) -> Result<(), ConditionGate> {
        if self.has_gate(ConditionGate::Pacified, now) {
            return Err(ConditionGate::Pacified);
        }
        Ok(())
    }

    /// ATTACK-0 §4 (as amended by #2106): a `pacified` was applied at or after `since`, current
    /// or since expired. A target held from `since` is cleared by that application.
    pub fn pacified_applied_since(&self, since: u64) -> bool {
        self.pacified_applied_at.is_some_and(|at| at >= since)
    }
}

fn sequence_total(steps: &[DotSequenceStep]) -> Option<u32> {
    steps.iter().try_fold(0u32, |sum, s| {
        sum.checked_add(s.amount.checked_mul(s.repetitions)?)
    })
}
fn sequence_duration(steps: &[DotSequenceStep]) -> Option<u32> {
    steps.iter().try_fold(0u32, |sum, s| {
        sum.checked_add(s.interval_ms.checked_mul(s.repetitions)?)
    })
}

/// §3.4: merges each actor's due ticks into the channel order `(due, actor, sequence)`.
///
/// `COND0-RL-03` bounds each actor on its own: the order covers the ticks that run in this
/// simulation tick. An actor's capped ticks do not hold back other actors' keys; they run in the
/// actor's next simulation ticks, keeping their original `due`, so they sort ahead of anything
/// newly due there.
pub fn channel_tick_order<A: Ord + Copy, S>(
    per_actor: impl IntoIterator<Item = (A, Vec<ConditionTick<S>>)>,
) -> Vec<(A, ConditionTick<S>)> {
    let mut all: Vec<(A, ConditionTick<S>)> = per_actor
        .into_iter()
        .flat_map(|(actor, ticks)| ticks.into_iter().map(move |tick| (actor, tick)))
        .collect();
    all.sort_by_key(|(actor, tick)| (tick.due, *actor, tick.sequence));
    all
}

fn draw_in_range(
    min: u32,
    max: u32,
    facts: &ApplicationFacts<'_>,
    purpose: &str,
) -> Result<u32, ConditionRefusal> {
    if min == max {
        return Ok(min);
    }
    let value = deterministic_decision_u64(facts.decision_root, facts.occurrence, purpose, 0)
        .map_err(|_| ConditionRefusal::DrawFailed)?;
    let span = u64::from(max - min) + 1;
    Ok(min + u32::try_from(value % span).map_err(|_| ConditionRefusal::DrawFailed)?)
}

/// §3 speed: the target is drawn uniformly in the range; the delta is target − base, fixed now.
/// Paralysis keeps the target at least 40 and leaves a base under 40 unchanged.
fn draw_speed_delta(
    paralysis: bool,
    range: SpeedRange,
    facts: &ApplicationFacts<'_>,
) -> Result<i64, ConditionRefusal> {
    let base = i64::from(facts.base_speed);
    if paralysis && base < PARALYSIS_SPEED_FLOOR {
        return Ok(0);
    }
    let end = |a: i32, b: i32| (i64::from(a) * (base - 40) + i64::from(b) * 1_000) / 1_000;
    let (low, high) = {
        let (x, y) = (end(range.a_min, range.b_min), end(range.a_max, range.b_max));
        (x.min(y), x.max(y))
    };
    draw_speed_endpoints(paralysis, low, high, facts)
}

fn draw_speed_endpoints(
    paralysis: bool,
    low: i64,
    high: i64,
    facts: &ApplicationFacts<'_>,
) -> Result<i64, ConditionRefusal> {
    let target = if low == high {
        low
    } else {
        let value =
            deterministic_decision_u64(facts.decision_root, facts.occurrence, COND_SPEED_DRAW, 0)
                .map_err(|_| ConditionRefusal::DrawFailed)?;
        let span = u64::try_from(i128::from(high) - i128::from(low) + 1)
            .map_err(|_| ConditionRefusal::DrawFailed)?;
        i64::try_from(i128::from(low) + i128::from(value % span))
            .map_err(|_| ConditionRefusal::DrawFailed)?
    };
    let target = if paralysis {
        target.max(PARALYSIS_SPEED_FLOOR)
    } else {
        target
    };
    target
        .checked_sub(i64::from(facts.base_speed))
        .ok_or(ConditionRefusal::DrawFailed)
}

#[cfg(test)]
#[path = "condition_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "reconciled_condition_tests.rs"]
mod reconciled_condition_tests;

#[cfg(test)]
#[path = "reconciled_schedule_tests.rs"]
mod reconciled_schedule_tests;

#[cfg(test)]
#[path = "condition_gate_tests.rs"]
mod condition_gate_tests;

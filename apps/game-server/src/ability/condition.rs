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

use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

use super::occurrence::valid_atom;

/// `COND0-RL-01`: instances per actor.
pub(crate) const COND0_RL_01_INSTANCES_PER_ACTOR: usize = 16;
/// `COND0-RL-02`: minimum tick interval, in milliseconds.
pub(crate) const COND0_RL_02_MIN_TICK_INTERVAL_MS: u32 = 1_000;
/// `COND0-RL-03`: damage ticks per actor per simulation tick. The store also counts regeneration
/// ticks against it, so a catch-up of any tick kind is bounded per pass (`RUN_EACH_BOUNDED`).
pub(crate) const COND0_RL_03_DAMAGE_TICKS_PER_SIM_TICK: usize = 4;
/// ITEM-USE-0 §6.1: the food time cap (a parity value, not a resource limit).
pub(crate) const FOOD_REGENERATION_CAP_MS: u32 = 1_200_000;
/// §3: paralysis never takes speed below this.
pub(crate) const PARALYSIS_SPEED_FLOOR: i64 = 40;

pub(crate) const COND_SPEED_DRAW: &str = "oteryn.condition.speed_draw.v1";
pub(crate) const COND_DOT_TOTAL_DRAW: &str = "oteryn.condition.dot_total_draw.v1";
pub(crate) const COND_CLEANSE_PICK: &str = "oteryn.condition.cleanse_pick.v1";
/// CONDITIONS-0 §5: immunity to the removed conflict key.
pub(crate) const CLEANSE_IMMUNITY_MS: u32 = 11_000;

const MICROS_PER_MS: u64 = 1_000;

/// The damage-over-time elements, one conflict key each (§3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum DotElement {
    Poison,
    Fire,
    Energy,
    Bleeding,
    Drown,
    Freezing,
    Dazzled,
    Cursed,
}

/// §3: at most one instance exists per conflict key; 13 keys today, under `COND0-RL-01`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ConflictKey {
    Speed,
    Element(DotElement),
    FoodRegeneration,
    Recovery,
    ManaShield,
    Light,
}

/// The closed condition types. A creature's content immunities name these; haste and paralysis
/// share the `speed` key but not an immunity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum ConditionType {
    Haste,
    Paralysis,
    DamageOverTime(DotElement),
    FoodRegeneration,
    Recovery,
    ManaShield,
    Light,
}

impl ConditionType {
    pub(crate) const fn conflict_key(self) -> ConflictKey {
        match self {
            Self::Haste | Self::Paralysis => ConflictKey::Speed,
            Self::DamageOverTime(element) => ConflictKey::Element(element),
            Self::FoodRegeneration => ConflictKey::FoodRegeneration,
            Self::Recovery => ConflictKey::Recovery,
            Self::ManaShield => ConflictKey::ManaShield,
            Self::Light => ConflictKey::Light,
        }
    }

    /// §5: the `negative` dispel tag: damage over time and paralysis.
    pub(crate) const fn is_negative(self) -> bool {
        matches!(self, Self::Paralysis | Self::DamageOverTime(_))
    }
}

/// A speed range in thousandths: the target speed is drawn in
/// `[a_min × (base − 40) + b_min, a_max × (base − 40) + b_max]` (§3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpeedRange {
    pub(crate) a_min: i32,
    pub(crate) b_min: i32,
    pub(crate) a_max: i32,
    pub(crate) b_max: i32,
}

/// The content values of one definition (§3: values come from content, never from code).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConditionValues {
    Speed {
        paralysis: bool,
        range: SpeedRange,
        duration_ms: u32,
    },
    DamageOverTime {
        element: DotElement,
        total_min: u32,
        total_max: u32,
        per_tick: u32,
        interval_ms: u32,
        /// `false`: the first tick is dealt at once on application.
        delayed: bool,
    },
    FoodRegeneration {
        added_ms: u32,
        interval_ms: u32,
    },
    Recovery {
        duration_ms: u32,
        interval_ms: u32,
    },
    ManaShield {
        duration_ms: u32,
    },
    Light {
        level: u8,
        duration_ms: u32,
    },
}

/// A versioned `ConditionDefinition` (GAME-ABILITY-01), built by the content loader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConditionDefinition {
    key: String,
    revision: u32,
    values: ConditionValues,
}

impl ConditionDefinition {
    /// `None` for an invalid key, a tick interval under `COND0-RL-02`, a zero duration or
    /// amount, or an inverted damage range.
    pub(crate) fn new(key: &str, revision: u32, values: ConditionValues) -> Option<Self> {
        let interval_ok = |ms: u32| ms >= COND0_RL_02_MIN_TICK_INTERVAL_MS;
        let valid = valid_atom(key)
            && match values {
                ConditionValues::Speed { duration_ms, .. }
                | ConditionValues::ManaShield { duration_ms }
                | ConditionValues::Light { duration_ms, .. } => duration_ms > 0,
                ConditionValues::DamageOverTime {
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
        })
    }

    pub(crate) fn key(&self) -> &str {
        &self.key
    }

    pub(crate) const fn revision(&self) -> u32 {
        self.revision
    }

    pub(crate) const fn values(&self) -> ConditionValues {
        self.values
    }

    pub(crate) const fn condition_type(&self) -> ConditionType {
        match self.values {
            ConditionValues::Speed {
                paralysis: true, ..
            } => ConditionType::Paralysis,
            ConditionValues::Speed { .. } => ConditionType::Haste,
            ConditionValues::DamageOverTime { element, .. } => {
                ConditionType::DamageOverTime(element)
            }
            ConditionValues::FoodRegeneration { .. } => ConditionType::FoodRegeneration,
            ConditionValues::Recovery { .. } => ConditionType::Recovery,
            ConditionValues::ManaShield { .. } => ConditionType::ManaShield,
            ConditionValues::Light { .. } => ConditionType::Light,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConditionSourceKind {
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
pub(crate) struct ConditionProvenance<S> {
    pub(crate) source: Option<S>,
    pub(crate) source_kind: ConditionSourceKind,
    pub(crate) definition_key: String,
    pub(crate) definition_revision: u32,
}

/// One live instance of a definition on one actor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConditionInstance<S> {
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
    /// The speed delta fixed at application.
    speed_delta: i64,
    mana_shield_capacity: u32,
    mana_shield_remaining: u32,
}

impl<S> ConditionInstance<S> {
    pub(crate) const fn definition(&self) -> &ConditionDefinition {
        &self.definition
    }

    pub(crate) const fn provenance(&self) -> &ConditionProvenance<S> {
        &self.provenance
    }

    pub(crate) const fn sequence(&self) -> u32 {
        self.sequence
    }

    pub(crate) const fn speed_delta(&self) -> i64 {
        self.speed_delta
    }

    pub(crate) const fn remaining_total(&self) -> u32 {
        self.remaining_total
    }

    /// `(remaining, capacity)` of a mana shield (§3.3).
    pub(crate) const fn mana_shield(&self) -> (u32, u32) {
        (self.mana_shield_remaining, self.mana_shield_capacity)
    }

    /// §3: the light level decays linearly over the duration.
    pub(crate) fn light_level(&self, now: u64) -> u8 {
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
pub(crate) struct ApplicationFacts<'a> {
    pub(crate) now: u64,
    /// The target's base speed, for a `SPEED` definition.
    pub(crate) base_speed: u16,
    /// The mana shield capacity from the content formula, at most maximum mana (§3.3).
    pub(crate) mana_shield_capacity: u32,
    /// §6.1 PvE re-entry protection: the target, or the applying player, is in its window.
    pub(crate) target_reentry_protected: bool,
    pub(crate) source_reentry_protected: bool,
    pub(crate) target_is_player: bool,
    pub(crate) decision_root: &'a GameplayDecisionRoot,
    pub(crate) occurrence: DecisionOccurrenceId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConditionRefusal {
    /// The target's content lists an immunity to this type (§6.1).
    Immune,
    /// §6.1 PvE re-entry protection.
    ReentryProtected,
    /// The policy kept the current instance (§3.1 not strictly greater; light shorter).
    KeptCurrent,
    /// ITEM-USE-0 §6.1: the food time would reach its cap; nothing is burned.
    Full,
    /// `COND0-RL-01`.
    InstanceLimit,
    DrawFailed,
    /// A prepared Cleanse deadline cannot be represented; no instance is removed.
    TimeOverflow,
    /// No instance identity is reused after exhausting the actor's sequence space.
    SequenceExhausted,
}

/// A committed application. A damage over time that is not `delayed` has its first tick due at
/// once: the owner's `take_due` pass in the same simulation tick deals it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Applied {
    pub(crate) sequence: u32,
    pub(crate) replaced: bool,
}

/// A due tick, in the §3.4 order `(due, actor, sequence)`. It carries the instance's frozen
/// provenance (§3.2), so the owner can build its Effect Plan even when this was the last tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConditionTick<S> {
    pub(crate) due: u64,
    pub(crate) sequence: u32,
    pub(crate) kind: TickKind,
    pub(crate) provenance: ConditionProvenance<S>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TickKind {
    /// One damage occurrence (§3.2); `refused` in a protection zone, where it is still used up.
    Damage {
        element: DotElement,
        amount: u32,
        refused: bool,
    },
    /// A regeneration tick (FOOD-REGEN-1, Recovery); `suppressed` regenerates nothing.
    Regeneration { key: ConflictKey, suppressed: bool },
}

/// The owner facts read with a tick pass, in the same work item.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TickFacts {
    pub(crate) in_protection_zone: bool,
    /// The element of a field the actor stands on: ticks of that element are not used up.
    pub(crate) standing_on_field: Option<DotElement>,
}

/// Frozen removal chosen before commit. The actor owner retains this exact plan for replay;
/// it must never reselect from changed conditions for the same occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CleansePlan {
    sequence: u32,
    key: ConflictKey,
    immune_until: u64,
}

impl CleansePlan {
    pub(crate) const fn conflict_key(self) -> ConflictKey {
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
pub(crate) struct ConditionStore<S> {
    /// Sorted by sequence.
    instances: Vec<ConditionInstance<S>>,
    /// At most one immunity per closed conflict key; actor movement retains this same store.
    cleanse_immunities: Vec<CleanseImmunity>,
    next_sequence: u32,
    /// The simulation tick (its semantic time) of the current pass and the ticks it has dealt.
    pass_at: u64,
    pass_ticks: usize,
}

impl<S> Default for ConditionStore<S> {
    fn default() -> Self {
        Self {
            instances: Vec::new(),
            cleanse_immunities: Vec::new(),
            next_sequence: 0,
            pass_at: 0,
            pass_ticks: 0,
        }
    }
}

impl<S: Clone> ConditionStore<S> {
    /// §6.2 fresh admission: no instances (food time is restored by COND-DUR-1).
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn instances(&self) -> &[ConditionInstance<S>] {
        &self.instances
    }

    pub(crate) fn get(&self, key: ConflictKey) -> Option<&ConditionInstance<S>> {
        self.instances
            .iter()
            .find(|instance| instance.definition.condition_type().conflict_key() == key)
    }

    /// The effective `SPEED` delta, 0 without one.
    pub(crate) fn speed_delta(&self) -> i64 {
        self.get(ConflictKey::Speed)
            .map_or(0, |instance| instance.speed_delta)
    }

    /// One application at admission (§3, §3.1, §6.1). The provenance is frozen here (§3.2): the
    /// definition key and revision are always those of `definition`, never a caller's copy.
    pub(crate) fn apply(
        &mut self,
        definition: &ConditionDefinition,
        source: Option<S>,
        source_kind: ConditionSourceKind,
        immunities: &[ConditionType],
        facts: &ApplicationFacts<'_>,
    ) -> Result<Applied, ConditionRefusal> {
        let now = facts.now;
        let condition_type = definition.condition_type();
        let key = condition_type.conflict_key();
        if immunities.contains(&condition_type) || self.cleanse_immunity_remaining(key, now) > 0 {
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
                .unwrap_or(now + ms(interval_ms))
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
            },
            sequence: self.next_sequence,
            ends_at: None,
            next_tick_at: None,
            immediate_due: None,
            started_at: now,
            remaining_total: 0,
            speed_delta: 0,
            mana_shield_capacity: 0,
            mana_shield_remaining: 0,
        };
        match definition.values {
            ConditionValues::Speed {
                paralysis,
                range,
                duration_ms,
            } => {
                instance.speed_delta = draw_speed_delta(paralysis, range, facts)?;
                instance.ends_at = Some(now + ms(duration_ms));
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
                instance.next_tick_at = Some(keep_timing(interval_ms));
                instance.immediate_due = (!delayed).then_some(now);
            }
            ConditionValues::FoodRegeneration {
                added_ms,
                interval_ms,
            } => {
                let remaining = existing.map_or(0, |existing| existing.remaining_at(now));
                let total = remaining + ms(added_ms);
                if total >= ms(FOOD_REGENERATION_CAP_MS) {
                    return Err(ConditionRefusal::Full);
                }
                instance.ends_at = Some(now + total);
                instance.next_tick_at = Some(keep_timing(interval_ms));
            }
            ConditionValues::Recovery {
                duration_ms,
                interval_ms,
            } => {
                instance.ends_at = Some(now + ms(duration_ms));
                instance.next_tick_at = Some(keep_timing(interval_ms));
            }
            ConditionValues::ManaShield { duration_ms } => {
                instance.ends_at = Some(now + ms(duration_ms));
                instance.mana_shield_capacity = facts.mana_shield_capacity;
                instance.mana_shield_remaining = facts.mana_shield_capacity;
            }
            ConditionValues::Light { duration_ms, .. } => {
                if existing.is_some_and(|existing| ms(duration_ms) < existing.remaining_at(now)) {
                    return Err(ConditionRefusal::KeptCurrent);
                }
                instance.ends_at = Some(now + ms(duration_ms));
            }
        }
        let sequence = instance.sequence;
        self.next_sequence = next_sequence;
        let replaced = current.is_some();
        if let Some(index) = current {
            self.instances.remove(index);
        }
        self.instances.push(instance);
        Ok(Applied { sequence, replaced })
    }

    /// Remaining immunity on the removed conflict key. CONDITIONS-0 §5 deliberately binds
    /// the key; haste and paralysis therefore share this temporary immunity.
    pub(crate) fn cleanse_immunity_remaining(&self, key: ConflictKey, now: u64) -> u64 {
        self.cleanse_immunities
            .iter()
            .find(|immunity| immunity.key == key)
            .map_or(0, |immunity| immunity.until.saturating_sub(now))
    }

    /// Chooses one eligible negative instance without changing the store. Drowning and
    /// already expired conditions are excluded; selection is stable in instance sequence order.
    pub(crate) fn prepare_cleanse(
        &self,
        facts: &ApplicationFacts<'_>,
    ) -> Result<Option<CleansePlan>, ConditionRefusal> {
        let mut candidates: Vec<_> = self
            .instances
            .iter()
            .filter(|instance| {
                let kind = instance.definition.condition_type();
                kind.is_negative()
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
    pub(crate) fn commit_cleanse(&mut self, plan: CleansePlan) -> bool {
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
    pub(crate) fn take_due(&mut self, now: u64, facts: TickFacts) -> Vec<ConditionTick<S>> {
        self.cleanse_immunities.retain(|immunity| immunity.until > now);
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
                ConditionValues::DamageOverTime {
                    element,
                    per_tick,
                    interval_ms,
                    ..
                } => {
                    if instance.immediate_due == Some(due) {
                        instance.immediate_due = None;
                    } else {
                        instance.next_tick_at = Some(due + u64::from(interval_ms) * MICROS_PER_MS);
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
                ConditionValues::FoodRegeneration { interval_ms, .. }
                | ConditionValues::Recovery { interval_ms, .. } => {
                    let key = instance.definition.condition_type().conflict_key();
                    instance.next_tick_at = Some(due + u64::from(interval_ms) * MICROS_PER_MS);
                    TickKind::Regeneration {
                        key,
                        suppressed: facts.in_protection_zone
                            && key == ConflictKey::FoodRegeneration,
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

    /// §6.1 death: every instance ends.
    pub(crate) fn clear_on_death(&mut self) {
        self.instances.clear();
        self.cleanse_immunities.clear();
    }
}

/// §3.4: merges each actor's due ticks into the channel order `(due, actor, sequence)`.
///
/// `COND0-RL-03` bounds each actor on its own: the order covers the ticks that run in this
/// simulation tick. An actor's capped ticks do not hold back other actors' keys; they run in the
/// actor's next simulation ticks, keeping their original `due`, so they sort ahead of anything
/// newly due there.
pub(crate) fn channel_tick_order<A: Ord + Copy, S>(
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
    let target = if low == high {
        low
    } else {
        let value =
            deterministic_decision_u64(facts.decision_root, facts.occurrence, COND_SPEED_DRAW, 0)
                .map_err(|_| ConditionRefusal::DrawFailed)?;
        let span = u64::try_from(high - low).map_err(|_| ConditionRefusal::DrawFailed)? + 1;
        low + i64::try_from(value % span).map_err(|_| ConditionRefusal::DrawFailed)?
    };
    let target = if paralysis {
        target.max(PARALYSIS_SPEED_FLOOR)
    } else {
        target
    };
    Ok(target - base)
}

#[cfg(test)]
#[path = "condition_tests.rs"]
mod tests;

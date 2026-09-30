//! CHARM-4: the combat-side Charm effect engine
//! (`reviews/OTERYN_GAME_CHARM0_BESTIARY_CHARM_PROGRESSION_DECISION_PACKET_2026-09-29.md` §4.3,
//! owner answers §7: 4c all 25 charms, 5a follow TibiaWiki).
//!
//! Given the attacking character, the target creature's Bestiary race, the charms that character
//! assigned to that race and their unlocked stages, [`evaluate_charm_hook`] evaluates each
//! charm whose effect belongs to the hook being run. The effect shapes and per-stage values are
//! those of the candidate catalogue `tools/content-schema/charm-authoring/charm.schema.json`
//! (#1293); the content loader (CHARM-1) builds [`CharmDefinition`]s through
//! [`CharmDefinition::new`], which re-checks the catalogue's semantic rules.
//!
//! - A charm applies only against the race it is assigned to: the character's charm state is read
//!   by that race through the narrow read-only [`CharmStateRead`] seam (CHARM-3 owns the state).
//! - Every trigger roll is one `oteryn_simulation_determinism` decision draw bound to the caller's
//!   `(GameplayDecisionRoot, DecisionOccurrenceId)`, so a replay of the same occurrence reproduces
//!   every roll. Evaluation is pure: it reads, rolls and returns typed outcomes, and never mutates.
//! - An effect whose runtime system does not exist yet is evaluated but fails closed: its outcome
//!   is [`CharmResult::FailedClosed`] with the named [`CharmMissingSystem`], and nothing applies.
//! - Any inconsistency in the charm state or the event facts fails the whole evaluation closed
//!   ([`CharmEvaluationError`]): the caller applies no charm effect for that event.

use oteryn_simulation_determinism::{
    DecisionError, DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

use crate::foundation::OwnerDamageResult;

/// The highest Charm stage (`charm.schema.json` `stage.maximum`).
pub(crate) const CHARM_STAGE_MAX: u8 = 3;
/// Charms one character may hold on one race at the same time: one major plus one minor
/// (coordinator catalogue update on #1293, answering CHARM-0 §4.2).
pub(crate) const CHARM_ASSIGNMENTS_PER_RACE_MAX: usize = 2;
/// Percent values are hundredths of a percent: 100% is 10,000.
pub(crate) const CHARM_PERCENT_HUNDREDTHS_WHOLE: u32 = 10_000;
/// `charm.schema.json` `percent.maximum` (1000%) in hundredths of a percent.
pub(crate) const CHARM_PERCENT_HUNDREDTHS_MAX: u32 = 100_000;

/// The decision purpose of every Charm trigger roll. The draw index separates the hook and the
/// charm category (see [`trigger_draw_index`]).
const CHARM_TRIGGER_PURPOSE: &str = "oteryn.charm.trigger.v1";

/// A percentage in hundredths of a percent (5% is 500, 2.5% is 250), exclusive of zero and at
/// most 1000% like the catalogue's `percent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct CharmPercent(u32);

impl CharmPercent {
    pub(crate) const fn from_hundredths(hundredths: u32) -> Option<Self> {
        if hundredths == 0 || hundredths > CHARM_PERCENT_HUNDREDTHS_MAX {
            None
        } else {
            Some(Self(hundredths))
        }
    }

    pub(crate) const fn hundredths(self) -> u32 {
        self.0
    }

    /// `floor(base × self)`, exact.
    fn of(self, base: u64) -> u64 {
        let product = u128::from(base) * u128::from(self.0);
        // The quotient is at most `base × 10`, which a u64 base keeps below 2^68; saturate
        // rather than wrap for bases above u64::MAX / 10.
        u64::try_from(product / u128::from(CHARM_PERCENT_HUNDREDTHS_WHOLE)).unwrap_or(u64::MAX)
    }
}

/// `charm.schema.json` `element`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CharmElement {
    Physical,
    Fire,
    Earth,
    Ice,
    Energy,
    Death,
    Holy,
}

/// How a charm's damage is typed and mitigated (`charm.schema.json` damage effects:
/// `element`, `ignores_resistances`, `reduced_by_armor`). Read from the catalogue shape, never
/// hard-coded per charm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct CharmDamageKind {
    /// The displayed element.
    pub(crate) element: CharmElement,
    /// `true`: neutral in effect, the creature's resistances do not modify it.
    pub(crate) ignores_resistances: bool,
    /// `true`: the creature's armor reduces it.
    pub(crate) reduced_by_armor: bool,
}

/// The attacker's own maximum an `attack_proc_resource_damage` charm scales with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CharmResource {
    Health,
    Mana,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum CharmCategory {
    Major,
    Minor,
}

impl CharmCategory {
    const fn draw_ordinal(self) -> u64 {
        match self {
            Self::Major => 0,
            Self::Minor => 1,
        }
    }
}

/// What a stage's value measures (`charm.schema.json` `stage_value`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CharmStageValue {
    /// The chance that the effect triggers on one hook event; at most 100%.
    TriggerChancePercent,
    /// An always-on bonus of this percent.
    EffectPercent,
}

/// One closed effect shape per `charm.schema.json` `effect` type. Parameters that do not vary by
/// stage live here; the stage value is in [`CharmDefinition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CharmEffect {
    /// Wound, Enflame, Poison, Freeze, Zap, Curse, Divine Wrath: a percent of the creature's
    /// maximum health, capped at `damage_cap_level_multiplier ×` the character's level before
    /// resistances.
    AttackProcDamage {
        damage: CharmDamageKind,
        percent_of_creature_max_health: CharmPercent,
        damage_cap_level_multiplier: u32,
    },
    /// Overpower, Overflux: a percent of the character's own maximum health or mana, capped at a
    /// percent of the creature's maximum health.
    AttackProcResourceDamage {
        damage: CharmDamageKind,
        resource: CharmResource,
        percent_of_own_maximum: CharmPercent,
        damage_cap_percent_of_creature_max_health: CharmPercent,
    },
    /// Carnage: on the character's own lethal hit, a percent of the killed creature's maximum
    /// health, capped at `damage_cap_level_multiplier ×` the character's level, to each monster
    /// around it.
    KillAreaDamage {
        damage: CharmDamageKind,
        percent_of_creature_max_health: CharmPercent,
        damage_cap_level_multiplier: u32,
    },
    /// Cripple.
    ParalyseCreatureOnAttack { duration_ms: u32 },
    /// Numb.
    ParalyseCreatureAfterItsAttack { duration_ms: u32 },
    /// Adrenaline Burst.
    HasteAfterHit { duration_ms: u32 },
    /// Fatal Hold.
    PreventCreatureFlee { duration_ms: u32 },
    /// Parry: the base damage taken is reflected to the aggressor.
    ReflectDamageTaken { damage: CharmDamageKind },
    /// Dodge.
    DodgeAttack,
    /// Cleanse.
    CleanseAfterHit,
    /// Bless.
    DeathLossReduction,
    /// Scavenge.
    SkinningChanceBonus,
    /// Gut.
    CreatureProductBonus,
    /// Low Blow.
    CriticalHitChance,
    /// Savage Blow.
    CriticalExtraDamage,
    /// Vampiric Embrace.
    LifeLeech,
    /// Void's Call.
    ManaLeech,
    /// Void Inversion.
    ManaDrainInversion,
}

/// The combat event at which an effect is evaluated. Every effect type belongs to exactly one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum CharmHook {
    /// Before the character's attack damage is committed.
    AttackDamageCalculation,
    /// After the character's attack damage on the creature is committed.
    AttackHit,
    /// The character's committed hit killed the creature.
    CreatureKilled,
    /// The creature attacks the character, before damage.
    IncomingCreatureAttack,
    /// The creature's damage on the character was taken.
    IncomingCreatureHit,
    /// The creature drains the character's mana.
    IncomingManaDrain,
    /// Loot of the killed creature is generated.
    CreatureLoot,
    /// The character skins or dusts the creature's corpse.
    Skinning,
    /// The character dies to the creature.
    CharacterDeath,
}

impl CharmHook {
    const fn draw_ordinal(self) -> u64 {
        match self {
            Self::AttackDamageCalculation => 0,
            Self::AttackHit => 1,
            Self::CreatureKilled => 2,
            Self::IncomingCreatureAttack => 3,
            Self::IncomingCreatureHit => 4,
            Self::IncomingManaDrain => 5,
            Self::CreatureLoot => 6,
            Self::Skinning => 7,
            Self::CharacterDeath => 8,
        }
    }
}

/// The runtime system an effect needs and that does not exist yet. An effect naming one is
/// evaluated but never applied: it fails closed with this reason until the system exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CharmMissingSystem {
    /// Carnage: no resolver selects the monsters around a killed creature (the ability commit
    /// takes exactly one resolved target).
    AreaTargetResolver,
    /// Cripple, Numb: the runtime actor carries no conditions (spell conditions S8 have no
    /// owner), so there is no paralysis.
    ParalysisCondition,
    /// Adrenaline Burst: no haste condition (same reason).
    HasteCondition,
    /// Cleanse: no negative conditions to remove (same reason).
    ConditionCleanse,
    /// Fatal Hold: creature AI has no fleeing behaviour to prevent.
    CreatureFlee,
    /// Dodge, Parry: creatures only surface an attack intent; no creature-to-character damage
    /// path exists to dodge or reflect.
    IncomingCreatureDamage,
    /// Void Inversion: no creature mana drain damage exists.
    ManaDrain,
    /// Bless: the PvE death outcome (`domain::death`) takes no killer race and no Charm input.
    DeathLossCharmInput,
    /// Scavenge: no skinning or dusting system.
    Skinning,
    /// Gut: the loot plan has no creature-product classification to scale.
    CreatureProductLoot,
    /// Low Blow, Savage Blow: attacks have no critical hits.
    CriticalHit,
    /// Vampiric Embrace, Void's Call: attacks have no life or mana leech.
    Leech,
}

impl CharmEffect {
    /// The one hook at which this effect is evaluated.
    pub(crate) const fn hook(&self) -> CharmHook {
        match self {
            Self::CriticalHitChance | Self::CriticalExtraDamage => {
                CharmHook::AttackDamageCalculation
            }
            Self::AttackProcDamage { .. }
            | Self::AttackProcResourceDamage { .. }
            | Self::ParalyseCreatureOnAttack { .. }
            | Self::PreventCreatureFlee { .. }
            | Self::LifeLeech
            | Self::ManaLeech => CharmHook::AttackHit,
            Self::KillAreaDamage { .. } => CharmHook::CreatureKilled,
            Self::DodgeAttack => CharmHook::IncomingCreatureAttack,
            Self::ReflectDamageTaken { .. }
            | Self::ParalyseCreatureAfterItsAttack { .. }
            | Self::HasteAfterHit { .. }
            | Self::CleanseAfterHit => CharmHook::IncomingCreatureHit,
            Self::ManaDrainInversion => CharmHook::IncomingManaDrain,
            Self::CreatureProductBonus => CharmHook::CreatureLoot,
            Self::SkinningChanceBonus => CharmHook::Skinning,
            Self::DeathLossReduction => CharmHook::CharacterDeath,
        }
    }

    /// What the stage value of this effect type measures (`charm_authoring.py` `STAGE_VALUE`).
    pub(crate) const fn stage_value(&self) -> CharmStageValue {
        match self {
            Self::DeathLossReduction
            | Self::SkinningChanceBonus
            | Self::CreatureProductBonus
            | Self::CriticalHitChance
            | Self::CriticalExtraDamage
            | Self::LifeLeech
            | Self::ManaLeech => CharmStageValue::EffectPercent,
            _ => CharmStageValue::TriggerChancePercent,
        }
    }

    /// `None` when the runtime system this effect needs exists: only the single-target attack
    /// procs, which the owner damage commit applies. Every other effect fails closed.
    pub(crate) const fn missing_system(&self) -> Option<CharmMissingSystem> {
        match self {
            Self::AttackProcDamage { .. } | Self::AttackProcResourceDamage { .. } => None,
            Self::KillAreaDamage { .. } => Some(CharmMissingSystem::AreaTargetResolver),
            Self::ParalyseCreatureOnAttack { .. } | Self::ParalyseCreatureAfterItsAttack { .. } => {
                Some(CharmMissingSystem::ParalysisCondition)
            }
            Self::HasteAfterHit { .. } => Some(CharmMissingSystem::HasteCondition),
            Self::CleanseAfterHit => Some(CharmMissingSystem::ConditionCleanse),
            Self::PreventCreatureFlee { .. } => Some(CharmMissingSystem::CreatureFlee),
            Self::ReflectDamageTaken { .. } | Self::DodgeAttack => {
                Some(CharmMissingSystem::IncomingCreatureDamage)
            }
            Self::ManaDrainInversion => Some(CharmMissingSystem::ManaDrain),
            Self::DeathLossReduction => Some(CharmMissingSystem::DeathLossCharmInput),
            Self::SkinningChanceBonus => Some(CharmMissingSystem::Skinning),
            Self::CreatureProductBonus => Some(CharmMissingSystem::CreatureProductLoot),
            Self::CriticalHitChance | Self::CriticalExtraDamage => {
                Some(CharmMissingSystem::CriticalHit)
            }
            Self::LifeLeech | Self::ManaLeech => Some(CharmMissingSystem::Leech),
        }
    }

    /// Whether the effect acts on the attacked creature and so needs it alive after the hit.
    const fn targets_living_creature(&self) -> bool {
        matches!(
            self,
            Self::AttackProcDamage { .. }
                | Self::AttackProcResourceDamage { .. }
                | Self::ParalyseCreatureOnAttack { .. }
                | Self::PreventCreatureFlee { .. }
        )
    }

    fn validate(&self) -> Result<(), CharmDefinitionError> {
        match *self {
            Self::AttackProcDamage {
                damage_cap_level_multiplier,
                ..
            }
            | Self::KillAreaDamage {
                damage_cap_level_multiplier,
                ..
            } if damage_cap_level_multiplier == 0 => Err(CharmDefinitionError::InvalidParameter),
            Self::ParalyseCreatureOnAttack { duration_ms }
            | Self::ParalyseCreatureAfterItsAttack { duration_ms }
            | Self::HasteAfterHit { duration_ms }
            | Self::PreventCreatureFlee { duration_ms }
                if duration_ms == 0 =>
            {
                Err(CharmDefinitionError::InvalidParameter)
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CharmDefinitionError {
    EmptyKey,
    /// `stage_value` does not match the effect type.
    StageValueMismatch,
    /// Stage values must strictly increase from stage 1 to 3.
    StageValuesNotIncreasing,
    /// A trigger chance above 100%.
    TriggerChanceAbove100,
    /// A zero duration or damage cap multiplier.
    InvalidParameter,
}

/// One validated catalogue charm: the facts combat needs from `content/charms/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CharmDefinition {
    key: String,
    category: CharmCategory,
    stage_value: CharmStageValue,
    stages: [CharmPercent; 3],
    effect: CharmEffect,
}

impl CharmDefinition {
    /// Re-checks the catalogue's semantic rules (`charm_authoring.py` `validate`) that combat
    /// relies on: the stage value kind matches the effect, stage values strictly increase and a
    /// trigger chance is at most 100%.
    pub(crate) fn new(
        key: &str,
        category: CharmCategory,
        stage_value: CharmStageValue,
        stages: [CharmPercent; 3],
        effect: CharmEffect,
    ) -> Result<Self, CharmDefinitionError> {
        if key.is_empty() {
            return Err(CharmDefinitionError::EmptyKey);
        }
        if stage_value != effect.stage_value() {
            return Err(CharmDefinitionError::StageValueMismatch);
        }
        if !(stages[0] < stages[1] && stages[1] < stages[2]) {
            return Err(CharmDefinitionError::StageValuesNotIncreasing);
        }
        if stage_value == CharmStageValue::TriggerChancePercent
            && stages[2].hundredths() > CHARM_PERCENT_HUNDREDTHS_WHOLE
        {
            return Err(CharmDefinitionError::TriggerChanceAbove100);
        }
        effect.validate()?;
        Ok(Self {
            key: key.to_owned(),
            category,
            stage_value,
            stages,
            effect,
        })
    }

    pub(crate) fn key(&self) -> &str {
        &self.key
    }

    pub(crate) const fn category(&self) -> CharmCategory {
        self.category
    }

    pub(crate) const fn effect(&self) -> CharmEffect {
        self.effect
    }

    /// The value of an unlocked stage `1..=3`.
    pub(crate) fn stage(&self, stage: u8) -> Option<CharmPercent> {
        usize::from(stage)
            .checked_sub(1)
            .and_then(|index| self.stages.get(index))
            .copied()
    }
}

/// Read-only access to the validated charm catalogue (filled by the CHARM-1 content loader).
pub(crate) trait CharmCatalogueRead {
    fn charm(&self, key: &str) -> Option<&CharmDefinition>;
}

/// One charm a character assigned to a race, with the stage it has unlocked (CHARM-0 §4.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CharmAssignment {
    pub(crate) charm_key: String,
    pub(crate) unlocked_stage: u8,
}

/// The charm state could not be read; the evaluation fails closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharmStateUnavailable;

/// The narrow, read-only view of one character's Charm state that CHARM-3 provides. Combat never
/// writes Charm state.
pub(crate) trait CharmStateRead {
    /// The `CharacterId` bytes of the character this state belongs to.
    fn character(&self) -> [u8; 16];
    /// Every charm this character assigned to `race_key` (the Creature definition key of the
    /// race's Bestiary entry), with its unlocked stage.
    fn assignments_for_race(
        &self,
        race_key: &str,
    ) -> Result<Vec<CharmAssignment>, CharmStateUnavailable>;
}

/// The character's facts an attack proc needs, read by the caller from the committed actor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CharmAttackerFacts {
    pub(crate) level: u32,
    pub(crate) max_health: u64,
    pub(crate) max_mana: u64,
}

/// What dealt a committed hit on the creature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CharmHitSource {
    /// The character's own attack on its main target, or a spell or rune on any creature it hits:
    /// charms evaluate.
    CharacterAttack,
    /// The character's auto-attack on a creature other than its main target (area ammunition such
    /// as Diamond Arrows): no charm evaluates. Tibia 15.25 (Vocation Adjustments 2026) made
    /// auto-attacks trigger charms only on their main target; spells and runes are unchanged, and
    /// Low Blow still applies to the whole area because it runs at `AttackDamageCalculation`
    /// (owner answer 16a, CHARM-0 §8).
    CharacterAutoAttackOffTarget,
    /// Charm damage (a proc, Carnage, Parry). Charms cannot chain: no charm evaluates, so a kill
    /// by Carnage never triggers another Carnage.
    CharmDamage,
}

/// The event whose hook is run, with the facts its effects need.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CharmHookEvent {
    /// Before the character's attack damage is committed.
    AttackDamageCalculation,
    /// The owner committed the character's hit on the creature: [`CharmHook::AttackHit`], and
    /// also [`CharmHook::CreatureKilled`] when this hit was lethal (the character's own last
    /// hit; a summon's hit is not the character's). A replayed commit reports the same health
    /// facts, so it evaluates to the same outcomes.
    CommittedHit {
        source: CharmHitSource,
        attacker: CharmAttackerFacts,
        creature_max_health: u64,
        health_before: i64,
        health_after: i64,
    },
    IncomingCreatureAttack,
    /// The creature's hit on the character, with its base damage before the character's own
    /// resistances and armor.
    IncomingCreatureHit {
        base_damage: u64,
    },
    IncomingManaDrain {
        mana_drained: u64,
    },
    CreatureLoot,
    Skinning,
    CharacterDeath,
}

impl CharmHookEvent {
    /// The committed-hit event of one owner damage commit (`commit_damage_for_attacker`).
    pub(crate) const fn committed_hit(
        result: &OwnerDamageResult,
        source: CharmHitSource,
        attacker: CharmAttackerFacts,
        creature_max_health: u64,
    ) -> Self {
        Self::CommittedHit {
            source,
            attacker,
            creature_max_health,
            health_before: result.health_before,
            health_after: result.health_after,
        }
    }
}

/// One hook evaluation.
#[derive(Clone, Copy)]
pub(crate) struct CharmEvaluationInput<'a> {
    /// The `CharacterId` bytes of the character whose charms apply.
    pub(crate) character: [u8; 16],
    /// The Creature definition key of the creature's Bestiary race.
    pub(crate) race_key: &'a str,
    pub(crate) decision_root: &'a GameplayDecisionRoot,
    /// The occurrence of the event (the committed hit, the incoming attack, the death...). It
    /// must be unique per event and target: one area attack hitting two creatures is two hits.
    pub(crate) occurrence: DecisionOccurrenceId,
    pub(crate) event: CharmHookEvent,
}

/// The typed effect a triggered charm produces. The caller applies it through the owning system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CharmEffectOutcome {
    /// Extra damage on the attacked creature, before its mitigation (`damage` says which applies).
    ProcDamage {
        damage: CharmDamageKind,
        amount: u64,
    },
    /// Carnage: this damage, before mitigation, to each monster on the four non-diagonal squares
    /// adjacent to the killed creature (not summons). It is [`CharmHitSource::CharmDamage`], so
    /// the kills it makes trigger no charm.
    AreaDamageAroundKill {
        damage: CharmDamageKind,
        amount: u64,
    },
    ParalyseCreature {
        duration_ms: u32,
    },
    Haste {
        duration_ms: u32,
    },
    PreventCreatureFlee {
        duration_ms: u32,
    },
    /// Parry: the base damage taken, back to the attacking creature, before its mitigation.
    ReflectDamage {
        damage: CharmDamageKind,
        amount: u64,
    },
    DodgeAttack,
    CleanseOneNegativeCondition,
    /// The drained mana is gained instead of lost.
    InvertManaDrain {
        mana_gained: u64,
    },
    /// Experience and skill loss is reduced by this percent.
    DeathLossReduction {
        percent: CharmPercent,
    },
    SkinningChanceBonus {
        percent: CharmPercent,
    },
    CreatureProductBonus {
        percent: CharmPercent,
    },
    CriticalHitChanceBonus {
        percent: CharmPercent,
    },
    CriticalExtraDamageBonus {
        percent: CharmPercent,
    },
    LifeLeechBonus {
        percent: CharmPercent,
    },
    ManaLeechBonus {
        percent: CharmPercent,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CharmResult {
    /// Apply this effect.
    Applied(CharmEffectOutcome),
    /// The effect was evaluated, but the runtime system it needs does not exist: nothing applies.
    FailedClosed {
        reason: CharmMissingSystem,
        evaluated: CharmEffectOutcome,
    },
    /// The trigger roll missed.
    NotTriggered,
    /// The hit killed the creature, so an effect on it has no target (no roll is made).
    NoLivingTarget,
    /// The effect triggered with a zero magnitude (for example a tiny creature): nothing applies.
    ZeroMagnitude,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CharmOutcome {
    pub(crate) charm_key: String,
    pub(crate) stage: u8,
    pub(crate) hook: CharmHook,
    pub(crate) result: CharmResult,
}

/// The evaluation failed closed: no charm effect applies for this event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CharmEvaluationError {
    /// The charm state belongs to another character.
    StateForOtherCharacter,
    StateUnavailable,
    TooManyAssignments,
    /// The same charm is assigned twice to the race.
    DuplicateCharm,
    /// Two charms of one category on one race.
    DuplicateCategory,
    UnknownCharm(String),
    /// An assigned stage outside `1..=3`.
    InvalidStage,
    /// The event facts are inconsistent (zero level, zero maximum, health not reduced...).
    InvalidEventFacts,
    Decision(DecisionError),
}

/// Evaluate every charm the character assigned to the creature's race whose effect belongs to the
/// event's hook(s). Outcomes are ordered by hook and then by category, independent of the order
/// the state view returns them in. `Ok` with no outcomes means no charm applies.
pub(crate) fn evaluate_charm_hook(
    input: &CharmEvaluationInput<'_>,
    state: &dyn CharmStateRead,
    catalogue: &dyn CharmCatalogueRead,
) -> Result<Vec<CharmOutcome>, CharmEvaluationError> {
    if state.character() != input.character {
        return Err(CharmEvaluationError::StateForOtherCharacter);
    }
    let lethal = validate_event(&input.event)?;
    let assignments = state
        .assignments_for_race(input.race_key)
        .map_err(|CharmStateUnavailable| CharmEvaluationError::StateUnavailable)?;
    if assignments.len() > CHARM_ASSIGNMENTS_PER_RACE_MAX {
        return Err(CharmEvaluationError::TooManyAssignments);
    }
    let mut resolved: Vec<(&CharmDefinition, u8, CharmPercent)> =
        Vec::with_capacity(assignments.len());
    for assignment in &assignments {
        let definition = catalogue
            .charm(&assignment.charm_key)
            .ok_or_else(|| CharmEvaluationError::UnknownCharm(assignment.charm_key.clone()))?;
        let value = definition
            .stage(assignment.unlocked_stage)
            .ok_or(CharmEvaluationError::InvalidStage)?;
        for (other, _, _) in &resolved {
            if other.key() == definition.key() {
                return Err(CharmEvaluationError::DuplicateCharm);
            }
            if other.category() == definition.category() {
                return Err(CharmEvaluationError::DuplicateCategory);
            }
        }
        resolved.push((definition, assignment.unlocked_stage, value));
    }
    resolved.sort_by_key(|(definition, _, _)| (definition.effect().hook(), definition.category()));

    let mut outcomes = Vec::new();
    for (definition, stage, value) in resolved {
        let effect = definition.effect();
        let hook = effect.hook();
        if !event_runs_hook(&input.event, lethal, hook) {
            continue;
        }
        let result = evaluate_effect(input, hook, lethal, definition, value)?;
        outcomes.push(CharmOutcome {
            charm_key: definition.key().to_owned(),
            stage,
            hook,
            result,
        });
    }
    Ok(outcomes)
}

/// Checks the event facts; `true` when a committed hit was lethal.
fn validate_event(event: &CharmHookEvent) -> Result<bool, CharmEvaluationError> {
    match *event {
        CharmHookEvent::CommittedHit {
            source: _,
            attacker,
            creature_max_health,
            health_before,
            health_after,
        } => {
            let before = u64::try_from(health_before)
                .map_err(|_| CharmEvaluationError::InvalidEventFacts)?;
            let after =
                u64::try_from(health_after).map_err(|_| CharmEvaluationError::InvalidEventFacts)?;
            if attacker.level == 0
                || attacker.max_health == 0
                || creature_max_health == 0
                || before == 0
                || after >= before
                || before > creature_max_health
            {
                return Err(CharmEvaluationError::InvalidEventFacts);
            }
            Ok(after == 0)
        }
        CharmHookEvent::IncomingCreatureHit { base_damage: 0 }
        | CharmHookEvent::IncomingManaDrain { mana_drained: 0 } => {
            Err(CharmEvaluationError::InvalidEventFacts)
        }
        _ => Ok(false),
    }
}

fn event_runs_hook(event: &CharmHookEvent, lethal: bool, hook: CharmHook) -> bool {
    match event {
        CharmHookEvent::AttackDamageCalculation => hook == CharmHook::AttackDamageCalculation,
        CharmHookEvent::CommittedHit { source, .. } => {
            *source == CharmHitSource::CharacterAttack
                && (hook == CharmHook::AttackHit || (lethal && hook == CharmHook::CreatureKilled))
        }
        CharmHookEvent::IncomingCreatureAttack => hook == CharmHook::IncomingCreatureAttack,
        CharmHookEvent::IncomingCreatureHit { .. } => hook == CharmHook::IncomingCreatureHit,
        CharmHookEvent::IncomingManaDrain { .. } => hook == CharmHook::IncomingManaDrain,
        CharmHookEvent::CreatureLoot => hook == CharmHook::CreatureLoot,
        CharmHookEvent::Skinning => hook == CharmHook::Skinning,
        CharmHookEvent::CharacterDeath => hook == CharmHook::CharacterDeath,
    }
}

/// `hook × 2 + category`: at most one charm per category and race, so every roll of one
/// occurrence has its own draw.
const fn trigger_draw_index(hook: CharmHook, category: CharmCategory) -> u64 {
    hook.draw_ordinal() * 2 + category.draw_ordinal()
}

/// A draw mapped onto `0..10_000` hundredths of a percent (multiply-shift, no modulo bias
/// beyond 2^-64).
fn hundredths_roll(draw: u64) -> u32 {
    // The product is below 10_000 · 2^64, so the shifted value is below 10_000.
    u32::try_from((u128::from(draw) * u128::from(CHARM_PERCENT_HUNDREDTHS_WHOLE)) >> 64)
        .unwrap_or(u32::MAX)
}

fn evaluate_effect(
    input: &CharmEvaluationInput<'_>,
    hook: CharmHook,
    lethal: bool,
    definition: &CharmDefinition,
    value: CharmPercent,
) -> Result<CharmResult, CharmEvaluationError> {
    let effect = definition.effect();
    if lethal && hook == CharmHook::AttackHit && effect.targets_living_creature() {
        return Ok(CharmResult::NoLivingTarget);
    }
    if effect.stage_value() == CharmStageValue::TriggerChancePercent {
        let draw = deterministic_decision_u64(
            input.decision_root,
            input.occurrence,
            CHARM_TRIGGER_PURPOSE,
            trigger_draw_index(hook, definition.category()),
        )
        .map_err(CharmEvaluationError::Decision)?;
        if hundredths_roll(draw) >= value.hundredths() {
            return Ok(CharmResult::NotTriggered);
        }
    }
    let Some(evaluated) = effect_outcome(effect, value, &input.event) else {
        return Ok(CharmResult::ZeroMagnitude);
    };
    Ok(match effect.missing_system() {
        None => CharmResult::Applied(evaluated),
        Some(reason) => CharmResult::FailedClosed { reason, evaluated },
    })
}

/// The pure effect of one triggered (or always-on) charm; `None` for a zero magnitude.
fn effect_outcome(
    effect: CharmEffect,
    value: CharmPercent,
    event: &CharmHookEvent,
) -> Option<CharmEffectOutcome> {
    let (attacker, creature_max_health) = match *event {
        CharmHookEvent::CommittedHit {
            attacker,
            creature_max_health,
            ..
        } => (Some(attacker), creature_max_health),
        _ => (None, 0),
    };
    let outcome = match effect {
        CharmEffect::AttackProcDamage {
            damage,
            percent_of_creature_max_health,
            damage_cap_level_multiplier,
        } => {
            let cap = u64::from(attacker?.level) * u64::from(damage_cap_level_multiplier);
            CharmEffectOutcome::ProcDamage {
                damage,
                amount: percent_of_creature_max_health
                    .of(creature_max_health)
                    .min(cap),
            }
        }
        CharmEffect::AttackProcResourceDamage {
            damage,
            resource,
            percent_of_own_maximum,
            damage_cap_percent_of_creature_max_health,
        } => {
            let attacker = attacker?;
            let maximum = match resource {
                CharmResource::Health => attacker.max_health,
                CharmResource::Mana => attacker.max_mana,
            };
            CharmEffectOutcome::ProcDamage {
                damage,
                amount: percent_of_own_maximum
                    .of(maximum)
                    .min(damage_cap_percent_of_creature_max_health.of(creature_max_health)),
            }
        }
        CharmEffect::KillAreaDamage {
            damage,
            percent_of_creature_max_health,
            damage_cap_level_multiplier,
        } => {
            let cap = u64::from(attacker?.level) * u64::from(damage_cap_level_multiplier);
            CharmEffectOutcome::AreaDamageAroundKill {
                damage,
                amount: percent_of_creature_max_health
                    .of(creature_max_health)
                    .min(cap),
            }
        }
        CharmEffect::ParalyseCreatureOnAttack { duration_ms }
        | CharmEffect::ParalyseCreatureAfterItsAttack { duration_ms } => {
            CharmEffectOutcome::ParalyseCreature { duration_ms }
        }
        CharmEffect::HasteAfterHit { duration_ms } => CharmEffectOutcome::Haste { duration_ms },
        CharmEffect::PreventCreatureFlee { duration_ms } => {
            CharmEffectOutcome::PreventCreatureFlee { duration_ms }
        }
        CharmEffect::ReflectDamageTaken { damage } => match *event {
            CharmHookEvent::IncomingCreatureHit { base_damage } => {
                CharmEffectOutcome::ReflectDamage {
                    damage,
                    amount: base_damage,
                }
            }
            _ => return None,
        },
        CharmEffect::DodgeAttack => CharmEffectOutcome::DodgeAttack,
        CharmEffect::CleanseAfterHit => CharmEffectOutcome::CleanseOneNegativeCondition,
        CharmEffect::ManaDrainInversion => match *event {
            CharmHookEvent::IncomingManaDrain { mana_drained } => {
                CharmEffectOutcome::InvertManaDrain {
                    mana_gained: mana_drained,
                }
            }
            _ => return None,
        },
        CharmEffect::DeathLossReduction => {
            CharmEffectOutcome::DeathLossReduction { percent: value }
        }
        CharmEffect::SkinningChanceBonus => {
            CharmEffectOutcome::SkinningChanceBonus { percent: value }
        }
        CharmEffect::CreatureProductBonus => {
            CharmEffectOutcome::CreatureProductBonus { percent: value }
        }
        CharmEffect::CriticalHitChance => {
            CharmEffectOutcome::CriticalHitChanceBonus { percent: value }
        }
        CharmEffect::CriticalExtraDamage => {
            CharmEffectOutcome::CriticalExtraDamageBonus { percent: value }
        }
        CharmEffect::LifeLeech => CharmEffectOutcome::LifeLeechBonus { percent: value },
        CharmEffect::ManaLeech => CharmEffectOutcome::ManaLeechBonus { percent: value },
    };
    match outcome {
        CharmEffectOutcome::ProcDamage { amount: 0, .. }
        | CharmEffectOutcome::AreaDamageAroundKill { amount: 0, .. } => None,
        outcome => Some(outcome),
    }
}

#[cfg(test)]
#[path = "charm_effects_tests.rs"]
mod tests;

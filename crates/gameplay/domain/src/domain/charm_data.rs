//! Pure Charm definitions and committed damage facts shared by progression and runtime owners.

/// The highest Charm stage (`charm.schema.json` `stage.maximum`).
pub const CHARM_STAGE_MAX: u8 = 3;
/// Charms one character may hold on one race at the same time: one major plus one minor
/// (coordinator catalogue update on #1293, answering CHARM-0 §4.2).
pub const CHARM_ASSIGNMENTS_PER_RACE_MAX: usize = 2;
/// Percent values are hundredths of a percent: 100% is 10,000.
pub const CHARM_PERCENT_HUNDREDTHS_WHOLE: u32 = 10_000;
/// `charm.schema.json` `percent.maximum` (1000%) in hundredths of a percent.
pub const CHARM_PERCENT_HUNDREDTHS_MAX: u32 = 100_000;

/// A percentage in hundredths of a percent (5% is 500, 2.5% is 250), exclusive of zero and at
/// most 1000% like the catalogue's `percent`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CharmPercent(u32);

impl CharmPercent {
    pub const fn from_hundredths(hundredths: u32) -> Option<Self> {
        if hundredths == 0 || hundredths > CHARM_PERCENT_HUNDREDTHS_MAX {
            None
        } else {
            Some(Self(hundredths))
        }
    }

    pub const fn hundredths(self) -> u32 {
        self.0
    }

    /// `floor(base × self)`, exact.
    pub fn of(self, base: u64) -> u64 {
        let product = u128::from(base) * u128::from(self.0);
        // The quotient is at most `base × 10`, which a u64 base keeps below 2^68; saturate
        // rather than wrap for bases above u64::MAX / 10.
        u64::try_from(product / u128::from(CHARM_PERCENT_HUNDREDTHS_WHOLE)).unwrap_or(u64::MAX)
    }
}

/// `charm.schema.json` `element`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharmElement {
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
pub struct CharmDamageKind {
    /// The displayed element.
    pub element: CharmElement,
    /// `true`: neutral in effect, the creature's resistances do not modify it.
    pub ignores_resistances: bool,
    /// `true`: the creature's armor reduces it.
    pub reduced_by_armor: bool,
}

/// The attacker's own maximum an `attack_proc_resource_damage` charm scales with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharmResource {
    Health,
    Mana,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CharmCategory {
    Major,
    Minor,
}

impl CharmCategory {
    pub const fn draw_ordinal(self) -> u64 {
        match self {
            Self::Major => 0,
            Self::Minor => 1,
        }
    }
}

/// What a stage's value measures (`charm.schema.json` `stage_value`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharmStageValue {
    /// The chance that the effect triggers on one hook event; at most 100%.
    TriggerChancePercent,
    /// An always-on bonus of this percent.
    EffectPercent,
}

/// One closed effect shape per `charm.schema.json` `effect` type. Parameters that do not vary by
/// stage live here; the stage value is in [`CharmDefinition`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CharmEffect {
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
pub enum CharmHook {
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
    pub const fn draw_ordinal(self) -> u64 {
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
pub enum CharmMissingSystem {
    /// Carnage: no resolver selects the monsters around a killed creature (the ability commit
    /// takes exactly one resolved target).
    AreaTargetResolver,
    /// Cripple, Numb: the pure condition store has no Charm-to-actor/movement composition.
    ParalysisCondition,
    /// Adrenaline Burst: no Charm haste consumer in actor/movement composition.
    HasteCondition,
    /// Cleanse: condition removal/immunity awaits COND-1b and actor composition.
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
    pub const fn hook(&self) -> CharmHook {
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
    pub const fn stage_value(&self) -> CharmStageValue {
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
    pub const fn missing_system(&self) -> Option<CharmMissingSystem> {
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
    pub const fn targets_living_creature(&self) -> bool {
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
pub enum CharmDefinitionError {
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
pub struct CharmDefinition {
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
    pub fn new(
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

    pub fn key(&self) -> &str {
        &self.key
    }

    pub const fn category(&self) -> CharmCategory {
        self.category
    }

    pub const fn effect(&self) -> CharmEffect {
        self.effect
    }

    /// Whether the server applies this charm's effect today: no [`CharmMissingSystem`] blocks
    /// it. This is the `effect_active` of the charm's `CharmStateV1` entry (Sol ruling, #162
    /// comment 5913269950): the client cannot know which systems exist on the server.
    pub const fn effect_active(&self) -> bool {
        self.effect.missing_system().is_none()
    }

    /// The value of an unlocked stage `1..=3`.
    pub fn stage(&self, stage: u8) -> Option<CharmPercent> {
        usize::from(stage)
            .checked_sub(1)
            .and_then(|index| self.stages.get(index))
            .copied()
    }
}

/// Read-only access to the validated charm catalogue (filled by the CHARM-1 content loader).
pub trait CharmCatalogueRead {
    fn charm(&self, key: &str) -> Option<&CharmDefinition>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnerDamageResult {
    pub applied: bool,
    pub health_before: i64,
    pub health_after: i64,
}

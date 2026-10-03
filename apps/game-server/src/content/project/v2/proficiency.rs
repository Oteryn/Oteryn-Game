//! Weapon Proficiency definitions (PROFICIENCY-0 §4.1, PROF-CONTENT-1).
//!
//! A `Proficiency` declaration holds the levels of one weapon proficiency tree and each level's
//! perks, promoted from the 15.30 client source with the D199 code map
//! (`tools/content-schema/proficiency-authoring`). Perks keep their source order, so a selected
//! perk is its index within the level. Values are exact ratios of the source decimals; the unit
//! is fixed by the perk kind (D199). Items bind a tree through
//! `ProjectV2WeaponProficiencyProfile::profile_binding`. The data-only importer makes the committed catalogue available to the server;
//! gameplay effect execution and activation remain separate consumers.

mod import;
pub use import::*;

use serde::{Deserialize, Serialize};

use super::{
    ProjectError, ProjectEvidenceLimits, ProjectV2DefinitionRef, ProjectV2ExactRatio,
    ProjectV2Family, validate_v2_ratio, validate_v2_source_text,
};

pub const PROJECT_V2_PROFICIENCY_MAX_LEVELS: usize = 7;
pub const PROJECT_V2_PROFICIENCY_MAX_PERKS_PER_LEVEL: usize = 3;

/// The progress threshold table a weapon uses (D197, D198, D200). A weapon whose class is not
/// evidenced has no proficiency binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2ProficiencyThresholdClass {
    Standard,
    Knight,
    Crossbow,
}

/// The static Weapon Proficiency binding of an Item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2WeaponProficiencyProfile {
    pub profile_binding: ProjectV2DefinitionRef,
    pub threshold_class: ProjectV2ProficiencyThresholdClass,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ProficiencyLevel {
    pub level: u8,
    /// Source order; a Character selects at most one perk per level by its index.
    pub perks: Vec<ProjectV2ProficiencyPerk>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2ProficiencySkill {
    MagicLevel,
    Shielding,
    Distance,
    Sword,
    Club,
    Axe,
    Fist,
    Fishing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2ProficiencyElement {
    Physical,
    Fire,
    Earth,
    Energy,
    Ice,
    Holy,
    Death,
    Healing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2ProficiencySpellAugment {
    BaseDamage,
    Healing,
    /// Negative seconds.
    Cooldown,
    LifeLeech,
    ManaLeech,
    CriticalExtraDamage,
    CriticalHitChance,
}

/// One perk, one closed shape per source `Type` code (D199). `value` is flat for kinds from
/// Types 0-4, 18-22 and 24, and a fraction (x100 = %) otherwise.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectV2ProficiencyPerk {
    AttackDamage {
        value: ProjectV2ExactRatio,
    },
    Defence {
        value: ProjectV2ExactRatio,
    },
    WeaponShieldDefence {
        value: ProjectV2ExactRatio,
    },
    SkillBonus {
        skill: ProjectV2ProficiencySkill,
        value: ProjectV2ExactRatio,
    },
    SpecializedMagicLevel {
        element: ProjectV2ProficiencyElement,
        value: ProjectV2ExactRatio,
    },
    SpellAugment {
        spell_client_id: u16,
        augment: ProjectV2ProficiencySpellAugment,
        value: ProjectV2ExactRatio,
    },
    BestiaryClassDamage {
        bestiary_class_id: u16,
        bestiary_class_name: String,
        value: ProjectV2ExactRatio,
    },
    BossDamage {
        value: ProjectV2ExactRatio,
    },
    CriticalHitChance {
        value: ProjectV2ExactRatio,
    },
    ElementalCriticalHitChance {
        element: ProjectV2ProficiencyElement,
        value: ProjectV2ExactRatio,
    },
    RuneCriticalHitChance {
        value: ProjectV2ExactRatio,
    },
    AutoAttackCriticalHitChance {
        value: ProjectV2ExactRatio,
    },
    CriticalExtraDamage {
        value: ProjectV2ExactRatio,
    },
    ElementalCriticalExtraDamage {
        element: ProjectV2ProficiencyElement,
        value: ProjectV2ExactRatio,
    },
    RuneCriticalExtraDamage {
        value: ProjectV2ExactRatio,
    },
    AutoAttackCriticalExtraDamage {
        value: ProjectV2ExactRatio,
    },
    ManaLeech {
        value: ProjectV2ExactRatio,
    },
    LifeLeech {
        value: ProjectV2ExactRatio,
    },
    ManaOnHit {
        value: ProjectV2ExactRatio,
    },
    LifeOnHit {
        value: ProjectV2ExactRatio,
    },
    ManaOnKill {
        value: ProjectV2ExactRatio,
    },
    LifeOnKill {
        value: ProjectV2ExactRatio,
    },
    DamageAtRange {
        /// Source `Range`, in tiles.
        range: u8,
        value: ProjectV2ExactRatio,
    },
    RangedHitChance {
        value: ProjectV2ExactRatio,
    },
    AttackRange {
        value: ProjectV2ExactRatio,
    },
    SkillScaledAutoAttackDamage {
        skill: ProjectV2ProficiencySkill,
        value: ProjectV2ExactRatio,
    },
    SkillScaledSpellDamage {
        skill: ProjectV2ProficiencySkill,
        value: ProjectV2ExactRatio,
    },
    SkillScaledHealing {
        skill: ProjectV2ProficiencySkill,
        value: ProjectV2ExactRatio,
    },
    AlphaStrikeDamage {
        value: ProjectV2ExactRatio,
    },
    OmegaStrikeDamage {
        value: ProjectV2ExactRatio,
    },
    ArmorPenetration {
        value: ProjectV2ExactRatio,
    },
    ElementalPierce {
        element: ProjectV2ProficiencyElement,
        value: ProjectV2ExactRatio,
    },
    HomingMissile {
        element: ProjectV2ProficiencyElement,
        missile_client_id: u16,
        /// Fraction in (0, 1].
        probability: ProjectV2ExactRatio,
        multiplier: ProjectV2ExactRatio,
    },
}

impl ProjectV2ProficiencyPerk {
    /// The perk's `value`; `HomingMissile` has none.
    pub fn value(&self) -> Option<ProjectV2ExactRatio> {
        use ProjectV2ProficiencyPerk as P;
        match self {
            P::HomingMissile { .. } => None,
            P::AttackDamage { value }
            | P::Defence { value }
            | P::WeaponShieldDefence { value }
            | P::SkillBonus { value, .. }
            | P::SpecializedMagicLevel { value, .. }
            | P::SpellAugment { value, .. }
            | P::BestiaryClassDamage { value, .. }
            | P::BossDamage { value }
            | P::CriticalHitChance { value }
            | P::ElementalCriticalHitChance { value, .. }
            | P::RuneCriticalHitChance { value }
            | P::AutoAttackCriticalHitChance { value }
            | P::CriticalExtraDamage { value }
            | P::ElementalCriticalExtraDamage { value, .. }
            | P::RuneCriticalExtraDamage { value }
            | P::AutoAttackCriticalExtraDamage { value }
            | P::ManaLeech { value }
            | P::LifeLeech { value }
            | P::ManaOnHit { value }
            | P::LifeOnHit { value }
            | P::ManaOnKill { value }
            | P::LifeOnKill { value }
            | P::DamageAtRange { value, .. }
            | P::RangedHitChance { value }
            | P::AttackRange { value }
            | P::SkillScaledAutoAttackDamage { value, .. }
            | P::SkillScaledSpellDamage { value, .. }
            | P::SkillScaledHealing { value, .. }
            | P::AlphaStrikeDamage { value }
            | P::OmegaStrikeDamage { value }
            | P::ArmorPenetration { value }
            | P::ElementalPierce { value, .. } => Some(*value),
        }
    }
}

pub(super) fn validate_v2_proficiency_levels(
    levels: &[ProjectV2ProficiencyLevel],
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    if levels.is_empty() || levels.len() > PROJECT_V2_PROFICIENCY_MAX_LEVELS {
        return Err(ProjectError::InvalidProject(
            "v2 Proficiency requires 1 to 7 levels",
        ));
    }
    for (index, level) in levels.iter().enumerate() {
        if usize::from(level.level) != index + 1 {
            return Err(ProjectError::InvalidProject(
                "v2 Proficiency levels are not 1..n in order",
            ));
        }
        if level.perks.is_empty() || level.perks.len() > PROJECT_V2_PROFICIENCY_MAX_PERKS_PER_LEVEL
        {
            return Err(ProjectError::InvalidProject(
                "v2 Proficiency level requires 1 to 3 perks",
            ));
        }
        for perk in &level.perks {
            validate_v2_proficiency_perk(perk, limits)?;
        }
    }
    Ok(())
}

fn validate_v2_proficiency_perk(
    perk: &ProjectV2ProficiencyPerk,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    match perk {
        ProjectV2ProficiencyPerk::HomingMissile {
            probability,
            multiplier,
            ..
        } => {
            validate_v2_ratio(*probability, "v2 Proficiency ratio is not canonical")?;
            validate_v2_ratio(*multiplier, "v2 Proficiency ratio is not canonical")?;
            if probability.numerator <= 0
                || probability.numerator.unsigned_abs() > probability.denominator
                || multiplier.numerator <= 0
            {
                return Err(ProjectError::InvalidProject(
                    "v2 Proficiency homing missile probability or multiplier is out of range",
                ));
            }
            return Ok(());
        }
        ProjectV2ProficiencyPerk::BestiaryClassDamage {
            bestiary_class_name,
            ..
        } => validate_v2_source_text(
            "v2 Proficiency bestiary class name",
            bestiary_class_name,
            limits,
        )?,
        _ => {}
    }
    let Some(value) = perk.value() else {
        return Ok(());
    };
    validate_v2_ratio(value, "v2 Proficiency ratio is not canonical")?;
    let cooldown = matches!(
        perk,
        ProjectV2ProficiencyPerk::SpellAugment {
            augment: ProjectV2ProficiencySpellAugment::Cooldown,
            ..
        }
    );
    if (cooldown && value.numerator >= 0) || (!cooldown && value.numerator <= 0) {
        return Err(ProjectError::InvalidProject(
            "v2 Proficiency value sign does not match its kind",
        ));
    }
    Ok(())
}

pub(super) fn validate_v2_item_proficiency(
    proficiency: &ProjectV2WeaponProficiencyProfile,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
) -> Result<(), ProjectError> {
    if proficiency.profile_binding.family != ProjectV2Family::Proficiency {
        return Err(ProjectError::InvalidProject(
            "v2 Item proficiency binding family mismatch",
        ));
    }
    require_ref(&proficiency.profile_binding)
}

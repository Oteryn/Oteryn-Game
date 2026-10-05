//! Typed loaded-document membership qualification. No digest->membership inference.
use crate::content::{
    EffectFamilyDocument, ProjectReferenceRecord, ProjectV2AuthoringProfileData, ProjectV2Draft,
    ProjectV2Family, ProjectV2FormulaAuthoring, ProjectV2SourceIdentityDisposition,
};
#[derive(Debug, Clone)]
pub(crate) struct QualifiedCreatureSelfHeal {
    target: String,
    ability: String,
    max_health: u64,
    minimum: u64,
    maximum: u64,
    interval_ms: u64,
    chance_ppm: u32,
    removes_paralysis: bool,
}
impl QualifiedCreatureSelfHeal {
    pub(crate) fn qualify(
        draft: &ProjectV2Draft,
        target: &str,
        ability: &str,
    ) -> Result<Self, &'static str> {
        if !draft.state.source_identity_bindings.iter().any(|b| {
            b.target.family == ProjectV2Family::Creature
                && b.target.key == target
                && b.target.revision == "definition-r1"
                && b.disposition == ProjectV2SourceIdentityDisposition::Exact
                && draft
                    .state
                    .sources
                    .iter()
                    .any(|s| s.key == b.source_key && s.revision == b.source_revision)
        }) {
            return Err("missing exact loaded Creature source membership");
        }
        let creature = draft
            .state
            .authoring_profiles
            .iter()
            .find(|p| {
                p.target.family == ProjectV2Family::Creature
                    && p.target.key == target
                    && p.target.revision == "definition-r1"
            })
            .ok_or("missing exact Creature profile")?;
        let ProjectV2AuthoringProfileData::Creature(creature) = &creature.data else {
            return Err("wrong Creature profile");
        };
        if !creature.abilities.iter().any(|r| {
            r.family == ProjectV2Family::Ability
                && r.key == ability
                && r.revision == "definition-r1"
        }) {
            return Err("ability outside exact Creature membership");
        }
        let behavior = draft
            .core
            .records
            .iter()
            .find_map(|r| match r {
                ProjectReferenceRecord::Creature {
                    identity, behavior, ..
                } if identity.key == target && identity.revision == "definition-r1" => {
                    Some(behavior)
                }
                _ => None,
            })
            .ok_or("missing native Creature behavior binding")?;
        let behavior_profile = draft
            .state
            .authoring_profiles
            .iter()
            .find(|p| {
                p.target.family == ProjectV2Family::Behavior
                    && p.target.key == behavior.key
                    && p.target.revision == behavior.revision
            })
            .ok_or("missing exact native Behavior profile")?;
        let ProjectV2AuthoringProfileData::Behavior(behavior_profile) = &behavior_profile.data
        else {
            return Err("wrong Behavior profile");
        };
        let schedule = behavior_profile
            .defenses
            .iter()
            .find(|d| {
                d.ability.family == ProjectV2Family::Ability
                    && d.ability.key == ability
                    && d.ability.revision == "definition-r1"
            })
            .ok_or("ability absent from actual Behavior.defenses")?;
        let ability_profile = draft
            .state
            .authoring_profiles
            .iter()
            .find(|p| {
                p.target.family == ProjectV2Family::Ability
                    && p.target.key == ability
                    && p.target.revision == "definition-r1"
            })
            .ok_or("missing exact Ability profile")?;
        let ProjectV2AuthoringProfileData::Ability(ability_profile) = &ability_profile.data else {
            return Err("wrong Ability profile");
        };
        if ability_profile
            .details
            .as_ref()
            .is_none_or(|d| d.needs_target)
        {
            return Err("selfheal needs source selfcast");
        }

        let details = ability_profile
            .details
            .as_deref()
            .ok_or("missing selfheal details")?;
        if details.area.is_some()
            || details.chain.is_some()
            || details.encounter.is_some()
            || details.windup.is_some()
            || !details.variants.is_empty()
        {
            return Err("not a closed source selfheal shape");
        }
        let mut removes_paralysis = false;
        for declared in &details.effects {
            match declared {
                crate::content::ProjectV2AbilityEffect::Executable(_) => {}
                crate::content::ProjectV2AbilityEffect::Inline(inline) => match &inline.operation {
                    crate::content::ProjectV2InlineEffectOperation::RemoveCondition {
                        condition,
                    } if condition == "paralyze" && !removes_paralysis => removes_paralysis = true,
                    _ => return Err("unsupported selfheal inline composition"),
                },
            }
        }
        let effects = draft
            .core
            .records
            .iter()
            .find_map(|r| match r {
                ProjectReferenceRecord::Ability { identity, effects }
                    if identity.key == ability && identity.revision == "definition-r1" =>
                {
                    Some(effects)
                }
                _ => None,
            })
            .ok_or("missing native Ability")?;
        if effects.len() != 1 {
            return Err("selfheal slice requires one exact native Effect");
        }
        let effect = &effects[0];
        let formula = draft
            .core
            .records
            .iter()
            .find_map(|r| match r {
                ProjectReferenceRecord::Effect {
                    identity,
                    effect_family: EffectFamilyDocument::Heal,
                    formula,
                    ..
                } if identity.key == effect.key && identity.revision == effect.revision => {
                    Some(formula)
                }
                _ => None,
            })
            .ok_or("native Effect is not Heal")?;
        let effect_profile = draft
            .state
            .authoring_profiles
            .iter()
            .find(|p| {
                p.target.family == ProjectV2Family::Effect
                    && p.target.key == effect.key
                    && p.target.revision == effect.revision
            })
            .ok_or("missing Effect profile")?;
        let ProjectV2AuthoringProfileData::Effect(effect_profile) = &effect_profile.data else {
            return Err("wrong Effect profile");
        };
        if effect_profile.damage_type != "healing"
            || !effect_profile.mitigated_by.is_empty()
            || effect_profile.affects.is_some()
        {
            return Err("not source selfheal Effect");
        }
        if !draft.core.records.iter().any(|r|matches!(r,ProjectReferenceRecord::Formula{identity} if identity.key==formula.key && identity.revision==formula.revision)){return Err("missing native Formula record")}
        let profile = draft
            .state
            .authoring_profiles
            .iter()
            .find(|p| {
                p.target.family == ProjectV2Family::Formula
                    && p.target.key == formula.key
                    && p.target.revision == formula.revision
            })
            .ok_or("missing exact Formula profile")?;
        let (minimum, maximum) = match &profile.data {
            ProjectV2AuthoringProfileData::Formula(ProjectV2FormulaAuthoring::Range {
                minimum,
                maximum,
            }) => (*minimum, *maximum),
            ProjectV2AuthoringProfileData::Formula(ProjectV2FormulaAuthoring::CasterMagnitude) => {
                let m = schedule
                    .magnitude
                    .as_ref()
                    .ok_or("CasterMagnitude requires actual defense magnitude")?;
                (m.minimum, m.maximum)
            }
            _ => return Err("not an admitted selfheal formula"),
        };
        let max_health = creature.health.ok_or("missing exact maximum health")?;
        if minimum > maximum
            || maximum > i64::MAX as u64
            || max_health == 0
            || max_health > i64::MAX as u64
        {
            return Err("invalid native selfheal limits");
        }
        // Every family is an actual typed record/profile membership; a source or artifact hash alone
        // is deliberately insufficient. Source-native admission and activation remain the boot owner.
        Ok(Self {
            target: target.into(),
            ability: ability.into(),
            max_health,
            minimum,
            maximum,
            interval_ms: schedule.interval_ms,
            chance_ppm: schedule.chance_ppm,
            removes_paralysis,
        })
    }
    pub(crate) fn removes_paralysis(&self) -> bool {
        self.removes_paralysis
    }
    pub(crate) fn magnitude_range(&self) -> (u64, u64) {
        (self.minimum, self.maximum)
    }
    pub(crate) fn source_schedule(&self) -> (u64, u32) {
        (self.interval_ms, self.chance_ppm)
    }
    pub(crate) fn register(
        &self,
        runtime: &crate::foundation::ChannelRuntimeV1,
    ) -> Result<crate::foundation::CreatureSelfHealRegistration, crate::foundation::CarrierError>
    {
        runtime.bind_creature_self_heal(
            &self.target,
            &self.ability,
            self.max_health,
            self.minimum,
            self.maximum,
        )
    }
}

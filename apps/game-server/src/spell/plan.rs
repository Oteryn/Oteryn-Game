//! Hands a resolved cast to the Ability pipeline (GAME-ABILITY-01: one typed pipeline for every
//! origin). Damage and heal become an [`EffectPlan`] for the owner commit; effects the Ability
//! pipeline does not type yet (condition removal, conjure) are returned beside it, unchanged.

use crate::ability::{
    AbilityError, AbilityIntent, AbilityOccurrence, CalculationStage, CommitGroup, Effect,
    EffectPlan, ProposalSource,
};

use super::{CastResolution, ResolvedEffect, SpellDefinition};

/// Calculation stages of a player cast, in the order the core ran them.
const STAGES: [&str; 2] = [
    "spell.formula.player_expression",
    "spell.draw.world_distribution",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CastPlan {
    /// Damage/heal for the owner commit; `None` when the cast has no damage or heal.
    pub(crate) effects: Option<EffectPlan>,
    /// Resolved effects outside the typed Ability effects.
    pub(crate) side_effects: Vec<ResolvedEffect>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CastPlanError {
    /// A damage effect needs a resolved target.
    MissingTarget,
    Ability(AbilityError),
}

impl From<AbilityError> for CastPlanError {
    fn from(error: AbilityError) -> Self {
        Self::Ability(error)
    }
}

/// Build the Ability plan of one accepted cast by `caster` on `target` (both exact actor atoms).
/// A heal of a self-target spell, or of a spell cast without a target, applies to the caster.
pub(crate) fn effect_plan(
    spell: &SpellDefinition,
    resolution: &CastResolution,
    caster: &str,
    target: Option<&str>,
    occurrence: AbilityOccurrence,
    owner_scope: &str,
) -> Result<CastPlan, CastPlanError> {
    let mut effects = Vec::new();
    let mut side_effects = Vec::new();
    for resolved in &resolution.effects {
        match resolved {
            ResolvedEffect::Damage { magnitude, .. } if *magnitude > 0 => {
                effects.push(Effect::damage(
                    target.ok_or(CastPlanError::MissingTarget)?,
                    *magnitude,
                )?);
            }
            ResolvedEffect::Heal { magnitude } if *magnitude > 0 => {
                let healed = if spell.self_target {
                    caster
                } else {
                    target.unwrap_or(caster)
                };
                effects.push(Effect::heal(healed, *magnitude)?);
            }
            // A zero draw changes no health; the Ability effects carry only positive magnitudes.
            ResolvedEffect::Damage { .. } | ResolvedEffect::Heal { .. } => {}
            other => side_effects.push(other.clone()),
        }
    }
    if effects.is_empty() {
        return Ok(CastPlan {
            effects: None,
            side_effects,
        });
    }
    let mut targets: Vec<&str> = effects
        .iter()
        .map(|effect| effect.target().as_str())
        .collect();
    targets.sort_unstable();
    targets.dedup();
    let intent = AbilityIntent::resolve(ProposalSource::Client, caster, &targets, &targets)?;
    let stages = STAGES
        .iter()
        .map(|stage| CalculationStage::new(stage))
        .collect::<Result<Vec<_>, _>>()?;
    let group = CommitGroup::atomic(owner_scope, occurrence.id().as_str())?;
    let plan = EffectPlan::immediate(occurrence, intent, effects, stages, group)?;
    Ok(CastPlan {
        effects: Some(plan),
        side_effects,
    })
}

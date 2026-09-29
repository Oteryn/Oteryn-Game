//! Hands a resolved cast to the Ability pipeline (GAME-ABILITY-01: one typed pipeline for every
//! origin). Damage and heal become an [`EffectPlan`] for the owner commit; effects the Ability
//! pipeline does not type yet (condition removal, conjure) are returned beside it, unchanged.
//! A chain cast becomes one plan per creature hit, each its own occurrence resolving at the hit's
//! delay.

use crate::ability::{
    AbilityError, AbilityIntent, AbilityOccurrence, CalculationStage, CommitGroup, Effect,
    EffectPlan, ProposalSource,
};

use super::chain::TilePosition;
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
    /// A chain cast needs [`chain_plans`]; a cast that does not chain needs [`effect_plan`].
    WrongPlanKind,
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
    if !resolution.chain.is_empty() {
        return Err(CastPlanError::WrongPlanKind);
    }
    plan(
        spell,
        &resolution.effects,
        caster,
        target,
        occurrence,
        owner_scope,
    )
}

/// The plan of one creature a chain reaches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChainHitPlan {
    /// Resolve this long after the cast (chain §3 step 9).
    pub(crate) delay_micros: u64,
    /// Tiles showing the spell's `chain_asset_binding` when this hit resolves (chain §3 step 10).
    pub(crate) effect_tiles: Vec<TilePosition>,
    pub(crate) plan: CastPlan,
}

/// Build the plans of an accepted chain cast, one per creature in hit order. Hit `n` is the
/// occurrence `<occurrence id>/chain-<n>` with the cast's revisions.
pub(crate) fn chain_plans(
    spell: &SpellDefinition,
    resolution: &CastResolution,
    caster: &str,
    occurrence: &AbilityOccurrence,
    owner_scope: &str,
) -> Result<Vec<ChainHitPlan>, CastPlanError> {
    if resolution.chain.is_empty() {
        return Err(CastPlanError::WrongPlanKind);
    }
    resolution
        .chain
        .iter()
        .enumerate()
        .map(|(index, hit)| {
            let id = format!("{}/chain-{index}", occurrence.id().as_str());
            let hit_occurrence = AbilityOccurrence::new(&id, occurrence.revisions().clone())?;
            Ok(ChainHitPlan {
                delay_micros: hit.hit.delay_micros,
                effect_tiles: hit.hit.effect_tiles.clone(),
                plan: plan(
                    spell,
                    &hit.effects,
                    caster,
                    Some(&hit.hit.actor),
                    hit_occurrence,
                    owner_scope,
                )?,
            })
        })
        .collect()
}

fn plan(
    spell: &SpellDefinition,
    resolved_effects: &[ResolvedEffect],
    caster: &str,
    target: Option<&str>,
    occurrence: AbilityOccurrence,
    owner_scope: &str,
) -> Result<CastPlan, CastPlanError> {
    let mut effects = Vec::new();
    let mut side_effects = Vec::new();
    for resolved in resolved_effects {
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

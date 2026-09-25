use super::effects::Effect;
use super::plan::{EffectPlan, SubOccurrenceRef};
use super::{AbilityError, AbilityOccurrenceId};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitReceipt {
    applied: bool,
    suboccurrences: Vec<SubOccurrenceRef>,
}

impl CommitReceipt {
    #[must_use]
    pub const fn applied(&self) -> bool {
        self.applied
    }

    #[must_use]
    pub const fn applied_suboccurrences(&self) -> usize {
        self.suboccurrences.len()
    }

    #[must_use]
    pub fn suboccurrences(&self) -> &[SubOccurrenceRef] {
        &self.suboccurrences
    }
}

#[derive(Debug)]
enum CommitRecord {
    Complete(EffectPlan),
    Sequential {
        plan: EffectPlan,
        next_effect: usize,
    },
}

impl CommitRecord {
    fn plan(&self) -> &EffectPlan {
        match self {
            Self::Complete(plan) | Self::Sequential { plan, .. } => plan,
        }
    }
}

#[derive(Debug, Default)]
pub struct AbilityEngine {
    committed: BTreeMap<AbilityOccurrenceId, CommitRecord>,
    fixture_health: BTreeMap<String, i64>,
}

impl AbilityEngine {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn commit(&mut self, plan: EffectPlan) -> Result<CommitReceipt, AbilityError> {
        let occurrence_id = plan.occurrence().id().clone();
        let next_effect = if let Some(existing) = self.committed.get(&occurrence_id) {
            let existing_plan = existing.plan();
            if existing_plan.occurrence().revisions() != plan.occurrence().revisions() {
                return Err(AbilityError::OccurrenceRevisionConflict);
            }
            if existing_plan != &plan {
                return Err(AbilityError::OccurrencePlanConflict);
            }
            match existing {
                CommitRecord::Complete(_) => {
                    return Ok(CommitReceipt {
                        applied: false,
                        suboccurrences: Vec::new(),
                    });
                }
                CommitRecord::Sequential { next_effect, .. } => *next_effect,
            }
        } else {
            0
        };

        match plan.commit_group().mode() {
            super::CommitGroupMode::Atomic => self.commit_atomic(occurrence_id, plan),
            super::CommitGroupMode::OrderedSequential => {
                self.commit_sequential(occurrence_id, plan, next_effect)
            }
        }
    }

    #[must_use]
    pub fn fixture_health(&self, target: &str) -> Option<i64> {
        self.fixture_health.get(target).copied()
    }
}

impl AbilityEngine {
    fn commit_atomic(
        &mut self,
        occurrence_id: AbilityOccurrenceId,
        plan: EffectPlan,
    ) -> Result<CommitReceipt, AbilityError> {
        let mut next_health = self.fixture_health.clone();
        for effect in plan.effects() {
            apply_fixture_effect(&mut next_health, effect)?;
        }
        self.fixture_health = next_health;
        self.committed
            .insert(occurrence_id, CommitRecord::Complete(plan));
        Ok(CommitReceipt {
            applied: true,
            suboccurrences: Vec::new(),
        })
    }

    fn commit_sequential(
        &mut self,
        occurrence_id: AbilityOccurrenceId,
        plan: EffectPlan,
        next_effect: usize,
    ) -> Result<CommitReceipt, AbilityError> {
        for index in next_effect..plan.effects().len() {
            let effect = plan.effects()[index].clone();
            if let Err(error) = apply_fixture_effect(&mut self.fixture_health, &effect) {
                self.committed.insert(
                    occurrence_id,
                    CommitRecord::Sequential {
                        plan,
                        next_effect: index,
                    },
                );
                return Err(error);
            }
        }
        let suboccurrences = (next_effect..plan.effects().len())
            .map(|ordinal| plan.sub_occurrence(ordinal))
            .collect::<Option<Vec<_>>>()
            .ok_or(AbilityError::MissingSubOccurrence)?;
        self.committed
            .insert(occurrence_id, CommitRecord::Complete(plan));
        Ok(CommitReceipt {
            applied: !suboccurrences.is_empty(),
            suboccurrences,
        })
    }
}

fn apply_fixture_effect(
    fixture_health: &mut BTreeMap<String, i64>,
    effect: &Effect,
) -> Result<(), AbilityError> {
    effect.validate_magnitude()?;
    let target = effect.target().as_str().to_owned();
    let health = fixture_health.entry(target).or_insert(0);
    *health = match effect {
        Effect::Damage { magnitude, .. } => health
            .checked_sub(*magnitude)
            .ok_or(AbilityError::NumericOverflow)?,
        Effect::Heal { magnitude, .. } => health
            .checked_add(*magnitude)
            .ok_or(AbilityError::NumericOverflow)?,
    };
    Ok(())
}

/// Byte-exact bounded encoding of every semantic plan field. The carrier
/// retains this entire value, rather than a digest with collision risk. Every
/// atom is validated at plan construction and cannot contain zero bytes.
fn encode_owner_damage_plan(plan: &EffectPlan) -> Result<Vec<u8>, AbilityError> {
    let mut encoded = Vec::new();
    encoded.try_reserve_exact(super::MAX_EFFECT_PLAN_BYTES)
        .map_err(|_| AbilityError::RetainedByteOverflow)?;
    fn append(out: &mut Vec<u8>, bytes: &[u8]) -> Result<(), AbilityError> {
        if out.len().checked_add(bytes.len()).is_none_or(|size| size > super::MAX_EFFECT_PLAN_BYTES) {
            return Err(AbilityError::EffectPlanTooLarge);
        }
        out.extend_from_slice(bytes);
        Ok(())
    }
    fn atom(out: &mut Vec<u8>, value: &str) -> Result<(), AbilityError> {
        append(out, value.as_bytes())?;
        append(out, &[0])
    }
    atom(&mut encoded, plan.occurrence().id().as_str())?;
    let revisions = plan.occurrence().revisions();
    for revision in [revisions.ruleset(), revisions.content(), revisions.world_policy(), revisions.formula(), revisions.simulation()] {
        atom(&mut encoded, revision)?;
    }
    let intent = plan.intent();
    append(&mut encoded, &[match intent.proposal_source() {
        super::ProposalSource::Client => 1,
        super::ProposalSource::Ai => 2,
        super::ProposalSource::Script => 3,
    }])?;
    atom(&mut encoded, intent.actor())?;
    append(&mut encoded, &(intent.candidate_count() as u64).to_be_bytes())?;
    append(&mut encoded, &(intent.resolved_targets().len() as u64).to_be_bytes())?;
    for target in intent.resolved_targets() {
        atom(&mut encoded, target.as_str())?;
    }
    append(&mut encoded, &(plan.effects().len() as u64).to_be_bytes())?;
    for effect in plan.effects() {
        append(&mut encoded, &[match effect { Effect::Damage { .. } => 1, Effect::Heal { .. } => 2 }])?;
        atom(&mut encoded, effect.target().as_str())?;
        append(&mut encoded, &effect.magnitude().to_be_bytes())?;
    }
    append(&mut encoded, &(plan.calculation_stages().len() as u64).to_be_bytes())?;
    for stage in plan.calculation_stages() {
        atom(&mut encoded, stage.as_str())?;
    }
    atom(&mut encoded, plan.commit_group().owner_scope())?;
    atom(&mut encoded, plan.commit_group().group_id())?;
    append(&mut encoded, &[match plan.commit_group().mode() {
        super::CommitGroupMode::Atomic => 1,
        super::CommitGroupMode::OrderedSequential => 2,
    }])?;
    Ok(encoded)
}

/// A real typed Ability→Foundation bridge, compiled into the game-server
/// library but never composed into live gameplay. The fixture BTreeMap engine
/// above is intentionally not on this path.
pub(crate) fn commit_exact_owner_damage(
    owner: &mut crate::foundation::CurrentOwnerExactActorCommit<'_>,
    resolved: &super::exact_actor_resolution::ResolvedExactActor,
    plan: &EffectPlan,
) -> Result<crate::foundation::OwnerDamageResult, OwnerCommitError> {
    use super::exact_actor_resolution::ExactActorSource;
    if plan.occurrence() != resolved.occurrence()
        || plan.intent().candidate_count() != 1
        || plan.intent().resolved_targets().len() != 1
        || plan.effects().len() != 1
        || plan.commit_group().mode() != super::CommitGroupMode::Atomic
        || !matches!(
            (resolved.source(), plan.intent().proposal_source()),
            (ExactActorSource::Client, super::ProposalSource::Client)
                | (ExactActorSource::Ai, super::ProposalSource::Ai)
        )
    {
        return Err(OwnerCommitError::InvalidPlan);
    }
    let Effect::Damage { target, magnitude } = &plan.effects()[0] else {
        return Err(OwnerCommitError::InvalidPlan);
    };
    if target != &plan.intent().resolved_targets()[0] || *magnitude <= 0 {
        return Err(OwnerCommitError::InvalidPlan);
    }
    let binding = encode_owner_damage_plan(plan).map_err(OwnerCommitError::Plan)?;
    owner.commit_damage(
        resolved.target(),
        plan.occurrence().id().as_str().as_bytes(),
        &binding,
        *magnitude,
    ).map_err(OwnerCommitError::Owner)
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OwnerCommitError {
    InvalidPlan,
    Plan(AbilityError),
    Owner(crate::foundation::CarrierError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::TargetId;

    #[test]
    fn invalid_effects_fail_before_fixture_mutation() -> Result<(), AbilityError> {
        let target = TargetId::new("target:fixture")?;
        for effect in [
            Effect::Damage {
                target: target.clone(),
                magnitude: 0,
            },
            Effect::Damage {
                target: target.clone(),
                magnitude: -7,
            },
            Effect::Heal {
                target: target.clone(),
                magnitude: 0,
            },
            Effect::Heal {
                target: target.clone(),
                magnitude: -7,
            },
        ] {
            let mut fixture_health = BTreeMap::from([(target.as_str().to_owned(), 23)]);
            assert_eq!(
                apply_fixture_effect(&mut fixture_health, &effect),
                Err(AbilityError::InvalidMagnitude)
            );
            assert_eq!(fixture_health.get(target.as_str()), Some(&23));
            assert_eq!(fixture_health.len(), 1);
        }
        Ok(())
    }
}

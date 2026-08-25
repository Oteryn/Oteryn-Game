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

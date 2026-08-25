use super::effects::Effect;
use super::intent::AbilityIntent;
use super::occurrence::valid_atom;
use super::{
    AbilityError, AbilityOccurrence, MAX_CALCULATION_STAGES, MAX_EFFECT_PLAN_BYTES,
    MAX_EFFECT_PLAN_ENTRIES,
};

const FIXTURE_COUNT_BYTES: usize = 8;
const FIXTURE_PROPOSAL_SOURCE_BYTES: usize = 1;
const FIXTURE_EFFECT_TAG_BYTES: usize = 1;
const FIXTURE_MAGNITUDE_BYTES: usize = 8;
const FIXTURE_GROUP_MODE_BYTES: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CalculationStage(String);

impl CalculationStage {
    pub fn new(value: &str) -> Result<Self, AbilityError> {
        if !valid_atom(value) {
            return Err(AbilityError::InvalidIdentifier);
        }
        Ok(Self(value.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitGroupMode {
    Atomic,
    OrderedSequential,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitGroup {
    owner_scope: String,
    group_id: String,
    mode: CommitGroupMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubOccurrenceRef {
    root_occurrence: AbilityOccurrence,
    owner_scope: String,
    group_id: String,
    ordinal: usize,
}

impl SubOccurrenceRef {
    #[must_use]
    pub fn root_occurrence(&self) -> &AbilityOccurrence {
        &self.root_occurrence
    }

    #[must_use]
    pub fn owner_scope(&self) -> &str {
        &self.owner_scope
    }

    #[must_use]
    pub fn group_id(&self) -> &str {
        &self.group_id
    }

    #[must_use]
    pub const fn ordinal(&self) -> usize {
        self.ordinal
    }
}

impl CommitGroup {
    pub fn atomic(owner_scope: &str, group_id: &str) -> Result<Self, AbilityError> {
        Self::new(owner_scope, group_id, CommitGroupMode::Atomic)
    }

    pub fn ordered_sequential(owner_scope: &str, group_id: &str) -> Result<Self, AbilityError> {
        Self::new(owner_scope, group_id, CommitGroupMode::OrderedSequential)
    }

    fn new(owner_scope: &str, group_id: &str, mode: CommitGroupMode) -> Result<Self, AbilityError> {
        if !valid_atom(owner_scope) || !valid_atom(group_id) {
            return Err(AbilityError::InvalidIdentifier);
        }
        Ok(Self {
            owner_scope: owner_scope.to_owned(),
            group_id: group_id.to_owned(),
            mode,
        })
    }

    #[must_use]
    pub fn owner_scope(&self) -> &str {
        &self.owner_scope
    }

    #[must_use]
    pub fn group_id(&self) -> &str {
        &self.group_id
    }

    #[must_use]
    pub const fn mode(&self) -> CommitGroupMode {
        self.mode
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectPlan {
    occurrence: AbilityOccurrence,
    intent: AbilityIntent,
    effects: Vec<Effect>,
    calculation_stages: Vec<CalculationStage>,
    retained_bytes: usize,
    commit_group: CommitGroup,
}

impl EffectPlan {
    pub fn immediate(
        occurrence: AbilityOccurrence,
        intent: AbilityIntent,
        effects: Vec<Effect>,
        calculation_stages: Vec<CalculationStage>,
        commit_group: CommitGroup,
    ) -> Result<Self, AbilityError> {
        Self::new(
            occurrence,
            intent,
            effects,
            calculation_stages,
            commit_group,
        )
    }

    pub fn new(
        occurrence: AbilityOccurrence,
        intent: AbilityIntent,
        mut effects: Vec<Effect>,
        mut calculation_stages: Vec<CalculationStage>,
        commit_group: CommitGroup,
    ) -> Result<Self, AbilityError> {
        if effects.is_empty() {
            return Err(AbilityError::EmptyEffectPlan);
        }
        if effects.len() > MAX_EFFECT_PLAN_ENTRIES {
            return Err(AbilityError::TooManyEffectPlanEntries);
        }
        if calculation_stages.len() > MAX_CALCULATION_STAGES {
            return Err(AbilityError::TooManyCalculationStages);
        }
        if effects.iter().any(|effect| {
            !intent
                .resolved_targets()
                .iter()
                .any(|target| target == effect.target())
        }) {
            return Err(AbilityError::TargetOutsideResolvedSet);
        }
        let retained_bytes = retained_plan_bytes(
            &occurrence,
            &intent,
            &effects,
            &calculation_stages,
            &commit_group,
        )?;
        if retained_bytes > MAX_EFFECT_PLAN_BYTES {
            return Err(AbilityError::EffectPlanTooLarge);
        }
        effects.sort_by(Effect::canonical_cmp);
        calculation_stages.sort();
        if calculation_stages.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(AbilityError::DuplicateCalculationStage);
        }
        let mut bounded_effects = Vec::with_capacity(effects.len());
        bounded_effects.extend(effects);
        let mut bounded_stages = Vec::with_capacity(calculation_stages.len());
        bounded_stages.extend(calculation_stages);
        Ok(Self {
            occurrence,
            intent,
            effects: bounded_effects,
            calculation_stages: bounded_stages,
            retained_bytes,
            commit_group,
        })
    }

    #[must_use]
    pub fn occurrence(&self) -> &AbilityOccurrence {
        &self.occurrence
    }

    #[must_use]
    pub fn intent(&self) -> &AbilityIntent {
        &self.intent
    }

    #[must_use]
    pub fn effects(&self) -> &[Effect] {
        &self.effects
    }

    #[must_use]
    pub fn calculation_stages(&self) -> &[CalculationStage] {
        &self.calculation_stages
    }

    #[must_use]
    pub const fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }

    #[must_use]
    pub fn commit_group(&self) -> &CommitGroup {
        &self.commit_group
    }

    #[must_use]
    pub fn sub_occurrence(&self, ordinal: usize) -> Option<SubOccurrenceRef> {
        self.effects.get(ordinal).map(|_| SubOccurrenceRef {
            root_occurrence: self.occurrence.clone(),
            owner_scope: self.commit_group.owner_scope().to_owned(),
            group_id: self.commit_group.group_id().to_owned(),
            ordinal,
        })
    }
}

fn retained_plan_bytes(
    occurrence: &AbilityOccurrence,
    intent: &AbilityIntent,
    effects: &[Effect],
    calculation_stages: &[CalculationStage],
    commit_group: &CommitGroup,
) -> Result<usize, AbilityError> {
    let mut total = 0;
    add_bytes(&mut total, occurrence.id().as_str().len())?;
    let revisions = occurrence.revisions();
    for revision in [
        revisions.ruleset(),
        revisions.content(),
        revisions.world_policy(),
        revisions.formula(),
        revisions.simulation(),
    ] {
        add_bytes(&mut total, revision.len())?;
    }
    add_bytes(&mut total, FIXTURE_PROPOSAL_SOURCE_BYTES)?;
    add_bytes(&mut total, FIXTURE_COUNT_BYTES)?;
    add_bytes(&mut total, FIXTURE_COUNT_BYTES)?;
    add_bytes(&mut total, intent.actor().len())?;
    for target in intent.resolved_targets() {
        add_bytes(&mut total, target.as_str().len())?;
    }
    for effect in effects {
        add_bytes(&mut total, FIXTURE_EFFECT_TAG_BYTES)?;
        add_bytes(&mut total, effect.target().as_str().len())?;
        add_bytes(&mut total, FIXTURE_MAGNITUDE_BYTES)?;
    }
    add_bytes(&mut total, FIXTURE_COUNT_BYTES)?;
    for stage in calculation_stages {
        add_bytes(&mut total, stage.as_str().len())?;
    }
    add_bytes(&mut total, FIXTURE_COUNT_BYTES)?;
    add_bytes(&mut total, commit_group.owner_scope().len())?;
    add_bytes(&mut total, commit_group.group_id().len())?;
    add_bytes(&mut total, FIXTURE_GROUP_MODE_BYTES)?;
    add_bytes(&mut total, FIXTURE_COUNT_BYTES)?;
    Ok(total)
}

fn add_bytes(total: &mut usize, bytes: usize) -> Result<(), AbilityError> {
    *total = total
        .checked_add(bytes)
        .ok_or(AbilityError::RetainedByteOverflow)?;
    Ok(())
}

#[path = "../src/ability/mod.rs"]
mod ability;

use ability::{
    AbilityEngine, AbilityError, AbilityIntent, AbilityOccurrence, AiAbilityAdapter,
    CalculationStage, ClientAbilityAdapter, CommitGroup, CommitGroupMode, CommitReceipt, Effect,
    EffectPlan, MAX_EFFECT_PLAN_BYTES, ProposalSource, RevisionSet, ScriptAbilityAdapter,
    SubOccurrenceRef, TargetId,
};

fn revisions(version: &str) -> Result<RevisionSet, AbilityError> {
    RevisionSet::new(
        "ruleset:v1",
        "content:v1",
        "world-policy:v1",
        version,
        "simulation:v1",
    )
}

fn stages(count: usize) -> Result<Vec<CalculationStage>, AbilityError> {
    (0..count)
        .map(|index| CalculationStage::new(&format!("stage:{index}")))
        .collect()
}

fn immediate_damage_plan(occurrence: AbilityOccurrence) -> Result<EffectPlan, AbilityError> {
    let intent =
        AbilityIntent::normalize(ProposalSource::Client, "actor:fixture", &["target:fixture"])?;
    EffectPlan::immediate(
        occurrence,
        intent,
        vec![Effect::damage("target:fixture", 7)?],
        stages(1)?,
        CommitGroup::atomic("runtime-scope:fixture", "commit-group:primary")?,
    )
}

#[test]
fn retry_reuses_the_original_occurrence_revision_without_double_commit() -> Result<(), AbilityError>
{
    let occurrence = AbilityOccurrence::new("ability-occurrence:1", revisions("formula:v1")?)?;
    let plan = immediate_damage_plan(occurrence.clone())?;
    let mut engine = AbilityEngine::new();

    let first: CommitReceipt = engine.commit(plan.clone())?;
    assert!(first.applied());
    assert_eq!(engine.fixture_health("target:fixture"), Some(-7));

    let retry = engine.commit(plan)?;
    assert!(!retry.applied());
    assert_eq!(engine.fixture_health("target:fixture"), Some(-7));

    let incompatible_retry = immediate_damage_plan(AbilityOccurrence::new(
        "ability-occurrence:1",
        revisions("formula:v2")?,
    )?)?;
    assert_eq!(
        engine.commit(incompatible_retry),
        Err(AbilityError::OccurrenceRevisionConflict)
    );
    assert_eq!(engine.fixture_health("target:fixture"), Some(-7));
    Ok(())
}

#[test]
fn plan_accepts_each_registered_maximum_and_rejects_max_plus_one_before_commit()
-> Result<(), AbilityError> {
    let occurrence = AbilityOccurrence::new("ability-occurrence:limits", revisions("formula:v1")?)?;
    let maximum_intent = AbilityIntent::normalize(
        ProposalSource::Script,
        "actor:fixture",
        &["target:a", "target:b"],
    )?;
    let resolved_targets: &[TargetId] = maximum_intent.resolved_targets();
    assert_eq!(resolved_targets.len(), 2);
    assert_eq!(maximum_intent.candidate_count(), 2);

    let maximum_plan = EffectPlan::new(
        occurrence.clone(),
        maximum_intent,
        vec![Effect::damage("target:a", 1)?, Effect::heal("target:b", 1)?],
        stages(8)?,
        CommitGroup::ordered_sequential("runtime-scope:fixture", "commit-group:sequential")?,
    )?;
    assert_eq!(maximum_plan.effects().len(), 2);
    assert_eq!(maximum_plan.intent().candidate_count(), 2);
    assert_eq!(maximum_plan.calculation_stages().len(), 8);
    assert_eq!(
        maximum_plan.commit_group().mode(),
        CommitGroupMode::OrderedSequential
    );
    assert_eq!(
        maximum_plan.commit_group().owner_scope(),
        "runtime-scope:fixture"
    );
    assert_eq!(
        maximum_plan.commit_group().group_id(),
        "commit-group:sequential"
    );
    let mut engine = AbilityEngine::new();
    assert!(engine.commit(maximum_plan)?.applied());
    assert_eq!(engine.fixture_health("target:a"), Some(-1));
    assert_eq!(engine.fixture_health("target:b"), Some(1));

    assert_eq!(
        AbilityIntent::normalize(
            ProposalSource::Client,
            "actor:fixture",
            &["target:a", "target:b", "target:c"],
        ),
        Err(AbilityError::TooManyTargetCandidates)
    );

    let valid_intent = AbilityIntent::normalize(
        ProposalSource::Ai,
        "actor:fixture",
        &["target:a", "target:b"],
    )?;
    assert_eq!(
        EffectPlan::new(
            occurrence.clone(),
            valid_intent.clone(),
            vec![
                Effect::damage("target:a", 1)?,
                Effect::damage("target:b", 1)?,
                Effect::heal("target:a", 1)?,
            ],
            stages(1)?,
            CommitGroup::atomic("runtime-scope:fixture", "commit-group:primary")?,
        ),
        Err(AbilityError::TooManyEffectPlanEntries)
    );
    assert_eq!(
        EffectPlan::new(
            occurrence.clone(),
            valid_intent.clone(),
            vec![Effect::damage("target:a", 1)?],
            stages(9)?,
            CommitGroup::atomic("runtime-scope:fixture", "commit-group:primary")?,
        ),
        Err(AbilityError::TooManyCalculationStages)
    );
    let base_plan = EffectPlan::new(
        AbilityOccurrence::new("o", revisions("formula:v1")?)?,
        valid_intent.clone(),
        vec![Effect::damage("target:a", 1)?],
        stages(1)?,
        CommitGroup::atomic("runtime-scope:fixture", "commit-group:bytes")?,
    )?;
    let exact_id = "o".repeat(1 + MAX_EFFECT_PLAN_BYTES - base_plan.retained_bytes());
    let exact_plan = EffectPlan::new(
        AbilityOccurrence::new(&exact_id, revisions("formula:v1")?)?,
        valid_intent.clone(),
        vec![Effect::damage("target:a", 1)?],
        stages(1)?,
        CommitGroup::atomic("runtime-scope:fixture", "commit-group:bytes")?,
    )?;
    assert_eq!(exact_plan.retained_bytes(), MAX_EFFECT_PLAN_BYTES);
    let oversized_id = "o".repeat(exact_id.len() + 1);
    assert_eq!(
        EffectPlan::new(
            AbilityOccurrence::new(&oversized_id, revisions("formula:v1")?)?,
            valid_intent,
            vec![Effect::damage("target:a", 1)?],
            stages(1)?,
            CommitGroup::atomic("runtime-scope:fixture", "commit-group:bytes")?,
        ),
        Err(AbilityError::EffectPlanTooLarge)
    );
    assert_eq!(engine.fixture_health("target:c"), None);
    Ok(())
}

#[test]
fn proposals_remain_non_mutating_until_the_authoritative_engine_commits() -> Result<(), AbilityError>
{
    let client = ClientAbilityAdapter::normalize("actor:client", &["target:fixture"])?;
    let ai = AiAbilityAdapter::normalize("actor:ai", &["target:fixture"])?;
    let script = ScriptAbilityAdapter::normalize("actor:script", &["target:fixture"])?;
    assert_eq!(client.proposal_source(), ProposalSource::Client);
    assert_eq!(ai.proposal_source(), ProposalSource::Ai);
    assert_eq!(script.proposal_source(), ProposalSource::Script);

    let engine = AbilityEngine::new();
    assert_eq!(engine.fixture_health("target:fixture"), None);
    Ok(())
}

#[test]
fn resolved_target_limit_rejects_resolver_output_after_candidate_validation()
-> Result<(), AbilityError> {
    assert_eq!(
        AbilityIntent::resolve(
            ProposalSource::Client,
            "actor:fixture",
            &["candidate:one"],
            &["target:a", "target:b", "target:c"],
        ),
        Err(AbilityError::TooManyResolvedTargets)
    );
    Ok(())
}

#[test]
fn plan_canonicalizes_calculation_and_effect_order_before_sequential_commit()
-> Result<(), AbilityError> {
    let occurrence =
        AbilityOccurrence::new("ability-occurrence:ordered", revisions("formula:v1")?)?;
    let intent = AbilityIntent::resolve(
        ProposalSource::Script,
        "actor:fixture",
        &["candidate:one"],
        &["target:a", "target:b"],
    )?;
    let plan = EffectPlan::new(
        occurrence,
        intent,
        vec![Effect::heal("target:b", 2)?, Effect::damage("target:a", 3)?],
        vec![
            CalculationStage::new("stage:z")?,
            CalculationStage::new("stage:a")?,
        ],
        CommitGroup::ordered_sequential("runtime-scope:fixture", "commit-group:ordered")?,
    )?;
    let effect_targets = plan
        .effects()
        .iter()
        .map(|effect| effect.target().as_str())
        .collect::<Vec<_>>();
    let stage_names = plan
        .calculation_stages()
        .iter()
        .map(CalculationStage::as_str)
        .collect::<Vec<_>>();
    assert_eq!(effect_targets, vec!["target:a", "target:b"]);
    assert_eq!(stage_names, vec!["stage:a", "stage:z"]);

    let mut engine = AbilityEngine::new();
    let receipt: CommitReceipt = engine.commit(plan)?;
    assert!(receipt.applied());
    assert_eq!(receipt.applied_suboccurrences(), 2);
    let suboccurrences: &[SubOccurrenceRef] = receipt.suboccurrences();
    assert_eq!(suboccurrences[0].ordinal(), 0);
    assert_eq!(suboccurrences[1].ordinal(), 1);
    assert_eq!(
        suboccurrences[0].root_occurrence().id().as_str(),
        "ability-occurrence:ordered"
    );
    assert_eq!(suboccurrences[0].owner_scope(), "runtime-scope:fixture");
    assert_eq!(suboccurrences[0].group_id(), "commit-group:ordered");
    assert_eq!(engine.fixture_health("target:a"), Some(-3));
    assert_eq!(engine.fixture_health("target:b"), Some(2));
    Ok(())
}

#[test]
fn sequential_commit_records_progress_without_replaying_prior_suboccurrences()
-> Result<(), AbilityError> {
    let setup_occurrence =
        AbilityOccurrence::new("ability-occurrence:partial-setup", revisions("formula:v1")?)?;
    let setup_intent =
        AbilityIntent::normalize(ProposalSource::Client, "actor:fixture", &["target:fixture"])?;
    let setup_plan = EffectPlan::new(
        setup_occurrence,
        setup_intent,
        vec![Effect::heal("target:fixture", i64::MAX)?],
        stages(1)?,
        CommitGroup::atomic("runtime-scope:fixture", "commit-group:partial-setup")?,
    )?;
    let occurrence =
        AbilityOccurrence::new("ability-occurrence:partial", revisions("formula:v1")?)?;
    let intent =
        AbilityIntent::normalize(ProposalSource::Client, "actor:fixture", &["target:fixture"])?;
    let plan = EffectPlan::new(
        occurrence,
        intent,
        vec![
            Effect::damage("target:fixture", 1)?,
            Effect::heal("target:fixture", 2)?,
        ],
        stages(1)?,
        CommitGroup::ordered_sequential("runtime-scope:fixture", "commit-group:partial")?,
    )?;
    let mut engine = AbilityEngine::new();
    assert!(engine.commit(setup_plan)?.applied());
    assert_eq!(engine.fixture_health("target:fixture"), Some(i64::MAX));

    assert_eq!(
        engine.commit(plan.clone()),
        Err(AbilityError::NumericOverflow)
    );
    assert_eq!(engine.fixture_health("target:fixture"), Some(i64::MAX - 1));
    assert_eq!(engine.commit(plan), Err(AbilityError::NumericOverflow));
    // Replaying the prior damage would lower health to MAX - 2, letting heal(2)
    // succeed. The unchanged overflow proves retry resumes at the failed effect.
    assert_eq!(engine.fixture_health("target:fixture"), Some(i64::MAX - 1));
    Ok(())
}

#[test]
fn atomic_commit_rejects_later_overflow_without_partial_fixture_mutation()
-> Result<(), AbilityError> {
    let occurrence = AbilityOccurrence::new("ability-occurrence:atomic", revisions("formula:v1")?)?;
    let intent =
        AbilityIntent::normalize(ProposalSource::Client, "actor:fixture", &["target:fixture"])?;
    let plan = EffectPlan::new(
        occurrence,
        intent,
        vec![
            Effect::heal("target:fixture", i64::MAX)?,
            Effect::heal("target:fixture", 1)?,
        ],
        stages(1)?,
        CommitGroup::atomic("runtime-scope:fixture", "commit-group:atomic")?,
    )?;
    let mut engine = AbilityEngine::new();

    assert_eq!(engine.commit(plan), Err(AbilityError::NumericOverflow));
    assert_eq!(engine.fixture_health("target:fixture"), None);
    Ok(())
}

#[test]
fn normalization_has_stable_membership_and_order_under_shuffled_candidates()
-> Result<(), AbilityError> {
    let normalized = AbilityIntent::normalize(
        ProposalSource::Client,
        "actor:fixture",
        &["target:b", "target:a"],
    )?;
    let targets = normalized
        .resolved_targets()
        .iter()
        .map(|target| target.as_str())
        .collect::<Vec<_>>();
    assert_eq!(targets, vec!["target:a", "target:b"]);
    Ok(())
}

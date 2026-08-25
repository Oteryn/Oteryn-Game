#[path = "../src/ai/mod.rs"]
mod ai;

use ai::{
    ActorId, AiError, AiProvenance, AiProvenanceInput, AiSnapshot, BootstrapOperation, Candidate,
    CandidateId, Decision, DecisionUnit, PathProposal, PathRequest, ResourceLimit, RouteStep,
    build_path_proposal, canonicalize_perception, resolve,
};

fn provenance() -> AiProvenance {
    AiProvenance::new(AiProvenanceInput {
        scope_id: 7,
        scope_generation: 3,
        actor_generation: 11,
        behavior_revision: 13,
        content_revision: 17,
        navigation_revision: 19,
        ruleset_revision: 23,
        determinism_profile_revision: 29,
    })
}

#[test]
fn every_hard_limit_accepts_max_and_rejects_max_plus_one_and_overflow() {
    for limit in ResourceLimit::ALL {
        assert_eq!(limit.admit(0, limit.maximum()), Ok(limit.maximum()));
        assert_eq!(
            limit.admit(0, limit.maximum() + 1),
            Err(AiError::CapacityExceeded(limit))
        );
        assert_eq!(
            limit.admit(usize::MAX, 1),
            Err(AiError::ArithmeticOverflow(limit))
        );
    }
}

#[test]
fn active_actor_envelope_accepts_256_and_rejects_257_before_snapshot_publication()
    -> Result<(), AiError> {
    let actors: Vec<_> = (0..ResourceLimit::ActiveActors.maximum())
        .map(|value| ActorId::new(value as u64))
        .collect();
    let snapshot = AiSnapshot::new(provenance(), &actors)?;
    assert_eq!(snapshot.active_actors().len(), 256);
    assert_eq!(snapshot.provenance(), provenance());

    let mut too_many = actors;
    too_many.push(ActorId::new(999));
    assert_eq!(
        AiSnapshot::new(provenance(), &too_many),
        Err(AiError::CapacityExceeded(ResourceLimit::ActiveActors))
    );
    Ok(())
}

#[test]
fn canonical_perception_accepts_64_and_rejects_65_and_shuffled_ties_choose_lowest_id()
-> Result<(), AiError> {
    let candidates: Vec<_> = (0..ResourceLimit::PerceptionCandidates.maximum())
        .rev()
        .map(|value| Candidate::new(CandidateId::new(value as u64), 1))
        .collect();
    let perception = canonicalize_perception(&candidates)?;
    assert_eq!(
        perception.candidates().first().map(Candidate::id),
        Some(CandidateId::new(0))
    );

    let mut too_many = candidates;
    too_many.push(Candidate::new(CandidateId::new(1_000), 1));
    assert_eq!(
        canonicalize_perception(&too_many),
        Err(AiError::CapacityExceeded(
            ResourceLimit::PerceptionCandidates
        ))
    );
    Ok(())
}

#[test]
fn resolution_is_deterministic_stale_safe_and_cannot_publish_partial_result() -> Result<(), AiError>
{
    let snapshot = AiSnapshot::new(provenance(), &[ActorId::new(4)])?;
    let perception = canonicalize_perception(&[
        Candidate::new(CandidateId::new(20), 7),
        Candidate::new(CandidateId::new(10), 7),
    ])?;
    let program = [
        DecisionUnit::acquire(1),
        DecisionUnit::idle(2),
        DecisionUnit::idle(3),
        DecisionUnit::idle(4),
    ];
    assert_eq!(
        resolve(&snapshot, &perception, provenance(), &program, 8),
        Ok(Decision::AcquireCandidate(CandidateId::new(10)))
    );
    assert_eq!(
        resolve(&snapshot, &perception, provenance(), &program, 9),
        Err(AiError::CapacityExceeded(ResourceLimit::EvaluationWork))
    );
    assert_eq!(
        resolve(
            &snapshot,
            &perception,
            provenance().with_actor_generation(99),
            &program,
            8
        ),
        Err(AiError::StaleProvenance)
    );
    assert_eq!(
        resolve(
            &snapshot,
            &perception,
            provenance().with_scope_generation(99),
            &program,
            8
        ),
        Err(AiError::StaleProvenance)
    );
    assert_eq!(
        resolve(
            &snapshot,
            &perception,
            provenance().with_content_revision(99),
            &program,
            8
        ),
        Err(AiError::StaleProvenance)
    );
    Ok(())
}

#[test]
fn resolution_canonicalizes_shuffled_unique_ordinals_and_rejects_duplicates() -> Result<(), AiError>
{
    let snapshot = AiSnapshot::new(provenance(), &[ActorId::new(4)])?;
    let perception = canonicalize_perception(&[Candidate::new(CandidateId::new(10), 7)])?;
    let canonical = [DecisionUnit::acquire(1), DecisionUnit::idle(2)];
    let shuffled = [DecisionUnit::idle(2), DecisionUnit::acquire(1)];
    let expected = Ok(Decision::AcquireCandidate(CandidateId::new(10)));

    assert_eq!(
        resolve(&snapshot, &perception, provenance(), &canonical, 2),
        expected
    );
    assert_eq!(
        resolve(&snapshot, &perception, provenance(), &shuffled, 2),
        expected
    );
    assert_eq!(
        resolve(
            &snapshot,
            &perception,
            provenance(),
            &[DecisionUnit::idle(1), DecisionUnit::acquire(1)],
            2,
        ),
        Err(AiError::InvalidInput)
    );
    Ok(())
}

#[test]
fn path_proposal_is_data_only_and_rejects_config_and_hard_capacity_boundaries()
-> Result<(), AiError> {
    let request = PathRequest::new(provenance(), 31, 1, 1_024);
    let route: Vec<_> = (0..ResourceLimit::RouteSteps.maximum())
        .map(|value| RouteStep::new(value as u64, 32))
        .collect();
    let proposal: PathProposal = build_path_proposal(request, &route)?;
    assert_eq!(
        proposal,
        build_path_proposal(PathRequest::new(provenance(), 31, 1, 1_024), &route)?
    );
    assert_eq!(proposal.steps().len(), 128);
    assert_eq!(proposal.steps().first().map(RouteStep::node), Some(0));
    assert_eq!(proposal.route_bytes(), 4_096);
    assert_eq!(proposal.work_id(), 31);
    assert_eq!(proposal.revalidate(provenance()), Ok(()));
    assert_eq!(
        proposal.revalidate(provenance().with_navigation_revision(41)),
        Err(AiError::StaleProvenance)
    );
    assert_eq!(
        build_path_proposal(PathRequest::new(provenance(), 31, 2, 1), &[]),
        Err(AiError::BootstrapPathRequestLimit)
    );
    assert_eq!(
        build_path_proposal(PathRequest::new(provenance(), 31, 1, 1_025), &[]),
        Err(AiError::CapacityExceeded(ResourceLimit::PathSearchWork))
    );
    Ok(())
}

#[test]
fn excluded_dimensions_fail_closed_without_a_foreign_mutation_operation() {
    for operation in [
        BootstrapOperation::Resolve,
        BootstrapOperation::PathProposal,
    ] {
        assert_eq!(operation.reject(), Ok(()));
    }
    for operation in [
        BootstrapOperation::ControlledActor,
        BootstrapOperation::Diagnostics,
        BootstrapOperation::Memory,
        BootstrapOperation::RepathRetry,
        BootstrapOperation::Script,
        BootstrapOperation::Spawn,
        BootstrapOperation::Timer,
    ] {
        assert_eq!(operation.reject(), Err(AiError::ExcludedDimension));
    }
}

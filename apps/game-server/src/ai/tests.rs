use super::{
    ActorId, AiError, AiProvenance, AiProvenanceInput, AiSnapshot, Candidate, CandidateId,
    DecisionUnit, PathRequest, ResourceLimit, RouteStep, build_path_proposal,
    canonicalize_perception, resolve,
};

fn provenance() -> AiProvenance {
    AiProvenance::new(AiProvenanceInput {
        scope_id: 1,
        scope_generation: 2,
        actor_generation: 3,
        behavior_revision: 4,
        content_revision: 5,
        navigation_revision: 6,
        ruleset_revision: 7,
        determinism_profile_revision: 8,
    })
}

#[test]
fn malformed_duplicate_snapshot_and_perception_inputs_fail_closed() {
    assert_eq!(
        AiSnapshot::new(provenance(), &[ActorId::new(9), ActorId::new(9)]),
        Err(AiError::InvalidInput)
    );
    assert_eq!(
        canonicalize_perception(&[
            Candidate::new(CandidateId::new(9), 1),
            Candidate::new(CandidateId::new(9), 2),
        ]),
        Err(AiError::InvalidInput)
    );
}

#[test]
fn evaluation_exhaustion_and_route_byte_overflow_publish_no_result() -> Result<(), AiError> {
    let snapshot = AiSnapshot::new(provenance(), &[ActorId::new(1)])?;
    let perception = canonicalize_perception(&[])?;
    assert_eq!(
        resolve(
            &snapshot,
            &perception,
            provenance(),
            &[DecisionUnit::idle(1), DecisionUnit::idle(2)],
            1,
        ),
        Err(AiError::EvaluationExhausted)
    );
    let route = [RouteStep::new(1, usize::MAX), RouteStep::new(2, 1)];
    assert_eq!(
        build_path_proposal(PathRequest::new(provenance(), 1, 1, 1), &route),
        Err(AiError::CapacityExceeded(ResourceLimit::RouteBytes))
    );
    Ok(())
}

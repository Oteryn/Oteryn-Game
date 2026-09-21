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
fn priority_separated_duplicate_candidate_ids_fail_for_every_input_permutation() {
    let duplicate_high = Candidate::new(CandidateId::new(9), 30);
    let unrelated = Candidate::new(CandidateId::new(4), 20);
    let duplicate_low = Candidate::new(CandidateId::new(9), 10);
    let permutations = [
        [duplicate_high, unrelated, duplicate_low],
        [duplicate_high, duplicate_low, unrelated],
        [unrelated, duplicate_high, duplicate_low],
        [unrelated, duplicate_low, duplicate_high],
        [duplicate_low, duplicate_high, unrelated],
        [duplicate_low, unrelated, duplicate_high],
    ];

    for input in permutations {
        assert_eq!(
            canonicalize_perception(&input),
            Err(AiError::InvalidInput),
            "duplicate identity escaped for input {input:?}"
        );
    }
}

#[test]
fn duplicate_candidate_ids_fail_at_input_head_middle_and_tail() {
    let cases = [
        [
            Candidate::new(CandidateId::new(9), 10),
            Candidate::new(CandidateId::new(9), 10),
            Candidate::new(CandidateId::new(4), 20),
        ],
        [
            Candidate::new(CandidateId::new(9), 30),
            Candidate::new(CandidateId::new(4), 20),
            Candidate::new(CandidateId::new(9), 10),
        ],
        [
            Candidate::new(CandidateId::new(4), 20),
            Candidate::new(CandidateId::new(9), 10),
            Candidate::new(CandidateId::new(9), 10),
        ],
    ];

    for input in cases {
        assert_eq!(
            canonicalize_perception(&input),
            Err(AiError::InvalidInput),
            "duplicate identity escaped for input {input:?}"
        );
    }
}

#[test]
fn unique_perception_candidates_preserve_canonical_order() -> Result<(), AiError> {
    let input = [
        Candidate::new(CandidateId::new(8), 10),
        Candidate::new(CandidateId::new(3), 30),
        Candidate::new(CandidateId::new(7), 20),
        Candidate::new(CandidateId::new(2), 20),
    ];

    let perception = canonicalize_perception(&input)?;

    assert_eq!(
        perception.candidates(),
        &[
            Candidate::new(CandidateId::new(3), 30),
            Candidate::new(CandidateId::new(2), 20),
            Candidate::new(CandidateId::new(7), 20),
            Candidate::new(CandidateId::new(8), 10),
        ]
    );
    Ok(())
}

#[test]
fn perception_candidate_capacity_behavior_is_unchanged() -> Result<(), AiError> {
    let maximum = ResourceLimit::PerceptionCandidates.maximum();
    let at_limit = (0..maximum)
        .map(|value| Candidate::new(CandidateId::new(value as u64), 1))
        .collect::<Vec<_>>();
    assert_eq!(
        canonicalize_perception(&at_limit)?.candidates().len(),
        maximum
    );

    let over_limit = (0..=maximum)
        .map(|value| Candidate::new(CandidateId::new(value as u64), 1))
        .collect::<Vec<_>>();
    assert_eq!(
        canonicalize_perception(&over_limit),
        Err(AiError::CapacityExceeded(
            ResourceLimit::PerceptionCandidates
        ))
    );
    Ok(())
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

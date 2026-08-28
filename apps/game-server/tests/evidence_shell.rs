mod support;

use support::evidence::*;

fn cell(tier: ExecutionTier) -> ComparisonCell {
    ComparisonCell {
        scenario: "foundation-login-relog",
        tier,
        client_artifact: "sha256:client",
        server_artifact: "sha256:server",
        platform_revision: "platform-r1",
        protocol_revision: 1,
        ruleset_revision: 2,
        content_revision: 3,
        world_bundle: "sha256:world",
        migration_revision: "migration-r1",
        database_image: "postgres-17:test",
        operating_system: "windows",
        target_triple: "x86_64-pc-windows-msvc",
        build_features: "e2e",
        random_seed: 42,
        clock_mode: "manual",
        fault_profile: "none",
        topology: "single-channel",
    }
}
fn real_tier1() -> BoundaryEvidence {
    BoundaryEvidence {
        production_process: true,
        production_transport: true,
        platform_admission: true,
        persistence_boundary: true,
        normal_networking: true,
        server_legality_checks: true,
        native_client: false,
        production_default_artifacts: false,
        test_adapter_present: false,
        direct_domain_mutation: false,
    }
}

fn real_tier2() -> BoundaryEvidence {
    BoundaryEvidence {
        native_client: true,
        test_adapter_present: true,
        ..real_tier1()
    }
}

fn real_tier3() -> BoundaryEvidence {
    BoundaryEvidence {
        native_client: true,
        production_default_artifacts: true,
        test_adapter_present: false,
        ..real_tier1()
    }
}

fn phases(tier: ExecutionTier) -> Vec<PhaseEvidence> {
    vec![
        PhaseEvidence::passed(Phase::Environment),
        PhaseEvidence::passed(Phase::Identity),
        PhaseEvidence::passed(Phase::WorldDiscovery),
        PhaseEvidence::passed(Phase::Gateway),
        PhaseEvidence::passed(Phase::GameSession),
        PhaseEvidence::passed(Phase::Transport),
        PhaseEvidence::passed(Phase::Admission),
        PhaseEvidence::passed(Phase::CharacterLease),
        PhaseEvidence::passed(Phase::WorldEntry),
        PhaseEvidence::passed(Phase::Gameplay),
        PhaseEvidence::passed(Phase::Persistence),
        PhaseEvidence::passed(Phase::AuditOutbox),
        match tier {
            ExecutionTier::Tier1HeadlessSystem => PhaseEvidence::not_applicable(
                Phase::ClientPresentation,
                "Tier 1 has no native-client presentation",
            ),
            ExecutionTier::Tier2NativeClient | ExecutionTier::Tier3ProductionSmoke => {
                PhaseEvidence::passed(Phase::ClientPresentation)
            }
        },
        PhaseEvidence::passed(Phase::Cleanup),
    ]
}
fn attempt(
    id: &'static str,
    tier: ExecutionTier,
    boundary: BoundaryEvidence,
    outcome: AttemptOutcome,
    cleanup: CleanupStatus,
) -> AttemptEvidence {
    AttemptEvidence {
        attempt_id: id,
        cell: cell(tier),
        boundary,
        phases: phases(tier),
        first_divergence: None,
        failure_class: None,
        cleanup,
        artifacts: vec![ArtifactEvidence {
            name: "attempt.json",
            digest: "sha256:evidence",
        }],
        start_unix_millis: 1_700_000_000_000,
        duration_millis: 125,
        outcome,
    }
}

fn failed_attempt(
    id: &'static str,
    outcome: AttemptOutcome,
    phase: Phase,
    failure_class: &'static str,
) -> AttemptEvidence {
    let mut evidence = attempt(
        id,
        ExecutionTier::Tier1HeadlessSystem,
        real_tier1(),
        outcome,
        CleanupStatus::Complete,
    );
    evidence.phases[phase as usize].status = PhaseStatus::Failed;
    evidence.first_divergence = Some(phase);
    evidence.failure_class = Some(failure_class);
    evidence
}

#[test]
fn tier1_rejects_mock_transport_and_direct_domain_shortcuts() {
    let mut mocked = real_tier1();
    mocked.production_transport = false;
    assert_eq!(
        validate_attempt(&attempt(
            "tier1-mock",
            ExecutionTier::Tier1HeadlessSystem,
            mocked,
            AttemptOutcome::Passed,
            CleanupStatus::Complete,
        )),
        Err(EvidenceError::TierBoundaryNotSatisfied),
    );

    let mut direct = real_tier1();
    direct.direct_domain_mutation = true;
    assert_eq!(
        validate_attempt(&attempt(
            "tier1-direct",
            ExecutionTier::Tier1HeadlessSystem,
            direct,
            AttemptOutcome::Passed,
            CleanupStatus::Complete,
        )),
        Err(EvidenceError::AuthoritativeShortcut),
    );
}
#[test]
fn tier2_requires_real_native_client_and_normal_networking() {
    let mut synthetic = real_tier2();
    synthetic.native_client = false;
    assert_eq!(
        validate_attempt(&attempt(
            "tier2-synthetic",
            ExecutionTier::Tier2NativeClient,
            synthetic,
            AttemptOutcome::Passed,
            CleanupStatus::Complete,
        )),
        Err(EvidenceError::TierBoundaryNotSatisfied),
    );

    let mut bypass = real_tier2();
    bypass.normal_networking = false;
    assert_eq!(
        validate_attempt(&attempt(
            "tier2-network-bypass",
            ExecutionTier::Tier2NativeClient,
            bypass,
            AttemptOutcome::Passed,
            CleanupStatus::Complete,
        )),
        Err(EvidenceError::TierBoundaryNotSatisfied),
    );
}
#[test]
fn population_classification_matches_adr_contract() {
    let pass = attempt(
        "pass-1",
        ExecutionTier::Tier1HeadlessSystem,
        real_tier1(),
        AttemptOutcome::Passed,
        CleanupStatus::Complete,
    );
    let fail = failed_attempt(
        "fail-1",
        AttemptOutcome::ProductFailure,
        Phase::Gameplay,
        "product-divergence",
    );

    assert_eq!(
        classify_population(std::slice::from_ref(&pass), 1).classification,
        PopulationClassification::Pass
    );
    assert_eq!(
        classify_population(std::slice::from_ref(&fail), 1).classification,
        PopulationClassification::Fail
    );
    assert_eq!(
        classify_population(&[pass.clone(), fail.clone()], 2).classification,
        PopulationClassification::Unstable,
    );
    assert_eq!(
        classify_population(std::slice::from_ref(&pass), 2).classification,
        PopulationClassification::NotEvaluated,
    );
}
#[test]
fn cleanup_unknown_blocks_pass_and_attempt_history_is_retained() {
    let pass = attempt(
        "attempt-pass",
        ExecutionTier::Tier1HeadlessSystem,
        real_tier1(),
        AttemptOutcome::Passed,
        CleanupStatus::Complete,
    );
    let unknown_cleanup = attempt(
        "attempt-cleanup-unknown",
        ExecutionTier::Tier1HeadlessSystem,
        real_tier1(),
        AttemptOutcome::Passed,
        CleanupStatus::Unknown,
    );

    let report = classify_population(&[pass, unknown_cleanup], 2);
    assert_eq!(report.classification, PopulationClassification::Blocked);
    assert_eq!(
        report.attempt_ids,
        vec!["attempt-pass", "attempt-cleanup-unknown"],
    );
}
#[test]
fn mismatched_comparison_cell_blocks_population() {
    let first = attempt(
        "cell-1",
        ExecutionTier::Tier1HeadlessSystem,
        real_tier1(),
        AttemptOutcome::Passed,
        CleanupStatus::Complete,
    );
    let mut second = attempt(
        "cell-2",
        ExecutionTier::Tier1HeadlessSystem,
        real_tier1(),
        AttemptOutcome::Passed,
        CleanupStatus::Complete,
    );
    second.cell.random_seed = 99;

    let report = classify_population(&[first, second], 2);
    assert_eq!(report.classification, PopulationClassification::Blocked);
}
#[test]
fn first_divergence_must_match_earliest_failed_phase() {
    let mut evidence = failed_attempt(
        "divergence",
        AttemptOutcome::ProductFailure,
        Phase::Admission,
        "product-divergence",
    );
    evidence.phases[Phase::Gameplay as usize].status = PhaseStatus::Failed;
    evidence.first_divergence = Some(Phase::Gameplay);

    assert_eq!(
        validate_attempt(&evidence),
        Err(EvidenceError::FirstDivergenceMismatch),
    );
}

#[test]
fn tier3_requires_native_client_boundary() {
    let mut production = real_tier2();
    production.native_client = false;
    assert_eq!(
        validate_attempt(&attempt(
            "tier3-no-native-client",
            ExecutionTier::Tier3ProductionSmoke,
            production,
            AttemptOutcome::Passed,
            CleanupStatus::Complete,
        )),
        Err(EvidenceError::TierBoundaryNotSatisfied),
    );
}

#[test]
fn incomplete_cleanup_and_infrastructure_failure_remain_explicit() {
    let incomplete = attempt(
        "cleanup-incomplete",
        ExecutionTier::Tier1HeadlessSystem,
        real_tier1(),
        AttemptOutcome::Passed,
        CleanupStatus::Incomplete,
    );
    assert_eq!(
        classify_population(std::slice::from_ref(&incomplete), 1).classification,
        PopulationClassification::Blocked,
    );

    let infrastructure = failed_attempt(
        "infrastructure-failure",
        AttemptOutcome::InfrastructureFailure,
        Phase::Environment,
        "infrastructure-runner",
    );
    assert_eq!(
        classify_population(std::slice::from_ref(&infrastructure), 1).classification,
        PopulationClassification::Fail,
    );
}

#[test]
fn missing_build_features_is_incomplete_evidence() {
    let mut evidence = attempt(
        "missing-build-features",
        ExecutionTier::Tier1HeadlessSystem,
        real_tier1(),
        AttemptOutcome::Passed,
        CleanupStatus::Complete,
    );
    evidence.cell.build_features = "";

    assert_eq!(
        validate_attempt(&evidence),
        Err(EvidenceError::EvidenceIncomplete),
    );
}

#[test]
fn partial_phase_list_cannot_pass_as_e2e() {
    let mut evidence = attempt(
        "partial-phases",
        ExecutionTier::Tier1HeadlessSystem,
        real_tier1(),
        AttemptOutcome::Passed,
        CleanupStatus::Complete,
    );
    evidence.phases = vec![
        PhaseEvidence::passed(Phase::Environment),
        PhaseEvidence::passed(Phase::Cleanup),
    ];
    assert_eq!(
        validate_attempt(&evidence),
        Err(EvidenceError::EvidenceIncomplete),
    );
}

#[test]
fn failed_attempt_requires_failure_evidence() {
    let evidence = attempt(
        "missing-failure-evidence",
        ExecutionTier::Tier1HeadlessSystem,
        real_tier1(),
        AttemptOutcome::ProductFailure,
        CleanupStatus::Complete,
    );

    assert_eq!(
        validate_attempt(&evidence),
        Err(EvidenceError::EvidenceIncomplete),
    );
}

#[test]
fn duplicate_attempt_ids_block_population() {
    let first = attempt(
        "same-physical-attempt",
        ExecutionTier::Tier1HeadlessSystem,
        real_tier1(),
        AttemptOutcome::Passed,
        CleanupStatus::Complete,
    );
    let second = first.clone();

    let report = classify_population(&[first, second], 2);
    assert_eq!(report.classification, PopulationClassification::Blocked);
}

#[test]
fn tier3_rejects_tier2_boundary_as_production_binary_proof() {
    let evidence = attempt(
        "tier3-instrumented-client",
        ExecutionTier::Tier3ProductionSmoke,
        real_tier2(),
        AttemptOutcome::Passed,
        CleanupStatus::Complete,
    );

    assert_eq!(
        validate_attempt(&evidence),
        Err(EvidenceError::TierBoundaryNotSatisfied),
    );
}

#[test]
fn tier2_requires_client_presentation_evidence() {
    let mut evidence = attempt(
        "tier2-no-presentation",
        ExecutionTier::Tier2NativeClient,
        real_tier2(),
        AttemptOutcome::Passed,
        CleanupStatus::Complete,
    );
    evidence.phases[Phase::ClientPresentation as usize].status =
        PhaseStatus::NotApplicable("presentation omitted");

    assert_eq!(
        validate_attempt(&evidence),
        Err(EvidenceError::EvidenceIncomplete),
    );
}

#[test]
fn cleanup_phase_must_match_complete_cleanup_summary() {
    let evidence = failed_attempt(
        "cleanup-summary-mismatch",
        AttemptOutcome::InfrastructureFailure,
        Phase::Cleanup,
        "cleanup-failure",
    );

    assert_eq!(
        validate_attempt(&evidence),
        Err(EvidenceError::CleanupIncomplete),
    );
}

#[test]
fn tier1_rejects_native_client_mislabeled_as_headless() {
    let evidence = attempt(
        "tier1-native-client",
        ExecutionTier::Tier1HeadlessSystem,
        real_tier2(),
        AttemptOutcome::Passed,
        CleanupStatus::Complete,
    );
    assert_eq!(
        validate_attempt(&evidence),
        Err(EvidenceError::TierBoundaryNotSatisfied),
    );
}

#[test]
fn tier3_accepts_production_default_native_boundary() {
    let evidence = attempt(
        "tier3-production",
        ExecutionTier::Tier3ProductionSmoke,
        real_tier3(),
        AttemptOutcome::Passed,
        CleanupStatus::Complete,
    );
    assert_eq!(validate_attempt(&evidence), Ok(()));
}

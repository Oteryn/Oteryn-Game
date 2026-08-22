#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionTier {
    Tier1HeadlessSystem,
    Tier2NativeClient,
    Tier3ProductionSmoke,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparisonCell {
    pub scenario: &'static str,
    pub tier: ExecutionTier,
    pub client_artifact: &'static str,
    pub server_artifact: &'static str,
    pub platform_revision: &'static str,
    pub protocol_revision: u64,
    pub ruleset_revision: u64,
    pub content_revision: u64,
    pub world_bundle: &'static str,
    pub migration_revision: &'static str,
    pub database_image: &'static str,
    pub operating_system: &'static str,
    pub target_triple: &'static str,
    pub build_features: &'static str,
    pub random_seed: u64,
    pub clock_mode: &'static str,
    pub fault_profile: &'static str,
    pub topology: &'static str,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Phase {
    Environment,
    Identity,
    WorldDiscovery,
    Gateway,
    GameSession,
    Transport,
    Admission,
    CharacterLease,
    WorldEntry,
    Gameplay,
    Persistence,
    AuditOutbox,
    ClientPresentation,
    Cleanup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseStatus {
    Passed,
    Failed,
    NotApplicable(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhaseEvidence {
    pub phase: Phase,
    pub status: PhaseStatus,
}
impl PhaseEvidence {
    #[must_use]
    pub const fn passed(phase: Phase) -> Self {
        Self {
            phase,
            status: PhaseStatus::Passed,
        }
    }

    #[must_use]
    pub const fn failed(phase: Phase) -> Self {
        Self {
            phase,
            status: PhaseStatus::Failed,
        }
    }

    #[must_use]
    pub const fn not_applicable(phase: Phase, reason: &'static str) -> Self {
        Self {
            phase,
            status: PhaseStatus::NotApplicable(reason),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoundaryEvidence {
    pub production_process: bool,
    pub production_transport: bool,
    pub platform_admission: bool,
    pub persistence_boundary: bool,
    pub normal_networking: bool,
    pub server_legality_checks: bool,
    pub native_client: bool,
    pub direct_domain_mutation: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupStatus {
    Complete,
    Incomplete,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptOutcome {
    Passed,
    ProductFailure,
    InfrastructureFailure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactEvidence {
    pub name: &'static str,
    pub digest: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttemptEvidence {
    pub attempt_id: &'static str,
    pub cell: ComparisonCell,
    pub boundary: BoundaryEvidence,
    pub phases: Vec<PhaseEvidence>,
    pub first_divergence: Option<Phase>,
    pub failure_class: Option<&'static str>,
    pub cleanup: CleanupStatus,
    pub artifacts: Vec<ArtifactEvidence>,
    pub start_unix_millis: u64,
    pub duration_millis: u64,
    pub outcome: AttemptOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceError {
    TierBoundaryNotSatisfied,
    AuthoritativeShortcut,
    FirstDivergenceMismatch,
    PhaseOrderInvalid,
    CleanupIncomplete,
    EvidenceIncomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopulationClassification {
    Pass,
    Unstable,
    Fail,
    Blocked,
    NotEvaluated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PopulationReport {
    pub classification: PopulationClassification,
    pub attempt_ids: Vec<&'static str>,
}

pub fn validate_attempt(attempt: &AttemptEvidence) -> Result<(), EvidenceError> {
    if attempt.boundary.direct_domain_mutation {
        return Err(EvidenceError::AuthoritativeShortcut);
    }
    if !tier_boundary_satisfied(attempt.cell.tier, attempt.boundary) {
        return Err(EvidenceError::TierBoundaryNotSatisfied);
    }
    validate_identity(attempt)?;
    validate_phases(attempt)?;
    if attempt.cleanup != CleanupStatus::Complete {
        return Err(EvidenceError::CleanupIncomplete);
    }
    Ok(())
}

fn tier_boundary_satisfied(tier: ExecutionTier, boundary: BoundaryEvidence) -> bool {
    let system = boundary.production_process
        && boundary.production_transport
        && boundary.platform_admission
        && boundary.persistence_boundary
        && boundary.normal_networking
        && boundary.server_legality_checks;
    match tier {
        ExecutionTier::Tier1HeadlessSystem => system,
        ExecutionTier::Tier2NativeClient | ExecutionTier::Tier3ProductionSmoke => {
            system && boundary.native_client
        }
    }
}
fn validate_identity(attempt: &AttemptEvidence) -> Result<(), EvidenceError> {
    let cell = &attempt.cell;
    let required = [
        attempt.attempt_id,
        cell.scenario,
        cell.client_artifact,
        cell.server_artifact,
        cell.platform_revision,
        cell.world_bundle,
        cell.migration_revision,
        cell.database_image,
        cell.operating_system,
        cell.target_triple,
        cell.clock_mode,
        cell.fault_profile,
        cell.topology,
    ];
    if required.iter().any(|value| value.is_empty())
        || cell.protocol_revision == 0
        || cell.ruleset_revision == 0
        || cell.content_revision == 0
        || attempt.artifacts.is_empty()
        || attempt
            .artifacts
            .iter()
            .any(|artifact| artifact.name.is_empty() || artifact.digest.is_empty())
    {
        return Err(EvidenceError::EvidenceIncomplete);
    }
    Ok(())
}

fn validate_phases(attempt: &AttemptEvidence) -> Result<(), EvidenceError> {
    if attempt.phases.is_empty() {
        return Err(EvidenceError::EvidenceIncomplete);
    }
    let mut previous = None;
    for phase in &attempt.phases {
        if matches!(phase.status, PhaseStatus::NotApplicable(reason) if reason.is_empty()) {
            return Err(EvidenceError::EvidenceIncomplete);
        }
        if previous.is_some_and(|earlier| phase.phase <= earlier) {
            return Err(EvidenceError::PhaseOrderInvalid);
        }
        previous = Some(phase.phase);
    }

    let earliest_failure = attempt
        .phases
        .iter()
        .find(|phase| phase.status == PhaseStatus::Failed)
        .map(|phase| phase.phase);
    if earliest_failure != attempt.first_divergence {
        return Err(EvidenceError::FirstDivergenceMismatch);
    }
    if attempt.outcome == AttemptOutcome::Passed && earliest_failure.is_some() {
        return Err(EvidenceError::FirstDivergenceMismatch);
    }
    Ok(())
}

pub fn classify_population(attempts: &[AttemptEvidence], minimum: usize) -> PopulationReport {
    let attempt_ids = attempts.iter().map(|attempt| attempt.attempt_id).collect();
    if attempts.len() < minimum || minimum == 0 {
        return PopulationReport {
            classification: PopulationClassification::NotEvaluated,
            attempt_ids,
        };
    }

    let Some(first) = attempts.first() else {
        return PopulationReport {
            classification: PopulationClassification::NotEvaluated,
            attempt_ids,
        };
    };
    if attempts.iter().any(|attempt| attempt.cell != first.cell)
        || attempts
            .iter()
            .any(|attempt| validate_attempt(attempt).is_err())
    {
        return PopulationReport {
            classification: PopulationClassification::Blocked,
            attempt_ids,
        };
    }

    let passed = attempts
        .iter()
        .filter(|attempt| attempt.outcome == AttemptOutcome::Passed)
        .count();
    let classification = if passed == attempts.len() {
        PopulationClassification::Pass
    } else if passed == 0 {
        PopulationClassification::Fail
    } else {
        PopulationClassification::Unstable
    };
    PopulationReport {
        classification,
        attempt_ids,
    }
}

use super::{AiError, AiProvenance, ResourceLimit};

const BOOTSTRAP_PATH_REQUESTS_PER_ACTOR: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathRequest {
    provenance: AiProvenance,
    work_id: u64,
    requests_for_actor: usize,
    search_work: usize,
}

impl PathRequest {
    #[must_use]
    pub const fn new(
        provenance: AiProvenance,
        work_id: u64,
        requests_for_actor: usize,
        search_work: usize,
    ) -> Self {
        Self {
            provenance,
            work_id,
            requests_for_actor,
            search_work,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteStep {
    node: u64,
    encoded_bytes: usize,
}

impl RouteStep {
    #[must_use]
    pub const fn new(node: u64, encoded_bytes: usize) -> Self {
        Self {
            node,
            encoded_bytes,
        }
    }

    #[must_use]
    pub const fn node(&self) -> u64 {
        self.node
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathProposal {
    provenance: AiProvenance,
    work_id: u64,
    steps: Vec<RouteStep>,
    route_bytes: usize,
}

impl PathProposal {
    #[must_use]
    pub fn steps(&self) -> &[RouteStep] {
        &self.steps
    }

    #[must_use]
    pub const fn route_bytes(&self) -> usize {
        self.route_bytes
    }

    pub fn revalidate(&self, current: AiProvenance) -> Result<(), AiError> {
        if self.provenance != current {
            return Err(AiError::StaleProvenance);
        }
        Ok(())
    }

    #[must_use]
    pub const fn work_id(&self) -> u64 {
        self.work_id
    }
}

/// Produce only bounded immutable route evidence. Route adoption remains an owning integration
/// responsibility and is intentionally absent from this bootstrap module.
pub fn build_path_proposal(
    request: PathRequest,
    route: &[RouteStep],
) -> Result<PathProposal, AiError> {
    ResourceLimit::PathRequestsPerActor.admit(0, request.requests_for_actor)?;
    if request.requests_for_actor > BOOTSTRAP_PATH_REQUESTS_PER_ACTOR {
        return Err(AiError::BootstrapPathRequestLimit);
    }
    ResourceLimit::PathSearchWork.admit(0, request.search_work)?;
    ResourceLimit::RouteSteps.admit(0, route.len())?;

    let route_bytes = route.iter().try_fold(0_usize, |total, step| {
        ResourceLimit::RouteBytes.admit(total, step.encoded_bytes)
    })?;

    Ok(PathProposal {
        provenance: request.provenance,
        work_id: request.work_id,
        steps: route.to_vec(),
        route_bytes,
    })
}

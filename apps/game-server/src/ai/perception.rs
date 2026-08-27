use super::{AiError, ResourceLimit};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CandidateId(u64);

impl CandidateId {
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    id: CandidateId,
    priority: u64,
}

impl Candidate {
    #[must_use]
    pub const fn new(id: CandidateId, priority: u64) -> Self {
        Self { id, priority }
    }

    #[must_use]
    pub const fn id(&self) -> CandidateId {
        self.id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Perception {
    candidates: Vec<Candidate>,
}

impl Perception {
    #[must_use]
    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }
}

/// Canonicalize an immutable fixture before any order-sensitive decision evaluation.
pub fn canonicalize_perception(input: &[Candidate]) -> Result<Perception, AiError> {
    ResourceLimit::PerceptionCandidates.admit(0, input.len())?;
    let mut candidates = input.to_vec();
    candidates.sort_unstable_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.id.cmp(&right.id))
    });
    if candidates
        .windows(2)
        .any(|window| window[0].id == window[1].id)
    {
        return Err(AiError::InvalidInput);
    }
    Ok(Perception { candidates })
}

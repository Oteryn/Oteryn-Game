use super::{AiError, AiProvenance, AiSnapshot, CandidateId, Perception, ResourceLimit};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    Idle,
    AcquireCandidate(CandidateId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecisionUnit {
    ordinal: u8,
    kind: DecisionUnitKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DecisionUnitKind {
    Acquire,
    Idle,
}

impl DecisionUnit {
    #[must_use]
    pub const fn acquire(ordinal: u8) -> Self {
        Self {
            ordinal,
            kind: DecisionUnitKind::Acquire,
        }
    }

    #[must_use]
    pub const fn idle(ordinal: u8) -> Self {
        Self {
            ordinal,
            kind: DecisionUnitKind::Idle,
        }
    }
}

pub fn resolve(
    snapshot: &AiSnapshot,
    perception: &Perception,
    current: AiProvenance,
    authored_units: &[DecisionUnit],
    evaluation_work: usize,
) -> Result<Decision, AiError> {
    snapshot.require_current(current)?;
    ResourceLimit::AuthoredUnits.admit(0, authored_units.len())?;
    ResourceLimit::EvaluationWork.admit(0, evaluation_work)?;
    if evaluation_work < authored_units.len() {
        return Err(AiError::EvaluationExhausted);
    }

    let mut units = authored_units.to_vec();
    units.sort_unstable_by_key(|unit| unit.ordinal);
    if units
        .windows(2)
        .any(|window| window[0].ordinal == window[1].ordinal)
    {
        return Err(AiError::InvalidInput);
    }

    for unit in units {
        match unit.kind {
            DecisionUnitKind::Acquire => {
                if let Some(candidate) = perception.candidates().first() {
                    return Ok(Decision::AcquireCandidate(candidate.id()));
                }
            }
            DecisionUnitKind::Idle => return Ok(Decision::Idle),
        }
    }
    Ok(Decision::Idle)
}

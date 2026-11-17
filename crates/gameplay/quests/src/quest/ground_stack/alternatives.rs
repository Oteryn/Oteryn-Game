//! Coherent alternatives over complete dense Ground patterns, never per-cell wildcards.
use super::{GroundStackError, GroundStackRequirement, GroundStackSnapshot, GroundStackWitness};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternVariant {
    Primary,
    Alternative,
}

/// Read evidence only. The Ground owner retains custody and transaction authority.
#[derive(Debug, PartialEq, Eq)]
pub struct PatternWitness<'a> {
    pub variant: PatternVariant,
    pub cells: Vec<GroundStackWitness<'a>>,
}

/// Match one complete configuration, preserving which coherent branch matched.
/// Both plans must be distinct dense patterns over exactly the same cells.
/// This returns observations, never authority to move Items or grant progress.
pub fn match_patterns<'a>(
    primary: &[GroundStackRequirement],
    alternative: &[GroundStackRequirement],
    observed: &'a [GroundStackSnapshot],
) -> Result<Option<PatternWitness<'a>>, GroundStackError> {
    for plan in [primary, alternative] {
        if !super::dense_pattern::admitted(plan, &[]) {
            return Err(GroundStackError::LimitExceeded);
        }
        for cell in plan {
            cell.validate()?;
        }
    }
    let mut distinct = false;
    for cell in primary {
        let Some(other) = alternative
            .iter()
            .find(|other| other.position == cell.position)
        else {
            return Err(GroundStackError::InvalidRequirement);
        };
        distinct |= cell.items != other.items;
    }
    if !distinct {
        return Err(GroundStackError::InvalidRequirement);
    }
    for (variant, plan) in [
        (PatternVariant::Primary, primary),
        (PatternVariant::Alternative, alternative),
    ] {
        if let Some(cells) = super::match_readset(plan, observed)? {
            return Ok(Some(PatternWitness { variant, cells }));
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests;

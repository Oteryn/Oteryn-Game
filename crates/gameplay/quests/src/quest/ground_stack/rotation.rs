//! Finite cyclic Ground move plans. The caller must revalidate and commit through
//! the Ground owner; a plan is neither custody authority nor a mutation receipt.
use super::{GroundStackEntry, GroundStackError, GroundStackRequirement, GroundStackSnapshot};
use crate::quest::world_callbacks::Position;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShiftAxis {
    Row,
    Column,
}

#[derive(Debug, PartialEq, Eq)]
pub struct GroundShift<'a> {
    pub source: Position,
    pub destination: Position,
    pub item: &'a GroundStackEntry,
}

/// Source append ordinals remain observations. Destination ordinals belong to
/// the durable Ground owner and are not invented by this planner.
#[derive(Debug, PartialEq, Eq)]
pub struct RotationPlan<'a> {
    pub moves: Vec<GroundShift<'a>>,
}

/// Plan the Source six-cell right/down shift, wrapping the last cell to the first.
/// The control is exactly two cells before the line. Moves are listed last-first;
/// the writer must lock roots in identity order and commit the entire plan atomically.
pub fn plan_rotation<'a>(
    axis: ShiftAxis,
    control: Position,
    required: &[GroundStackRequirement],
    observed: &'a [GroundStackSnapshot],
) -> Result<Option<RotationPlan<'a>>, GroundStackError> {
    const WIDTH: usize = 6;
    if required.len() != WIDTH || observed.len() > WIDTH {
        return Err(GroundStackError::LimitExceeded);
    }
    if required
        .iter()
        .any(|r| r.items.len() != 1 || r.items[0].quantity != 1)
    {
        return Err(GroundStackError::InvalidRequirement);
    }
    let mut positions = Vec::with_capacity(WIDTH);
    for offset in 2_u16..8 {
        positions.push(match axis {
            ShiftAxis::Row => Position {
                x: control
                    .x
                    .checked_add(offset)
                    .ok_or(GroundStackError::InvalidRequirement)?,
                ..control
            },
            ShiftAxis::Column => Position {
                y: control
                    .y
                    .checked_add(offset)
                    .ok_or(GroundStackError::InvalidRequirement)?,
                ..control
            },
        });
    }
    if positions
        .iter()
        .any(|p| !required.iter().any(|r| r.position == *p))
    {
        return Err(GroundStackError::InvalidRequirement);
    }
    let Some(cells) = super::match_readset(required, observed)? else {
        return Ok(None);
    };
    let mut moves = Vec::with_capacity(WIDTH);
    for (index, source) in positions.iter().enumerate().rev() {
        let cell = cells
            .iter()
            .find(|cell| cell.position == *source)
            .ok_or(GroundStackError::InvalidObservation)?;
        moves.push(GroundShift {
            source: *source,
            destination: positions[(index + 1) % WIDTH],
            item: cell
                .entries
                .first()
                .ok_or(GroundStackError::InvalidObservation)?,
        });
    }
    Ok(Some(RotationPlan { moves }))
}

#[cfg(test)]
mod tests;

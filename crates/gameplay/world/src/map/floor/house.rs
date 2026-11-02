//! The same bundle floor algorithm for a separately authenticated House owner.
//! This returns facts only; it cannot admit a Channel actor or commit movement.
use super::*;

pub fn resolve_house_step(
    cells: &NativeEntryMovementCells,
    origin: MovementLocalPosition,
    direction: CardinalStep,
    pz_blocked: bool,
    blocked: &dyn CellBlocker,
) -> Result<(MovementLocalPosition, bool), MovementError> {
    let table = table_of(cells).ok_or(MovementError::NotQualified)?;
    // The shared resolver performs cardinal arithmetic. Reject an invalid
    // authored origin before it can overflow, even on an untrusted fixture.
    if !(0..=u16::MAX as i32).contains(&origin.x)
        || !(0..=u16::MAX as i32).contains(&origin.y)
        || !(0..=15).contains(&origin.floor)
    {
        return Err(MovementError::CoordinateOverflow);
    }
    destination(cells, table, origin, direction, pz_blocked, blocked)
}

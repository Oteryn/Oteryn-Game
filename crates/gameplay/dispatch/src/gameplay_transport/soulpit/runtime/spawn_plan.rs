//! Crystal Zone:randomPosition selects uniformly from qualified tiles, not its bounding box.
use super::*;
use crate::content::{CollisionClass, LogicalCell, NativeEntryMovementCells};

pub(in crate::gameplay_transport) fn source_spawn_tile(
    cells: &NativeEntryMovementCells,
    blocked: &[LogicalCell],
    at: MovementLocalPosition,
) -> bool {
    let cell = LogicalCell {
        x: at.x,
        y: at.y,
        z: i32::from(at.floor),
    };
    cells.index().lookup(cells.scope(), cell) == Ok(CollisionClass::Walkable)
        && !blocked.contains(&cell)
        && cells
            .spell_tiles()
            .lookup(cells.scope(), cell)
            .is_ok_and(|tile| !tile.flags().block_projectile)
}

pub(in crate::gameplay_transport) fn source_wave_positions(
    root: [u8; 32],
    binding: SoulpitActivityBinding,
    wave: u8,
    mut qualifies: impl FnMut(MovementLocalPosition) -> bool,
) -> Result<[MovementLocalPosition; MONSTERS_PER_WAVE], SoulpitRuntimeError> {
    // zones.lua:12-36 / functions/tile.lua:39-83: ground and solid/projectile
    // properties decide the list; creatures are deliberately ignored here.
    let mut candidates = Vec::new();
    for x in 32362..=32390 {
        for y in 31132..=31153 {
            let at = MovementLocalPosition { x, y, floor: 8 };
            if qualifies(at) {
                candidates.push(at);
            }
        }
    }
    let id = occurrence(binding, b"wave", u64::from(wave));
    let stacks = wave_stacks(wave).map_err(|_| SoulpitRuntimeError::InvalidState)?;
    let mut positions = [MovementLocalPosition {
        x: 32362,
        y: 31132,
        floor: 8,
    }; MONSTERS_PER_WAVE];
    for (ordinal, stack) in stacks.into_iter().enumerate() {
        if stack == SoulpitStack::Boss {
            positions[ordinal] = MovementLocalPosition {
                x: 32376,
                y: 31144,
                floor: 8,
            };
        } else if !candidates.is_empty() {
            let index = draw(
                root,
                id,
                "soulpit_valid_position",
                ordinal as u64,
                0,
                i64::try_from(candidates.len() - 1)
                    .map_err(|_| SoulpitRuntimeError::InvalidState)?,
            )?;
            positions[ordinal] = *candidates
                .get(usize::try_from(index).map_err(|_| SoulpitRuntimeError::InvalidState)?)
                .ok_or(SoulpitRuntimeError::InvalidState)?;
        }
    }
    // An empty qualified set leaves every attempted cell rejected by the same
    // scene predicate. Its empty physical receipt is still retained for replay.
    Ok(positions)
}

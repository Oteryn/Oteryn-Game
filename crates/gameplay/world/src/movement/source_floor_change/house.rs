//! Native source floor rules over current House item collision and height facts.
//! No actor, session or scope grant is constructed by this pure map projection.
use super::*;

pub fn resolve_house_step(
    cells: &NativeEntryMovementCells,
    origin: MovementLocalPosition,
    direction: CardinalStep,
    current_items: impl Fn(MovementLocalPosition) -> Option<(bool, u16)>,
) -> Result<MovementLocalPosition, MovementError> {
    if !is_source_profile(cells) {
        return Err(MovementError::NotQualified);
    }
    destination_with_lookup(origin, direction, |position| {
        let (blocked, extra_height) = current_items(position).ok_or(MovementError::NotQualified)?;
        tile(cells, position)?
            .map(|tile| {
                let height_count = tile
                    .source_step()
                    .ok_or(MovementError::NotQualified)?
                    .height_count
                    .checked_add(extra_height)
                    .ok_or(MovementError::CapacityExceeded)?;
                Ok(TileView {
                    tile,
                    solid: tile.flags().block_solid || blocked,
                    height_count,
                })
            })
            .transpose()
    })
}

use super::super::{ComposedFreshAdmission, format_connection_trace, operator_event, pvp_live};
use crate::content::{LogicalCell, SpellTileLookupError};
use crate::foundation::MovementPositionSnapshot;
use crate::movement::{CardinalStep, MovementError};

pub(super) fn log(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    pvp: &pvp_live::PvpBlocker<'_>,
    origin: Option<MovementPositionSnapshot>,
    direction: CardinalStep,
    error: MovementError,
) {
    match error {
        MovementError::BlockedByCreature(creature) => operator_event(&format!(
            "event=step_blocked cause=creature creature={}",
            format_connection_trace(creature.placement_identity()),
        )),
        MovementError::BlockedByItem(item) => operator_event(&format!(
            "event=step_blocked cause=block_solid item_instance={}",
            format_connection_trace(item),
        )),
        MovementError::Cell(crate::content::static_cell_engine::StaticCellEngineError::Absent) => {
            operator_event("event=step_blocked cause=missing_tile");
        }
        _ => log_entered_tile(owner, pvp, origin, direction),
    }
}

fn log_entered_tile(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    pvp: &pvp_live::PvpBlocker<'_>,
    origin: Option<MovementPositionSnapshot>,
    direction: CardinalStep,
) {
    let (dx, dy) = direction.delta();
    let tile = origin.and_then(|origin| {
        let at = origin.position();
        let cell = LogicalCell {
            x: at.x.checked_add(dx)?,
            y: at.y.checked_add(dy)?,
            z: i32::from(at.floor),
        };
        Some(pvp.cells.spell_tiles().lookup(pvp.cells.scope(), cell))
    });
    match tile {
        Some(Ok(tile)) if pvp.blocked && tile.flags().protection_zone => {
            operator_event("event=step_blocked cause=protection_zone");
        }
        Some(Ok(tile)) if tile.flags().block_solid => {
            let item = owner
                .active_generation
                .and_then(|active| active.native_gameplay())
                .and_then(|native| {
                    tile.ground_source_id()
                        .into_iter()
                        .chain(tile.top_source_ids().iter().copied())
                        .find(|&id| {
                            native.item_policy_for_source_id(id).is_some_and(|policy| {
                                policy.record().attributes.blocks_movement == Some(true)
                            })
                        })
                });
            if let Some(item) = item {
                operator_event(&format!(
                    "event=step_blocked cause=block_solid item_id={item}"
                ));
            } else {
                operator_event("event=step_blocked cause=solid_tile");
            }
        }
        Some(Err(SpellTileLookupError::Absent)) => {
            operator_event("event=step_blocked cause=missing_tile");
        }
        _ => operator_event("event=step_blocked cause=unclassified_collision"),
    }
}

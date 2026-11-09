//! An explicit contact changes NPC anchor admission, never the movement-cell facts.
use super::{CellFacts, HoldReason, npc_contact_cell_reason};

pub(super) fn reason(
    mut cell: CellFacts,
    solid_contact: bool,
    non_walkable_contact: bool,
) -> Option<HoldReason> {
    if non_walkable_contact && cell.ground == Some(false) {
        cell.ground = Some(true);
    }
    npc_contact_cell_reason(cell, solid_contact)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m38_non_walkable_contact_requires_exact_opt_in_and_keeps_movement_closed() {
        let cell = CellFacts {
            tile: true,
            ground: Some(false),
            ..CellFacts::default()
        };
        assert_eq!(reason(cell, false, false), Some(HoldReason::NotWalkable));
        assert_eq!(reason(cell, false, true), None);
        assert_eq!(cell.ground, Some(false));
    }

    #[test]
    fn m38_non_walkable_contact_preserves_every_other_terrain_hold() {
        let base = CellFacts {
            tile: true,
            ground: Some(false),
            ..CellFacts::default()
        };
        for (cell, held) in [
            (
                CellFacts {
                    tile: false,
                    ..base
                },
                HoldReason::NoTile,
            ),
            (
                CellFacts {
                    ground: None,
                    ..base
                },
                HoldReason::NoGround,
            ),
            (
                CellFacts {
                    unclassified: true,
                    ..base
                },
                HoldReason::UnclassifiedTerrain,
            ),
            (
                CellFacts {
                    floor_change: true,
                    ..base
                },
                HoldReason::FloorChange,
            ),
            (
                CellFacts {
                    teleport: true,
                    ..base
                },
                HoldReason::Teleport,
            ),
            (CellFacts { wall: true, ..base }, HoldReason::Wall),
            (
                CellFacts {
                    block_solid: true,
                    ..base
                },
                HoldReason::BlockSolid,
            ),
        ] {
            assert_eq!(reason(cell, false, true), Some(held));
        }
    }
}

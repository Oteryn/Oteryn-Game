use super::{CellFacts, HoldReason, npc_contact_cell_reason};

fn solid_ground() -> CellFacts {
    CellFacts {
        tile: true,
        ground: Some(true),
        block_solid: true,
        ..CellFacts::default()
    }
}

#[test]
fn solid_contact_permission_changes_only_the_npc_admission() {
    let cell = solid_ground();
    assert_eq!(
        npc_contact_cell_reason(cell, false),
        Some(HoldReason::BlockSolid)
    );
    assert_eq!(npc_contact_cell_reason(cell, true), None);
    assert!(cell.block_solid, "the collision facts remain solid");
}

#[test]
fn explicit_solid_contacts_retain_every_other_terrain_hold() {
    let base = solid_ground();
    let cases = [
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
                ground: Some(false),
                ..base
            },
            HoldReason::NotWalkable,
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
    ];
    for (cell, expected) in cases {
        assert_eq!(npc_contact_cell_reason(cell, true), Some(expected));
        assert_eq!(npc_contact_cell_reason(cell, false), Some(expected));
    }
}

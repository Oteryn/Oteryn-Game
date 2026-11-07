use super::*;

#[test]
fn an_item_is_lifted_by_its_own_height_and_the_heights_below_it() {
    // Ground, a height-8 table, an item on it, another height-8 item, a top item of height 8.
    let (lifts, elevation) = stack_lifts([(0, 0), (3, 8), (3, 0), (3, 8), (4, 8)]);
    assert_eq!(lifts, [0, 8, 8, 16, 8]);
    assert_eq!(elevation, 16);
    assert_eq!(item_height(8), 8);
    assert_eq!(item_height(u32::MAX), MAX_ELEVATION);
    assert_eq!(item_height(i32::MAX as u32 + 1), MAX_ELEVATION);
    let (lifts, elevation) = stack_lifts([(3, 16), (3, 16)]);
    assert_eq!(lifts, [16, MAX_ELEVATION]);
    assert_eq!(elevation, MAX_ELEVATION);
}

#[test]
fn a_cell_beyond_the_scan_margin_is_not_drawn() {
    assert!(within_reach([-MAX_REACH_PX, 0]));
    assert!(!within_reach([0, -MAX_REACH_PX - 1]));
}

#[test]
fn tile_projection_keeps_layer_order_and_filters_unassigned_or_unreachable_cells() {
    let cell = |sprite_id, offset_x| DrawCell {
        sprite_id,
        cell_x: 0,
        cell_y: 0,
        offset_x,
        offset_y: 0,
    };
    let items = [
        (0, 0, vec![cell(3, 0)]),
        (3, 8, vec![cell(4, 0), cell(5, -56), cell(6, -57)]),
        (3, u32::MAX, vec![cell(7, 0), cell(0, 0), cell(99, 0)]),
        (4, 8, vec![cell(8, 0)]),
    ];
    let slot = |cell: &DrawCell| (cell.sprite_id != 99).then_some(cell.sprite_id as u16);
    let tile = project_tile(&items, slot);
    assert_eq!(tile.elevation, MAX_ELEVATION);
    assert_eq!(
        tile.under,
        [
            Draw {
                cell: 3,
                offset: [0, 0]
            },
            Draw {
                cell: 4,
                offset: [-8, -8]
            },
            Draw {
                cell: 5,
                offset: [-64, -8]
            },
            Draw {
                cell: 7,
                offset: [-24, -24]
            },
        ]
    );
    assert_eq!(
        tile.over,
        [Draw {
            cell: 8,
            offset: [-8, -8]
        }]
    );
    assert_eq!(project_cell(&cell(0, 0), 0, slot), None);
    assert_eq!(project_cell(&cell(99, 0), 0, slot), None);
    assert_eq!(project_cell(&cell(3, -65), 0, slot), None);
    assert_eq!(
        project_cell(&cell(3, -64), 0, slot),
        Some(Draw {
            cell: 3,
            offset: [-64, 0]
        })
    );
}

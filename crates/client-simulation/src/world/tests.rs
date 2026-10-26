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

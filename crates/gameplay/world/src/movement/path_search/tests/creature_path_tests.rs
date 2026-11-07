use super::*;

#[test]
fn walk_back_reaches_the_exact_cell_at_fifty_and_retains_both_bounds() {
    let (route, work) = measured_route(at(0, 0), at(50, 0), true, open);
    let route = route.unwrap();
    assert_eq!(walk(at(0, 0), &route), at(50, 0));
    assert_eq!(route.len(), 50);
    assert_eq!(work, 50);
    assert_eq!(measured_route(at(0, 0), at(51, 0), true, open).0, None);
    let (_, work) = measured_route(at(0, 0), at(50, 50), true, |cell| {
        (cell != at(50, 50)).then_some(0)
    });
    assert_eq!(work, PATH_SEARCH_WORK);
}

#[test]
fn a_harmful_field_stub_costs_180_beyond_occupancy_and_changes_the_detour() {
    // The upper lane has four occupied cells; the lower one has a single field. With no
    // field owner the lower lane wins; +180 makes the upper lane cheaper even with +40 each.
    let search = |field| {
        measured_route(at(0, 0), at(6, 0), true, |cell| {
            if !(0..=6).contains(&cell.x)
                || !(-1..=1).contains(&cell.y)
                || (cell.y == 0 && (1..=5).contains(&cell.x))
            {
                return None;
            }
            Some(if cell.y == -1 && (1..=4).contains(&cell.x) {
                CREATURE_COST
            } else if cell == at(3, 1) {
                field
            } else {
                0
            })
        })
        .0
        .unwrap()
    };
    let without = search(0);
    let with = search(180);
    assert_eq!(without.first(), Some(&CardinalStep::South));
    assert_eq!(with.first(), Some(&CardinalStep::North));
    assert_eq!(walk(at(0, 0), &with), at(6, 0));
}

#[test]
fn measured_and_existing_chase_share_routes_and_actual_expansion_refunds() {
    let expected = chase(at(0, 0), at(6, 0), open);
    let (route, work) = measured_route(at(0, 0), at(6, 0), false, open);
    assert_eq!(route, expected);
    assert_eq!(work, 5);
    assert_eq!(
        measured_route(at(0, 0), at(1, 0), false, open),
        (Some(vec![]), 0)
    );
}

#[test]
fn walk_back_detours_cardinally_with_the_same_reference_ties() {
    let wall =
        |cell: MovementLocalPosition| (!(cell.x == 2 && (0..=2).contains(&cell.y))).then_some(0);
    let (route, work) = measured_route(at(0, 0), at(5, 2), true, wall);
    // After crossing the wall, equal estimates prefer North before East.
    let expected = vec![
        CardinalStep::East,
        CardinalStep::South,
        CardinalStep::South,
        CardinalStep::South,
        CardinalStep::East,
        CardinalStep::East,
        CardinalStep::North,
        CardinalStep::East,
        CardinalStep::East,
    ];
    assert_eq!(route.as_ref(), Some(&expected));
    assert_eq!(walk(at(0, 0), &expected), at(5, 2));
    assert!(work <= PATH_SEARCH_WORK);
}

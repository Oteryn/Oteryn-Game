use super::*;
use std::collections::BTreeMap;

#[test]
fn repeated_labels_read_each_tile_once_and_the_next_search_reads_fresh_facts() {
    let target = at(6, 0);
    let mut reads = BTreeMap::new();
    let (blocked, work) = measured_route(at(0, 0), target, true, |cell| {
        *reads.entry((cell.x, cell.y)).or_insert(0) += 1;
        (cell != target).then_some(0)
    });
    assert_eq!(blocked, None);
    assert_eq!(work, PATH_SEARCH_WORK);
    assert!(reads.values().all(|count| *count == 1));
    assert_eq!(reads.get(&(target.x, target.y)), Some(&1));
    let (route, work) = measured_route(at(0, 0), target, true, open);
    let route = route.unwrap();
    assert_eq!(route, vec![CardinalStep::East; 6]);
    assert_eq!(work, 6);
    assert_eq!(walk(at(0, 0), &route), target);
}

#[test]
fn compact_labels_keep_cardinal_ties_and_checked_coordinates_at_plane_edges() {
    for origin in [i32::MIN, i32::MAX - 4] {
        let start = at(origin, origin);
        let target = at(origin + 4, origin + 4);
        let (route, work) = measured_route(start, target, false, open);
        let route = route.unwrap();
        assert_eq!(
            route,
            [CardinalStep::East; 3]
                .into_iter()
                .chain([CardinalStep::South; 3])
                .collect::<Vec<_>>()
        );
        assert_eq!(work, 6);
        assert_eq!(chebyshev(walk(start, &route), target), 1);
    }
}

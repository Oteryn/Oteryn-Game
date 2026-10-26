use super::*;

#[test]
fn scene_projection_preserves_painter_order_offsets_and_wrapping_anchor() -> Result<(), &'static str>
{
    let draw = |cell| Draw {
        cell,
        offset: [2, 3],
    };
    let map = MapTile {
        under: vec![draw(3), draw(4)],
        elevation: 8,
        over: vec![draw(5)],
    };
    let player = [draw(6), draw(7)];
    let mut draws = Vec::new();
    // The anchor wraps the session origin past MAX onto map (MIN, MIN).
    visit_scene(
        (i32::MAX, i32::MAX),
        Some((1, 1)),
        ((i32::MAX, i32::MAX), &player),
        |x, y| ((x, y) == (i32::MIN, i32::MIN)).then_some(&map),
        |_, _| Some(draw(8)),
        |tile, draw| {
            draws.push((tile, draw));
            Ok(())
        },
    )?;
    assert_eq!(
        draws.iter().map(|(_, draw)| draw.cell).collect::<Vec<_>>(),
        [3, 4, 8, 6, 7, 5]
    );
    assert!(draws.iter().all(|(tile, _)| *tile == (i32::MAX, i32::MAX)));
    assert_eq!(
        draws
            .iter()
            .map(|(_, draw)| draw.offset)
            .collect::<Vec<_>>(),
        [[2, 3], [2, 3], [-6, -5], [-6, -5], [-6, -5], [2, 3]]
    );
    assert_eq!(scene_origin((i32::MIN, i32::MIN)), (i32::MIN, i32::MIN));
    assert_eq!(scene_origin((0, 0)), (-7, -5));
    Ok(())
}

#[test]
fn scene_projection_without_a_floor_skips_map_lookup_and_propagates_batch_errors() {
    let lookups = std::cell::Cell::new(0);
    let player = [Draw {
        cell: 2,
        offset: [0, 0],
    }];
    let result = visit_scene(
        (0, 0),
        None,
        ((0, 0), &player),
        |_, _| {
            lookups.set(lookups.get() + 1);
            None
        },
        |_, _| None,
        |_, draw| {
            assert_eq!(draw, player[0]);
            Err("batch full")
        },
    );
    assert_eq!(result, Err("batch full"));
    assert_eq!(lookups.get(), 0);
}

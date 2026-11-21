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
        [8, 8, 8, 3, 4, 8, 6, 7, 5]
    );
    let last = (i32::MAX, i32::MAX);
    assert_eq!(
        draws.iter().map(|(tile, _)| *tile).collect::<Vec<_>>(),
        [
            (i32::MAX - 1, i32::MAX - 1),
            (i32::MAX - 1, i32::MAX),
            (i32::MAX, i32::MAX - 1),
            last,
            last,
            last,
            last,
            last,
            last,
        ]
    );
    assert_eq!(
        draws
            .iter()
            .map(|(_, draw)| draw.offset)
            .collect::<Vec<_>>(),
        [
            [2, 3],
            [2, 3],
            [2, 3],
            [2, 3],
            [2, 3],
            [-6, -5],
            [-6, -5],
            [-6, -5],
            [2, 3]
        ]
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

#[test]
fn scene_visitors_traverse_diagonals_from_south_to_east() -> Result<(), &'static str> {
    let draw = Draw {
        cell: 1,
        offset: [0, 0],
    };
    let mut single = Vec::new();
    visit_scene(
        (0, 0),
        None,
        ((-1, -1), &[]),
        |_, _| None,
        |_, _| Some(draw),
        |tile, _| {
            single.push(tile);
            Ok(())
        },
    )?;
    let mut entities = Vec::new();
    visit_scene_entities(
        (0, 0),
        None,
        ((-1, -1), &[]),
        |_, _| None,
        |_, _| vec![draw],
        |tile, _| {
            entities.push(tile);
            Ok(())
        },
    )?;
    let mut expected = Vec::new();
    // The walking camera scans a guard tile on every edge, in the same painter order.
    for diagonal in -2..SCENE_ROWS as i32 + SCENE_COLUMNS as i32 + 2 * SCAN_MARGIN + 1 {
        for x in -1..SCENE_COLUMNS as i32 + SCAN_MARGIN + 1 {
            let y = diagonal - x;
            if (-1..SCENE_ROWS as i32 + SCAN_MARGIN + 1).contains(&y) {
                expected.push((x, y));
            }
        }
    }
    assert_eq!(
        &expected[..6],
        &[(-1, -1), (-1, 0), (0, -1), (-1, 1), (0, 0), (1, -1)]
    );
    assert_eq!(single, expected);
    assert_eq!(entities, expected);
    Ok(())
}

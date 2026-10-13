use super::*;
use oteryn_client_assets::DrawCell;
use oteryn_session::map_view::{
    MapDefinition, MapItem, MapOrigin, MapViewHeader, WorldMapViewSnapshot,
    encode_world_map_snapshot,
};
use std::collections::BTreeMap;
use std::num::NonZeroU32;

type TestResult = Result<(), Box<dyn std::error::Error>>;
fn position(x: i32, y: i32, floor: i16) -> ActorPosition {
    ActorPosition { x, y, floor }
}
fn map(tiles: Vec<MapTile>, visible: i16) -> Result<MapView, String> {
    let mut map = MapView::new([1; 32]).map_err(|e| format!("{e:?}"))?;
    let data = encode_world_map_snapshot(&WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 1,
            origin: position(100, 100, -7),
            first_visible_floor: visible,
        },
        tiles,
    })
    .map_err(|e| format!("{e:?}"))?;
    map.replace_snapshot(&data).map_err(|e| format!("{e:?}"))?;
    Ok(map)
}
fn tile(x: i32, y: i32, floor: i16, count: usize) -> MapTile {
    MapTile {
        position: position(x, y, floor),
        ground_speed: 150,
        more: true,
        items: vec![
            MapItem {
                definition: MapDefinition::Item(NonZeroU32::MIN),
                count: 1,
                sub_type: 0,
                origin: MapOrigin::DisplayOnly,
                appearance_id: 0
            };
            count
        ],
    }
}
fn view() -> Result<TileView, BatchError> {
    TileView::new(TileCoord::new(93, 95), 32, 15, 11)
}
fn cells(id: u32) -> Vec<PlacedCell> {
    vec![PlacedCell {
        key: CellKey {
            sprite_id: id,
            cell_x: 0,
            cell_y: 0,
        },
        offset_x: 0,
        offset_y: 0,
    }]
}
fn resolved(id: u32, phase: u8) -> Option<ResolvedEntry> {
    Some(ResolvedEntry {
        cells: vec![DrawCell {
            sprite_id: id,
            cell_x: 0,
            cell_y: 0,
            offset_x: 0,
            offset_y: 0,
        }],
        elevation: 0,
        ground: phase == 0,
        ground_border: phase == 1,
        on_bottom: phase == 2,
        on_top: phase == 4,
    })
}
fn actor(x: i32, y: i32, floor: i16, id: u32) -> MapActor {
    MapActor {
        position: position(x, y, floor),
        previous: None,
        offset: [0, 0],
        cells: cells(id),
    }
}

#[test]
fn phase_merge_places_actor_after_common_and_before_arch_in_received_order() -> TestResult {
    let map = map(vec![tile(100, 100, -7, 5)], -7)?;
    let mut page = AtlasPage::new();
    let frame = compose(&map, &[actor(100, 100, -7, 6)], &mut page, &view()?, |_| {
        vec![
            resolved(5, 4),
            resolved(2, 1),
            resolved(4, 3),
            resolved(1, 0),
            resolved(3, 2),
        ]
    })?;
    assert_eq!(
        frame
            .uploads()
            .iter()
            .map(|(key, _)| key.sprite_id)
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 6, 5]
    );
    assert_eq!(frame.quads().quad_count(), 6);
    Ok(())
}

#[test]
fn floors_are_projected_and_hidden_actor_and_items_are_not_drawn() -> TestResult {
    for (visible, count) in [(-7, 1), (0, 3)] {
        let map = map(vec![tile(100, 100, -7, 1), tile(101, 101, -6, 1)], visible)?;
        let mut page = AtlasPage::new();
        let frame = compose(
            &map,
            &[actor(101, 101, -6, 3)],
            &mut page,
            &view()?,
            |tile| vec![resolved(if tile.position.floor == -7 { 1 } else { 2 }, 0)],
        )?;
        assert_eq!(frame.quads().quad_count(), count);
        assert!(
            frame
                .quads()
                .batches()
                .iter()
                .flatten()
                .all(|quad| quad.position == [224.0, 160.0])
        );
    }
    Ok(())
}

#[test]
fn missing_assets_and_unknown_appearances_are_visible_not_counted_as_oversized() -> TestResult {
    let missing = map(vec![tile(100, 100, -7, 2)], -7)?;
    let mut page = AtlasPage::new();
    let frame = MapDraw::placeholders().frame(&missing, &[], &mut page, &view()?)?;
    assert_eq!(frame.quads().quad_count(), 2);
    assert_eq!(frame.oversized_entries(), 0);
    assert_eq!(page.placeholder_count(), 2);
    assert!(frame.uploads().is_empty());
    let mut known = tile(100, 100, -7, 3);
    known.items[1].appearance_id = u16::MAX;
    known.items[2].appearance_id = 104;
    let known = map(vec![known], -7)?;
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut loaded = MapDraw::load(&root)?;
    let mut page = AtlasPage::new();
    let frame = loaded.frame(&known, &[], &mut page, &view()?)?;
    // Appearance 104 has sixteen position patterns, not sixteen cells per tile.
    assert_eq!(frame.quads().quad_count(), 3);
    assert_eq!(page.placeholder_count(), 2);
    assert_eq!(frame.uploads().len(), 1);
    for &(key, _) in frame.uploads() {
        assert_eq!(loaded.cell_rgba(key)?.len(), 32 * 32 * 4);
    }
    Ok(())
}

#[test]
fn actor_on_omitted_tile_draws_and_walking_uses_later_source_or_destination_phase() -> TestResult {
    let map = map(vec![tile(100, 99, -7, 1), tile(100, 100, -7, 1)], -7)?;
    let mut north = actor(100, 99, -7, 3);
    north.previous = Some(position(100, 100, -7));
    north.offset = [0, 16];
    let mut page = AtlasPage::new();
    let frame = compose(
        &map,
        &[north, actor(101, 100, -7, 4)],
        &mut page,
        &view()?,
        |tile| vec![resolved(if tile.position.y == 99 { 1 } else { 2 }, 0)],
    )?;
    assert_eq!(
        frame
            .uploads()
            .iter()
            .map(|(key, _)| key.sprite_id)
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    let quads = &frame.quads().batches()[0];
    assert_eq!(quads[2].position, [224.0, 144.0]);
    assert_eq!(quads[3].position, [256.0, 160.0]);
    Ok(())
}

#[test]
fn walking_actor_stays_under_both_tiles_on_top_even_with_a_resident_atlas() -> TestResult {
    for north in [false, true] {
        let earlier = position(100, if north { 99 } else { 100 }, -7);
        let later = position(if north { 100 } else { 101 }, 100, -7);
        let map = map(
            vec![
                tile(earlier.x, earlier.y, -7, 3),
                tile(later.x, later.y, -7, 3),
            ],
            -7,
        )?;
        let mut walker = actor(
            if north { earlier.x } else { later.x },
            if north { earlier.y } else { later.y },
            -7,
            7,
        );
        walker.previous = Some(if north { later } else { earlier });
        walker.offset = if north { [0, 16] } else { [-16, 0] };
        let mut page = AtlasPage::new();
        let mut slots = BTreeMap::new();
        for resident in [false, true] {
            let frame = compose(
                &map,
                std::slice::from_ref(&walker),
                &mut page,
                &view()?,
                |tile| {
                    let base = if tile.position == earlier { 0 } else { 3 };
                    vec![
                        resolved(base + 1, 0),
                        resolved(base + 2, 3),
                        resolved(base + 3, 4),
                    ]
                },
            )?;
            if resident {
                assert!(frame.uploads().is_empty());
            }
            slots.extend(
                frame
                    .uploads()
                    .iter()
                    .map(|(key, slot)| (key.sprite_id, *slot)),
            );
            let quads = frame.quads().batches().iter().flatten().collect::<Vec<_>>();
            assert_eq!(quads.len(), 7);
            for (quad, id) in quads.iter().zip([1, 2, 4, 5, 7, 3, 6]) {
                assert_eq!(
                    quad.uv,
                    AtlasPage::uv_rect(*slots.get(&id).ok_or("missing slot")?)
                );
            }
            assert_eq!(
                quads[4].position,
                if north {
                    [224.0, 144.0]
                } else {
                    [240.0, 160.0]
                }
            );
        }
    }
    Ok(())
}

#[test]
fn neighbor_anchor_keeps_up_left_cells_entering_the_actual_view() -> TestResult {
    let map = map(vec![tile(108, 100, -7, 1)], -7)?;
    let mut page = AtlasPage::new();
    let frame = compose(&map, &[], &mut page, &view()?, |_| {
        let mut entry = resolved(1, 3);
        if let Some(entry) = entry.as_mut() {
            entry.cells[0].offset_x = -32;
        }
        vec![entry]
    })?;
    assert_eq!(frame.quads().batches()[0][0].position[0], 448.0);
    Ok(())
}

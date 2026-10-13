use super::*;
use oteryn_renderer::TileCoord;
use oteryn_session::{
    ActorPosition,
    map_view::{MapViewHeader, WorldMapViewSnapshot, encode_world_map_snapshot},
};

fn map() -> Result<MapView, String> {
    let payload = encode_world_map_snapshot(&WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 1,
            origin: ActorPosition {
                x: 100,
                y: 100,
                floor: -7,
            },
            first_visible_floor: 0,
        },
        tiles: vec![],
    })
    .map_err(|e| format!("{e:?}"))?;
    let mut map = MapView::new([1; 32]).map_err(|e| format!("{e:?}"))?;
    map.replace_snapshot(&payload)
        .map_err(|e| format!("{e:?}"))?;
    Ok(map)
}

#[test]
fn rebuilds_keep_pending_pixels_and_reset_reuploads_resident_cells()
-> Result<(), Box<dyn std::error::Error>> {
    let view = TileView::new(TileCoord::new(93, 95), 48, 15, 11)?;
    let target = Targetable {
        tile: TileCoord::new(100, 100),
        kind: crate::input::TargetKind::Object,
        glyph: false,
        entity: None,
    };
    let mut scene = StreamedScene::compose(
        &map()?,
        vec![],
        Arc::new(Mutex::new(MapDraw::placeholders())),
        None,
        &view,
        Some(target),
    )?;
    assert_eq!(scene.frame().quads().quad_count(), 1);
    assert_eq!(scene.pending.len(), 1);
    scene.rebuild(&view, Some(target))?;
    // The second frame has no new LRU upload, but the first is still unpresented.
    assert!(scene.frame().uploads().is_empty());
    let uploads = scene.take_uploads();
    assert_eq!(uploads.len(), 1);
    assert_eq!(uploads[0].rgba, builtin_cell(TARGET_CELL));
    assert!(scene.take_uploads().is_empty());
    scene.reset_page(&view, Some(target))?;
    assert_eq!(scene.take_uploads().len(), 1);
    assert_eq!(scene.atlas().width(), 2048);
    assert_eq!(scene.atlas().height(), 2080);
    Ok(())
}

#[test]
fn placeholder_row_contains_owned_pixels_and_empty_actor_is_a_real_quad()
-> Result<(), Box<dyn std::error::Error>> {
    let view = TileView::new(TileCoord::new(93, 95), 48, 15, 11)?;
    let scene = StreamedScene::compose(
        &map()?,
        vec![MapActor {
            position: ActorPosition {
                x: 100,
                y: 100,
                floor: -7,
            },
            previous: None,
            offset: [0, 0],
            cells: vec![],
        }],
        Arc::new(Mutex::new(MapDraw::placeholders())),
        None,
        &view,
        None,
    )?;
    assert_eq!(scene.frame().quads().quad_count(), 1);
    assert!(scene.frame().uploads().is_empty());
    let index = (2048 * 2048 * 4 + 15 * 2048 * 4 + 15 * 4) as usize;
    assert_eq!(&scene.atlas().rgba()[index..index + 4], &[220, 40, 40, 255]);
    Ok(())
}

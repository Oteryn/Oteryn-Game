use super::*;
use oteryn_session::EntityRef;

fn scene() -> Result<Scene, BatchError> {
    let world = Arc::new(World::builtin()?);
    let markers = [TileCoord::new(3, -1), TileCoord::new(-2, -2)];
    Scene::centered_on(
        world,
        Some(TileCoord::new(0, 0)),
        TileCoord::new(0, 0),
        StepDir::South,
        &markers,
    )
}

#[test]
fn scene_builds_one_quad_per_tile_and_one_per_sprite() -> Result<(), BatchError> {
    let scene = scene()?;
    assert_eq!(scene.tiles().len(), (SCENE_COLUMNS * SCENE_ROWS) as usize);
    assert_eq!(scene.tiles().vertex_count(), 165 * 6);
    assert_eq!(scene.sprites().len(), 3);
    assert_eq!(scene.sprites().vertex_count(), 18);
    Ok(())
}

#[test]
fn a_visible_entity_is_drawn_and_a_click_selects_it_with_its_reference() -> Result<(), BatchError> {
    let world = Arc::new(World::builtin()?);
    let entity = EntityRef {
        identity: [7; 16],
        generation: 3,
    };
    let on = |x| Targetable {
        tile: TileCoord::new(x, 1),
        kind: TargetKind::Entity,
        glyph: true,
        entity: Some(entity),
    };
    let mut scene = Scene::centered_with(
        world,
        Some(TileCoord::new(0, 0)),
        TileCoord::new(0, 0),
        StepDir::South,
        &[],
        // Two on one tile draw one glyph.
        &[on(2), on(2)],
    )?;
    assert_eq!(scene.sprites().len(), 2);
    assert_eq!(
        scene.select_tile(TileCoord::new(2, 1))?.map(|t| t.entity),
        Some(Some(entity))
    );
    Ok(())
}

#[test]
fn scene_view_fits_the_default_window_and_maps_clicks_to_tiles() -> Result<(), BatchError> {
    let scene = scene()?;
    assert_eq!(scene.view().viewport_px(), (720, 528));
    // The player sprite sits on tile (0, 0), drawn last, and a click on it resolves to it.
    let [x, y] = scene.view().tile_to_screen(TileCoord::new(0, 0));
    assert_eq!(scene.sprites().instances()[2].position, [x, y]);
    assert_eq!(
        scene.view().screen_to_tile(x + 1.0, y + 1.0),
        Some(TileCoord::new(0, 0))
    );
    assert_eq!(scene.view().screen_to_tile(720.0, 0.0), None);
    Ok(())
}

#[test]
fn clicking_an_entity_or_an_object_targets_it_and_draws_a_highlight() -> Result<(), BatchError> {
    let mut scene = scene()?;
    assert_eq!(scene.target(), None);
    for (tile, kind) in [
        (TileCoord::new(0, 0), TargetKind::Entity),
        (TileCoord::new(-2, -2), TargetKind::Object),
    ] {
        // Click position -> tile -> selection -> highlight on the batch.
        let [x, y] = scene.view().tile_to_screen(tile);
        let clicked =
            crate::input::click_tile(scene.view(), f64::from(x) + 5.0, f64::from(y) + 5.0);
        assert_eq!(clicked, Some(tile));
        let picked = scene.select_tile(tile)?;
        assert_eq!(picked.map(|p| p.kind), Some(kind));
        assert_eq!(scene.target(), picked);
        assert_eq!(scene.sprites().len(), 4);
        assert_eq!(scene.sprites().instances()[3].position, [x, y]);
    }
    // An empty tile keeps the target; clearing removes the highlight.
    assert_eq!(scene.select_tile(TileCoord::new(5, 3))?, None);
    assert_eq!(scene.sprites().len(), 4);
    scene.clear_target()?;
    assert_eq!(scene.sprites().len(), 3);
    Ok(())
}

#[test]
fn a_displaced_sprite_two_tiles_past_the_view_is_drawn() -> Result<(), BatchError> {
    use crate::world::MapTile;
    // Column 16 and row 12 of the view, displaced back into its last column and row.
    let reach = |x, y| {
        let tile = MapTile {
            under: vec![Draw {
                cell: MARKER_CELL,
                offset: [-40, -40],
            }],
            ..MapTile::default()
        };
        ((x, y), tile)
    };
    let world = Arc::new(World::builtin_with([reach(9, 0), reach(0, 7)])?);
    let scene = Scene::centered_on(
        world,
        Some(TileCoord::new(0, 0)),
        TileCoord::new(0, 0),
        StepDir::South,
        &[],
    )?;
    assert_eq!(scene.sprites().len(), 3);
    Ok(())
}

#[test]
fn a_view_at_the_edge_of_the_coordinate_space_builds() -> Result<(), BatchError> {
    let world = Arc::new(World::builtin()?);
    let own = TileCoord::new(i32::MAX - 8, i32::MAX - 6);
    let scene = Scene::centered_on(world, Some(TileCoord::new(0, 0)), own, StepDir::South, &[])?;
    assert_eq!(scene.sprites().len(), 1);
    let world = Arc::new(World::builtin()?);
    let scene = Scene::centered_on(
        world,
        Some(TileCoord::new(i32::MAX, i32::MAX)),
        own,
        StepDir::South,
        &[],
    )?;
    assert_eq!(scene.sprites().len(), 1);
    Ok(())
}

#[test]
fn the_start_area_draws_real_sprites_around_the_player() -> Result<(), String> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let world = Arc::new(World::load(&root)?);
    let start = TileCoord::new(crate::world::START.0, crate::world::START.1);
    let scene = Scene::centered_on(
        world,
        Some(TileCoord::new(0, 0)),
        start,
        StepDir::South,
        &[],
    )
    .map_err(|error| error.to_string())?;
    // Far more than a quad per tile, all inside the atlas and the batch cap.
    assert!(scene.sprites().len() > (SCENE_COLUMNS * SCENE_ROWS) as usize);
    for quad in scene.sprites().instances() {
        assert!(quad.uv.iter().all(|uv| (0.0..=1.0).contains(uv)));
    }
    // The same area through an anchored session position.
    let anchored = Scene::centered_on(
        Arc::clone(&scene.world),
        Some(TileCoord::new(start.x - 1, start.y + 1)),
        TileCoord::new(1, -1),
        StepDir::South,
        &[],
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(anchored.sprites().len(), scene.sprites().len());
    // A join position at the edge of the coordinate space still maps exactly onto the start.
    let edge = TileCoord::new(i32::MIN + 20, i32::MAX - 20);
    let anchored = Scene::centered_on(
        Arc::clone(&scene.world),
        Some(TileCoord::new(
            start.x.wrapping_sub(edge.x),
            start.y.wrapping_sub(edge.y),
        )),
        edge,
        StepDir::South,
        &[],
    )
    .map_err(|error| error.to_string())?;
    assert_eq!(anchored.sprites().len(), scene.sprites().len());
    // Off the map's floor only the player is drawn.
    let off_floor = Scene::centered_on(Arc::clone(&scene.world), None, start, StepDir::South, &[])
        .map_err(|error| error.to_string())?;
    assert_eq!(
        off_floor.sprites().len(),
        scene.world.player(StepDir::South).len()
    );
    Ok(())
}

use super::*;

#[test]
fn the_map_is_drawn_only_on_the_join_floor() -> Result<(), String> {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let world = Arc::new(World::load(&root)?);
    let mut view = view().map_err(|e| e.to_string())?;
    view.apply(&moved_to(crate::map::START.0, crate::map::START.1, 7))
        .map_err(|e| e.to_string())?;
    view.set_world(world).map_err(|e| e.to_string())?;
    let on_floor = view.scene().sprites().len();
    view.apply(&moved_to(100, 200, -1))
        .map_err(|e| e.to_string())?;
    let off_floor = view.scene().sprites().len();
    view.apply(&moved_to(crate::map::START.0, crate::map::START.1, 7))
        .map_err(|e| e.to_string())?;
    assert!(off_floor < on_floor / 10, "{off_floor} of {on_floor}");
    assert_eq!(view.scene().sprites().len(), on_floor);
    Ok(())
}

#[test]
fn pushed_domain_1_and_2_deltas_move_the_view() -> Result<(), BatchError> {
    let mut view = view()?;
    let mut pushed = Pushed::default();
    pushed.set_marker(entry(key(101, 200, 0, 0)));
    pushed.own = Some(ActorPosition {
        x: 101,
        y: 200,
        floor: 0,
    });
    view.apply_pushed(&pushed)?;
    assert_eq!(view.own(), TileCoord::new(101, 200));
    assert_eq!(view.marker_count(), 2);
    Ok(())
}

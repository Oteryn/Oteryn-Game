#[test]
fn tutorial_map_window_is_decoded_at_the_real_join_position() -> Result<(), String> {
    let center = (32472, 32288, 7);
    let world = World::load_around(&repo(), center)?;
    assert_eq!(world.center(), center);
    assert!(
        !world
            .tile(center.0, center.1)
            .ok_or("no landing tile")?
            .under
            .is_empty()
    );
    assert!(world.tile(START.0, START.1).is_none());
    assert!(World::load_around(&repo(), (0, 0, -1)).is_err());
    Ok(())
}

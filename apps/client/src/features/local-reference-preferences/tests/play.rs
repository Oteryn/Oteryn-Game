use super::*;

#[test]
fn reference_preferences_are_local_and_reject_unknown_panel_ids()
-> Result<(), Box<dyn std::error::Error>> {
    let mut play = view()?;
    assert_eq!(play.preferences().ui_scale, 100);
    assert!(play.preferences().panel_visible("minimap"));

    let mut changed = play.preferences().clone();
    changed.ui_scale = 125;
    changed.visible_panels.retain(|panel| panel != "minimap");
    play.set_preferences(changed)?;
    assert_eq!(play.preferences().ui_scale, 125);
    assert!(!play.preferences().panel_visible("minimap"));

    let mut invalid = play.preferences().clone();
    invalid.visible_panels.push("unregistered-panel".to_owned());
    assert!(play.set_preferences(invalid).is_err());
    Ok(())
}

#[test]
fn preferences_are_restored_when_scene_rebuild_fails() -> Result<(), BatchError> {
    let mut play = view()?;
    let old = play.preferences().clone();
    let mut changed = old.clone();
    changed.ui_scale = 125;
    play.local.set_world(Arc::new(World::builtin_with([(
        (100, 200),
        crate::map::MapTile {
            under: vec![crate::map::Draw {
                cell: u16::MAX,
                offset: [0, 0],
            }],
            ..crate::map::MapTile::default()
        },
    )])?));
    play.map_floor = Some(play.floor);
    assert!(play.set_preferences(changed).is_err());
    assert_eq!(play.preferences(), &old);
    Ok(())
}

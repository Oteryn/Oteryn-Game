#[test]
fn removing_a_mark_clears_the_cache_and_reopens_an_empty_sidebar_editor() -> TestResult {
    let mut app = fixture()?;
    let at = {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let play = owner.play.as_mut().ok_or("play")?;
        let (x, y, floor, _, _) = play.view.minimap_camera();
        play.view
            .edit_minimap_marker((x, y, floor), "Camp".into(), 3)?;
        (x, y, floor)
    };
    open_marker_editor(&mut app)?;
    activate(&mut app, "Remove mark")?;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .view
        .satellite_map(|_, marks| {
            assert!(!marks.iter().any(|mark| (mark.x, mark.y, mark.floor) == at));
        });
    open_marker_editor(&mut app)?;
    let field = app
        .world_mut()
        .query::<&bevy::text::EditableText>()
        .single(app.world())?;
    assert_eq!(field.value(), "Mark");
    assert!(
        !app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Remove mark")
    );
    Ok(())
}

#[test]
fn removing_a_remembered_mark_updates_scoped_preferences_and_sidebar_quads() -> TestResult {
    use oteryn_session::{
        ActorPosition,
        world_map::{MapViewHeader, WorldMapViewSnapshot},
    };
    let mut app = fixture()?;
    let get_quads = |app: &App| -> Result<usize, Box<dyn std::error::Error>> {
        Ok(app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .view
            .scene()
            .minimap_quad_range()
            .len())
    };
    let unmarked;
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let owner = &mut *owner;
        let play = owner.play.as_mut().ok_or("play")?;
        let (x, y, floor) = play.view.satellite_player();
        play.view.set_live_map(&WorldMapViewSnapshot {
            header: MapViewHeader {
                content_generation: [0; 32],
                bundle_digest: [0; 32],
                reset_epoch: 1,
                origin: ActorPosition { x, y, floor },
                first_visible_floor: floor,
            },
            tiles: vec![],
        })?;
        unmarked = play.view.scene().minimap_quad_range().len();
        let mut preferences = play.view.preferences().clone();
        preferences.minimap.add_mark(crate::play::MinimapMark {
            x,
            y,
            floor,
            label: "Imported camp".into(),
        })?;
        owner.settings.apply(preferences.clone());
        play.view.set_preferences(preferences)?;
    }
    assert_eq!(get_quads(&app)?, unmarked + 1);
    open_marker_editor(&mut app)?;
    activate(&mut app, "Remove mark")?;
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .settings
            .preferences
            .minimap
            .marks
            .is_empty()
    );
    assert_eq!(get_quads(&app)?, unmarked);
    Ok(())
}

fn finish_marker_preferences(app: &mut App) -> TestResult {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    while owner.settings.preference_save_pending() {
        if std::time::Instant::now() >= deadline {
            return Err("marker preferences write did not complete".into());
        }
        owner.poll([320, 240]);
        std::thread::yield_now();
    }
    Ok(())
}

#[test]
fn removing_a_remembered_mark_reports_storage_failure_after_activation() -> TestResult {
    let root = tempfile::tempdir()?;
    let path = root.path().join("preferences.json");
    let mut app = fixture()?;
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.settings = crate::play::SettingsState::new(Some(path.clone()));
        let play = owner.play.as_mut().ok_or("play")?;
        let (x, y, floor) = play.view.satellite_player();
        play.view
            .set_live_map(&oteryn_session::world_map::WorldMapViewSnapshot {
                header: oteryn_session::world_map::MapViewHeader {
                    content_generation: [0; 32],
                    bundle_digest: [0; 32],
                    reset_epoch: 1,
                    origin: oteryn_session::ActorPosition { x, y, floor },
                    first_visible_floor: floor,
                },
                tiles: vec![],
            })?;
        let mut preferences = play.view.preferences().clone();
        preferences.minimap.add_mark(crate::play::MinimapMark {
            x,
            y,
            floor,
            label: "Imported camp".into(),
        })?;
        play.view.set_preferences(preferences.clone())?;
        owner.settings.apply(preferences);
    }
    std::fs::write(&path, "damaged")?;
    open_marker_editor(&mut app)?;
    activate(&mut app, "Remove mark")?;
    let owner = app.world().non_send::<HostOwner>();
    assert!(owner.settings.preferences.minimap.marks.is_empty());
    finish_marker_preferences(&mut app)?;
    let owner = app.world().non_send::<HostOwner>();
    assert!(owner.status.contains("could not save"), "{}", owner.status);
    let feedback = owner.status.clone();
    assert_eq!(std::fs::read(&path)?, b"damaged");

    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    app.add_systems(Update, crate::bevy_shell::ui::project);
    app.update();
    let world = app.world_mut();
    assert!(world.query::<&Text>().iter(world).any(|text| text.0 == feedback));
    Ok(())
}

#[test]
fn renaming_then_removing_an_imported_mark_does_not_reveal_the_original() -> TestResult {
    let root = tempfile::tempdir()?;
    let path = root.path().join("preferences.json");
    let mut app = fixture()?;
    let at = {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.settings = crate::play::SettingsState::new(Some(path.clone()));
        let play = owner.play.as_mut().ok_or("play")?;
        let (x, y, floor) = play.view.satellite_player();
        play.view
            .set_live_map(&oteryn_session::world_map::WorldMapViewSnapshot {
                header: oteryn_session::world_map::MapViewHeader {
                    content_generation: [0; 32],
                    bundle_digest: [0; 32],
                    reset_epoch: 1,
                    origin: oteryn_session::ActorPosition { x, y, floor },
                    first_visible_floor: floor,
                },
                tiles: vec![],
            })?;
        let mut preferences = play.view.preferences().clone();
        preferences.minimap.add_mark(crate::play::MinimapMark {
            x,
            y,
            floor,
            label: "Imported camp".into(),
        })?;
        play.view.set_preferences(preferences.clone())?;
        owner.settings.apply(preferences);
        (x, y, floor)
    };
    open_marker_editor(&mut app)?;
    *app.world_mut()
        .query::<&mut bevy::text::EditableText>()
        .single_mut(app.world_mut())? = bevy::text::EditableText::new("Renamed camp");
    activate(&mut app, "Save mark")?;
    let owner = app.world().non_send::<HostOwner>();
    let play = owner.play.as_ref().ok_or("play")?;
    assert_eq!(
        play.view
            .remembered_minimap_mark(at)
            .ok_or("remembered mark")?
            .label,
        "Renamed camp"
    );
    assert!(
        !play
            .view
            .satellite_map(|_, marks| marks.iter().any(|mark| (mark.x, mark.y, mark.floor) == at))
    );
    open_marker_editor(&mut app)?;
    activate(&mut app, "Remove mark")?;
    open_marker_editor(&mut app)?;
    let field = app
        .world_mut()
        .query::<&bevy::text::EditableText>()
        .single(app.world())?;
    assert_eq!(field.value(), "Mark");
    finish_marker_preferences(&mut app)?;
    assert!(crate::play::PreferenceStore::load(path)?.preferences().minimap.marks.is_empty());
    Ok(())
}

#[test]
fn marker_editor_closes_when_the_map_rebinds_in_the_same_session() -> TestResult {
    let mut app = fixture()?;
    open_marker_editor(&mut app)?;
    let generation = app.world().non_send::<HostOwner>().generation();
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let play = owner.play.as_mut().ok_or("active minimap")?;
        play.view.bind_minimap_world("22222222-2222-4222-8222-222222222222");
    }
    app.update();
    assert_eq!(app.world().non_send::<HostOwner>().generation(), generation);
    assert!(!app.world_mut().query::<&Text>().iter(app.world()).any(|text| text.0 == "Save mark"));
    Ok(())
}

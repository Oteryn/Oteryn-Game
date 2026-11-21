#[test]
fn secondary_minimap_click_edits_the_viewed_coordinate() -> TestResult {
    let mut app = fixture()?;
    activate(&mut app, "E")?;
    activate(&mut app, "Floor up")?;
    open_marker_editor(&mut app)?;
    for mut text in app
        .world_mut()
        .query::<&mut bevy::text::EditableText>()
        .iter_mut(app.world_mut())
    {
        *text = bevy::text::EditableText::new("Cave entrance");
    }
    activate(&mut app, "Save mark")?;
    let owner = app.world().non_send::<HostOwner>();
    let play = owner.play.as_ref().ok_or("play")?;
    assert!(play.view.satellite_map(|_, marks| marks.iter().any(|m| (
        m.x,
        m.y,
        m.floor,
        m.label.as_str()
    ) == (
        105,
        200,
        6,
        "Cave entrance"
    ))));
    assert_eq!(play.view.satellite_player(), (100, 200, 7));
    Ok(())
}

#[test]
fn exclusive_marker_refusals_appear_in_playing_status() -> TestResult {
    use oteryn_input_actions::{
        ButtonState, KeyCode as InputKey, Modifier, Modifiers, NormalizedInputEvent,
    };

    let mut visible = Vec::new();
    for shortcut in [true, false] {
        let mut app = fixture()?;
        let root = std::env::temp_dir().join(format!(
            "minimap-marker-status-{}-{shortcut}",
            std::process::id()
        ));
        let mut writer = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), 7, &[])?;
        writer.bind_test_minimap(root.clone());
        {
            let mut owner = app.world_mut().non_send_mut::<HostOwner>();
            let play = owner.play.as_mut().ok_or("play")?;
            play.view.bind_test_minimap(root.clone());
            if shortcut {
                owner.route_input(
                    &[NormalizedInputEvent::Key {
                        code: InputKey::new(16)?,
                        state: ButtonState::Pressed,
                        modifiers: Modifiers::NONE.with(Modifier::Alt).with(Modifier::Shift),
                        repeat: false,
                    }],
                    false,
                );
            }
        }
        if !shortcut {
            open_marker_editor(&mut app)?;
            activate(&mut app, "Save mark")?;
        }
        app.update();
        let shows_refusal = app
            .world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("The minimap is busy in another client"));
        drop(app);
        drop(writer);
        std::fs::remove_dir_all(root)?;
        visible.push(shows_refusal);
    }
    assert_eq!(
        visible,
        [true, true],
        "shortcut and editor must both display the refusal"
    );
    Ok(())
}

#[test]
fn minimap_drag_uses_the_pointer_windows_logical_pixels() -> TestResult {
    use bevy::picking::{
        events::{Pointer, PointerDrag},
        pointer::{Location, PointerButton, PointerId},
    };
    use bevy::window::WindowRef;
    let mut app = fixture()?;
    let mut window = Window::default();
    window.resolution.set_scale_factor_override(Some(2.));
    let target = app.world_mut().spawn(window).id();
    let image = app
        .world_mut()
        .query_filtered::<Entity, With<MiniImage>>()
        .single(app.world())?;
    app.world_mut().entity_mut(image).insert(ComputedNode {
        size: SIZE.as_vec2() * 2.,
        inverse_scale_factor: 0.5,
        ..default()
    });
    let before = camera(&app)?;
    app.world_mut().trigger(PointerDrag {
        entity: image,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: bevy::camera::NormalizedRenderTarget::Window(
                    WindowRef::Entity(target).normalize(None).ok_or("window")?,
                ),
                position: Vec2::ZERO,
            },
        ),
        button: PointerButton::Primary,
        distance: Vec2::new(8., 0.),
        delta: Vec2::new(8., 0.),
    });
    assert_eq!(camera(&app)?.0, before.0 - 2 * before.3);
    Ok(())
}

#[test]
fn parity_a_minimap_pointer_gestures_use_pointed_camera_position() -> TestResult {
    use bevy::picking::{
        backend::HitData,
        events::{Pointer, PointerDrag, PointerPress, PointerScroll},
        pointer::{Location, PointerButton, PointerId},
    };
    let mut app = fixture()?;
    let entity = app
        .world_mut()
        .query_filtered::<Entity, With<MiniImage>>()
        .single(app.world())?;
    app.world_mut()
        .get_mut::<RelativeCursorPosition>(entity)
        .ok_or("cursor")?
        .normalized = Some(Vec2::new(0.25, 0.));
    let pointer = Pointer::new(
        PointerId::Mouse,
        Location {
            target: bevy::camera::NormalizedRenderTarget::None {
                width: 124,
                height: 100,
            },
            position: Vec2::new(93., 50.),
        },
    );
    let before = camera(&app)?;
    app.world_mut().trigger(PointerPress {
        entity,
        pointer: pointer.clone(),
        button: PointerButton::Secondary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        count: 1,
    });
    app.world_mut().trigger(PointerClick {
        entity,
        pointer: pointer.clone(),
        button: PointerButton::Secondary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        duration: std::time::Duration::ZERO,
        count: 1,
    });
    app.update();
    activate(&mut app, "Save mark")?;
    app.update();
    let owner = app.world().non_send::<HostOwner>();
    owner
        .play
        .as_ref()
        .ok_or("play")?
        .view
        .satellite_map(|_, marks| {
            assert_eq!(marks.len(), 1);
            assert_eq!(
                (marks[0].x, marks[0].y, marks[0].floor),
                (before.0 + 8 * before.3, before.1, before.2)
            );
        });
    app.world_mut().trigger(PointerDrag {
        entity,
        pointer: pointer.clone(),
        button: PointerButton::Primary,
        distance: Vec2::new(8., -4.),
        delta: Vec2::new(8., -4.),
    });
    assert_eq!(camera(&app)?.0, before.0 - 2 * before.3);
    assert_eq!(camera(&app)?.1, before.1 + before.3);
    let zoom = camera(&app)?.3;
    let scroll = |pointer, y| PointerScroll {
        entity,
        pointer,
        unit: bevy::input::mouse::MouseScrollUnit::Line,
        x: 0.,
        y,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        phase: bevy::input::touch::TouchPhase::Moved,
    };
    app.world_mut().trigger(scroll(pointer.clone(), -1.));
    assert!(camera(&app)?.3 > zoom, "wheel down must zoom out");
    let floor = camera(&app)?.2;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    app.world_mut().trigger(scroll(pointer, -1.));
    assert_eq!(
        camera(&app)?.2,
        floor - 1,
        "Ctrl-wheel down must browse the floor above"
    );
    Ok(())
}

#[test]
fn secondary_minimap_cancel_does_not_persist_a_mark() -> TestResult {
    use bevy::picking::{
        backend::HitData,
        events::Pointer,
        pointer::{Location, PointerButton, PointerId},
    };
    let mut app = fixture()?;
    activate(&mut app, "E")?;
    activate(&mut app, "Floor up")?;
    let image = app
        .world_mut()
        .query_filtered::<Entity, With<MiniImage>>()
        .single(app.world())?;
    app.world_mut()
        .entity_mut(image)
        .insert(RelativeCursorPosition {
            normalized: Some(Vec2::new(-0.25, -0.25)),
            ..default()
        });
    app.world_mut().trigger(PointerClick {
        entity: image,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: bevy::camera::NormalizedRenderTarget::None {
                    width: 124,
                    height: 100,
                },
                position: Vec2::ZERO,
            },
        ),
        button: PointerButton::Secondary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        duration: std::time::Duration::ZERO,
        count: 1,
    });
    app.update();
    activate(&mut app, "Cancel")?;
    let owner = app.world().non_send::<HostOwner>();
    let play = owner.play.as_ref().ok_or("play")?;
    play.view
        .satellite_map(|_, marks| assert!(marks.is_empty()));
    Ok(())
}
#[test]
fn create_mark_key_marks_the_player_position_through_the_router() -> TestResult {
    use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
    let (link, _, _) = play_channel();
    let view = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), 7, &[])?;
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(ActivePlay { view, link });
    owner.phase = Phase::Playing;
    let key = NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(16)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Alt).with(Modifier::Shift),
        repeat: false,
    };
    owner.route_input(&[key], false);
    let play = owner.play.as_ref().ok_or("play")?;
    assert!(play.view.satellite_map(|_, marks| {
        marks
            .iter()
            .any(|mark| (mark.x, mark.y, mark.floor) == (100, 200, 7))
    }));
    Ok(())
}

#[test]
fn routed_shortcut_edits_existing_player_mark() -> TestResult {
    use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
    let mut app = fixture()?;
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    let play = owner.play.as_mut().ok_or("play")?;
    let at = play.view.satellite_player();
    play.view.edit_minimap_marker(at, "Camp".into(), 3)?;
    owner.route_input(
        &[NormalizedInputEvent::Key {
            code: oteryn_input_actions::KeyCode::new(16)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Alt).with(Modifier::Shift),
            repeat: false,
        }],
        false,
    );
    app.update();
    let field = app
        .world_mut()
        .query::<&bevy::text::EditableText>()
        .single(app.world())?;
    assert_eq!(field.value(), "Camp");
    activate(&mut app, "Save mark")?;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .view
        .satellite_map(|_, marks| {
            assert_eq!(marks.len(), 1);
            assert_eq!((marks[0].x, marks[0].y, marks[0].floor), at);
            assert_eq!(marks[0].color, 3);
        });
    Ok(())
}

#[test]
fn routed_shortcut_toggles_toolbar_opened_skills() -> TestResult {
    use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
    let mut app = fixture()?;
    super::super::hud_panes::install(&mut app);
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .use_shared_frontend();
    app.update();
    let named = |app: &mut App, name: &str| {
        app.world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find_map(|(e, n)| (n.as_str() == name).then_some(e))
            .ok_or("missing widget or button")
    };
    let button = named(&mut app, "control-skills")?;
    let frame = named(&mut app, "Skills")?;
    for display in [Display::None, Display::Flex] {
        app.world_mut().trigger(Activate { entity: button });
        app.update();
        assert_eq!(
            app.world().get::<Node>(frame).ok_or("frame")?.display,
            display
        );
    }
    for display in [Display::None, Display::Flex] {
        for state in [ButtonState::Pressed, ButtonState::Released] {
            app.world_mut().non_send_mut::<HostOwner>().route_input(
                &[NormalizedInputEvent::Key {
                    code: oteryn_input_actions::KeyCode::new(22)?,
                    state,
                    modifiers: Modifiers::one(Modifier::Alt),
                    repeat: false,
                }],
                false,
            );
        }
        app.update();
        app.update();
        assert_eq!(
            app.world().get::<Node>(frame).ok_or("frame")?.display,
            display
        );
    }
    Ok(())
}

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
    assert_eq!(std::fs::read(&path)?, b"damaged");
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

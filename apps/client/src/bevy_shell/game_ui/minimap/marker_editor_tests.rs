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

include!("marker_removal_tests.rs");

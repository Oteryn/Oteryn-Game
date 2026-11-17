use super::*;
use crate::{
    bevy_shell::owner::{ActivePlay, Phase},
    play::{PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, ui_widgets::Activate};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn fixture() -> Result<App, Box<dyn std::error::Error>> {
    let (link, _, _) = play_channel();
    let view = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), -7, &[])?;
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(ActivePlay { view, link });
    owner.phase = Phase::Playing;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        source::GlobalUiPlugin,
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<bevy::input_focus::InputFocus>()
    .init_resource::<ButtonInput<KeyCode>>()
    .insert_non_send(owner)
    .add_systems(
        Startup,
        (crate::bevy_shell::ui::setup, super::super::setup).chain(),
    );
    app.add_systems(Update, super::super::project);
    install(&mut app);
    app.update();
    Ok(app)
}
fn activate(app: &mut App, label: &str) -> TestResult {
    let name = match label {
        "N" | "W" | "E" | "S" => format!("Rose {label}"),
        "Centre" => "Rose centre".to_owned(),
        "Zoom out" => "minimap-zoom-out".to_owned(),
        "Zoom in" => "minimap-zoom-in".to_owned(),
        "Floor up" => "minimap-floor-up".to_owned(),
        "Floor down" => "minimap-floor-down".to_owned(),
        // Dialog buttons keep their caption text.
        other => {
            let text = app
                .world_mut()
                .query::<(Entity, &Text)>()
                .iter(app.world())
                .find_map(|(e, t)| (t.0 == other).then_some(e))
                .ok_or("missing minimap control")?;
            let button = app
                .world()
                .get::<ChildOf>(text)
                .ok_or("button parent")?
                .parent();
            app.world_mut().trigger(Activate { entity: button });
            app.update();
            return Ok(());
        }
    };
    let button = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(e, n)| (n.as_str() == name).then_some(e))
        .ok_or("missing minimap control")?;
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    Ok(())
}
#[test]
fn the_minimap_is_a_tibia_viewport_with_a_rose_and_icon_buttons() -> TestResult {
    let mut app = fixture()?;
    let size = |app: &mut App, wanted: &str| {
        app.world_mut()
            .query::<(&Name, &Node)>()
            .iter(app.world())
            .find(|(name, _)| name.as_str() == wanted)
            .map(|(_, node)| (node.width, node.height))
    };
    assert_eq!(
        size(&mut app, "Minimap viewport"),
        Some((px(108.), px(111.)))
    );
    assert_eq!(size(&mut app, "minimap-rose"), Some((px(43.), px(43.))));
    for key in [
        "minimap-floor-up",
        "minimap-floor-down",
        "minimap-zoom-in",
        "minimap-zoom-out",
        "minimap-centre",
        "minimap-full-map",
    ] {
        assert_eq!(size(&mut app, key), Some((px(20.), px(20.))), "{key}");
    }
    let centre = app
        .world_mut()
        .query::<(&Name, &ChildOf)>()
        .iter(app.world())
        .find(|(name, _)| name.as_str() == "Rose centre")
        .ok_or("centre")?
        .1
        .parent();
    let rose = app.world().get::<ChildOf>(centre).ok_or("rose")?.parent();
    let cells = app.world().get::<Children>(centre).ok_or("grid")?;
    let flow: Vec<_> = cells
        .iter()
        .filter(|id| {
            app.world().get::<Node>(*id).is_some_and(|n| {
                n.position_type == PositionType::Relative && n.display != Display::None
            })
        })
        .collect();
    assert_eq!(flow.len(), 9);
    assert_eq!(
        app.world()
            .get::<Name>(flow[4])
            .ok_or("centre cell")?
            .as_str(),
        "Rose centre"
    );
    let centre_button = flow[4];
    use super::super::pointer_layout;
    app.add_systems(Update, crate::bevy_shell::ui::toolbar::project_toolbar);
    pointer_layout::install(&mut app, UVec2::new(1280, 720));
    app.update();
    app.update();
    let point = pointer_layout::center(&app, rose)?;
    pointer_layout::hit(&mut app, point, centre_button)?;
    // No compass text buttons and no camera prose.
    let texts: Vec<String> = app
        .world_mut()
        .query_filtered::<&Text, Without<art::ArtFallback>>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect();
    for prose in ["N", "W", "E", "S", "Centre", "Zoom in"] {
        assert!(!texts.iter().any(|text| text == prose), "{prose}");
    }
    assert!(!texts.iter().any(|text| text.starts_with("Floor")));
    Ok(())
}
#[test]
fn sidebar_exposes_all_navigation_controls() -> TestResult {
    let mut app = fixture()?;
    for label in [
        "N",
        "W",
        "Centre",
        "E",
        "S",
        "Zoom out",
        "Zoom in",
        "Floor up",
        "Floor down",
    ] {
        activate(&mut app, label)?;
    }
    Ok(())
}

type CameraState = (i32, i32, i16, i32, bool);
fn camera(app: &App) -> Result<CameraState, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .view
        .minimap_camera())
}
#[test]
fn sidebar_navigation_changes_shared_camera_and_centre_restores_following() -> TestResult {
    let mut app = fixture()?;
    assert_eq!(camera(&app)?, (100, 200, -7, 1, true));
    activate(&mut app, "W")?;
    activate(&mut app, "N")?;
    assert_eq!(camera(&app)?, (95, 195, -7, 1, false));
    activate(&mut app, "Zoom out")?;
    activate(&mut app, "E")?;
    activate(&mut app, "S")?;
    activate(&mut app, "Floor up")?;
    assert_eq!(camera(&app)?, (105, 205, -6, 2, false));
    activate(&mut app, "Centre")?;
    assert_eq!(camera(&app)?, (100, 200, -7, 2, true));
    activate(&mut app, "Zoom in")?;
    activate(&mut app, "Zoom in")?;
    assert_eq!(camera(&app)?.3, 1);
    for _ in 0..20 {
        activate(&mut app, "Zoom out")?;
        activate(&mut app, "Floor up")?;
    }
    assert_eq!(camera(&app)?.2, 0);
    assert_eq!(camera(&app)?.3, 8);
    for _ in 0..20 {
        activate(&mut app, "Floor down")?;
    }
    assert_eq!(camera(&app)?.2, -15);
    Ok(())
}
fn key(app: &mut App, key: KeyCode) {
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.reset_all();
        keys.press(KeyCode::AltLeft);
        keys.press(key);
    }
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
}
#[test]
fn alt_navigation_shortcuts_respect_modal_and_hidden_sidebar() -> TestResult {
    let mut app = fixture()?;
    key(&mut app, KeyCode::ArrowLeft);
    key(&mut app, KeyCode::Home);
    key(&mut app, KeyCode::PageDown);
    assert_eq!(camera(&app)?, (95, 200, -8, 2, false));
    let modal = app
        .world_mut()
        .spawn((Node::default(), super::super::GameModal))
        .id();
    key(&mut app, KeyCode::ArrowRight);
    activate(&mut app, "Centre")?;
    assert_eq!(camera(&app)?, (95, 200, -8, 2, false));
    app.world_mut().despawn(modal);
    let region = app.world().resource::<GameRegions>().minimap;
    app.world_mut()
        .get_mut::<Node>(region)
        .ok_or("region")?
        .display = Display::None;
    key(&mut app, KeyCode::PageUp);
    activate(&mut app, "Centre")?;
    assert_eq!(camera(&app)?, (95, 200, -8, 2, false));
    app.world_mut()
        .get_mut::<Node>(region)
        .ok_or("region")?
        .display = Display::Flex;
    activate(&mut app, "Centre")?;
    key(&mut app, KeyCode::End);
    key(&mut app, KeyCode::PageUp);
    assert_eq!(camera(&app)?, (100, 200, -6, 1, false));
    Ok(())
}
mod foreground_click {
    use super::*;
    use bevy::{
        input_focus::{FocusCause, InputFocus},
        picking::{
            backend::HitData,
            events::Pointer,
            pointer::{Location, PointerButton, PointerId},
        },
    };
    #[test]
    fn minimap_click_respects_foreground_modal() -> Result<(), Box<dyn std::error::Error>> {
        let mut host = HostOwner::new(None)?;
        use oteryn_session::{
            ActorPosition,
            world_map::{MapTile, MapViewHeader, WorldMapViewSnapshot},
        };
        let (link, mut commands, outbox) = crate::play::play_channel();
        outbox.bevy_target_walk_selected();
        let position = ActorPosition {
            x: 100,
            y: 100,
            floor: -7,
        };
        let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), -7, &[])?;
        view.set_live_map(&WorldMapViewSnapshot {
            header: MapViewHeader {
                content_generation: [0; 32],
                bundle_digest: [0; 32],
                reset_epoch: 1,
                origin: position,
                first_visible_floor: -7,
            },
            tiles: vec![MapTile {
                position,
                items: vec![],
                more: false,
                ground_speed: 0,
            }],
        })?;
        host.phase = Phase::Playing;
        host.play = Some(ActivePlay { view, link });
        let generation = host.generation();
        let mut app = App::new();
        app.insert_non_send(host).init_resource::<InputFocus>();
        let image = app
            .world_mut()
            .spawn((
                MiniImage(generation),
                mouse::DragState::default(),
                RelativeCursorPosition {
                    normalized: Some(Vec2::new(0.25, -0.25)),
                    ..default()
                },
                Node::default(),
            ))
            .observe(click)
            .id();
        let focused = app.world_mut().spawn_empty().id();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(focused, FocusCause::Navigated);
        let modal = app
            .world_mut()
            .spawn((
                super::super::super::GameModal,
                Node::default(),
                GlobalZIndex(999),
            ))
            .id();
        let click = || PointerClick {
            entity: image,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target: bevy::camera::NormalizedRenderTarget::None {
                        width: 100,
                        height: 100,
                    },
                    position: Vec2::ZERO,
                },
            ),
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
            duration: std::time::Duration::ZERO,
            count: 1,
        };
        app.world_mut().trigger(click());
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(focused));
        app.world_mut().despawn(modal);
        app.world_mut().trigger(click());
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let play = owner.play.as_mut().ok_or("play")?;
        play.view.tick(&play.link).map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            commands.try_recv()?,
            crate::combat_input::PlayCommand::TargetWalk(ActorPosition {
                x: 108,
                y: 94,
                floor: -7
            })
        );
        Ok(())
    }
}

#[test]
fn secondary_minimap_click_edits_the_viewed_coordinate() -> TestResult {
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
            normalized: Some(Vec2::ZERO),
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
        -6,
        "Cave entrance"
    ))));
    assert_eq!(play.view.satellite_player(), (100, 200, -7));
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
        floor + 1,
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
    let view = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), -7, &[])?;
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
            .any(|mark| (mark.x, mark.y, mark.floor) == (100, 200, -7))
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

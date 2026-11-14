use super::*;
use crate::bevy_shell::owner::HostOwner;
use crate::{bevy_shell::owner::ActivePlay, play::PlayView};
fn owner() -> Result<
    (
        HostOwner,
        tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    ),
    Box<dyn std::error::Error>,
> {
    let mut owner = HostOwner::new(None)?;
    let (link, commands, _) = crate::play::play_channel();
    link.use_shared_frontend();
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(ActivePlay { view, link });
    Ok((owner, commands))
}
#[test]
fn touch_controls_window_shows_owner_data_only() -> Result<(), Box<dyn std::error::Error>> {
    let (mut owner, _commands) = owner()?;
    open(&mut owner);
    let mut expected = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    expected.use_shared_frontend();
    expected.enable_touch_controls();
    let play = owner.play.as_mut().ok_or("play")?;
    expected
        .tick(&play.link)
        .map_err(|_| "expected play ended")?;
    play.view.tick(&play.link).map_err(|_| "play ended")?;
    assert_eq!(
        play.view.scene().sprites().instances(),
        expected.scene().sprites().instances()
    );
    assert!(
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .view
            .touch_controls_visible()
    );
    Ok(())
}
#[test]
fn touch_controls_actions_are_bound_and_confirmed() -> Result<(), Box<dyn std::error::Error>> {
    let (mut owner, _commands) = owner()?;
    let generation = owner.generation();
    assert!(visibility(&mut owner, generation, false, true));
    assert!(!visibility(&mut owner, generation, false, true));
    assert!(visibility(&mut owner, generation, true, false));
    Ok(())
}
#[test]
fn touch_controls_rejected_when_session_replaced_or_target_changed()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut owner, _commands) = owner()?;
    let generation = owner.generation();
    assert!(!visibility(
        &mut owner,
        generation.wrapping_add(1),
        false,
        true
    ));
    assert!(!visibility(&mut owner, generation, true, false));
    owner.open_graphics();
    assert!(!visibility(&mut owner, generation, false, true));
    assert!(
        !owner
            .play
            .as_ref()
            .ok_or("play")?
            .view
            .touch_controls_visible()
    );
    owner.play = None;
    assert!(!visibility(&mut owner, generation, false, true));
    Ok(())
}
#[test]
fn touch_controls_window_registered() {
    assert!(include_str!("../mod.rs").contains("touch_controls::install(app)"));
    assert!(include_str!("../../ui.rs").contains("Self::TouchControls =>"));
    assert!(include_str!("../../ui/toolbar.rs").contains("Control::TouchControls"));
    assert!(include_str!("../../ui/actions.rs").contains("touch_controls::open"));
}
#[test]
fn touch_controls_gestures_require_owned_starts() -> Result<(), Box<dyn std::error::Error>> {
    let (mut owner, _commands) = owner()?;
    let mut gestures = Gestures::default();
    let viewport = super::super::viewport::WorldViewport {
        origin: UVec2::ZERO,
        size: UVec2::new(480, 352),
    };
    let phase = crate::play::TouchPhase::Started;
    assert!(!gestures.route(&mut owner, viewport, 1, phase, Some([72., 280.]), false));
    assert!(gestures.route(&mut owner, viewport, 1, phase, Some([72., 280.]), true));
    assert!(!gestures.route(&mut owner, viewport, 1, phase, Some([72., 280.]), true));
    assert!(!gestures.route(
        &mut owner,
        viewport,
        2,
        crate::play::TouchPhase::Ended,
        Some([72., 280.]),
        true
    ));
    assert!(!gestures.route(
        &mut owner,
        viewport,
        1,
        crate::play::TouchPhase::Ended,
        None,
        true
    ));
    Ok(())
}

#[test]
fn touch_controls_viewport_change_cancels_gesture() -> Result<(), Box<dyn std::error::Error>> {
    let (mut owner, _commands) = owner()?;
    let mut gestures = Gestures::default();
    let mut viewport = super::super::viewport::WorldViewport {
        origin: UVec2::ZERO,
        size: UVec2::new(480, 352),
    };
    assert!(gestures.route(
        &mut owner,
        viewport,
        1,
        TouchPhase::Started,
        Some([72., 280.]),
        true
    ));
    viewport.origin.x += 1;
    assert!(!gestures.route(
        &mut owner,
        viewport,
        1,
        TouchPhase::Ended,
        Some([72., 280.]),
        true
    ));
    assert!(gestures.contacts.is_empty());
    Ok(())
}

#[test]
fn touch_controls_changed_entity_rejects_tap() -> Result<(), Box<dyn std::error::Error>> {
    let (mut owner, mut commands) = owner()?;
    let make_view = |identity| {
        let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
        view.use_shared_frontend();
        view.with_entities(vec![oteryn_session::WorldSpatialEntity {
            display_name: None,
            kind: oteryn_session::EntityKind::Creature,
            entity: oteryn_session::EntityRef {
                identity: [identity; 16],
                generation: 7,
            },
            position: oteryn_session::ActorPosition {
                x: 101,
                y: 100,
                floor: 7,
            },
            detail: oteryn_session::EntityDetail::Actor {
                direction: oteryn_session::StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        }])
    };
    let play = owner.play.as_mut().ok_or("play")?;
    play.view = make_view(1)?;
    let tile = oteryn_renderer::TileCoord::new(101, 100);
    let [x, y] = play.view.scene().view().tile_to_screen(tile);
    let point = Some([f64::from(x) + 2., f64::from(y) + 2.]);
    let mut gestures = Gestures::default();
    let viewport = super::super::viewport::WorldViewport {
        origin: UVec2::ZERO,
        size: UVec2::new(480, 352),
    };
    assert!(gestures.route(&mut owner, viewport, 1, TouchPhase::Started, point, true));
    owner.play.as_mut().ok_or("play")?.view = make_view(2)?;
    owner
        .play
        .as_mut()
        .ok_or("play")?
        .view
        .enable_touch_controls();
    assert!(!gestures.route(&mut owner, viewport, 1, TouchPhase::Ended, point, true));
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn touch_controls_native_events_respect_chat_focus() -> Result<(), Box<dyn std::error::Error>> {
    use crate::bevy_shell::{
        game_ui::GameRegions,
        input::{NativeInput, route_touch},
    };
    use bevy::ecs::system::RunSystemOnce;
    use bevy::input::touch::{TouchInput, TouchPhase as NativePhase};
    use bevy::picking::{
        backend::HitData,
        hover::HoverMap,
        pointer::{Location, PointerId, PointerLocation},
    };
    let (owner, mut commands) = owner()?;
    let (w, h) = owner
        .play
        .as_ref()
        .ok_or("play")?
        .view
        .scene()
        .view()
        .viewport_px();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .init_resource::<bevy::input_focus::InputFocus>()
        .init_resource::<HoverMap>()
        .init_resource::<bevy::picking::pointer::PointerMap>()
        .init_resource::<Gestures>()
        .add_message::<TouchInput>()
        .add_message::<bevy::window::WindowFocused>()
        .add_message::<bevy::window::WindowOccluded>()
        .insert_non_send(owner)
        .insert_non_send(NativeInput::default())
        .insert_resource(WorldViewport {
            origin: UVec2::ZERO,
            size: UVec2::new(w, h),
        })
        .add_systems(Update, route_touch)
        .add_systems(Last, cancel_blocked);
    let world = app.world_mut().spawn(Node::default()).id();
    app.insert_resource(GameRegions {
        vitals: world,
        minimap: world,
        equipment: world,
        containers: world,
        battle: world,
        skills: world,
        vip: world,
        analyzers: world,
        chat: world,
        bars: world,
        world,
    });
    let mut window = Window::default();
    window.resolution.set_physical_resolution(w, h);
    window.resolution.set_scale_factor_override(Some(1.));
    let window = app
        .world_mut()
        .spawn((window, bevy::window::PrimaryWindow))
        .id();
    let target = bevy::camera::NormalizedRenderTarget::Window(
        bevy::window::WindowRef::Entity(window)
            .normalize(None)
            .ok_or("window")?,
    );
    let scale = (w as f32 / 480.).min(h as f32 / 352.);
    let position = Vec2::new(72. * scale, h as f32 - 72. * scale);
    app.world_mut().spawn((
        PointerId::Touch(9),
        PointerLocation::new(Location { target, position }),
    ));
    app.world_mut().resource_mut::<HoverMap>().insert(
        PointerId::Touch(9),
        [(world, HitData::new(world, 0., None, None))]
            .into_iter()
            .collect(),
    );
    app.update();
    let touch = TouchInput {
        phase: NativePhase::Started,
        position,
        window,
        force: None,
        id: 9,
    };
    app.world_mut().write_message(touch);
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .view
            .touch_controls_visible()
    );
    assert_eq!(app.world().resource::<Gestures>().contacts.len(), 1);
    app.world_mut().write_message(TouchInput {
        phase: NativePhase::Moved,
        position: position + Vec2::new(48. * scale, 0.),
        window,
        force: None,
        id: 9,
    });
    app.update();
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let play = owner.play.as_mut().ok_or("play")?;
        let origin = play.view.scene().view().origin();
        play.view.tick(&play.link).map_err(|_| "play ended")?;
        assert_eq!(
            play.view.scene().view().origin(),
            origin,
            "movement waits for the server reply"
        );
    }
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::Step(crate::input::StepDir::East)
    ));
    assert!(commands.try_recv().is_err());
    let editor = app
        .world_mut()
        .spawn((Node::default(), bevy::text::EditableText::new("chat")))
        .id();
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .set(editor, bevy::input_focus::FocusCause::Navigated);
    app.update();
    assert!(app.world().resource::<Gestures>().contacts.is_empty());
    app.world_mut()
        .resource_mut::<bevy::input_focus::InputFocus>()
        .clear();
    app.update();
    app.world_mut().write_message(TouchInput {
        phase: NativePhase::Started,
        position,
        window,
        force: None,
        id: 9,
    });
    app.update();
    assert_eq!(app.world().resource::<Gestures>().contacts.len(), 1);
    app.world_mut()
        .spawn((super::super::GameModal, Node::default()));
    app.world_mut()
        .run_system_once(cancel_blocked)
        .map_err(|error| format!("touch cancellation system: {error:?}"))?;
    assert!(app.world().resource::<Gestures>().contacts.is_empty());
    Ok(())
}

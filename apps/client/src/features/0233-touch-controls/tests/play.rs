use super::*;
use crate::play::feature_0233_touch_controls::state::{Layout, Phase};

fn shared_view() -> Result<PlayView, BatchError> {
    let mut view = super::view()?;
    view.use_shared_frontend();
    Ok(view)
}

#[test]
fn touch_overlay_is_hidden_until_touch_and_highlights_the_selected_mode() -> Result<(), BatchError>
{
    let mut view = shared_view()?;
    let (link, _, _) = play_channel();
    link.use_shared_frontend();
    let now = std::time::Instant::now();
    view.project_presentation(now)?;
    let hidden = view.scene().sprites().instances().to_vec();
    let layout = Layout::new(view.scene().view());
    touch(&mut view, &link, 1, Phase::Started, layout.center);
    view.project_presentation(now)?;
    let normal = view.scene().sprites().instances().to_vec();
    assert!(normal.len() > hidden.len());
    touch(&mut view, &link, 1, Phase::Ended, layout.center);
    touch(&mut view, &link, 2, Phase::Started, layout.button_center(1));
    touch(&mut view, &link, 2, Phase::Ended, layout.button_center(1));
    view.project_presentation(now)?;
    assert_ne!(view.scene().sprites().instances(), normal);
    touch(&mut view, &link, 3, Phase::Started, layout.button_center(1));
    touch(&mut view, &link, 3, Phase::Ended, layout.button_center(1));
    view.project_presentation(now)?;
    assert_eq!(view.scene().sprites().instances(), normal);
    Ok(())
}

#[cfg(not(windows))]
fn touch(view: &mut PlayView, link: &PlayLink, id: u64, phase: Phase, at: [f64; 2]) {
    assert!(view.touch_input(link, id, phase, at).is_ok());
}

// On the native target these same queue tests drive the actual winit event adapter.
#[cfg(windows)]
fn touch(view: &mut PlayView, link: &PlayLink, id: u64, phase: Phase, at: [f64; 2]) {
    use winit::event::{DeviceId, Touch, TouchPhase, WindowEvent};
    let phase = match phase {
        Phase::Started => TouchPhase::Started,
        Phase::Moved => TouchPhase::Moved,
        Phase::Ended => TouchPhase::Ended,
        Phase::Cancelled => TouchPhase::Cancelled,
    };
    assert_eq!(
        view.touch_window_event(
            link,
            &WindowEvent::Touch(Touch {
                device_id: DeviceId::dummy(),
                phase,
                location: winit::dpi::PhysicalPosition::new(at[0], at[1]),
                force: None,
                id,
            })
        ),
        Ok(true)
    );
}

#[test]
fn touch_joystick_uses_paced_keyboard_steps_and_release_stops()
-> Result<(), Box<dyn std::error::Error>> {
    for (offset, expected) in [
        ([0.0, -48.0], [StepDir::North, StepDir::North]),
        ([48.0, -48.0], [StepDir::North, StepDir::East]),
        ([48.0, 0.0], [StepDir::East, StepDir::East]),
        ([48.0, 48.0], [StepDir::South, StepDir::East]),
        ([0.0, 48.0], [StepDir::South, StepDir::South]),
        ([-48.0, 48.0], [StepDir::South, StepDir::West]),
        ([-48.0, 0.0], [StepDir::West, StepDir::West]),
        ([-48.0, -48.0], [StepDir::North, StepDir::West]),
    ] {
        let mut touched = shared_view()?;
        let mut keyboard = shared_view()?;
        let (link, mut commands, outbox) = play_channel();
        link.use_shared_frontend();
        let (keys, mut key_commands, _) = play_channel();
        keys.use_shared_frontend();
        let layout = Layout::new(touched.scene().view());
        let center = layout.center;
        touch(&mut touched, &link, 1, Phase::Started, center);
        touched.tick(&link).map_err(|_| "play ended")?;
        assert!(commands.try_recv().is_err(), "centre is a dead zone");
        touch(
            &mut touched,
            &link,
            1,
            Phase::Moved,
            [center[0] + offset[0], center[1] + offset[1]],
        );
        for direction in expected {
            keyboard.arrow(direction);
            keyboard.tick(&keys).map_err(|_| "play ended")?;
            touched.tick(&link).map_err(|_| "play ended")?;
            assert_eq!(commands.try_recv()?, key_commands.try_recv()?);
            touched.tick(&link).map_err(|_| "play ended")?;
            assert!(commands.try_recv().is_err(), "wait for the paced outcome");
            outbox.stepped(moved_to(100, 200, 0));
            keyboard.apply(&moved_to(100, 200, 0))?;
        }
        touch(&mut touched, &link, 1, Phase::Ended, center);
        touched.tick(&link).map_err(|_| "play ended")?;
        assert!(commands.try_recv().is_err());
        touch(&mut touched, &link, 1, Phase::Started, center);
        touch(
            &mut touched,
            &link,
            1,
            Phase::Moved,
            [center[0] + 48.0, center[1]],
        );
        touch(&mut touched, &link, 1, Phase::Cancelled, center);
        touched.tick(&link).map_err(|_| "play ended")?;
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}

#[test]
fn touch_hotbar_world_taps_equal_mouse_context_and_follow_keys()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::world_interaction::Action;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    for mode in 0..4 {
        let entity = WorldSpatialEntity {
            display_name: None,
            kind: oteryn_session::EntityKind::Creature,
            entity: EntityRef {
                identity: [3; 16],
                generation: 7,
            },
            position: ActorPosition {
                x: 101,
                y: 200,
                floor: 0,
            },
            detail: oteryn_session::EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        };
        let mut touched = shared_view()?.with_entities(vec![entity])?;
        let mut mouse = shared_view()?.with_entities(vec![entity])?;
        if mode == 3 {
            let mut map = super::feature_local_world_view::live_snapshot(101);
            map.tiles[0].items[0].origin =
                oteryn_session::world_map::MapOrigin::Handle(std::num::NonZeroU64::MIN);
            touched.set_live_map(&map)?;
            mouse.set_live_map(&map)?;
            touched = touched.with_entities(Vec::new())?;
            mouse = mouse.with_entities(Vec::new())?;
        }
        let (link, mut commands, outbox) = play_channel();
        link.use_shared_frontend();
        let (other, mut equivalent, other_outbox) = play_channel();
        other.use_shared_frontend();
        outbox.world_context_initialize(true, true);
        other_outbox.world_context_initialize(true, true);
        outbox.target_move_initialize(true);
        other_outbox.target_move_initialize(true);
        let layout = Layout::new(touched.scene().view());
        let at = layout.button_center(mode);
        touch(&mut touched, &link, 1, Phase::Started, at);
        touch(&mut touched, &link, 1, Phase::Ended, at);
        assert_eq!(touched.touch.mode, Some(mode));
        let tile = TileCoord::new(101, 200);
        let [x, y] = touched.scene().view().tile_to_screen(tile);
        let at = [f64::from(x) + 2.0, f64::from(y) + 2.0];
        touch(&mut touched, &link, 2, Phase::Started, at);
        touch(&mut touched, &link, 2, Phase::Ended, at);
        match mode {
            0 => mouse.primary_click(tile, &other)?,
            2 => {
                mouse.scene.select_tile(tile)?;
                mouse.target_move_input(
                    &other,
                    &[NormalizedInputEvent::Key {
                        code: KeyCode::new(9)?,
                        state: ButtonState::Pressed,
                        modifiers: Modifiers::one(Modifier::Control),
                        repeat: false,
                    }],
                );
            }
            _ => {
                let menu = mouse.world_context(tile, &other).ok_or("menu missing")?;
                assert!(mouse.world_context_choose(
                    &other,
                    &menu,
                    if mode == 1 { Action::Look } else { Action::Use }
                ));
            }
        }
        touched.tick(&link).map_err(|_| "play ended")?;
        mouse.tick(&other).map_err(|_| "play ended")?;
        assert_eq!(commands.try_recv()?, equivalent.try_recv()?);
        assert!(commands.try_recv().is_err(), "no duplicate mouse action");
        touch(
            &mut touched,
            &link,
            3,
            Phase::Started,
            layout.button_center(mode),
        );
        touch(
            &mut touched,
            &link,
            3,
            Phase::Ended,
            layout.button_center(mode),
        );
        assert_eq!(touched.touch.mode, None);
    }
    Ok(())
}

#[test]
fn touch_default_tap_dead_zone_and_contact_ownership() -> Result<(), Box<dyn std::error::Error>> {
    let mut touched = shared_view()?;
    let mut mouse = shared_view()?;
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    let (other, mut equivalent, other_outbox) = play_channel();
    other.use_shared_frontend();
    outbox.target_walk_initialize(true);
    other_outbox.target_walk_initialize(true);
    assert!(!touched.touch.visible);
    let tile = TileCoord::new(98, 200);
    let [x, y] = touched.scene().view().tile_to_screen(tile);
    let at = [f64::from(x) + 2.0, f64::from(y) + 2.0];
    touch(&mut touched, &link, 1, Phase::Started, at);
    touch(&mut touched, &link, 1, Phase::Ended, at);
    assert!(touched.touch.visible);
    mouse.primary_click(tile, &other)?;
    touched.tick(&link).map_err(|_| "play ended")?;
    mouse.tick(&other).map_err(|_| "play ended")?;
    assert_eq!(commands.try_recv()?, equivalent.try_recv()?);
    let layout = Layout::new(touched.scene().view());
    let center = layout.center;
    touch(&mut touched, &link, 2, Phase::Started, center);
    touch(
        &mut touched,
        &link,
        2,
        Phase::Moved,
        [center[0] + 48.0, center[1]],
    );
    touch(&mut touched, &link, 99, Phase::Ended, center);
    assert!(
        touched.touch.direction.is_some(),
        "another contact cannot release the stick"
    );
    touch(
        &mut touched,
        &link,
        2,
        Phase::Moved,
        [center[0] + layout.scale, center[1]],
    );
    touched.tick(&link).map_err(|_| "play ended")?;
    assert!(
        commands.try_recv().is_err(),
        "returning to the dead zone stops"
    );
    touch(
        &mut touched,
        &link,
        2,
        Phase::Moved,
        [center[0] + 48.0, center[1]],
    );
    touched.cancel_touch_controls();
    touched.tick(&link).map_err(|_| "play ended")?;
    assert!(
        commands.try_recv().is_err(),
        "focus loss stops held movement"
    );
    touch(&mut touched, &link, 3, Phase::Started, at);
    touch(&mut touched, &link, 3, Phase::Cancelled, at);
    touched.tick(&link).map_err(|_| "play ended")?;
    assert!(commands.try_recv().is_err(), "cancel is not a world tap");
    Ok(())
}

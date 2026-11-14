use super::*;
use crate::{
    bevy_shell::owner::{ActivePlay, Phase},
    play::{PlayView, play_channel},
};
use bevy::asset::AssetPlugin;

type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(super) fn fixture() -> Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    ),
    Box<dyn std::error::Error>,
> {
    let (link, commands, _) = play_channel();
    link.use_shared_frontend();
    assert!(link.spell_bar_bind(&["71".into(), "".into(), "".into(), "".into()]));
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
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
    .insert_non_send(owner)
    .add_systems(
        Startup,
        (crate::bevy_shell::ui::setup, super::super::setup).chain(),
    );
    app.add_systems(Update, super::super::project);
    super::super::action_bar_editor::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands))
}

fn slot(app: &mut App, wanted: usize) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &ActionControl)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            matches!(control, ActionControl::Slot(index) if *index == wanted).then_some(entity)
        })
        .ok_or_else(|| "slot".into())
}

#[test]
fn native_slot_activation_reaches_existing_cast_owner_without_local_prediction() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    let slot = slot(&mut app, 0)?;
    let labels = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .spell_slot_labels();
    assert!(commands.try_recv().is_err());
    app.world_mut().trigger(Activate { entity: slot });
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Combat(
        crate::combat_input::CombatCommand::Cast(id)) if id.get() == 71)
    );
    assert!(commands.try_recv().is_err());
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels(),
        labels
    );
    Ok(())
}

#[test]
fn stale_modal_disabled_and_empty_slots_never_dispatch_casts() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    let first = slot(&mut app, 0)?;
    app.world_mut()
        .entity_mut(first)
        .insert(Generation(u64::MAX));
    app.world_mut().trigger(Activate { entity: first });
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(first).insert(Generation(0));
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    app.world_mut().trigger(Activate { entity: first });
    assert!(commands.try_recv().is_err());
    app.world_mut().non_send_mut::<HostOwner>().cancel();
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bar_gesture(BarGesture::Configure {
            bar: 0,
            dock: crate::play::BarDock::Bottom,
            enabled: false,
            locked: true,
        });
    app.world_mut().trigger(Activate { entity: first });
    assert!(commands.try_recv().is_err());
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bar_gesture(BarGesture::Configure {
            bar: 0,
            dock: crate::play::BarDock::Bottom,
            enabled: true,
            locked: true,
        });
    let empty = slot(&mut app, 1)?;
    app.world_mut().trigger(Activate { entity: empty });
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn visible_slots_follow_real_computed_width_and_saved_scroll_offset() -> TestResult {
    let (mut app, _) = fixture()?;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bar_gesture(BarGesture::Scroll { bar: 0, by: 40 });
    for (width, expected) in [(218., 40..45), (110., 40..42)] {
        for (bar, mut computed) in app
            .world_mut()
            .query::<(&BarRow, &mut ComputedNode)>()
            .iter_mut(app.world_mut())
        {
            if bar.0 == 0 {
                computed.size = Vec2::new(width, 34.);
                computed.inverse_scale_factor = 1.;
            }
        }
        app.update();
        let visible = app
            .world_mut()
            .query::<(&ActionControl, &Node)>()
            .iter(app.world())
            .filter_map(|(control, node)| match control {
                ActionControl::Slot(index)
                    if *index < ROW_SLOTS && node.display != Display::None =>
                {
                    Some(*index)
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(visible, expected.collect::<Vec<_>>());
    }
    Ok(())
}

#[test]
fn unlocking_and_selecting_slot_opens_shared_editor_without_casting() -> TestResult {
    let (mut app, mut commands) = fixture()?;
    let lock = app
        .world_mut()
        .query::<(Entity, &ActionControl)>()
        .iter(app.world())
        .find_map(|(entity, control)| matches!(control, ActionControl::Lock(0)).then_some(entity))
        .ok_or("lock")?;
    app.world_mut().trigger(Activate { entity: lock });
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .bar_layout()
            .rows[0]
            .locked
    );
    let first = slot(&mut app, 0)?;
    app.world_mut().trigger(Activate { entity: first });
    app.update();
    assert!(app.world().resource::<editor::EditorState>().active());
    assert!(commands.try_recv().is_err());
    // Disconnect clears the session-bound edit and its text focus.
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    app.update();
    assert!(!app.world().resource::<editor::EditorState>().active());
    assert!(
        app.world()
            .resource::<bevy::input_focus::InputFocus>()
            .get()
            .is_none()
    );
    Ok(())
}

#[test]
fn action_button_obeys_existing_server_observed_death_gate() -> TestResult {
    let (mut app, _) = fixture()?;
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    assert!(link.spell_bar_bind(&["71".into(), "".into(), "".into(), "".into()]));
    outbox.push(vec![oteryn_session::SessionEvent::ActorVitals(
        oteryn_session::AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::ActorVitals {
                health: 0,
                max_health: 100,
                ..Default::default()
            },
        },
    )]);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    let first = slot(&mut app, 0)?;
    app.world_mut().trigger(Activate { entity: first });
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn native_drop_swaps_only_unlocked_slots_then_casts_new_binding() -> TestResult {
    use bevy::picking::{
        backend::HitData,
        events::{Pointer, PointerDragDrop},
        pointer::{Location, PointerButton, PointerId},
    };
    let (mut app, mut commands) = fixture()?;
    let from = slot(&mut app, 0)?;
    let to = slot(&mut app, 1)?;
    let window = app.world_mut().spawn_empty().id();
    let target = bevy::camera::NormalizedRenderTarget::Window(
        bevy::window::WindowRef::Entity(window)
            .normalize(None)
            .ok_or("window")?,
    );
    let drop = || PointerDragDrop {
        entity: to,
        dropped: from,
        button: PointerButton::Primary,
        pointer: Pointer::new(
            PointerId::Mouse,
            Location {
                target: target.clone(),
                position: Vec2::ZERO,
            },
        ),
        hit: HitData::new(window, 0., None, None),
    };
    app.world_mut().trigger(drop());
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels()[0]
            .contains("71")
    );
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bar_gesture(BarGesture::Configure {
            bar: 0,
            dock: crate::play::BarDock::Bottom,
            enabled: true,
            locked: false,
        });
    super::super::modal::install(&mut app);
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, Node::default(), GlobalZIndex(80)))
        .id();
    app.world_mut().trigger(drop());
    let original = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .spell_slot_labels();
    assert!(original[0].contains("71"));
    assert!(
        !original[1].contains("71"),
        "background drop changed the binding behind the modal"
    );
    app.world_mut().entity_mut(modal).despawn();
    app.update();
    app.world_mut().trigger(drop());
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .spell_slot_labels()[1]
            .contains("71")
    );
    assert!(commands.try_recv().is_err());
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bar_gesture(BarGesture::Configure {
            bar: 0,
            dock: crate::play::BarDock::Bottom,
            enabled: true,
            locked: true,
        });
    app.world_mut().trigger(Activate { entity: to });
    assert!(
        matches!(commands.try_recv()?,crate::combat_input::PlayCommand::Combat(crate::combat_input::CombatCommand::Cast(id)) if id.get() == 71)
    );
    Ok(())
}

#[test]
fn inventory_modal_blocks_prior_action_slot_focus_enter_and_direct_activation() -> TestResult {
    use bevy::input::keyboard::{Key, KeyboardInput};
    use bevy::input_focus::{FocusCause, FocusedInput, InputFocus};
    for keyboard in [true, false] {
        let (mut app, mut commands) = fixture()?;
        app.add_plugins(bevy::ui_widgets::ButtonPlugin);
        super::super::modal::install(&mut app);
        let button = slot(&mut app, 0)?;
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(button, FocusCause::Navigated);
        app.world_mut()
            .spawn((super::super::GameModal, Node::default(), GlobalZIndex(80)));
        if keyboard {
            let window = app.world_mut().spawn_empty().id();
            let key = KeyboardInput {
                key_code: KeyCode::Enter,
                logical_key: Key::Enter,
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            };
            app.world_mut()
                .trigger(FocusedInput::new(button, key, window));
            app.world_mut().flush();
        } else {
            app.world_mut().trigger(Activate { entity: button });
        }
        assert!(
            commands.try_recv().is_err(),
            "background cast escaped the local modal fence"
        );
    }
    Ok(())
}

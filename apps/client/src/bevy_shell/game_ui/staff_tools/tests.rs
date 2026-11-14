use super::*;
use crate::play::play_channel;
type TestResult = Result<(), Box<dyn std::error::Error>>;
#[test]
fn staff_tools_window_shows_server_data_only() -> TestResult {
    let (mut app, _commands, outbox) = fixture()?;
    outbox.bevy_staff_panel(Panel {
        selected: true,
        open: true,
        summary: "Control sequence 17; mute until 30 ms".into(),
        feedback: "Staff authorization revoked".into(),
        ..Default::default()
    });
    app.update();
    let text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("Control sequence 17; mute until 30 ms"));
    assert!(text.contains("Staff authorization revoked"));
    assert!(!text.contains("Audited action confirmed"));
    Ok(())
}
#[test]
fn staff_tools_actions_are_bound_and_confirmed() -> TestResult {
    for tools in [false, true] {
        for receipt in [false, true] {
            let (mut app, mut commands, outbox) = fixture()?;
            outbox.bevy_staff_panel(Panel { tools, ..ready() });
            app.update();
            click(&mut app, Kind::Submit(receipt))?;
            assert!(commands.try_recv().is_err());
            app.update();
            click(&mut app, Kind::Confirm)?;
            click(&mut app, Kind::Confirm)?;
            let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
                return Err("wrong dispatcher".into());
            };
            assert_eq!(
                (command.capability, command.kind),
                if tools {
                    (330, if receipt { 561 } else { 560 })
                } else {
                    (51, if receipt { 61 } else { 60 })
                }
            );
            let envelope =
                oteryn_session::property_services::staff::decode_envelope(&command.payload)?;
            assert_eq!(envelope.principal, "operator");
            assert_eq!(envelope.canonical, b" exact signed action \n");
            assert_eq!(envelope.signature, vec![1, 2, 255]);
            assert!(commands.try_recv().is_err());
            assert_eq!(current_panel(&app)?.summary, "server summary");
            outbox.bevy_staff_reply(&command)?;
            app.update();
            assert_eq!(
                current_panel(&app)?.summary,
                "Control sequence 42; mute until 0 ms"
            );
            assert!(current_panel(&app)?.fields.iter().all(String::is_empty));
        }
    }
    Ok(())
}
#[test]
fn staff_tools_rejected_when_session_replaced_or_target_changed() -> TestResult {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_staff_panel(ready());
    let expected = link.staff_tools_snapshot();
    assert!(!link.staff_tools_submit_bound(&expected, false, false));
    link.staff_edit(1, "changed target".into());
    assert!(!link.staff_tools_submit_bound(&expected, false, true));
    outbox.bevy_staff_panel(ready());
    let modal = link.bevy_server_modal_fixture();
    assert!(!link.staff_tools_submit_bound(&expected, false, true));
    drop(modal);
    let (replacement, mut other_commands, replacement_outbox) = play_channel();
    replacement.use_shared_frontend();
    replacement_outbox.bevy_staff_panel(ready());
    let replacement_snapshot = replacement.staff_tools_snapshot();
    assert!(!replacement.staff_tools_submit_bound(&expected, false, true));
    assert!(replacement_snapshot.matches(&replacement.staff_tools_snapshot()));
    assert!(other_commands.try_recv().is_err());
    outbox.bevy_staff_end();
    assert!(!link.staff_tools_submit_bound(&expected, false, true));
    assert!(commands.try_recv().is_err());
    outbox.bevy_staff_panel(ready());
    assert!(link.staff_tools_submit_bound(&link.staff_tools_snapshot(), false, true));
    let crate::combat_input::PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("staff request".into());
    };
    outbox.bevy_staff_end();
    let ended = link.staff_tools_snapshot();
    outbox.bevy_staff_reply(&command)?;
    assert!(ended.matches(&link.staff_tools_snapshot()));
    assert!(replacement_snapshot.matches(&replacement.staff_tools_snapshot()));
    Ok(())
}
#[test]
fn staff_tools_window_registered() -> TestResult {
    assert!(include_str!("../mod.rs").contains("staff_tools::install(app)"));
    assert!(include_str!("../../ui.rs").contains("shell.staff_tools"));
    let (mut app, _commands, outbox) = fixture()?;
    outbox.bevy_staff_panel(Panel {
        open: false,
        ..ready()
    });
    app.update();
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            matches!(control, crate::bevy_shell::ui::Control::StaffTools).then_some(entity)
        })
        .ok_or("staff toolbar")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(current_panel(&app)?.open);
    click(&mut app, Kind::Close)?;
    app.update();
    assert!(!current_panel(&app)?.open);
    owner_chord(&mut app)?;
    app.update();
    assert!(current_panel(&app)?.open);
    key(&mut app, KeyCode::Escape);
    app.update();
    assert!(!current_panel(&app)?.open);
    Ok(())
}
fn ready() -> Panel {
    Panel {
        selected: true,
        open: true,
        summary: "server summary".into(),
        fields: [
            "operator".into(),
            " exact signed action \n".into(),
            "0102ff".into(),
        ],
        ..Default::default()
    }
}
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    crate::play::Outbox,
);
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    use bevy::asset::AssetPlugin;
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    let mut owner = HostOwner::new(None)?;
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn click(app: &mut App, kind: Kind) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("staff control absent")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn current_panel(app: &App) -> Result<Panel, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .staff_panel())
}

fn key(app: &mut App, code: KeyCode) {
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: code,
            logical_key: if code == KeyCode::Escape {
                bevy::input::keyboard::Key::Escape
            } else {
                bevy::input::keyboard::Key::F10
            },
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
}

#[test]
fn staff_tools_modal_keyboard_and_widgets_reject_changes() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    outbox.bevy_staff_panel(ready());
    app.update();
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(220), Node::default()))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(higher, bevy::input_focus::FocusCause::Navigated);
    click(&mut app, Kind::Submit(false))?;
    key(&mut app, KeyCode::Escape);
    app.update();
    assert!(current_panel(&app)?.open);
    assert!(!current_panel(&app)?.pending);
    assert!(app.world().resource::<State>().confirmation.is_none());
    assert!(commands.try_recv().is_err());
    Ok(())
}

fn route_key(app: &mut App, code: u16, modifiers: oteryn_input_actions::Modifiers) -> TestResult {
    let event = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(code)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers,
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[event], false);
    Ok(())
}
fn owner_chord(app: &mut App) -> TestResult {
    route_key(
        app,
        39,
        oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Alt),
    )
}
#[test]
fn staff_tools_key_does_not_toggle_audio() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    outbox.bevy_staff_panel(Panel {
        open: false,
        ..ready()
    });
    app.update();
    let muted = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .view
        .preferences()
        .audio
        .muted;
    owner_chord(&mut app)?;
    app.update();
    assert!(current_panel(&app)?.open);
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .view
            .preferences()
            .audio
            .muted,
        muted
    );
    click(&mut app, Kind::Close)?;
    app.update();
    route_key(&mut app, 67, oteryn_input_actions::Modifiers::NONE)?;
    key(&mut app, KeyCode::F10);
    app.update();
    assert!(
        !current_panel(&app)?.open,
        "F10 belongs to audio, not staff tools"
    );
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .view
            .preferences()
            .audio
            .muted,
        !muted
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn staff_tools_toolbar_requires_selected_owner() -> TestResult {
    let (mut app, _commands, outbox) = fixture()?;
    app.add_systems(Update, crate::bevy_shell::ui::project_toolbar);
    app.update();
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            matches!(control, crate::bevy_shell::ui::Control::StaffTools).then_some(entity)
        })
        .ok_or("staff toolbar")?;
    assert_eq!(
        app.world().get::<Node>(toolbar).ok_or("node")?.display,
        Display::None
    );
    outbox.bevy_staff_panel(ready());
    app.update();
    assert_eq!(
        app.world().get::<Node>(toolbar).ok_or("node")?.display,
        Display::Flex
    );
    outbox.bevy_staff_end();
    app.update();
    assert_eq!(
        app.world().get::<Node>(toolbar).ok_or("node")?.display,
        Display::None
    );
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    app.update();
    assert_eq!(
        app.world().get::<Node>(toolbar).ok_or("node")?.display,
        Display::None
    );
    Ok(())
}

use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus, ui_widgets::Activate};
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn fixture() -> Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
        Outbox,
    ),
    Box<dyn std::error::Error>,
> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.premium_game_initialize(true);
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn binding(app: &App) -> Result<crate::premium::game::Binding, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("fixture session")?
        .link
        .premium_overview_binding())
}
fn toggle(app: &mut App) {
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
}
fn control(app: &mut App, close: bool) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.close == close).then_some(e))
        .ok_or("control absent".into())
}
fn command(
    receiver: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<FeatureCommand, Box<dyn std::error::Error>> {
    match receiver.try_recv()? {
        PlayCommand::Feature(c) => Ok(c),
        _ => Err("wrong command".into()),
    }
}
fn settle(outbox: &Outbox, command: &FeatureCommand) -> TestResult {
    let overview = oteryn_client_runtime::premium_overview::Overview {
        state: oteryn_client_runtime::premium_overview::State::Active,
        authority_revision: 42,
        observed_at_us: 1_800_000_000_000_000,
        expires_at_us: Some(1_800_086_400_000_000),
        valid_for_us: 60_000_000,
        features: vec![oteryn_client_runtime::premium_overview::Feature {
            title: "Server feature".into(),
            description: "Server description".into(),
            enabled: true,
        }],
    };
    outbox.premium_game_result(
        command,
        &GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: overview.encode().ok_or("encode")?,
        },
    );
    Ok(())
}
#[test]
fn premium_overview_window_shows_owner_data_only() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    toggle(&mut app);
    let request = command(&mut commands)?;
    assert!(!binding(&app)?.text.contains("Server feature"));
    settle(&outbox, &request)?;
    app.update();
    let expected = binding(&app)?.text;
    assert!(expected.contains("Server feature [available]"));
    assert!(expected.contains("Authority revision: 42"));
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == expected)
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn premium_overview_actions_are_bound_and_confirmed() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    toggle(&mut app);
    let request = command(&mut commands)?;
    assert_eq!(
        (request.capability, request.kind, request.payload.as_slice()),
        (97, 113, [5].as_slice())
    );
    let refresh = control(&mut app, false)?;
    app.world_mut().trigger(Activate { entity: refresh });
    assert!(commands.try_recv().is_err());
    settle(&outbox, &request)?;
    app.update();
    let before = binding(&app)?.text;
    let refresh = control(&mut app, false)?;
    app.world_mut().trigger(Activate { entity: refresh });
    app.world_mut().trigger(Activate { entity: refresh });
    let _ = command(&mut commands)?;
    assert!(commands.try_recv().is_err());
    assert_eq!(binding(&app)?.text, before);
    // Refresh is read-only; the native client has no paid/destructive action to confirm.
    Ok(())
}
#[test]
fn premium_overview_rejected_when_session_replaced_or_target_changed() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    toggle(&mut app);
    let request = command(&mut commands)?;
    let stale = control(&mut app, false)?;
    settle(&outbox, &request)?;
    app.world_mut().trigger(Activate { entity: stale });
    assert!(commands.try_recv().is_err());
    app.update();
    let refresh = control(&mut app, false)?;
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(999), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: refresh });
    assert!(commands.try_recv().is_err());
    app.world_mut().despawn(modal);
    app.update();
    let (replacement, mut receiver, _) = play_channel();
    replacement.use_shared_frontend();
    let previous = std::mem::replace(
        &mut app
            .world_mut()
            .non_send_mut::<HostOwner>()
            .play
            .as_mut()
            .ok_or("play")?
            .link,
        replacement,
    );
    app.world_mut().trigger(Activate { entity: refresh });
    assert!(receiver.try_recv().is_err());
    assert!(commands.try_recv().is_err());
    let expected = previous.premium_overview_binding();
    drop(commands);
    assert!(!previous.premium_overview_action_bound(&expected, false, true));
    Ok(())
}
#[test]
fn premium_overview_window_registered() -> TestResult {
    assert!(include_str!("../mod.rs").contains("premium_overview::install(app);"));
    assert!(include_str!("../../ui/toolbar.rs").contains("Control::PremiumOverview"));
    let (mut app, mut commands, _) = fixture()?;
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.id() == "shell.premium_overview").then_some(e))
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(binding(&app)?.open);
    let _ = command(&mut commands)?;
    let close = control(&mut app, true)?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert!(!binding(&app)?.open);
    crate::bevy_shell::controls::tests::shortcut(
        &mut app,
        19,
        oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Alt)
            .with(oteryn_input_actions::Modifier::Shift),
    )?;
    app.update();
    assert!(binding(&app)?.open);
    assert!(binding(&app)?.pending);
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn premium_overview_close_preserves_pending_and_admits_unavailable_blocked_owner() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    toggle(&mut app);
    let request = command(&mut commands)?;
    let close = control(&mut app, true)?;
    let modal = {
        let owner = app.world().non_send::<HostOwner>();
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .bevy_server_modal_fixture()
    };
    app.world_mut().trigger(Activate { entity: close });
    assert!(!binding(&app)?.open);
    assert!(binding(&app)?.pending);
    drop(modal);
    toggle(&mut app);
    assert!(binding(&app)?.pending);
    assert!(commands.try_recv().is_err());
    settle(&outbox, &request)?;
    app.update();
    assert!(binding(&app)?.text.contains("Server feature [available]"));
    // Capability-unavailable panels still offer Close.
    outbox.premium_game_initialize(false);
    toggle(&mut app);
    assert!(binding(&app)?.open);
    let close = control(&mut app, true)?;
    app.world_mut().trigger(Activate { entity: close });
    assert!(!binding(&app)?.open);
    Ok(())
}
#[test]
fn premium_overview_owner_chord_respects_focused_chat_editor() -> TestResult {
    let (mut app, mut commands, _) = fixture()?;
    let editor = app
        .world_mut()
        .spawn(bevy::text::EditableText::new("chat draft"))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, bevy::input_focus::FocusCause::Navigated);
    crate::bevy_shell::controls::tests::shortcut(
        &mut app,
        19,
        oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Alt)
            .with(oteryn_input_actions::Modifier::Shift),
    )?;
    assert!(!binding(&app)?.open);
    assert!(commands.try_recv().is_err());
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(editor));
    Ok(())
}
#[test]
fn premium_overview_escape_closes_pending_with_modal_and_editor_focus() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    toggle(&mut app);
    let request = command(&mut commands)?;
    let editor = app
        .world_mut()
        .spawn(bevy::text::EditableText::new("chat draft"))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(editor, bevy::input_focus::FocusCause::Navigated);
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(999), Node::default()))
        .id();
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert!(!binding(&app)?.open);
    assert!(binding(&app)?.pending);
    settle(&outbox, &request)?;
    assert!(!binding(&app)?.pending);
    app.world_mut().despawn(modal);
    app.world_mut().resource_mut::<InputFocus>().clear();
    toggle(&mut app);
    assert!(binding(&app)?.text.contains("Server feature [available]"));
    // Reopening a settled view requests a fresh observation through the same owner.
    let _ = command(&mut commands)?;
    assert!(commands.try_recv().is_err());
    Ok(())
}

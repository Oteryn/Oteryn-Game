use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus, ui_widgets::Activate};
use oteryn_client_runtime::{
    raw_feature::GatedCommandResult,
    world_event::{
        CAPABILITY_WORLD_EVENT_V1, COMMAND_TYPE_WORLD_EVENT_QUERY, WorldEventState,
        encode_world_event_state,
    },
};
use oteryn_session::CommandStatus;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_world_event_selected(true);
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
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn toolbar(app: &mut App) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.id() == "shell.world_event").then_some(e))
        .ok_or("world event toolbar missing")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
fn button(app: &mut App, kind: Kind) -> std::result::Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == kind).then_some(e))
        .ok_or("world event control missing".into())
}
fn panel(app: &App) -> std::result::Result<Panel, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .world_event_panel())
}
fn reply(outbox: &Outbox, active: bool, status: CommandStatus) -> Result {
    outbox.bevy_world_event_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload: encode_world_event_state(&WorldEventState {
            reset_epoch: 42,
            active,
            announcement: if active {
                "Server festival".into()
            } else {
                String::new()
            },
        })
        .map_err(|e| format!("{e:?}"))?,
    });
    Ok(())
}
fn query(commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>) -> Result {
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("wrong request".into());
    };
    assert_eq!(command.capability, CAPABILITY_WORLD_EVENT_V1);
    assert_eq!(command.kind, COMMAND_TYPE_WORLD_EVENT_QUERY);
    assert!(command.payload.is_empty());
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn world_event_window_window_shows_server_data_only() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    toolbar(&mut app)?;
    query(&mut commands)?;
    for (active, status) in [
        (true, CommandStatus::Accepted),
        (false, CommandStatus::DuplicateReplay),
        (true, CommandStatus::Rejected),
    ] {
        reply(&outbox, active, status)?;
        app.update();
        let expected = oteryn_client_ui::world_event::panel_text(&panel(&app)?);
        let body = app
            .world_mut()
            .query::<(&Name, &Text)>()
            .iter(app.world())
            .find_map(|(n, t)| (n.as_str() == "world_event.status").then_some(t.0.clone()))
            .ok_or("status text")?;
        assert_eq!(body, expected);
        let refresh = button(&mut app, Kind::Refresh)?;
        app.world_mut().trigger(Activate { entity: refresh });
        query(&mut commands)?;
    }
    Ok(())
}
#[test]
fn world_event_window_actions_are_bound_and_confirmed() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    toolbar(&mut app)?;
    query(&mut commands)?;
    let refresh = button(&mut app, Kind::Refresh)?;
    app.world_mut().trigger(Activate { entity: refresh });
    assert!(commands.try_recv().is_err());
    reply(&outbox, true, CommandStatus::Accepted)?;
    app.update();
    let refresh = button(&mut app, Kind::Refresh)?;
    app.world_mut().trigger(Activate { entity: refresh });
    query(&mut commands)?;
    assert!(panel(&app)?.pending);
    assert!(panel(&app)?.state.is_none());
    app.update();
    let close = button(&mut app, Kind::Close)?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    reply(&outbox, true, CommandStatus::Accepted)?;
    app.update();
    assert!(!panel(&app)?.open);
    assert!(panel(&app)?.state.is_none());
    Ok(())
}
#[test]
fn world_event_window_rejected_when_session_replaced_or_target_changed() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    toolbar(&mut app)?;
    query(&mut commands)?;
    reply(&outbox, true, CommandStatus::Accepted)?;
    app.update();
    let refresh = button(&mut app, Kind::Refresh)?;
    let binding = app
        .world()
        .resource::<State>()
        .snapshot
        .as_ref()
        .ok_or("snapshot")?
        .1
        .clone();
    outbox.bevy_world_event_selected(false);
    let before = panel(&app)?;
    app.world_mut().trigger(Activate { entity: refresh });
    assert_eq!(panel(&app)?, before);
    assert!(commands.try_recv().is_err());
    outbox.bevy_world_event_selected(true);
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .world_event_request();
    query(&mut commands)?;
    reply(&outbox, false, CommandStatus::Accepted)?;
    let changed = panel(&app)?;
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .world_event_dispatch_bound(&binding, Kind::Refresh)
    );
    app.world_mut().trigger(Activate { entity: refresh });
    assert_eq!(panel(&app)?, changed);
    assert!(commands.try_recv().is_err());
    app.update();
    let refresh = button(&mut app, Kind::Refresh)?;
    let binding = app
        .world()
        .resource::<State>()
        .snapshot
        .as_ref()
        .ok_or("changed snapshot")?
        .1
        .clone();
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(999), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: refresh });
    assert!(commands.try_recv().is_err());
    app.world_mut().despawn(modal);
    app.update();
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    let server_modal = link.bevy_server_modal_fixture();
    assert!(!link.world_event_dispatch_bound(&binding, Kind::Refresh));
    drop(server_modal);
    let (replacement, mut replacement_commands, replacementoutbox) = play_channel();
    replacement.use_shared_frontend();
    replacementoutbox.bevy_world_event_selected(true);
    assert!(!replacement.world_event_dispatch_bound(&binding, Kind::Refresh));
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = replacement;
    app.world_mut().trigger(Activate { entity: refresh });
    assert!(replacement_commands.try_recv().is_err());
    outbox.bevy_world_event_end();
    reply(&outbox, true, CommandStatus::Accepted)?;
    assert!(!panel(&app)?.open);
    Ok(())
}
#[test]
fn world_event_window_window_registered() -> Result {
    let mut registered = App::new();
    super::super::install(&mut registered);
    assert!(registered.world().contains_resource::<State>());
    let (mut app, mut commands, outbox) = fixture()?;
    toolbar(&mut app)?;
    query(&mut commands)?;
    assert!(panel(&app)?.open);
    button(&mut app, Kind::Close)?;
    let close = button(&mut app, Kind::Close)?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert!(!panel(&app)?.open);
    // The outstanding query must settle before a new query can be issued.
    reply(&outbox, false, CommandStatus::Accepted)?;
    let shortcut = bevy::input::keyboard::KeyboardInput {
        key_code: KeyCode::KeyN,
        logical_key: bevy::input::keyboard::Key::Character("n".into()),
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    };
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(999), Node::default()))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(modal, bevy::input_focus::FocusCause::Navigated);
    app.world_mut().write_message(shortcut.clone());
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    app.update();
    assert!(!panel(&app)?.open);
    assert!(commands.try_recv().is_err());
    app.world_mut().despawn(modal);
    app.update();
    app.world_mut().write_message(shortcut);
    app.update();
    assert!(panel(&app)?.open);
    query(&mut commands)?;
    Ok(())
}

use super::*;
use crate::{combat_input::PlayCommand, play::play_channel};
use oteryn_client_runtime::{cast::Message, server_mechanic::ServerMechanicResponse};
use oteryn_session::CommandStatus;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn cast_control_window_shows_server_data_only() -> Result {
    let (link, _, outbox) = play_channel();
    outbox.bevy_cast_initialize();
    outbox.bevy_cast_result(Message::CasterState {
        cast_id: [1; 16],
        revision: 9,
        epoch: 3,
        delay: 17,
        count: 4,
    })?;
    let panel = link.cast_control_panel();
    assert_eq!(
        presentation(&panel),
        oteryn_client_ui::cast_consent::panel_text(&panel)
    );
    assert!(presentation(&panel).contains("4 viewers"));
    assert!(presentation(&panel).contains("17"));
    assert_eq!(settings(&panel), ["17", "50", "0", "1"]);
    let (mut app, _rx, server) = fixture()?;
    open(app.world().non_send::<HostOwner>());
    server.bevy_cast_result(Message::CasterState {
        cast_id: [1; 16],
        revision: 9,
        epoch: 3,
        delay: 17,
        count: 4,
    })?;
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == presentation(&panel))
    );
    let values: Vec<_> = app
        .world_mut()
        .query::<(&Field, &EditableText)>()
        .iter(app.world())
        .map(|(field, value)| (field.0, value.value().to_string()))
        .collect();
    for (index, value) in settings(&panel).into_iter().enumerate() {
        assert!(values.contains(&(index, value)));
    }

    Ok(())
}

#[test]
fn cast_control_actions_are_bound_and_confirmed() -> Result {
    let (link, mut rx, outbox) = play_channel();
    outbox.bevy_cast_initialize();
    open_link(&link, &mut rx, &outbox)?;
    ui_actions()?;
    for action in [0, 1, 2, 3, 4, 5] {
        outbox.bevy_cast_result(Message::CasterState {
            cast_id: [1; 16],
            revision: 9,
            epoch: 3,
            delay: 17,
            count: 4,
        })?;
        outbox.bevy_cast_consent_result(&ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: Message::ConsentState {
                revision: 6,
                allowed: false,
            }
            .encode()
            .map_err(|e| format!("{e:?}"))?,
        });
        let snapshot = link.cast_control_snapshot();
        let mut draft = settings(&snapshot.panel);
        if action == 0 {
            draft = ["28".into(), "9".into(), "1".into(), "12".into()];
        }
        if needs_confirmation(action) {
            assert!(!link.cast_control_dispatch_bound(&snapshot, action, &draft, false));
            assert!(rx.try_recv().is_err());
        }
        assert!(link.cast_control_dispatch_bound(&snapshot, action, &draft, true));
        let PlayCommand::ServerMechanic(command) = rx.try_recv()? else {
            return Err("command".into());
        };
        assert_eq!(
            command.capability,
            if (2..=4).contains(&action) { 292 } else { 290 }
        );
        assert_eq!(
            command.command_type,
            if (2..=4).contains(&action) { 503 } else { 502 }
        );
        let expected = match action {
            0 => Message::Update {
                revision: 9,
                access_revision: 12,
                delay: 28,
                cap: 9,
                password: true,
            },
            1 => Message::Stop { revision: 9 },
            2 => Message::ConsentQuery,
            3 => Message::ConsentSet {
                revision: 6,
                allowed: true,
            },
            4 => Message::ConsentSet {
                revision: 6,
                allowed: false,
            },
            _ => Message::ControlQuery,
        };
        assert_eq!(
            Message::decode(&command.payload).map_err(|e| format!("{e:?}"))?,
            expected
        );
        assert!(rx.try_recv().is_err());
        assert!(!link.cast_control_dispatch_bound(&snapshot, action, &draft, true));
        let after = link.cast_control_panel();
        assert!(after.active);
        assert!(!after.allow_shown);
        assert_eq!(after.revision, 9);
        assert_eq!(settings(&after), ["17", "50", "0", "1"]);
    }
    Ok(())
}

#[test]
fn cast_control_rejected_when_session_replaced_or_target_changed() -> Result {
    let (link, mut rx, outbox) = play_channel();
    outbox.bevy_cast_initialize();
    open_link(&link, &mut rx, &outbox)?;
    let snapshot = link.cast_control_snapshot();
    let draft = settings(&snapshot.panel);
    let (replacement, mut replacement_rx, replacement_outbox) = play_channel();
    replacement_outbox.bevy_cast_initialize();
    open_link(&replacement, &mut replacement_rx, &replacement_outbox)?;
    assert!(!replacement.cast_control_dispatch_bound(&snapshot, 5, &draft, true));
    assert!(link.cast_control_close_bound(&snapshot));
    assert!(!link.cast_control_dispatch_bound(&snapshot, 5, &draft, true));
    open_link(&link, &mut rx, &outbox)?;
    let (modal_link, mut modal_rx, modal_outbox) = play_channel();
    modal_outbox.bevy_cast_initialize();
    open_link(&modal_link, &mut modal_rx, &modal_outbox)?;
    let modal_snapshot = modal_link.cast_control_snapshot();
    let _modal = modal_link.bevy_server_modal_fixture();
    assert!(!modal_link.cast_control_dispatch_bound(&modal_snapshot, 5, &draft, true));
    assert!(modal_snapshot.same(&modal_link.cast_control_snapshot()));
    assert!(modal_rx.try_recv().is_err());
    ui_rejections()?;
    outbox.bevy_cast_result(Message::CasterState {
        cast_id: [2; 16],
        revision: 10,
        epoch: 4,
        delay: 22,
        count: 5,
    })?;
    assert!(!link.cast_control_dispatch_bound(&snapshot, 5, &draft, true));
    outbox.bevy_cast_end();
    assert!(!link.cast_control_dispatch_bound(&snapshot, 5, &draft, true));
    assert!(rx.try_recv().is_err());
    Ok(())
}

#[test]
fn cast_control_window_registered() -> Result {
    assert_eq!(
        crate::bevy_shell::ui::Control::Cast.id(),
        "shell.cast_control"
    );
    let mut app = bevy::prelude::App::new();
    install(&mut app);
    assert!(app.world().contains_resource::<State>());
    registered()?;
    Ok(())
}

type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    crate::play::Outbox,
);
fn fixture() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let (link, rx, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_cast_initialize();
    let mut owner = HostOwner::new(None)?;
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<bevy::input_focus::InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, rx, outbox))
}
fn click(app: &mut App, kind: Kind) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("control")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    // A confirmation is projected on the next update, after the observer's edits settle.
    app.update();
    Ok(())
}
fn ui_actions() -> Result {
    let (mut app, mut rx, outbox) = fixture()?;
    open(app.world().non_send::<HostOwner>());
    app.update();
    assert!(matches!(rx.try_recv()?, PlayCommand::ServerMechanic(_)));
    for action in [0, 1, 2, 3, 4, 5] {
        outbox.bevy_cast_result(Message::CasterState {
            cast_id: [1; 16],
            revision: 9,
            epoch: 3,
            delay: 17,
            count: 4,
        })?;
        outbox.bevy_cast_consent_result(&ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: Message::ConsentState {
                revision: 6,
                allowed: false,
            }
            .encode()
            .map_err(|e| format!("{e:?}"))?,
        });
        app.update();
        click(&mut app, Kind::Request(action))?;
        if needs_confirmation(action) {
            assert!(rx.try_recv().is_err());
            click(&mut app, Kind::Confirm)?;
        }
        let PlayCommand::ServerMechanic(command) = rx.try_recv()? else {
            return Err("command".into());
        };
        assert_eq!(
            command.capability,
            if (2..=4).contains(&action) { 292 } else { 290 }
        );
        assert!(rx.try_recv().is_err());
    }
    Ok(())
}
fn shortcut(app: &mut App) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    for key in [KeyCode::ControlLeft, KeyCode::AltLeft, KeyCode::ShiftLeft] {
        keys.press(key);
    }
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::KeyS,
            logical_key: bevy::input::keyboard::Key::Character("s".into()),
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
}
fn registered() -> Result {
    let (mut app, _rx, _outbox) = fixture()?;
    let entity = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.id() == "shell.cast_control").then_some(entity))
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    assert!(app.world().resource::<State>().root.is_some());
    app.world_mut().trigger(Activate { entity });
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    app.world_mut().trigger(Activate { entity });
    app.update();
    assert!(app.world().resource::<State>().root.is_some());
    shortcut(&mut app);
    assert!(app.world().resource::<State>().root.is_none());
    shortcut(&mut app);
    assert!(app.world().resource::<State>().root.is_some());
    click(&mut app, Kind::Close)?;
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}

fn ui_rejections() -> Result {
    let (mut app, mut rx, outbox) = fixture()?;
    open(app.world().non_send::<HostOwner>());
    app.update();
    assert!(matches!(rx.try_recv()?, PlayCommand::ServerMechanic(_)));
    outbox.bevy_cast_result(Message::End)?;
    app.update();
    click(&mut app, Kind::Request(0))?;
    let confirm = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == Kind::Confirm).then_some(entity))
        .ok_or("confirm")?;
    let higher = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(250), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: confirm });
    app.update();
    assert!(rx.try_recv().is_err());
    app.world_mut().despawn(higher);
    app.update();
    let owner = app.world_mut().non_send_mut::<HostOwner>();
    owner
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .cast_control_toggle_bound();
    app.world_mut().trigger(Activate { entity: confirm });
    app.update();
    assert!(rx.try_recv().is_err());
    Ok(())
}

fn open_link(
    link: &crate::play::PlayLink,
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    outbox: &crate::play::Outbox,
) -> Result {
    assert!(link.cast_control_toggle_bound());
    let PlayCommand::ServerMechanic(command) = rx.try_recv()? else {
        return Err("query".into());
    };
    assert_eq!(
        Message::decode(&command.payload).map_err(|e| format!("{e:?}"))?,
        Message::ControlQuery
    );
    outbox.bevy_cast_result(Message::End)?;
    Ok(())
}

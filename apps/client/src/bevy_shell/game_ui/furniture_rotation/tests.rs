use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::raw_feature::{
    CommandStatus, FeatureCommand, GatedCommandResult, house_rotation as wire,
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.furniture_rotation_selected();
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
fn panel(app: &App) -> Result<Panel, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .house_rotation_panel())
}
fn request(
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<FeatureCommand, Box<dyn std::error::Error>> {
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("wrong command".into());
    };
    assert_eq!((command.capability, command.kind), (445, 730));
    Ok(command)
}
fn publish(outbox: &Outbox, command: &FeatureCommand, reply: wire::Reply) -> TestResult {
    outbox.furniture_rotation_reply(
        command,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload: reply.encode().ok_or("encode")?,
        },
    );
    Ok(())
}
fn catalogue(page: u16) -> wire::Catalogue {
    wire::Catalogue {
        page,
        more: page == 0,
        entries: vec![wire::Entry {
            label: "Server cupboard".into(),
            target: wire::Target::Item {
                item: [7; 16],
                expected_revision: 42,
            },
        }],
    }
}
fn click(app: &mut App, kind: Kind) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("control")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn open(app: &mut App) {
    toggle(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
}
fn ready(
    app: &mut App,
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    outbox: &Outbox,
) -> TestResult {
    open(app);
    let command = request(commands)?;
    assert_eq!(
        wire::Request::decode(&command.payload),
        Some(wire::Request::Query(0))
    );
    publish(outbox, &command, wire::Reply::Catalogue(catalogue(0)))?;
    app.update();
    Ok(())
}
#[test]
fn furniture_rotation_window_shows_server_data_only() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    open(&mut app);
    assert!(panel(&app)?.catalogue.is_none());
    let command = request(&mut commands)?;
    publish(&outbox, &command, wire::Reply::Catalogue(catalogue(0)))?;
    app.update();
    let expected = oteryn_client_ui::house_furniture_rotation::panel_text(&panel(&app)?);
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == expected)
    );
    assert_eq!(panel(&app)?.catalogue, Some(catalogue(0)));
    Ok(())
}
#[test]
fn furniture_rotation_actions_are_bound_and_confirmed() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    ready(&mut app, &mut commands, &outbox)?;
    for (kind, page) in [
        (Kind::Request(Intent::Next), 1),
        (Kind::Request(Intent::Previous), 0),
        (Kind::Request(Intent::Refresh), 0),
    ] {
        click(&mut app, kind)?;
        let command = request(&mut commands)?;
        assert_eq!(
            wire::Request::decode(&command.payload),
            Some(wire::Request::Query(page))
        );
        assert!(commands.try_recv().is_err());
        publish(&outbox, &command, wire::Reply::Catalogue(catalogue(page)))?;
        app.update();
    }
    click(&mut app, Kind::Select(0))?;
    app.update();
    let before = panel(&app)?;
    click(&mut app, Kind::Review)?;
    assert!(commands.try_recv().is_err());
    app.update();
    click(&mut app, Kind::Cancel)?;
    app.update();
    assert_eq!(panel(&app)?, before);
    click(&mut app, Kind::Review)?;
    app.update();
    click(&mut app, Kind::Confirm)?;
    click(&mut app, Kind::Confirm)?;
    let command = request(&mut commands)?;
    assert_eq!(
        wire::Request::decode(&command.payload),
        catalogue(0).rotate(0)
    );
    assert!(commands.try_recv().is_err());
    assert_eq!(panel(&app)?.catalogue, before.catalogue);
    publish(
        &outbox,
        &command,
        wire::Reply::Applied(wire::UseDisposition::Committed),
    )?;
    assert!(panel(&app)?.catalogue.is_none());
    Ok(())
}
#[test]
fn furniture_rotation_rejected_when_session_replaced_or_target_changed() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    ready(&mut app, &mut commands, &outbox)?;
    click(&mut app, Kind::Select(0))?;
    app.update();
    click(&mut app, Kind::Review)?;
    app.update();
    let binding = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .house_rotation_binding();
    let modal = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .bevy_server_modal_fixture();
    let before_modal = panel(&app)?;
    click(&mut app, Kind::Confirm)?;
    assert_eq!(panel(&app)?, before_modal);
    assert!(commands.try_recv().is_err());
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .house_rotation_dispatch_bound(&binding, BoundAction::Request(Intent::Rotate))
    );
    drop(modal);
    let overlay = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(999), Node::default()))
        .id();
    click(&mut app, Kind::Confirm)?;
    assert_eq!(panel(&app)?, before_modal);
    assert!(commands.try_recv().is_err());
    app.world_mut().despawn(overlay);
    app.update();
    outbox.furniture_rotation_change_target();
    let changed = panel(&app)?;
    click(&mut app, Kind::Confirm)?;
    assert_eq!(panel(&app)?, changed);
    assert!(commands.try_recv().is_err());
    let (other, mut other_commands, other_outbox) = play_channel();
    other.use_shared_frontend();
    other_outbox.furniture_rotation_selected();
    assert!(
        other.house_rotation_dispatch_bound(&other.house_rotation_binding(), BoundAction::Toggle)
    );
    let query = request(&mut other_commands)?;
    publish(&other_outbox, &query, wire::Reply::Catalogue(catalogue(0)))?;
    assert!(
        other
            .house_rotation_dispatch_bound(&other.house_rotation_binding(), BoundAction::Select(0))
    );
    assert_eq!(other.house_rotation_panel(), binding.panel);
    assert!(!other.house_rotation_dispatch_bound(&binding, BoundAction::Request(Intent::Rotate)));
    assert!(other_commands.try_recv().is_err());
    outbox.furniture_rotation_end();
    click(&mut app, Kind::Confirm)?;
    assert!(commands.try_recv().is_err());
    assert!(!panel(&app)?.selected);
    // A reply from the closed session cannot revive the catalogue.
    let command = FeatureCommand {
        capability: 445,
        kind: 730,
        payload: wire::Request::Query(0).encode().ok_or("query")?,
    };
    publish(&outbox, &command, wire::Reply::Catalogue(catalogue(0)))?;
    assert!(panel(&app)?.catalogue.is_none());
    Ok(())
}
#[test]
fn furniture_rotation_window_registered() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            (control.id() == "shell.furniture_rotation").then_some(entity)
        })
        .ok_or("toolbar registration")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(panel(&app)?.open);
    let query = request(&mut commands)?;
    publish(&outbox, &query, wire::Reply::Catalogue(catalogue(0)))?;
    app.update();
    click(&mut app, Kind::Close)?;
    app.update();
    assert!(!panel(&app)?.open);
    let event = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(9)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Alt)
            .with(oteryn_input_actions::Modifier::Shift),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[event], false);
    app.update();
    assert!(
        !panel(&app)?.open,
        "furniture rotation has no Bevy shortcut"
    );
    while let Ok(command) = commands.try_recv() {
        if let PlayCommand::Feature(command) = command {
            assert_ne!(command.capability, 445);
        }
    }
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(panel(&app)?.open);
    let _ = request(&mut commands)?;
    for key in [KeyCode::ControlLeft, KeyCode::AltLeft, KeyCode::ShiftLeft] {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
    }
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::KeyF,
            logical_key: bevy::input::keyboard::Key::Character("F".into()),
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert!(
        panel(&app)?.open,
        "the former shortcut must not close the window"
    );
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
    assert!(!panel(&app)?.open);
    Ok(())
}

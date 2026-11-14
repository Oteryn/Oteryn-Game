use super::super::GameModal;
use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::asset::AssetPlugin;
use oteryn_client_runtime::raw_feature::{
    CommandStatus, FeatureCommand, GatedCommandResult, house_wrap as wire,
};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture(selected: bool) -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    if selected {
        outbox.furniture_wrapping_test_selected();
    }
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn click(app: &mut App, kind: Kind) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("control absent")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
fn toggle(app: &mut App) {
    super::toggle(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
}
fn panel(app: &App) -> Panel {
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .map(|play| play.link.house_furniture_panel())
        .unwrap_or_default()
}
fn command(
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> std::result::Result<FeatureCommand, Box<dyn std::error::Error>> {
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("wrong command".into());
    };
    assert_eq!((command.capability, command.kind), (207, 402));
    Ok(command)
}
fn catalogue() -> wire::Catalogue {
    wire::Catalogue {
        page: 0,
        more: true,
        acl_revision: 4,
        position: [100, 101, 7],
        entries: vec![
            wire::CatalogueEntry {
                item: [7; 16],
                revision: 3,
                action: wire::Action::Wrap,
                name: "Server oven".into(),
            },
            wire::CatalogueEntry {
                item: [8; 16],
                revision: 9,
                action: wire::Action::Unwrap,
                name: "Server kit".into(),
            },
        ],
    }
}
fn reply(outbox: &Outbox, command: &FeatureCommand, payload: Vec<u8>) {
    outbox.furniture_wrapping_test_reply(
        command,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: CommandStatus::Accepted,
            payload,
        },
    );
}
fn load(
    app: &mut App,
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    outbox: &Outbox,
) -> Result {
    toggle(app);
    let query = command(commands)?;
    assert_eq!(wire::decode_catalogue_query(&query.payload), Some(0));
    reply(outbox, &query, catalogue().encode().ok_or("catalogue")?);
    app.update();
    Ok(())
}
#[test]
fn furniture_wrapping_window_shows_server_data_only() -> Result {
    let (mut app, mut commands, outbox) = fixture(true)?;
    toggle(&mut app);
    assert!(panel(&app).catalogue.is_none());
    let query = command(&mut commands)?;
    reply(&outbox, &query, catalogue().encode().ok_or("catalogue")?);
    app.update();
    let expected = oteryn_client_ui::house_furniture_wrap::panel_text(&panel(&app));
    let text = app
        .world_mut()
        .query::<(&Name, &Text)>()
        .iter(app.world())
        .find_map(|(name, text)| {
            (name.as_str() == "furniture_wrapping.catalogue").then_some(text.0.clone())
        })
        .ok_or("catalogue text")?;
    assert_eq!(text, expected);
    assert!(text.contains("Server oven — Wrap") && text.contains("Server kit — Place"));
    assert_eq!(panel(&app).catalogue, Some(catalogue()));
    let (mut unavailable, mut commands, _) = fixture(false)?;
    toggle(&mut unavailable);
    assert!(panel(&unavailable).open);
    assert!(!panel(&unavailable).selected);
    assert!(panel(&unavailable).catalogue.is_none());
    assert!(commands.try_recv().is_err());

    Ok(())
}
#[test]
fn furniture_wrapping_actions_are_bound_and_confirmed() -> Result {
    for index in [0, 1] {
        let (mut app, mut commands, outbox) = fixture(true)?;
        load(&mut app, &mut commands, &outbox)?;
        click(&mut app, Kind::Select(index))?;
        click(&mut app, Kind::Request(Intent::Apply))?;
        assert!(commands.try_recv().is_err());
        assert!(!panel(&app).pending);
        assert!(panel(&app).feedback.contains("again to confirm"));
        click(&mut app, Kind::Request(Intent::Apply))?;
        let sent = command(&mut commands)?;
        assert_eq!(
            sent.payload,
            catalogue().request(index).ok_or("target")?.encode()
        );
        assert!(panel(&app).pending);
        assert_eq!(panel(&app).catalogue, Some(catalogue()));
        let pending = panel(&app);
        click(&mut app, Kind::Request(Intent::Apply))?;
        assert_eq!(panel(&app), pending);
        assert!(commands.try_recv().is_err());
        reply(&outbox, &sent, wire::Reply::Committed(10).encode());
        app.update();
        assert!(panel(&app).catalogue.is_none());
        assert!(panel(&app).feedback.contains("revision 10"));
    }
    for (intent, page) in [
        (Intent::Refresh, 0),
        (Intent::Next, 1),
        (Intent::Previous, 0),
    ] {
        let (mut app, mut commands, outbox) = fixture(true)?;
        load(&mut app, &mut commands, &outbox)?;
        click(&mut app, Kind::Request(intent))?;
        assert_eq!(
            wire::decode_catalogue_query(&command(&mut commands)?.payload),
            Some(page)
        );
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
#[test]
fn furniture_wrapping_rejected_when_session_replaced_or_target_changed() -> Result {
    let (mut app, mut commands, outbox) = fixture(true)?;
    load(&mut app, &mut commands, &outbox)?;
    click(&mut app, Kind::Select(0))?;
    click(&mut app, Kind::Request(Intent::Apply))?;
    let stale = app
        .world()
        .resource::<State>()
        .snapshot
        .clone()
        .ok_or("snapshot")?;
    click(&mut app, Kind::Select(1))?;
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let before = link.house_furniture_panel();
    assert!(!link.house_furniture_dispatch_bound(&stale.1, Action::Request(Intent::Apply), false));
    assert_eq!(link.house_furniture_panel(), before);
    assert!(!link.house_furniture_dispatch_bound(
        &link.house_furniture_snapshot(),
        Action::Request(Intent::Apply),
        true
    ));
    assert_eq!(link.house_furniture_panel(), before);
    // Identical visible state in another session must still reject the old consent.
    let (mut replacement, mut replacement_commands, replacement_outbox) = fixture(true)?;
    load(
        &mut replacement,
        &mut replacement_commands,
        &replacement_outbox,
    )?;
    click(&mut replacement, Kind::Select(0))?;
    click(&mut replacement, Kind::Request(Intent::Apply))?;
    assert_eq!(panel(&replacement), stale.1.panel);
    let replacement_owner = replacement.world().non_send::<HostOwner>();
    let other = &replacement_owner
        .play
        .as_ref()
        .ok_or("replacement play")?
        .link;
    assert!(!other.house_furniture_dispatch_bound(&stale.1, Action::Request(Intent::Apply), false));
    assert_eq!(other.house_furniture_panel(), stale.1.panel);
    assert!(replacement_commands.try_recv().is_err());
    let overlay = app
        .world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(999)))
        .id();
    let before = panel(&app);
    click(&mut app, Kind::Request(Intent::Apply))?;
    assert_eq!(panel(&app), before);
    assert!(commands.try_recv().is_err());
    app.world_mut().despawn(overlay);
    app.update();
    click(&mut app, Kind::Close)?;
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    assert!(!link.house_furniture_dispatch_bound(&stale.1, Action::Request(Intent::Apply), false));
    outbox.furniture_wrapping_test_end();
    let ended = link.house_furniture_panel();
    assert!(!link.house_furniture_dispatch_bound(
        &link.house_furniture_snapshot(),
        Action::Request(Intent::Refresh),
        false
    ));
    assert!(!link.house_furniture_dispatch_bound(
        &link.house_furniture_snapshot(),
        Action::Toggle,
        false
    ));
    assert_eq!(link.house_furniture_panel(), ended);
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn furniture_wrapping_window_registered() -> Result {
    let (mut app, mut commands, _) = fixture(true)?;
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            (control.id() == "shell.furniture_wrapping").then_some(entity)
        })
        .ok_or("toolbar entry")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(panel(&app).open);
    command(&mut commands)?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(!panel(&app).open);
    app.world_mut()
        .resource_mut::<Messages<bevy::input::keyboard::KeyboardInput>>()
        .write(bevy::input::keyboard::KeyboardInput {
            key_code: SHORTCUT,
            logical_key: bevy::input::keyboard::Key::F8,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert!(panel(&app).open);
    click(&mut app, Kind::Close)?;
    assert!(!panel(&app).open);
    Ok(())
}

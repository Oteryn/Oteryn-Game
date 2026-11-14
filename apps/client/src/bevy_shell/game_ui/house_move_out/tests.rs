use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::house_move_out as wire;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> Result<Fixture> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_house_move_out_panel(Panel {
        selected: true,
        fields: ["oteryn:content.house.test".into(), "2000000000000".into()],
        ..Default::default()
    });
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
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    Ok((app, commands, outbox))
}
fn panel(app: &App) -> Result<Panel> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .house_move_out_panel())
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
    Ok(())
}
fn request(commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>, action: u8) -> Result {
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("wrong command".into());
    };
    assert_eq!(command.capability, 59);
    assert_eq!(command.kind, if action == 0 { 77 } else { 76 });
    let decoded =
        wire::decode_house_move_out_request(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(decoded.house, "oteryn:content.house.test");
    assert_eq!(decoded.action, action);
    assert_eq!(
        decoded.due_at_unix_ms,
        if action == 1 { 2000000000000 } else { 0 }
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn house_move_out_window_shows_server_data_only() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    assert!(panel(&app)?.summary.is_empty());
    click(&mut app, Kind::Request(Intent::Query))?;
    request(&mut commands, 0)?;
    let mut operation = [0; 16];
    operation[6] = 0x70;
    operation[8] = 0x80;
    operation[15] = 1;
    let reply = wire::HouseMoveOutReply {
        operation: Some(operation),
        state: 2,
        due_at_unix_ms: 2000000000000,
        census_count: 37,
        delivered: 12,
    };
    outbox.bevy_house_move_out_reply(
        &wire::encode_house_move_out_reply(&reply).map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    let expected = oteryn_client_ui::house_move_out::panel_text(&panel(&app)?);
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == expected)
    );
    assert!(expected.contains("delivered 12/37"));
    assert_eq!(
        panel(&app)?.fields,
        ["oteryn:content.house.test", "2000000000000"]
    );
    Ok(())
}
#[test]
fn house_move_out_actions_are_bound_and_confirmed() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    let before = panel(&app)?;
    click(&mut app, Kind::Request(Intent::Notice))?;
    click(&mut app, Kind::Back)?;
    assert_eq!(panel(&app)?, before);
    assert!(commands.try_recv().is_err());
    for (intent, action) in [(Intent::Query, 0), (Intent::Notice, 1), (Intent::Cancel, 2)] {
        let before = panel(&app)?;
        click(&mut app, Kind::Request(intent))?;
        if intent != Intent::Query {
            assert!(commands.try_recv().is_err());
            assert_eq!(panel(&app)?, before);
            click(&mut app, Kind::Confirm)?;
        }
        request(&mut commands, action)?;
        assert!(panel(&app)?.pending);
        assert_eq!(panel(&app)?.summary, before.summary);
        click(&mut app, Kind::Request(intent))?;
        assert!(commands.try_recv().is_err());
        outbox.bevy_house_move_out_reply(
            &wire::encode_house_move_out_reply(&wire::HouseMoveOutReply {
                operation: None,
                state: 0,
                due_at_unix_ms: 0,
                census_count: 0,
                delivered: 0,
            })
            .map_err(|e| format!("{e:?}"))?,
        );
        app.update();
        assert!(!panel(&app)?.pending);
    }
    Ok(())
}
#[test]
fn house_move_out_rejected_when_session_replaced_or_target_changed() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    click(&mut app, Kind::Request(Intent::Notice))?;
    let mut changed = panel(&app)?;
    changed.fields[0] = "oteryn:content.house.changed".into();
    outbox.bevy_house_move_out_panel(changed.clone());
    // An observer may run before the next projection; it must re-prove the owner.
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == Kind::Confirm).then_some(e))
        .ok_or("confirm")?;
    app.world_mut().trigger(Activate { entity });
    assert!(commands.try_recv().is_err());
    assert_eq!(panel(&app)?, changed);
    app.update();
    click(&mut app, Kind::Request(Intent::Cancel))?;
    let old_snapshot = app
        .world()
        .resource::<State>()
        .snapshot
        .clone()
        .ok_or("snapshot")?;
    let (link, mut replacement_commands, replacement_outbox) = play_channel();
    link.use_shared_frontend();
    replacement_outbox.bevy_house_move_out_panel(panel(&app)?);
    assert!(!link.house_move_out_dispatch_bound(
        &old_snapshot,
        Action::Request {
            intent: Intent::Cancel,
            fields: old_snapshot.panel.fields.clone(),
            confirmed: true,
        }
    ));
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == Kind::Confirm).then_some(e))
        .ok_or("confirm")?;
    app.world_mut().trigger(Activate { entity });
    assert!(replacement_commands.try_recv().is_err());
    assert!(commands.try_recv().is_err());
    app.update();
    click(&mut app, Kind::Request(Intent::Cancel))?;
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    click(&mut app, Kind::Confirm)?;
    assert!(replacement_commands.try_recv().is_err());
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    let snapshot = link.house_move_out_snapshot();
    assert!(link.house_move_out_dispatch_bound(&snapshot, Action::Close));
    let closed = link.house_move_out_panel();
    assert!(!link.house_move_out_dispatch_bound(
        &snapshot,
        Action::Request {
            intent: Intent::Cancel,
            fields: snapshot.panel.fields.clone(),
            confirmed: true,
        }
    ));
    assert_eq!(link.house_move_out_panel(), closed);
    assert!(link.house_move_out_toggle_bound());
    let snapshot = link.house_move_out_snapshot();
    let _modal = link.bevy_server_modal_fixture();
    assert!(!link.house_move_out_dispatch_bound(
        &snapshot,
        Action::Request {
            intent: Intent::Cancel,
            fields: snapshot.panel.fields.clone(),
            confirmed: true,
        }
    ));
    assert_eq!(link.house_move_out_panel(), snapshot.panel);
    drop(_modal);
    drop(replacement_commands);
    assert!(!link.house_move_out_dispatch_bound(&snapshot, Action::Close));
    assert_eq!(link.house_move_out_panel(), snapshot.panel);
    Ok(())
}
#[test]
fn house_move_out_window_registered() -> Result {
    let (mut app, mut commands, _) = fixture()?;
    assert!(
        app.world_mut()
            .query::<&crate::bevy_shell::ui::Control>()
            .iter(app.world())
            .any(|control| control.id() == "shell.house_move_out")
    );
    click(&mut app, Kind::Close)?;
    assert!(!panel(&app)?.open);
    assert!(app.world().resource::<State>().root.is_none());
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.id() == "shell.house_move_out").then_some(entity))
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(panel(&app)?.open);
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::F24,
            logical_key: bevy::input::keyboard::Key::F24,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert!(!panel(&app)?.open);
    assert!(app.world().resource::<State>().root.is_none());
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::F24,
            logical_key: bevy::input::keyboard::Key::F24,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert!(panel(&app)?.open);
    assert!(app.world().resource::<State>().root.is_some());
    assert!(commands.try_recv().is_err());
    Ok(())
}

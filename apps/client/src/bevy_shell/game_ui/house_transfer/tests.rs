use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{
    asset::AssetPlugin,
    input::keyboard::{Key, KeyboardInput},
    input_focus::InputFocus,
};
use oteryn_client_runtime::{house_transfer as wire, raw_feature::FeatureCommand};
use oteryn_session::{CommandStatus, GatedCommandResult};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);

fn fixture() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let (link, receiver, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_house_transfer_selected(true);
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
    Ok((app, receiver, outbox))
}
fn panel(app: &App) -> Panel {
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .map(|play| play.link.house_transfer_panel())
        .unwrap_or_default()
}
fn open(app: &mut App) {
    super::open(app.world().non_send::<HostOwner>());
    app.update();
}
fn control(app: &mut App, kind: Kind) -> std::result::Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("house transfer control missing".into())
}
fn click(app: &mut App, kind: Kind) -> Result {
    let entity = control(app, kind)?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    app.update();
    Ok(())
}
fn draft(app: &mut App, value: &str) -> Result {
    let mut fields = app
        .world_mut()
        .query_filtered::<&mut EditableText, With<HouseKey>>();
    let mut field = fields.single_mut(app.world_mut())?;
    *field = EditableText::new(value);
    Ok(())
}
fn text(app: &mut App) -> String {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
fn response(outbox: &Outbox, command: &FeatureCommand, status: CommandStatus) -> Result {
    let request =
        wire::decode_house_transfer_intent(&command.payload).map_err(|e| format!("{e:?}"))?;
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id[15] = 1;
    let payload = wire::encode_house_transfer_ack(&wire::HouseTransferAck {
        transfer: id,
        session: id,
        world: id,
        channel: id,
        house: (!request.exit).then_some(request.house),
        scope_generation: 2,
        connection_generation: 1,
        lease_generation: 1,
        selection: vec![86],
        position: [121, 234, 7],
    })
    .map_err(|e| format!("{e:?}"))?;
    outbox.bevy_house_transfer_response(
        command,
        &GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status,
            payload,
        },
    );
    Ok(())
}
fn feature(
    receiver: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> std::result::Result<FeatureCommand, Box<dyn std::error::Error>> {
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("wrong command".into());
    };
    assert_eq!((command.capability, command.kind), (86, 102));
    Ok(command)
}
fn key(app: &mut App, key_code: KeyCode) {
    app.world_mut().write_message(KeyboardInput {
        key_code,
        logical_key: Key::Unidentified(bevy::input::keyboard::NativeKey::Unidentified),
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.update();
}

#[test]
fn house_transfer_window_shows_server_data_only() -> Result {
    let (mut app, mut receiver, outbox) = fixture()?;
    open(&mut app);
    assert!(panel(&app).summary.is_empty());
    assert!(text(&mut app).contains("House key"));
    draft(&mut app, "oteryn:content.house.test")?;
    click(&mut app, Kind::Request(Intent::Enter))?;
    click(&mut app, Kind::Confirm)?;
    let command = feature(&mut receiver)?;
    assert!(panel(&app).summary.is_empty());
    assert!(text(&mut app).contains("Waiting for the server"));
    response(&outbox, &command, CommandStatus::Accepted)?;
    app.update();
    let owner = panel(&app);
    assert_eq!(
        owner.summary,
        "oteryn:content.house.test at 121,234,7; scope 2"
    );
    assert!(text(&mut app).contains(&oteryn_client_ui::house_transfer::panel_text(&owner)));
    draft(&mut app, "oteryn:content.house.other")?;
    click(&mut app, Kind::Request(Intent::Enter))?;
    click(&mut app, Kind::Confirm)?;
    let refused = feature(&mut receiver)?;
    response(&outbox, &refused, CommandStatus::Rejected)?;
    app.update();
    assert_eq!(panel(&app).summary, owner.summary);
    assert!(text(&mut app).contains("The server refused"));
    Ok(())
}

#[test]
fn house_transfer_actions_are_bound_and_confirmed() -> Result {
    let (mut app, mut receiver, outbox) = fixture()?;
    open(&mut app);
    for intent in [Intent::Enter, Intent::Exit] {
        draft(&mut app, "oteryn:content.house.test")?;
        let before_review = panel(&app);
        click(&mut app, Kind::Request(intent))?;
        assert_eq!(panel(&app), before_review);
        assert!(receiver.try_recv().is_err(), "review must not send");
        assert!(!before_review.pending);
        let action = match intent {
            Intent::Enter => "Enter",
            Intent::Exit => "Exit",
        };
        assert!(
            text(&mut app).contains(&format!("Confirm {action} for oteryn:content.house.test?"))
        );
        click(&mut app, Kind::Cancel)?;
        assert_eq!(panel(&app), before_review);
        assert!(receiver.try_recv().is_err(), "cancel must not send");
        click(&mut app, Kind::Request(intent))?;
        assert!(receiver.try_recv().is_err());
        let confirm = control(&mut app, Kind::Confirm)?;
        app.world_mut().trigger(Activate { entity: confirm });
        app.world_mut().trigger(Activate { entity: confirm });
        app.update();
        app.update();
        let command = feature(&mut receiver)?;
        assert!(receiver.try_recv().is_err(), "consent can send only once");
        let request =
            wire::decode_house_transfer_intent(&command.payload).map_err(|e| format!("{e:?}"))?;
        assert_eq!(request.house, "oteryn:content.house.test");
        assert_eq!(request.exit, intent == Intent::Exit);
        let before = panel(&app);
        assert!(before.pending);
        click(&mut app, Kind::Request(intent))?;
        assert!(receiver.try_recv().is_err());
        assert_eq!(panel(&app), before);
        response(&outbox, &command, CommandStatus::Accepted)?;
        app.update();
        assert!(!panel(&app).pending);
        assert!(text(&mut app).contains("Transfer committed"));
    }
    assert!(panel(&app).summary.starts_with("Channel at 121,234,7"));
    Ok(())
}

#[test]
fn house_transfer_confirmation_rejects_changed_target_and_modal() -> Result {
    let (mut app, mut receiver, _outbox) = fixture()?;
    open(&mut app);
    draft(&mut app, "oteryn:content.house.test")?;
    click(&mut app, Kind::Request(Intent::Enter))?;
    let confirm = control(&mut app, Kind::Confirm)?;
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(220), Node::default()))
        .id();
    let before = panel(&app);
    app.world_mut().trigger(Activate { entity: confirm });
    app.update();
    assert!(receiver.try_recv().is_err());
    assert_eq!(panel(&app), before);
    app.world_mut().entity_mut(higher).despawn();
    app.update();
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .house_transfer_edit(0, "oteryn:content.house.changed".into());
    let changed = panel(&app);
    app.world_mut().trigger(Activate { entity: confirm });
    app.update();
    assert!(receiver.try_recv().is_err());
    assert_eq!(panel(&app), changed);
    assert!(control(&mut app, Kind::Confirm).is_err());
    Ok(())
}

#[test]
fn house_transfer_rejected_when_session_replaced_or_target_changed() -> Result {
    let (mut app, mut receiver, outbox) = fixture()?;
    open(&mut app);
    draft(&mut app, "oteryn:content.house.test")?;
    let stale = control(&mut app, Kind::Request(Intent::Enter))?;
    {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        let expected = link.house_transfer_snapshot();
        link.house_transfer_edit(0, "oteryn:content.house.changed".into());
        let changed = link.house_transfer_panel();
        assert!(!link.house_transfer_dispatch_bound(
            &expected,
            Some(Intent::Enter),
            "oteryn:content.house.test"
        ));
        assert_eq!(link.house_transfer_panel(), changed);
    }
    app.world_mut().trigger(Activate { entity: stale });
    assert!(receiver.try_recv().is_err());
    app.update();
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(220), Node::default()))
        .id();
    let before = panel(&app);
    click(&mut app, Kind::Request(Intent::Enter))?;
    assert_eq!(panel(&app), before);
    assert!(receiver.try_recv().is_err());
    app.world_mut().entity_mut(higher).despawn();
    app.update();
    {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        let expected = link.house_transfer_snapshot();
        let _modal = link.bevy_server_modal_fixture();
        assert!(!link.house_transfer_dispatch_bound(
            &expected,
            Some(Intent::Enter),
            "oteryn:content.house.test"
        ));
        assert_eq!(link.house_transfer_panel(), before);
        let (replacement, _receiver, events) = play_channel();
        events.bevy_house_transfer_selected(true);
        replacement.house_transfer_open();
        replacement.house_transfer_edit(0, expected.panel.fields[0].clone());
        assert_eq!(replacement.house_transfer_panel(), expected.panel);
        assert!(!replacement.house_transfer_dispatch_bound(
            &expected,
            Some(Intent::Enter),
            "oteryn:content.house.test"
        ));
    }
    let stale = control(&mut app, Kind::Request(Intent::Enter))?;
    let old = app.world_mut().non_send_mut::<HostOwner>().play.take();
    app.world_mut().trigger(Activate { entity: stale });
    assert!(receiver.try_recv().is_err());
    drop(old);
    outbox.bevy_house_transfer_end();
    app.update();
    assert!(!panel(&app).open);
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        0
    );
    Ok(())
}

#[test]
fn house_transfer_window_registered() -> Result {
    let registrations = include_str!("../mod.rs");
    assert!(registrations.contains("pub(super) mod house_transfer;"));
    assert!(registrations.contains("house_transfer::install(app);"));
    let (mut app, mut receiver, _outbox) = fixture()?;
    let button = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.id() == "shell.house_transfer").then_some(entity))
        .ok_or("toolbar entry")?;
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(panel(&app).open);
    key(&mut app, KeyCode::Escape);
    assert!(!panel(&app).open);
    let event = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(22)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Alt),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(std::slice::from_ref(&event), true);
    app.update();
    assert!(!panel(&app).open);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[event], false);
    app.update();
    assert!(!panel(&app).open);
    key(&mut app, KeyCode::Escape);
    assert!(!panel(&app).open);
    key(&mut app, KeyCode::F8);
    assert!(!panel(&app).open);
    assert!(receiver.try_recv().is_err());
    Ok(())
}

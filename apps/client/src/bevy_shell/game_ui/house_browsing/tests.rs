use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};
use oteryn_protocol_oteryn::house as wire;
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, queue, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.house_browsing_initialize();
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
    Ok((app, queue, outbox))
}
fn link(app: &App) -> Result<&crate::play::PlayLink, Box<dyn std::error::Error>> {
    Ok(&app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link)
}
fn choose(app: &mut App, kind: Kind) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == kind).then_some(e))
        .ok_or("house control missing")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn field(app: &mut App, index: usize, value: &str) {
    let world = app.world_mut();
    let mut query = world.query::<(&Field, &mut EditableText)>();
    for (field, mut text) in query.iter_mut(world) {
        if field.0 == index {
            text.editor_mut().set_text(value);
            text.pending_edits.clear();
        }
    }
}
fn request(
    queue: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<FeatureCommand, Box<dyn std::error::Error>> {
    let PlayCommand::Feature(command) = queue.try_recv()? else {
        return Err("wrong dispatch".into());
    };
    Ok(command)
}
fn reply(outbox: &Outbox, command: &FeatureCommand, payload: Vec<u8>) {
    outbox.house_browsing_reply(
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
    queue: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    outbox: &Outbox,
) -> TestResult {
    link(app)?.house_open();
    app.update();
    field(app, 0, "01010101010171018101010101010101");
    field(app, 1, "oteryn:content.area.city.thais");
    choose(app, Kind::Request(0))?;
    let query = request(queue)?;
    let rows = [wire::HouseSummary {
        details: Some(wire::HouseDetails {
            name: "Server Harbour".into(),
            size: 73,
            beds: 2,
            entrance: (123, 456, 0),
        }),
        house: "oteryn:content.house.harbour".into(),
        kind: wire::HouseKind::PrivateHouse,
        state: 1,
        rent: 555,
        revision: 4,
        acl_revision: 9,
        owned: true,
        move_out_ms: None,
        price: 12345,
        ends_ms: None,
        own_maximum: None,
    }];
    reply(
        outbox,
        &query,
        wire::encode_house_page(&rows, None, true).map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    Ok(())
}
#[test]
fn house_browsing_window_shows_server_data_only() -> TestResult {
    let (mut app, mut queue, outbox) = fixture()?;
    load(&mut app, &mut queue, &outbox)?;
    let model = link(&app)?.house_panel().model;
    let text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains(&oteryn_client_ui::house::panel_text(&model)));
    assert!(text.contains("Server Harbour"));
    assert!(text.contains("12345"));
    field(&mut app, 2, "1");
    choose(&mut app, Kind::Map)?;
    app.update();
    let text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("123,456,0"));
    assert!(queue.try_recv().is_err());
    Ok(())
}
#[test]
fn house_browsing_actions_are_bound_and_confirmed() -> TestResult {
    let (mut app, mut queue, outbox) = fixture()?;
    load(&mut app, &mut queue, &outbox)?;
    field(&mut app, 2, "1");
    field(&mut app, 3, "23456");
    choose(&mut app, Kind::Request(2))?;
    assert!(queue.try_recv().is_err());
    field(&mut app, 6, "CONFIRM");
    choose(&mut app, Kind::Request(2))?;
    let command = request(&mut queue)?;
    let intent = wire::decode_house_intent(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(intent.action, wire::HouseAction::Bid(23456));
    assert_eq!(intent.house, "oteryn:content.house.harbour");
    assert_eq!(link(&app)?.house_panel().model.page.rows[0].price, 12345);
    choose(&mut app, Kind::Request(2))?;
    assert!(queue.try_recv().is_err());
    reply(
        &outbox,
        &command,
        wire::encode_house_result(wire::HouseDisposition::Ok, 10),
    );
    app.update();
    field(&mut app, 2, "1");
    choose(&mut app, Kind::Request(11))?;
    let query = request(&mut queue)?;
    reply(
        &outbox,
        &query,
        wire::encode_house_acl_snapshot(&wire::HouseAclSnapshot {
            revision: 12,
            members: vec![[2, 2, 2, 2, 2, 2, 0x72, 2, 0x82, 2, 2, 2, 2, 2, 2, 2]],
        })
        .map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    field(&mut app, 6, "CONFIRM");
    choose(&mut app, Kind::Request(3))?;
    let command = request(&mut queue)?;
    assert!(matches!(
        wire::decode_house_intent(&command.payload)
            .map_err(|e| format!("{e:?}"))?
            .action,
        wire::HouseAction::Acl { revision: 12, .. }
    ));
    Ok(())
}
#[test]
fn house_browsing_window_registered() -> TestResult {
    let mut registered = App::new();
    super::super::install(&mut registered);
    assert!(registered.world().contains_resource::<State>());
    let (mut app, _queue, _outbox) = fixture()?;
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.id() == "shell.house_browsing").then_some(e))
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(link(&app)?.house_panel().model.open);
    choose(&mut app, Kind::Close)?;
    app.update();
    assert!(!link(&app)?.house_panel().model.open);
    let event = bevy::input::keyboard::KeyboardInput {
        key_code: KeyCode::F8,
        logical_key: bevy::input::keyboard::Key::Unidentified(
            bevy::input::keyboard::NativeKey::Unidentified,
        ),
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    };
    app.world_mut().write_message(event);
    app.update();
    assert!(link(&app)?.house_panel().model.open);
    Ok(())
}

#[test]
fn house_browsing_frontend_rejects_stale_controls_and_other_modal() -> TestResult {
    let (mut app, mut queue, outbox) = fixture()?;
    load(&mut app, &mut queue, &outbox)?;
    let before = link(&app)?.house_browsing_snapshot();
    let modal = app
        .world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(300)))
        .id();
    choose(&mut app, Kind::Request(0))?;
    assert_eq!(link(&app)?.house_browsing_snapshot(), before);
    assert!(queue.try_recv().is_err());
    app.world_mut().despawn(modal);
    app.update();
    let (replacement, mut replacement_queue, replacement_outbox) = play_channel();
    replacement_outbox.house_browsing_initialize();
    replacement.house_open();
    let replacement_before = replacement.house_browsing_snapshot();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = replacement;
    choose(&mut app, Kind::Request(0))?;
    assert_eq!(link(&app)?.house_browsing_snapshot(), replacement_before);
    assert!(replacement_queue.try_recv().is_err());
    Ok(())
}

#[test]
fn house_browsing_ownership_form_confirms_server_selection() -> TestResult {
    use oteryn_protocol_oteryn::house_sale as sale;
    let (mut app, mut queue, outbox) = fixture()?;
    link(&app)?.house_open();
    app.update();
    choose(&mut app, Kind::Ownership)?;
    let query = request(&mut queue)?;
    let data = sale::HouseSaleReply {
        now_unix_ms: 1_800_000_000_000,
        has_more: false,
        rows: vec![sale::HouseSaleRow {
            house: "oteryn:content.house.harbour".into(),
            name: "Server Harbour".into(),
            revision: 7,
            owner: "Elara Vale".into(),
            recipient: String::new(),
            transfer_state: 0,
            due_at_unix_ms: 0,
            price: 0,
            rent: 50,
            allowed: 10,
            move_out_state: 0,
        }],
    };
    reply(
        &outbox,
        &query,
        sale::encode_reply(&data).map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    field(&mut app, 7, "Mira Vale");
    field(
        &mut app,
        8,
        &oteryn_client_ui::house::format_ownership_date(1_800_172_800_000),
    );
    field(&mut app, 9, "200");
    choose(&mut app, Kind::Review(3))?;
    assert!(queue.try_recv().is_err());
    app.update();
    let text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for term in ["Server Harbour", "Mira Vale", "200 gold", "50 gold rent"] {
        assert!(text.contains(term));
    }
    choose(&mut app, Kind::Confirm)?;
    let command = request(&mut queue)?;
    let intent = sale::decode_request(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (
            intent.action,
            intent.expected_revision,
            intent.recipient.as_str(),
            intent.price
        ),
        (3, 7, "Mira Vale", 200)
    );
    assert_eq!(
        link(&app)?.house_ownership_panel().model.data(),
        Some(&data)
    );
    choose(&mut app, Kind::Confirm)?;
    assert!(queue.try_recv().is_err());
    reply(
        &outbox,
        &command,
        sale::encode_reply(&data).map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    choose(&mut app, Kind::Back)?;
    app.update();
    assert!(!link(&app)?.house_ownership_panel().open);
    Ok(())
}

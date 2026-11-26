use super::*;
use crate::{
    bevy_shell::owner::ActivePlay,
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::asset::AssetPlugin;
use oteryn_client_runtime::{
    cyclopedia_character::items::{Page, Row},
    raw_feature::{CommandStatus, GatedCommandResult},
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Receiver = tokio::sync::mpsc::UnboundedReceiver<PlayCommand>;
fn fixture(selected: bool) -> Result<(App, Receiver, Outbox), Box<dyn std::error::Error>> {
    let (link, rx, outbox) = play_channel();
    outbox.bevy_cyclopedia_initialize(selected, false, false);
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    link.use_shared_frontend();
    owner.play = Some(ActivePlay { view, link });
    owner.phase = Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .store_inbox_set_open(true);
    app.update();
    Ok((app, rx, outbox))
}
fn request(rx: &mut Receiver) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let PlayCommand::Feature(c) = rx.try_recv()? else {
        return Err("expected summary".into());
    };
    assert_eq!((c.capability, c.kind), (199, 388));
    Ok(c.payload)
}
fn reply(outbox: &Outbox, page: Page) -> TestResult {
    outbox.bevy_cyclopedia_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: page.encode().map_err(|e| format!("{e:?}"))?,
    });
    Ok(())
}
fn texts(app: &mut App) -> Vec<String> {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect()
}
fn button(app: &mut App, kind: Kind) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == kind).then_some(e))
        .ok_or("button absent".into())
}
fn click(app: &mut App, kind: Kind) -> TestResult {
    let entity = button(app, kind)?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
#[test]
fn store_inbox_projects_confirmed_quantities_and_pages_remote_summary() -> TestResult {
    let (mut app, mut rx, outbox) = fixture(true)?;
    assert_eq!(request(&mut rx)?, [130, 0, 0]);
    assert!(
        texts(&mut app)
            .iter()
            .any(|t| t == "Waiting for the server…")
    );
    let rows = (0..24)
        .map(|i| Row {
            definition: format!("oteryn:item/test-{i:02}"),
            quantities: [5, 0, 0, if i == 1 { 7 } else { 0 }],
        })
        .collect();
    reply(
        &outbox,
        Page {
            offset: 0,
            total: 25,
            rows,
        },
    )?;
    app.update();
    let text = texts(&mut app);
    assert!(text.iter().any(|t| t == "oteryn:item/test-01 × 7"));
    assert!(!text.iter().any(|t| t.contains("test-00")));
    click(&mut app, Kind::Next)?;
    assert_eq!(request(&mut rx)?, [130, 24, 0]);
    click(&mut app, Kind::Next)?;
    assert!(rx.try_recv().is_err());
    reply(
        &outbox,
        Page {
            offset: 24,
            total: 25,
            rows: vec![Row {
                definition: "oteryn:item/z-last".into(),
                quantities: [1, 0, 0, 0],
            }],
        },
    )?;
    app.update();
    assert!(
        texts(&mut app)
            .iter()
            .any(|t| t == "No inbox items on this summary page.")
    );
    let next = button(&mut app, Kind::Next)?;
    assert!(app.world().get::<InteractionDisabled>(next).is_some());
    click(&mut app, Kind::Previous)?;
    assert_eq!(request(&mut rx)?, [130, 0, 0]);
    // A refusal keeps confirmed rows and exposes the actual owner error.
    outbox.bevy_cyclopedia_result(&GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: CommandStatus::Rejected,
        payload: vec![],
    });
    app.update();
    assert!(texts(&mut app).iter().any(|t| t.contains("server refused")));
    click(&mut app, Kind::Refresh)?;
    assert_eq!(request(&mut rx)?, [130, 0, 0]);
    click(&mut app, Kind::Close)?;
    assert!(app.world().resource::<State>().root.is_none());
    assert!(rx.try_recv().is_err());
    Ok(())
}
#[test]
fn store_inbox_unavailable_session_and_stale_controls_send_nothing() -> TestResult {
    let (mut app, mut rx, _) = fixture(false)?;
    assert!(rx.try_recv().is_err());
    assert!(
        texts(&mut app)
            .iter()
            .any(|t| t == "Inbox summary is unavailable in this session.")
    );
    click(&mut app, Kind::Refresh)?;
    assert!(rx.try_recv().is_err());
    let close = button(&mut app, Kind::Close)?;
    let mut stale = *app.world().get::<Control>(close).ok_or("close")?;
    stale.generation = stale.generation.wrapping_add(1);
    let entity = app.world_mut().spawn(stale).observe(activate).id();
    app.world_mut().trigger(Activate { entity });
    assert!(app.world().resource::<State>().root.is_some());
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    assert!(rx.try_recv().is_err());
    Ok(())
}

fn store_command(rx: &mut Receiver) -> Result<(u32, Vec<u8>), Box<dyn std::error::Error>> {
    let PlayCommand::ServerMechanic(command) = rx.try_recv()? else {
        return Err("expected Store command".into());
    };
    assert_eq!(command.capability, wire::CAPABILITY_STORE_V1);
    Ok((command.command_type, command.payload))
}

fn item_reply(outbox: &Outbox, payload: Vec<u8>) {
    use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
    outbox.bevy_store_reply(&ServerMechanicResponse {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload,
    });
}

fn inbox_entry(key: [u8; 16], item_id: u8) -> Vec<u8> {
    let mut row = vec![10, 20, 10, 16];
    row.extend(key);
    row.extend([16, item_id]);
    row
}

fn inbox_key(ordinal: u8) -> [u8; 16] {
    [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, ordinal]
}

#[test]
fn store_inbox_loads_claim_button_after_delayed_store_browse() -> TestResult {
    let (mut app, mut rx, outbox) = fixture(false)?;
    outbox.bevy_store_initialize();
    {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        link.store_inbox_set_open(false);
        link.store_set_open(true);
        assert_eq!(store_command(&mut rx)?.0, wire::browse::COMMAND_TYPE);
        link.store_set_open(false);
        link.store_inbox_set_open(true);
    }
    app.update();
    assert!(rx.try_recv().is_err());
    assert!(button(&mut app, Kind::Claim(0)).is_err());

    let browse = wire::browse::Page {
        categories: Vec::new(),
        products: Vec::new(),
        next: String::new(),
        total: 0,
        transferable: 0,
        transferable_known: true,
    };
    item_reply(&outbox, browse.encode().map_err(|e| format!("{e:?}"))?);
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let play = owner.play.as_mut().ok_or("play")?;
        play.view.tick(&play.link).map_err(|_| "play tick")?;
    }
    assert!(
        matches!(store_command(&mut rx), Ok((186, payload)) if payload == wire::encode_inbox_query(None)),
        "opening the Inbox during a browse must query claim entries after the browse reply"
    );
    item_reply(&outbox, inbox_entry(inbox_key(1), 42));
    app.update();
    let claim = button(&mut app, Kind::Claim(0))?;
    assert!(app.world().get::<InteractionDisabled>(claim).is_none());
    assert!(texts(&mut app).iter().any(|title| title == "Claim item 42"));

    {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        link.store_inbox_set_open(false);
        link.store_set_open(true);
        assert_eq!(store_command(&mut rx)?.0, wire::browse::COMMAND_TYPE);
        link.store_set_open(false);
        link.store_inbox_set_open(true);
    }
    app.update();
    click(&mut app, Kind::Close)?;
    item_reply(&outbox, browse.encode().map_err(|e| format!("{e:?}"))?);
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let play = owner.play.as_mut().ok_or("play")?;
        play.view.tick(&play.link).map_err(|_| "play tick")?;
    }
    assert!(
        rx.try_recv().is_err(),
        "closing the Inbox must cancel its queued query"
    );
    Ok(())
}

#[test]
fn store_inbox_next_claim_page_selects_the_item_after_twenty_deliveries() -> TestResult {
    let (mut app, mut rx, outbox) = fixture(false)?;
    outbox.bevy_store_initialize();
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .store_inbox_set_open(true);
    assert_eq!(
        store_command(&mut rx)?,
        (186, wire::encode_inbox_query(None))
    );
    let first_page = (1..=20)
        .flat_map(|ordinal| inbox_entry(inbox_key(ordinal), 43))
        .chain([18, 16])
        .chain(inbox_key(20))
        .collect();
    item_reply(&outbox, first_page);
    app.update();
    let button = app
        .world_mut()
        .query::<(Entity, &Control, &Children)>()
        .iter(app.world())
        .find(|(_, _, children)| {
            children.iter().any(|child| {
                app.world()
                    .get::<Text>(child)
                    .is_some_and(|text| text.0 == "Next claim page")
            })
        })
        .map(|(entity, _, _)| entity);
    assert!(
        button.is_some(),
        "the Inbox must offer a separate next claim page"
    );
    let entity = button.ok_or("next claim page")?;
    assert!(app.world().get::<InteractionDisabled>(entity).is_none());
    app.world_mut().trigger(Activate { entity });
    assert_eq!(
        store_command(&mut rx)?,
        (186, wire::encode_inbox_query(Some(inbox_key(20))))
    );
    item_reply(&outbox, inbox_entry(inbox_key(21), 42));
    app.update();
    assert!(texts(&mut app).iter().any(|title| title == "Claim item 42"));
    assert!(!texts(&mut app).iter().any(|title| title == "Claim item 43"));
    click(&mut app, Kind::Claim(0))?;
    let (kind, payload) = store_command(&mut rx)?;
    assert_eq!(kind, wire::COMMAND_TYPE_STORE_INBOX_CLAIM);
    let claim = wire::decode_inbox_claim_request(&payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        claim.fulfilment_unit_id,
        "00000000-0000-7000-8000-000000000015"
    );
    item_reply(
        &outbox,
        wire::encode_claim_receipt(&wire::StoreClaimReceipt {
            operation_id: inbox_key(22),
            fulfilment_unit_id: claim.fulfilment_unit_id,
            result: wire::StoreClaimCode::Committed,
            recovery_generation: 1,
            decided_at_unix_ms: 1,
        })
        .map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    click(&mut app, Kind::FirstClaimPage)?;
    assert_eq!(
        store_command(&mut rx)?,
        (186, wire::encode_inbox_query(None))
    );
    Ok(())
}

#[test]
fn store_inbox_separates_claim_pages_and_places_lamps_with_native_use() -> TestResult {
    use oteryn_client_runtime::world_interaction::world_object as use_wire;
    let (mut app, mut rx, outbox) = fixture(true)?;
    let _ = request(&mut rx)?;
    outbox.bevy_store_initialize();
    app.update();
    click(&mut app, Kind::Refresh)?;
    assert!(
        matches!(rx.try_recv(), Ok(PlayCommand::ServerMechanic(command))
        if command.payload == wire::encode_inbox_query(None))
    );
    let cursor = [0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 2];
    let lamp_definition = std::num::NonZeroU32::new(33).ok_or("lamp definition")?;
    let page = wire::StoreInboxPage {
        entries: vec![wire::StoreInboxEntry {
            fulfilment_unit_id: cursor,
            item_id: 28552,
        }],
        next: Some(cursor),
        lamp_definition: Some(lamp_definition),
        claims_require_channel: true,
    };
    let response = |page: &wire::StoreInboxPage| -> Result<_, Box<dyn std::error::Error>> {
        Ok(GatedCommandResult {
            command_id: 2,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload: wire::encode_inbox_page(page).map_err(|e| format!("{e:?}"))?,
        })
    };
    outbox.bevy_store_reply(&response(&page)?);
    app.update();
    assert!(
        texts(&mut app)
            .iter()
            .any(|text| text.contains("Return to the Channel"))
    );
    let claim = button(&mut app, Kind::Claim(0))?;
    assert!(app.world().get::<InteractionDisabled>(claim).is_some());
    click(&mut app, Kind::Claim(0))?;
    assert!(
        rx.try_recv().is_err(),
        "ordinary claims require leaving the House"
    );
    click(&mut app, Kind::NextClaims)?;
    assert!(
        matches!(rx.try_recv(), Ok(PlayCommand::ServerMechanic(command))
        if command.payload == wire::encode_inbox_query(Some(cursor)))
    );

    outbox.bevy_store_reply(&response(&wire::StoreInboxPage { next: None, ..page })?);
    app.update();
    click(&mut app, Kind::PlaceLamp)?;
    let PlayCommand::ServerMechanic(command) = rx.try_recv()? else {
        return Err("missing native USE".into());
    };
    assert_eq!(command.capability, use_wire::CAPABILITY_ITEM_USE_V1);
    assert_eq!(command.command_type, use_wire::COMMAND_TYPE_USE_INTENT);
    let use_intent = use_wire::decode_use_intent_target(&[4, 15], &command.payload)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        use_intent.target,
        use_wire::UseTarget::ItemByDefinition(lamp_definition)
    );
    Ok(())
}

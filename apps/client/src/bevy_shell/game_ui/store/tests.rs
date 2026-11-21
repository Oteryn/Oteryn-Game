use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use oteryn_client_runtime::{
    cui3::store::{self as wire, browse},
    server_mechanic::{CommandStatus, ServerMechanicCommand, ServerMechanicResponse},
};
use oteryn_renderer::TileCoord;
type ResultTest = Result<(), Box<dyn std::error::Error>>;
type Receiver = tokio::sync::mpsc::UnboundedReceiver<PlayCommand>;
fn command(rx: &mut Receiver) -> Result<ServerMechanicCommand, Box<dyn std::error::Error>> {
    match rx.try_recv()? {
        PlayCommand::ServerMechanic(c) => Ok(c),
        _ => Err("store command".into()),
    }
}
fn page(total: u64) -> browse::Page {
    browse::Page {
        categories: vec![
            browse::Category {
                key: "featured".into(),
                parent: String::new(),
                title: "Home".into(),
            },
            browse::Category {
                key: "cosmetics".into(),
                parent: String::new(),
                title: "Cosmetics".into(),
            },
            browse::Category {
                key: "cosmetics.outfits".into(),
                parent: "cosmetics".into(),
                title: "Outfits".into(),
            },
        ],
        products: vec![browse::Product {
            category: "featured".into(),
            revision: "server-revision".into(),
            artwork: browse::Artwork::default(),
            offer: wire::StoreOffer {
                offer_key: "store:test".into(),
                title: "Test product".into(),
                price: 100,
                currency: "TIBIA_COIN".into(),
            },
        }],
        next: String::new(),
        total,
        transferable: total,
        transferable_known: true,
    }
}
fn reply(outbox: &Outbox, payload: Vec<u8>, status: CommandStatus) {
    outbox.bevy_store_reply(&ServerMechanicResponse {
        command_id: 1,
        server_sequence: 1,
        status,
        payload,
    });
}
fn fixture(total: u64) -> Result<(App, Receiver, Outbox), Box<dyn std::error::Error>> {
    fixture_in(App::new(), total)
}
fn fixture_in(
    mut app: App,
    total: u64,
) -> Result<(App, Receiver, Outbox), Box<dyn std::error::Error>> {
    let (link, mut rx, outbox) = play_channel();
    outbox.bevy_store_initialize();
    link.store_set_open(true);
    assert_eq!(command(&mut rx)?.command_type, browse::COMMAND_TYPE);
    reply(
        &outbox,
        page(total).encode().map_err(|e| format!("{e:?}"))?,
        CommandStatus::Accepted,
    );
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
    });
    app.insert_non_send(owner)
        .init_resource::<InputFocus>()
        .insert_resource(Skins {
            buttons: source::ButtonImages {
                up: Handle::default(),
                down: Handle::default(),
            },
            checked: Handle::default(),
            unchecked: Handle::default(),
            frame: Handle::default(),
            inset: Handle::default(),
            font: Handle::default(),
        });
    install(&mut app);
    app.update();
    Ok((app, rx, outbox))
}
fn click(app: &mut App, kind: Kind) -> ResultTest {
    let entity = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(e, a)| (a.kind == kind).then_some(e))
        .ok_or("store button")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
fn text(app: &mut App) -> String {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
#[test]
fn store_category_selects_products() -> ResultTest {
    let (mut app, mut rx, outbox) = fixture(250)?;
    assert!(text(&mut app).contains("Test product"));
    click(&mut app, Kind::Category(2))?;
    let query = browse::Query::decode(&command(&mut rx)?.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(query.category, "cosmetics.outfits");
    let mut next = page(250);
    next.products[0].offer.title = "Outfit from server".into();
    next.products[0].category = query.category;
    reply(
        &outbox,
        next.encode().map_err(|e| format!("{e:?}"))?,
        CommandStatus::Accepted,
    );
    app.update();
    assert!(text(&mut app).contains("Outfit from server"));
    assert!(text(&mut app).contains("server-revision"));
    assert!(!text(&mut app).contains("Test product"));
    Ok(())
}
#[test]
fn store_buy_disabled_when_short() -> ResultTest {
    let (mut app, mut rx, _) = fixture(99)?;
    assert!(text(&mut app).contains("99"));
    let disabled = app
        .world_mut()
        .query::<(&Action, Option<&bevy::ui::InteractionDisabled>)>()
        .iter(app.world())
        .any(|(a, d)| a.kind == Kind::Buy && d.is_some());
    assert!(disabled);
    click(&mut app, Kind::Buy)?;
    assert!(app.world().resource::<State>().confirmation.is_none());
    assert!(rx.try_recv().is_err());
    Ok(())
}
#[test]
fn store_purchase_confirm_sends_intent() -> ResultTest {
    let (mut app, mut rx, outbox) = fixture(250)?;
    click(&mut app, Kind::Buy)?;
    assert!(rx.try_recv().is_err());
    assert!(text(&mut app).contains("Confirm purchase"));
    click(&mut app, Kind::Confirm)?;
    let c = command(&mut rx)?;
    assert_eq!((c.capability, c.command_type), (154, 185));
    let purchase = wire::decode_purchase_request(&c.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (
            purchase.offer_key.as_str(),
            purchase.price,
            purchase.catalog_revision.as_str()
        ),
        ("store:test", 100, "server-revision")
    );
    reply(&outbox, vec![], CommandStatus::Rejected);
    app.update();
    assert!(text(&mut app).contains("refused"));
    Ok(())
}
#[test]
fn store_unresolved_purchase_retries_with_original_bytes() -> ResultTest {
    for status in [CommandStatus::Rejected, CommandStatus::Accepted] {
        let (mut app, mut rx, outbox) = fixture(250)?;
        click(&mut app, Kind::Buy)?;
        click(&mut app, Kind::Confirm)?;
        let original = command(&mut rx)?;
        reply(&outbox, Vec::new(), status);
        app.update();
        let store_panel = |app: &App| -> Result<StorePanel, Box<dyn std::error::Error>> {
            Ok(app
                .world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .store_panel())
        };
        let unresolved = store_panel(&app)?;
        assert!(!unresolved.pending);
        assert_eq!(
            unresolved.confirmed_purchase.as_ref(),
            Some(&original.payload)
        );
        assert!(text(&mut app).contains("Retry purchase"));
        let retry = app
            .world_mut()
            .query::<(Entity, &Action)>()
            .iter(app.world())
            .find_map(|(entity, action)| (action.kind == Kind::RetryPurchase).then_some(entity))
            .ok_or("retry action missing")?;
        app.world_mut().trigger(Activate { entity: retry });
        assert_eq!(command(&mut rx)?, original);
        app.world_mut().trigger(Activate { entity: retry });
        assert!(rx.try_recv().is_err());
        let request = wire::decode_purchase_request(&original.payload)
            .map_err(|error| format!("{error:?}"))?;
        reply(
            &outbox,
            wire::encode_purchase_result(&wire::StorePurchaseResult {
                code: wire::StorePurchaseCode::Accepted,
                order_id: Some([7, 7, 7, 7, 7, 7, 0x70, 7, 0x80, 7, 7, 7, 7, 7, 7, 7]),
                operation_id: Some(request.purchase_request_id),
            })
            .map_err(|error| format!("{error:?}"))?,
            CommandStatus::Accepted,
        );
        app.update();
        assert!(store_panel(&app)?.confirmed_purchase.is_none());
        assert!(!text(&mut app).contains("Retry purchase"));
        app.world_mut().trigger(Activate { entity: retry });
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        assert!(!link.store_retry_purchase_bound(&unresolved));
        assert!(!link.store_retry_purchase_bound(&link.store_panel()));
        assert!(rx.try_recv().is_err());
    }
    Ok(())
}

#[test]
fn store_gift_panel() -> ResultTest {
    let (mut app, mut rx, outbox) = fixture(250)?;
    click(&mut app, Kind::Gift)?;
    assert!(text(&mut app).contains("Recipient"));
    assert!(text(&mut app).contains("Transfer coins"));
    for (field, mut value) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        *value = EditableText::new(match field.0 {
            0 => "Recipient Name",
            _ => "100",
        });
    }
    click(&mut app, Kind::GiftControl(0))?;
    let c = command(&mut rx)?;
    assert_eq!(
        (c.capability, c.command_type),
        (
            oteryn_client_runtime::coin_transfer::CAPABILITY,
            oteryn_client_runtime::coin_transfer::COMMAND
        )
    );
    use oteryn_client_runtime::coin_transfer as transfer;
    let review = transfer::decode(&c.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (review.action, review.recipient.as_str(), review.amount),
        (1, "Recipient Name", 100)
    );
    let quote = serde_json::json!({"result":"ok","quote":"019a65f0-0000-7000-8000-000000000001","recipient":"Recipient NAME","amount":"100","transferable":"250","non_transferable":"0"});
    outbox.bevy_coin_gift_reply(&ServerMechanicResponse {
        command_id: 3,
        server_sequence: 3,
        status: CommandStatus::Accepted,
        payload: transfer::encode_reply(&quote.to_string()).map_err(|e| format!("{e:?}"))?,
    });
    app.update();
    assert!(text(&mut app).contains("Reviewed recipient: Recipient NAME; coins: 100"));
    click(&mut app, Kind::GiftControl(1))?;
    let confirmation = command(&mut rx)?;
    let request = transfer::decode(&confirmation.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (request.action, request.recipient.as_str(), request.amount),
        (2, "Recipient NAME", 100)
    );
    outbox.bevy_coin_gift_reply(&ServerMechanicResponse {
        command_id: 4,
        server_sequence: 4,
        status: CommandStatus::Rejected,
        payload: vec![],
    });
    app.update();
    assert!(text(&mut app).contains("Gift response unavailable"));
    click(&mut app, Kind::GiftControl(3))?;
    assert_eq!(command(&mut rx)?.payload, confirmation.payload);
    Ok(())
}

include!("store_session_cases.rs");

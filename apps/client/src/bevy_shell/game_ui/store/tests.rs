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
    let mut app = App::new();
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
#[test]
fn store_history_page() -> ResultTest {
    let (mut app, mut rx, outbox) = fixture(250)?;
    click(&mut app, Kind::History(false))?;
    let c = command(&mut rx)?;
    assert_eq!((c.capability, c.command_type), (285, 501));
    outbox.bevy_purchase_history_reply(&ServerMechanicResponse {
        command_id: 2,
        server_sequence: 2,
        status: CommandStatus::Accepted,
        payload: oteryn_client_runtime::discovery::store::encode_history_page(
            &oteryn_client_runtime::discovery::store::StoreHistoryPage {
                entries: vec![oteryn_client_runtime::discovery::store::StoreHistoryEntry {
                    order_id: [1, 1, 1, 1, 1, 1, 0x70, 1, 0x80, 1, 1, 1, 1, 1, 1, 1],
                    offer_key: "store:test".into(),
                    catalog_revision: "server-revision".into(),
                    price: 100,
                    currency: "TIBIA_COIN".into(),
                    purchased_at_unix_ms: 1234,
                }],
                next_page_cursor: String::new(),
            },
        )
        .map_err(|e| format!("{e:?}"))?,
    });
    app.update();
    assert!(text(&mut app).contains("store:test"));
    assert!(text(&mut app).contains("1234 ms"));
    assert!(text(&mut app).contains("End of purchase history"));
    click(&mut app, Kind::Ledger(false))?;
    assert_eq!(command(&mut rx)?.command_type, 184);
    Ok(())
}
#[test]
fn store_stale_confirmation_and_closed_window_cannot_purchase() -> ResultTest {
    let (mut app, mut rx, outbox) = fixture(250)?;
    click(&mut app, Kind::Buy)?;
    let confirm = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(e, a)| (a.kind == Kind::Confirm).then_some(e))
        .ok_or("confirm")?;
    // An unsolicited catalogue/balance change invalidates the exact displayed consent.
    reply(
        &outbox,
        page(200).encode().map_err(|e| format!("{e:?}"))?,
        CommandStatus::Accepted,
    );
    app.world_mut().trigger(Activate { entity: confirm });
    app.update();
    assert!(rx.try_recv().is_err());
    assert!(app.world().resource::<State>().confirmation.is_none());
    click(&mut app, Kind::Close)?;
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}
#[test]
fn store_shortcut_requests_catalogue() -> ResultTest {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (link, mut rx, outbox) = play_channel();
    outbox.bevy_store_initialize();
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
    });
    let modifiers = Modifiers::one(Modifier::Control).with(Modifier::Shift);
    owner.route_input(
        &[NormalizedInputEvent::Key {
            code: KeyCode::new(22)?,
            state: ButtonState::Pressed,
            modifiers,
            repeat: false,
        }],
        false,
    );
    assert!(owner.play.as_ref().ok_or("play")?.link.store_panel().open);
    assert_eq!(command(&mut rx)?.command_type, browse::COMMAND_TYPE);
    Ok(())
}

#[test]
fn store_shortcut_rejects_extra_modifiers() -> ResultTest {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (link, mut rx, outbox) = play_channel();
    outbox.bevy_store_initialize();
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
    });
    for extra in [Modifier::Alt, Modifier::Super] {
        owner.route_input(
            &[NormalizedInputEvent::Key {
                code: KeyCode::new(22)?,
                state: ButtonState::Pressed,
                modifiers: Modifiers::one(Modifier::Control)
                    .with(Modifier::Shift)
                    .with(extra),
                repeat: false,
            }],
            false,
        );
        assert!(!owner.play.as_ref().ok_or("play")?.link.store_panel().open);
        assert!(rx.try_recv().is_err());
    }
    Ok(())
}

#[test]
fn purchase_history_shortcut_opens_visible_history_page() -> ResultTest {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (mut app, _rx, _) = fixture(250)?;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .store_set_open(false);
    app.update();
    app.world_mut().non_send_mut::<HostOwner>().route_input(
        &[NormalizedInputEvent::Key {
            code: KeyCode::new(11)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control).with(Modifier::Alt),
            repeat: false,
        }],
        false,
    );
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .store_panel()
            .open
    );
    app.update();
    assert!(matches!(
        app.world().resource::<State>().page,
        Page::History
    ));
    assert!(
        app.world_mut()
            .query_filtered::<&Node, With<Overlay>>()
            .iter(app.world())
            .any(|node| node.display != Display::None)
    );
    Ok(())
}

#[test]
fn team_finder_shortcut_presents_real_browse_controls() -> ResultTest {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (mut app, mut rx, outbox) = fixture(250)?;
    outbox.bevy_team_finder_supported();
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .store_set_open(false);
    app.update();
    app.world_mut().non_send_mut::<HostOwner>().route_input(
        &[NormalizedInputEvent::Key {
            code: KeyCode::new(23)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Control).with(Modifier::Alt),
            repeat: false,
        }],
        false,
    );
    app.update();
    let entity = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, name)| (name.as_str() == "Team Finder: Browse teams").then_some(entity))
        .ok_or("team browse control")?;
    app.world_mut().trigger(Activate { entity });
    assert!(std::iter::from_fn(|| rx.try_recv().ok()).any(|command| matches!(command, PlayCommand::ServerMechanic(command) if command.capability == 232)));
    Ok(())
}

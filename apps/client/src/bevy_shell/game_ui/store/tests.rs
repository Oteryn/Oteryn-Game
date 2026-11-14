use super::*;
use crate::play::{PlayView, play_channel};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::{cui3::store as wire, server_mechanic::ServerMechanicResponse};
use oteryn_session::CommandStatus;
type Error = Box<dyn std::error::Error>;
type TestResult = Result<(), Error>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    crate::play::Outbox,
);
fn fixture() -> Result<Fixture, Error> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_store_initialize();
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
fn page() -> wire::browse::Page {
    wire::browse::Page {
        categories: vec![wire::browse::Category {
            key: "server-only".into(),
            parent: String::new(),
            title: "Server category".into(),
        }],
        products: vec![wire::browse::Product {
            category: "server-only".into(),
            revision: "server-revision".into(),
            offer: wire::StoreOffer {
                offer_key: "store:server-only".into(),
                title: "Server offer".into(),
                price: 73,
                currency: "COINS".into(),
            },
            artwork: Default::default(),
        }],
        next: "opaque-catalogue-cursor".into(),
        total: 1000,
        transferable: 500,
        transferable_known: true,
    }
}
fn reply(outbox: &crate::play::Outbox, payload: Vec<u8>) {
    outbox.bevy_store_response(&ServerMechanicResponse {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload,
    });
}
fn snapshot(app: &App) -> Result<StoreSnapshot, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .store_snapshot())
}
fn click(app: &mut App, action: Action) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.action == action).then_some(e))
        .ok_or("control")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn sent(
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
) -> Result<oteryn_client_runtime::server_mechanic::ServerMechanicCommand, Box<dyn std::error::Error>>
{
    let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("command".into());
    };
    Ok(command)
}
fn load(
    app: &mut App,
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    outbox: &crate::play::Outbox,
) -> TestResult {
    click(app, Action::Refresh)?;
    assert_eq!(sent(commands)?.command_type, wire::browse::COMMAND_TYPE);
    reply(outbox, page().encode().map_err(|e| format!("{e:?}"))?);
    app.update();
    Ok(())
}
#[test]
fn store_catalogue_lists_server_offers_only() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    assert_eq!(
        app.world_mut()
            .query::<&ProductRow>()
            .iter(app.world())
            .count(),
        0
    );
    load(&mut app, &mut commands, &outbox)?;
    assert_eq!(
        app.world_mut()
            .query::<&ProductRow>()
            .iter(app.world())
            .count(),
        1
    );
    let texts = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(texts.contains("Server category"));
    assert!(texts.contains("Server offer"));
    assert!(!texts.contains("Exercise Weapons"));
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn store_purchase_requires_bound_consent() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    load(&mut app, &mut commands, &outbox)?;
    click(&mut app, Action::ConfirmPurchase)?;
    assert!(commands.try_recv().is_err());
    click(&mut app, Action::ReviewPurchase)?;
    app.update();
    assert!(commands.try_recv().is_err());
    click(&mut app, Action::ConfirmPurchase)?;
    let command = sent(&mut commands)?;
    let quote = wire::decode_purchase_request(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (
            quote.offer_key.as_str(),
            quote.price,
            quote.currency.as_str()
        ),
        ("store:server-only", 73, "COINS")
    );
    assert!(snapshot(&app)?.panel.purchase.is_none());
    assert_eq!(
        snapshot(&app)?
            .panel
            .browser
            .page
            .as_ref()
            .ok_or("page")?
            .total,
        1000
    );
    reply(
        &outbox,
        wire::encode_purchase_result(&wire::StorePurchaseResult {
            code: wire::StorePurchaseCode::Accepted,
            order_id: Some(uuid()),
            operation_id: Some(uuid()),
        })
        .map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    assert_eq!(
        snapshot(&app)?.panel.purchase.ok_or("purchase")?.code,
        wire::StorePurchaseCode::Accepted
    );
    Ok(())
}
#[test]
fn store_history_pages_server_cursor() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    click(&mut app, Action::History)?;
    let first = sent(&mut commands)?;
    assert_eq!((first.capability, first.command_type), (285, 501));
    outbox.bevy_store_history_response(&ServerMechanicResponse {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: wire::encode_history_page(&wire::StoreHistoryPage {
            entries: vec![],
            next_page_cursor: "server-history-cursor".into(),
        })
        .map_err(|e| format!("{e:?}"))?,
    });
    app.update();
    click(&mut app, Action::Next)?;
    assert_eq!(
        wire::decode_history_request(&sent(&mut commands)?.payload)
            .map_err(|e| format!("{e:?}"))?
            .page_cursor,
        "server-history-cursor"
    );
    outbox.bevy_store_history_response(&ServerMechanicResponse {
        command_id: 2,
        server_sequence: 2,
        status: CommandStatus::Accepted,
        payload: wire::encode_history_page(&wire::StoreHistoryPage {
            entries: vec![],
            next_page_cursor: String::new(),
        })
        .map_err(|e| format!("{e:?}"))?,
    });
    app.update();
    click(&mut app, Action::Next)?;
    assert!(commands.try_recv().is_err());
    set_field(&mut app, Field::Filter, "server-only")?;
    set_field(&mut app, Field::From, "10")?;
    set_field(&mut app, Field::Until, "20")?;
    click(&mut app, Action::Filter)?;
    assert_eq!(snapshot(&app)?.history.offer_filter, "server-only");
    assert_eq!(snapshot(&app)?.history.from_ms, 10);
    app.update();
    click(&mut app, Action::Ledger)?;
    assert_eq!(sent(&mut commands)?.command_type, 184);
    reply(
        &outbox,
        wire::encode_coin_history_page(&wire::CoinHistoryPage {
            entries: vec![wire::CoinHistoryEntry {
                entry_id: 9,
                kind: "CREDIT".into(),
                currency: "COINS".into(),
                available_delta: 25,
                reserved_delta: 0,
                order_id: None,
                offer_key: None,
                quantity: None,
                created_at: 15,
            }],
            next_page_cursor: "9".into(),
        })
        .map_err(|e| format!("{e:?}"))?,
    );
    app.update();
    set_field(&mut app, Field::Filter, "CREDIT")?;
    click(&mut app, Action::Filter)?;
    assert_eq!(snapshot(&app)?.panel.coin_history.kind, "CREDIT");
    app.update();
    click(&mut app, Action::Next)?;
    assert_eq!(
        wire::decode_coin_history_request(&sent(&mut commands)?.payload)
            .map_err(|e| format!("{e:?}"))?
            .page_cursor,
        "9"
    );
    Ok(())
}
#[test]
fn store_actions_rejected_when_session_replaced_or_pending() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    load(&mut app, &mut commands, &outbox)?;
    click(&mut app, Action::ReviewPurchase)?;
    app.update();
    click(&mut app, Action::ConfirmPurchase)?;
    let _ = sent(&mut commands)?;
    let pending = snapshot(&app)?;
    for action in [
        Action::ConfirmPurchase,
        Action::Refresh,
        Action::History,
        Action::Ledger,
        Action::Close,
    ] {
        click(&mut app, action)?;
    }
    assert!(snapshot(&app)? == pending);
    assert!(commands.try_recv().is_err());
    let (replacement, mut other_commands, other_outbox) = play_channel();
    replacement.use_shared_frontend();
    other_outbox.bevy_store_initialize();
    let old = snapshot(&app)?;
    assert!(!replacement.store_dispatch_bound(&old, StoreAction::Close));
    assert!(other_commands.try_recv().is_err());
    outbox.bevy_store_end();
    let ended = snapshot(&app)?;
    let owner = app.world().non_send::<HostOwner>();
    assert!(
        !owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .store_dispatch_bound(&old, StoreAction::Close)
    );
    assert!(snapshot(&app)? == ended);
    Ok(())
}

fn set_field(app: &mut App, kind: Field, value: &str) -> TestResult {
    let mut query = app.world_mut().query::<(&Field, &mut EditableText)>();
    let (_, mut text) = query
        .iter_mut(app.world_mut())
        .find(|(f, _)| **f == kind)
        .ok_or("field")?;
    *text = EditableText::new(value);
    Ok(())
}
#[test]
fn store_gift_requires_server_review_and_exact_target_consent() -> TestResult {
    use oteryn_client_runtime::coin_transfer as gift;
    let (mut app, mut commands, outbox) = fixture()?;
    click(&mut app, Action::Gift)?;
    app.update();
    set_field(&mut app, Field::Recipient, "alice")?;
    set_field(&mut app, Field::Amount, "25")?;
    click(&mut app, Action::ConfirmGift)?;
    assert!(commands.try_recv().is_err());
    click(&mut app, Action::ReviewGift)?;
    let request = gift::decode(&sent(&mut commands)?.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (request.action, request.recipient.as_str(), request.amount),
        (1, "alice", 25)
    );
    outbox.bevy_store_gift_response(&ServerMechanicResponse {command_id:1,server_sequence:1,status:CommandStatus::Accepted,payload:gift::encode_reply(r#"{"result":"ok","quote":"11111111-1111-7111-8111-111111111111","recipient":"Alice","amount":"25","transferable":"500","non_transferable":"500"}"#).map_err(|e|format!("{e:?}"))?});
    app.update();
    set_field(&mut app, Field::Recipient, "Bob")?;
    let before = snapshot(&app)?;
    click(&mut app, Action::ConfirmGift)?;
    assert!(commands.try_recv().is_err());
    assert!(snapshot(&app)? == before);
    set_field(&mut app, Field::Recipient, "Alice")?;
    click(&mut app, Action::ConfirmGift)?;
    let confirmed = gift::decode(&sent(&mut commands)?.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        (
            confirmed.action,
            confirmed.recipient.as_str(),
            confirmed.amount
        ),
        (2, "Alice", 25)
    );
    click(&mut app, Action::ConfirmGift)?;
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn store_owner_rechecks_modal_and_closed_channel_under_lock() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    load(&mut app, &mut commands, &outbox)?;
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let expected = link.store_snapshot();
    let modal = link.bevy_server_modal_fixture();
    for action in [
        StoreAction::Close,
        StoreAction::GiftOpen,
        StoreAction::Select(0),
        StoreAction::History(false),
        StoreAction::Ledger(false),
        StoreAction::Purchase(StoreCustomization {
            name: String::new(),
            sex: 0,
            identity: None,
        }),
        StoreAction::Gift {
            action: 0,
            fields: ["Alice".into(), "25".into()],
        },
        StoreAction::Filter {
            ledger: false,
            key: String::new(),
            from: String::new(),
            until: String::new(),
        },
        StoreAction::Browse {
            category: "server-only".into(),
            search: String::new(),
            sort: 0,
            step: 0,
        },
    ] {
        assert!(!link.store_dispatch_bound(&expected, action));
        assert!(link.store_snapshot() == expected);
    }
    assert!(commands.try_recv().is_err());
    drop(modal);
    drop(commands);
    assert!(!link.store_dispatch_bound(&expected, StoreAction::Close));
    assert!(link.store_snapshot() == expected);
    Ok(())
}

#[test]
fn store_customization_consent_carries_exact_name_and_target() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    click(&mut app, Action::Refresh)?;
    let _ = sent(&mut commands)?;
    let mut catalogue = page();
    catalogue.products[0].offer.offer_key = "store:hireling:25438".into();
    reply(&outbox, catalogue.encode().map_err(|e| format!("{e:?}"))?);
    app.update();
    set_field(&mut app, Field::Name, "Reviewed Name")?;
    set_field(
        &mut app,
        Field::Identity,
        "11111111-1111-7111-8111-111111111111",
    )?;
    click(&mut app, Action::ReviewPurchase)?;
    app.update();
    assert!(commands.try_recv().is_err());
    click(&mut app, Action::ConfirmPurchase)?;
    let request = sent(&mut commands)?;
    let (quote, custom) = wire::decode_purchase_request_with_hireling(&request.payload)
        .map_err(|e| format!("{e:?}"))?;
    assert_eq!((quote.price, quote.currency.as_str()), (73, "COINS"));
    let custom = custom.ok_or("customization")?;
    assert_eq!(custom.name.as_deref(), Some("Reviewed Name"));
    assert_eq!(
        custom.hireling_id,
        Some([
            0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x71, 0x11, 0x81, 0x11, 0x11, 0x11, 0x11, 0x11,
            0x11, 0x11
        ])
    );
    Ok(())
}

fn uuid() -> [u8; 16] {
    let mut id = [1; 16];
    id[6] = 0x71;
    id[8] = 0x81;
    id
}

#[test]
fn store_identical_replacement_session_invalidates_existing_consent() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    load(&mut app, &mut commands, &outbox)?;
    click(&mut app, Action::ReviewPurchase)?;
    app.update();
    let expected = snapshot(&app)?;
    let (link, mut replacement_commands, replacement_outbox) = play_channel();
    link.use_shared_frontend();
    replacement_outbox.bevy_store_copy_view(&expected);
    assert_eq!(link.store_panel(), expected.panel);
    assert!(!link.store_dispatch_bound(&expected, StoreAction::Close));
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    click(&mut app, Action::ConfirmPurchase)?;
    assert!(replacement_commands.try_recv().is_err());
    assert_eq!(snapshot(&app)?.panel, expected.panel);
    app.update();
    assert!(app.world().resource::<State>().consent.is_none());
    Ok(())
}

#[test]
fn store_unresolved_purchase_retries_with_original_bytes() -> TestResult {
    for malformed in [false, true] {
        let (mut app, mut commands, outbox) = fixture()?;
        load(&mut app, &mut commands, &outbox)?;
        click(&mut app, Action::ReviewPurchase)?;
        app.update();
        click(&mut app, Action::ConfirmPurchase)?;
        let original = sent(&mut commands)?;
        outbox.bevy_store_response(&ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: if malformed {
                CommandStatus::Accepted
            } else {
                CommandStatus::Rejected
            },
            payload: vec![],
        });
        app.update();
        assert!(!snapshot(&app)?.panel.pending);
        assert_eq!(
            snapshot(&app)?.panel.confirmed_purchase,
            Some(original.payload.clone())
        );
        let retry = app
            .world_mut()
            .query::<(Entity, &Text, &ChildOf)>()
            .iter(app.world())
            .find_map(|(_, text, parent)| (text.0 == "Retry purchase").then_some(parent.parent()))
            .ok_or("Retry purchase action missing")?;
        app.world_mut().trigger(Activate { entity: retry });
        let retried = sent(&mut commands)?;
        assert_eq!(retried, original);
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        let unresolved = link.store_snapshot();
        reply(
            &outbox,
            wire::encode_purchase_result(&wire::StorePurchaseResult {
                code: wire::StorePurchaseCode::Accepted,
                order_id: Some(uuid()),
                operation_id: Some(uuid()),
            })
            .map_err(|e| format!("{e:?}"))?,
        );
        app.update();
        assert!(snapshot(&app)?.panel.confirmed_purchase.is_none());
        app.world_mut().trigger(Activate { entity: retry });
        assert!(commands.try_recv().is_err());
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        assert!(!link.store_dispatch_bound(&unresolved, StoreAction::RetryPurchase));
        assert!(!link.store_dispatch_bound(&link.store_snapshot(), StoreAction::RetryPurchase));
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}

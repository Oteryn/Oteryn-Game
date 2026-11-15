use crate::play::play_channel;
#[test]
fn cosmetic_token_balance_uses_real_store_input_queue() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut incoming, outbox) = play_channel();
    outbox.store_initialize(true, false);
    outbox.store_tokens_initialize(true);
    link.store_request(6187, vec![]);
    let command = incoming.try_recv()?;
    assert!(matches!(command, crate::combat_input::PlayCommand::ServerMechanic(c) if c.capability == 479 && c.command_type == 6187));
    use oteryn_client_runtime::{cui3::store::*, server_mechanic::{CommandStatus, ServerMechanicResponse}};
    outbox.store_result(&ServerMechanicResponse { command_id: 1, status: CommandStatus::Accepted, server_sequence: 1, payload: 870_u64.to_le_bytes().to_vec() });
    assert_eq!(link.store_panel().cosmetic_tokens, Some(870));
    link.store_browse("cosmetics.outfits", "", 0, 0);
    assert!(matches!(incoming.try_recv(), Ok(crate::combat_input::PlayCommand::ServerMechanic(c)) if c.command_type == browse::COMMAND_TYPE));
    let page = browse::Page { categories: vec![browse::Category { key: "cosmetics".into(), parent: String::new(), title: "Cosmetics".into() }, browse::Category { key: "cosmetics.outfits".into(), parent: "cosmetics".into(), title: "Outfits".into() }], products: vec![browse::Product {
        category: "cosmetics.outfits".into(), revision: "reference-revision".into(), artwork: browse::Artwork { kind: 2, looks: [1814,1815] },
        offer: StoreOffer { offer_key: "store:outfit:field-surgeon".into(), title: "Field Surgeon (full outfit)".into(), price: 870, currency: "TIBIA_COIN".into() }
    }], next: String::new(), total: 1000, transferable: 0, transferable_known: false };
    outbox.store_result(&ServerMechanicResponse { command_id: 2, status: CommandStatus::Accepted, server_sequence: 2, payload: page.encode().map_err(|e| format!("{e:?}"))? });
    assert!(link.store_payment(true));
    let quote = oteryn_client_ui::store::confirmation_quote(&link.store_panel()).ok_or("displayed quote")?;
    assert_eq!(quote.offers[0].currency, "COSMETIC_TOKEN");
    assert!(link.store_confirm_purchase(&quote, "", "", ""));
    let crate::combat_input::PlayCommand::ServerMechanic(command) = incoming.try_recv()? else { return Err("wrong queue command".into()); };
    assert_eq!((command.capability,command.command_type),(479,6188));
    let request = decode_purchase_request(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!((request.price, request.currency.as_str()), (870,"COSMETIC_TOKEN"));
    assert!(!link.store_payment(false), "an uncertain confirmed payment cannot change currency");
    outbox.store_result(&ServerMechanicResponse { command_id: 3, status: CommandStatus::Accepted, server_sequence: 3, payload: encode_purchase_result(&StorePurchaseResult { code: StorePurchaseCode::Accepted, order_id: Some(request.purchase_request_id), operation_id: Some(request.purchase_request_id) }).map_err(|e| format!("{e:?}"))? });
    assert!(link.store_panel().token_balance_refresh);
    link.store_request(6187, vec![]);
    assert!(incoming.try_recv().is_ok());
    outbox.store_result(&ServerMechanicResponse { command_id: 4, status: CommandStatus::Accepted, server_sequence: 4, payload: 0_u64.to_le_bytes().to_vec() });
    assert_eq!(link.store_panel().cosmetic_tokens, Some(0));
    assert!(oteryn_client_ui::store::panel_text(&link.store_panel()).contains("Cosmetic Tokens (this character): 0"));
    Ok(())
}

#[test]
fn store_hireling_confirmation_queues_server_modal_selection()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cui3::store::*;
    let (link, mut commands, outbox) = play_channel();
    outbox.store_initialize(true, true);
    let page = StoreCataloguePage {
        catalog_revision: "revision".into(),
        offers: vec![StoreOffer {
            offer_key: "store:hireling:25438".into(),
            title: "Name Change".into(),
            price: 250,
            currency: "TIBIA_COIN".into(),
        }],
        next_page_cursor: String::new(),
        total_coins: 1000,
        transferable_coins: 0,
    };
    crate::play::mailbox::lock(&link.mailbox).store.catalogue = Some(page.clone());
    assert!(link.store_confirm_purchase(&page, "Morgan", "1", ""));
    let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("modal command".into());
    };
    assert_eq!((command.capability, command.command_type), (450, 6196));
    let (_, custom) =
        decode_purchase_request_with_hireling(&command.payload).map_err(|_| "request")?;
    assert_eq!(custom.and_then(|c| c.name), Some("Morgan".into()));
    let answer = oteryn_client_runtime::server_modal::Answer {
        dialog_id: 42,
        button_id: 1,
        choice_id: Some(1),
    };
    let dialog = oteryn_client_runtime::server_modal::Dialog {
        id: 42,
        title: "Choose a Hireling".into(),
        message: "Select a hireling below".into(),
        choices: vec![(1, "Alex".into())],
        buttons: vec![(1, "Select".into()), (2, "Cancel".into())],
        cancel_button: Some(2),
        image: None,
    };
    outbox.store_result(
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: oteryn_client_runtime::server_modal::Reply::Dialog(dialog).encode()?,
        },
    );
    assert!(
        crate::play::mailbox::lock(&link.mailbox)
            .store
            .confirmed_purchase
            .is_some()
    );
    outbox.store_modal_answered(oteryn_client_runtime::server_modal::Answer {
        dialog_id: 43,
        ..answer
    });
    assert!(
        crate::play::mailbox::lock(&link.mailbox)
            .store
            .confirmed_purchase
            .is_some()
    );
    outbox.store_modal_answered(answer);
    assert!(
        crate::play::mailbox::lock(&link.mailbox)
            .store
            .confirmed_purchase
            .is_none()
    );
    Ok(())
}
#[test]
fn store_panel_opens_and_sends() {
    let (link, mut rx, outbox) = play_channel();
    outbox.store_initialize(true, false);
    link.store_request(183, vec![]);
    assert!(matches!(
        rx.try_recv(),
        Ok(crate::combat_input::PlayCommand::ServerMechanic(_))
    ));
}

use crate::play::feature_0154_store::view::native;

#[test]
fn native_store_confirmation_and_server_quote() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    let _open: fn(&crate::play::PlayLink) = native::open_window;
    use oteryn_client_runtime::cui3::store::*;
    let mut p = oteryn_client_ui::store::StorePanel {
        selected: true,
        catalogue: Some(StoreCataloguePage {
            catalog_revision: "rev-7".into(),
            offers: vec![StoreOffer {
                offer_key: "mount".into(),
                title: "Mount".into(),
                price: 25,
                currency: "coins".into(),
            }],
            next_page_cursor: String::new(),
            total_coins: 100,
            transferable_coins: 0,
        }),
        ..Default::default()
    };
    assert!(native::purchase(&mut p, 0, false).is_none());
    let c = native::purchase(&mut p, 0, true).ok_or("missing purchase")?;
    let quote = decode_purchase_request(&c.payload).map_err(|_| "invalid purchase")?;
    assert_eq!(
        (
            quote.offer_key.as_str(),
            quote.catalog_revision.as_str(),
            quote.price
        ),
        ("mount", "rev-7", 25)
    );
    assert!(native::purchase(&mut p, 0, true).is_none());
    assert!(native::panel_text(&p).contains("Pending"));
    assert_eq!(c.command_type, 185);
    let response = oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
        command_id: 1,
        status: oteryn_client_runtime::server_mechanic::CommandStatus::Accepted,
        server_sequence: 1,
        payload: encode_purchase_result(&StorePurchaseResult {
            code: StorePurchaseCode::Accepted,
            order_id: Some(quote.purchase_request_id),
            operation_id: Some(quote.purchase_request_id),
        })
        .map_err(|_| "invalid fixture")?,
    };
    oteryn_client_ui::store::apply_result(&mut p, &response);
    assert!(native::panel_text(&p).contains("Accepted"));
    assert!(!p.pending);
    oteryn_client_ui::store::apply_result(
        &mut p,
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            status: oteryn_client_runtime::server_mechanic::CommandStatus::Rejected,
            ..response
        },
    );
    assert!(native::panel_text(&p).contains("refused"));
    p.category = "featured".into();
    let catalogue = oteryn_client_ui::store::catalogue(&mut p).ok_or("missing catalogue")?;
    assert_eq!(catalogue.command_type, 183);
    assert_eq!(
        decode_catalogue_request(&catalogue.payload)
            .map_err(|_| "invalid catalogue")?
            .category,
        "featured"
    );
    Ok(())
}

#[test]
fn gap_store_confirmation_retry_keeps_transaction_and_quote()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cui3::store::*;
    let mut p = oteryn_client_ui::store::StorePanel {
        selected: true,
        catalogue: Some(StoreCataloguePage {
            catalog_revision: "server-revision".into(),
            offers: vec![StoreOffer {
                offer_key: "mount".into(),
                title: "Mount".into(),
                price: 77,
                currency: "coins".into(),
            }],
            next_page_cursor: String::new(),
            total_coins: 100,
            transferable_coins: 0,
        }),
        ..Default::default()
    };
    native::customization(&mut p, "Alex", "2", "01010101-0101-7001-8001-010101010101");
    assert_eq!(p.hireling_sex, 2);
    assert!(p.hireling_id.is_some());
    native::customization(&mut p, "Alex", "bad", "invalid");
    assert!(p.hireling_id.is_none());
    let first = native::purchase(&mut p, 0, true).ok_or("first confirmation")?;
    p.pending = false; // Lost response: retry the same explicit confirmation.
    let retry = native::purchase(&mut p, 0, true).ok_or("retry")?;
    assert_eq!(first.payload, retry.payload);
    let quote = decode_purchase_request(&first.payload).map_err(|_| "quote")?;
    oteryn_client_ui::store::apply_result(
        &mut p,
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            status: oteryn_client_runtime::server_mechanic::CommandStatus::Accepted,
            server_sequence: 1,
            payload: encode_purchase_result(&StorePurchaseResult {
                code: StorePurchaseCode::Accepted,
                order_id: Some(quote.purchase_request_id),
                operation_id: Some(quote.purchase_request_id),
            })
            .map_err(|_| "result")?,
        },
    );
    assert!(p.confirmed_purchase.is_none());
    let next = native::purchase(&mut p, 0, true).ok_or("new transaction")?;
    assert_ne!(first.payload, next.payload);
    Ok(())
}

#[test]
fn store_native_preview_remains_visible_after_catalogue_feedback() {
    use oteryn_client_runtime::cui3::store as wire;
    use oteryn_client_ui::store::StorePanel;
    let p = StorePanel {
        selected: true,
        feedback: Some("Catalogue received".into()),
        catalogue: Some(wire::StoreCataloguePage {
            catalog_revision: "r1".into(),
            total_coins: 100,
            transferable_coins: 0,
            next_page_cursor: String::new(),
            offers: vec![wire::StoreOffer {
                offer_key: "mount".into(),
                title: "Horse".into(),
                price: 73,
                currency: "coins".into(),
            }],
        }),
        ..StorePanel::default()
    };
    assert!(native::panel_text(&p).contains("Preview: Horse — 73 coins"));
}

#[test]
fn coin_gift_real_caller_requires_server_review_and_retries_exact_confirmation()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.store_initialize(true, false);
    let fields = ["Alice".into(), "25".into()];
    assert!(!link.coin_gift_control(1, fields.clone()));
    assert!(commands.try_recv().is_err());
    assert!(link.coin_gift_control(0, fields.clone()));
    let crate::combat_input::PlayCommand::ServerMechanic(review) = commands.try_recv()? else {
        return Err("review".into());
    };
    assert_eq!(
        (review.capability, review.command_type),
        (154, oteryn_protocol_oteryn::coin_transfer::COMMAND)
    );
    let wire = oteryn_protocol_oteryn::coin_transfer::decode(&review.payload)
        .map_err(|_| "review wire")?;
    assert_eq!(wire.amount, 25);
    outbox.coin_gift_result(&oteryn_session::GatedCommandResult{command_id:1,server_sequence:1,status:oteryn_session::CommandStatus::Accepted,payload:oteryn_protocol_oteryn::coin_transfer::encode_reply(r#"{"result":"ok","quote":"019a65f0-0000-7000-8000-000000000001","recipient":"Alice","amount":"25","transferable":"100","non_transferable":"50"}"#).map_err(|_|"quote")?});
    assert!(!link.coin_gift_control(1, ["Bob".into(), "25".into()]));
    assert!(commands.try_recv().is_err());
    assert!(link.coin_gift_control(0, fields.clone()));
    commands.try_recv()?;
    outbox.coin_gift_result(&oteryn_session::GatedCommandResult{command_id:2,server_sequence:2,status:oteryn_session::CommandStatus::Accepted,payload:oteryn_protocol_oteryn::coin_transfer::encode_reply(r#"{"result":"ok","quote":"019a65f0-0000-7000-8000-000000000002","recipient":"Alice","amount":"25","transferable":"100","non_transferable":"50"}"#).map_err(|_|"quote")?});
    assert!(link.coin_gift_control(1, fields.clone()));
    let crate::combat_input::PlayCommand::ServerMechanic(first) = commands.try_recv()? else {
        return Err("confirm".into());
    };
    outbox.coin_gift_result(&oteryn_session::GatedCommandResult {
        command_id: 3,
        server_sequence: 3,
        status: oteryn_session::CommandStatus::Accepted,
        payload: oteryn_protocol_oteryn::coin_transfer::encode_reply(r#"{"result":"unavailable"}"#)
            .map_err(|_| "reply")?,
    });
    assert!(!link.coin_gift_control(1, fields.clone()));
    assert!(link.coin_gift_control(3, fields));
    let crate::combat_input::PlayCommand::ServerMechanic(retry) = commands.try_recv()? else {
        return Err("retry".into());
    };
    assert_eq!(first, retry);
    assert!(link.store_panel().gift.receipt.is_empty());
    Ok(())
}

include!("ledger.rs");

#[test]
fn store_category_navigation_sends_server_key_with_fresh_cursor()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cui3::store::*;
    let (link, mut rx, outbox) = play_channel();
    outbox.store_initialize(true, false);
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.store.catalogue_category = "consumables.exercise-weapons".into();
        mailbox.store.catalogue = Some(StoreCataloguePage {
            catalog_revision: "previous-category".into(),
            offers: Vec::new(),
            next_page_cursor: "previous-page".into(),
            total_coins: 0,
            transferable_coins: 0,
        });
    }
    assert!(link.store_category("houses.hirelings"));
    let crate::combat_input::PlayCommand::ServerMechanic(command) = rx.try_recv()? else {
        return Err("missing catalogue request".into());
    };
    let request = decode_catalogue_request(&command.payload).map_err(|_| "bad category request")?;
    assert_eq!(request.category, "houses.hirelings");
    assert!(request.page_cursor.is_empty());
    assert!(!link.store_category("not-a-category"));
    Ok(())
}
#[path = "browser.rs"]
mod browser;

#[path = "cosmetic_tokens.rs"]
mod cosmetic_tokens;

#[test]
fn store_retry_blocks_catalogue_replacement() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cui3::store::*;
    let (link, mut commands, outbox) = play_channel();
    outbox.store_initialize(true, false);
    let page = StoreCataloguePage {
        catalog_revision: "A".into(),
        offers: vec![StoreOffer {
            offer_key: "mount".into(),
            title: "Mount A".into(),
            price: 25,
            currency: "TIBIA_COIN".into(),
        }],
        next_page_cursor: String::new(),
        total_coins: 100,
        transferable_coins: 0,
    };
    crate::play::mailbox::lock(&link.mailbox).store.catalogue = Some(page.clone());
    assert!(link.store_confirm_purchase(&page, "", "", ""));
    let first = commands.try_recv()?;
    outbox.store_result(
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            status: oteryn_session::CommandStatus::Rejected,
            server_sequence: 1,
            payload: vec![],
        },
    );
    link.store_browse("featured", "", 0, 0);
    assert!(
        commands.try_recv().is_err(),
        "retry quote must not be replaced by browsing"
    );
    let category = oteryn_client_ui::store::categories()
        .into_iter()
        .next()
        .ok_or("category")?;
    assert!(!link.store_category(&category));
    assert!(commands.try_recv().is_err());
    link.store_request(183, vec![]);
    assert!(commands.try_recv().is_err());
    assert_eq!(link.store_panel().catalogue, Some(page.clone()));
    assert!(link.store_confirm_purchase(&page, "", "", ""));
    let crate::combat_input::PlayCommand::ServerMechanic(first) = first else {
        return Err("purchase".into());
    };
    let crate::combat_input::PlayCommand::ServerMechanic(retry) = commands.try_recv()? else {
        return Err("retry".into());
    };
    assert_eq!(first.payload, retry.payload);
    Ok(())
}

#[test]
fn activity_discard_releases_independent_pending_and_retains_gift_identity()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, mut commands, outbox) = play_channel();
    outbox.store_initialize(true, false);
    outbox.daily_reward_initialize(true);
    let fields = ["Alice".into(), "25".into()];
    assert!(link.coin_gift_control(0, fields.clone()));
    commands.try_recv()?;
    outbox.coin_gift_result(&oteryn_session::GatedCommandResult { command_id: 1, server_sequence: 1, status: oteryn_session::CommandStatus::Accepted, payload: oteryn_protocol_oteryn::coin_transfer::encode_reply(r#"{"result":"ok","quote":"019a65f0-0000-7000-8000-000000000001","recipient":"Alice","amount":"25","transferable":"100","non_transferable":"50"}"#).map_err(|_| "quote")? });
    assert!(link.coin_gift_control(1, fields.clone()));
    let confirmed = link.store_panel().gift.confirmed;
    {
        let mut m = crate::play::mailbox::lock(&link.mailbox);
        m.daily_reward.returner.install(Some([0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 7]), false);
        m.hireling.selected = true;
    }
    link.returner_reward_request(true);
    link.hireling_request(oteryn_client_ui::hireling::Intent::Refresh);
    crate::play::mailbox::lock(&link.mailbox).activity_reset_commands = true;
    assert!(outbox.death_command(&mut commands).is_err());
    {
        let m = crate::play::mailbox::lock(&link.mailbox);
        assert!(m.store.gift.pending.is_none());
        assert!(!m.daily_reward.returner.pending);
        assert!(m.daily_reward.returner_pending.is_none());
        assert!(m.hireling.pending.is_none());
        assert_eq!(m.store.gift.confirmed, confirmed);
        assert_eq!(
            m.daily_reward.returner.offer.as_ref().map(|o| o.id),
            Some([0, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 7])
        );
    }
    assert!(link.coin_gift_control(3, fields));
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::ServerMechanic(c) if oteryn_protocol_oteryn::coin_transfer::decode(&c.payload).ok() == confirmed)
    );
    link.returner_reward_request(true);
    assert!(commands.try_recv().is_ok());
    Ok(())
}

#[path = "../native/commands.rs"]
mod store_controls;
#[test]
fn store_gender_control_is_distinct_from_classic_store() {
    #[cfg(windows)]
    let _handle = store_controls::handle;
    let mut panel = oteryn_client_ui::store::StorePanel::default();
    assert!(store_controls::preview_control(&mut panel, 15, 0));
    assert_eq!(panel.browser.gender, 1);
    assert!(!store_controls::preview_control(
        &mut panel,
        store_controls::CLASSIC_STORE,
        0
    ));
    assert!(!store_controls::preview_control(&mut panel, 15, 1));
    assert_eq!(panel.browser.gender, 1);
}

#[test]
fn native_coin_history_can_request_older_pages_and_restart()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cui3::store::*;
    let (link, mut commands, outbox) = play_channel();
    outbox.store_initialize(true, false);
    assert!(store_controls::history_control(&link, 13, 0));
    commands.try_recv()?;
    outbox.store_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Accepted,
        payload: encode_coin_history_page(&CoinHistoryPage {
            entries: vec![CoinHistoryEntry {
                entry_id: 42,
                kind: "TRANSFER".into(),
                currency: "TIBIA_COIN".into(),
                available_delta: -25,
                reserved_delta: 0,
                order_id: None,
                offer_key: None,
                quantity: None,
                created_at: 1,
            }],
            next_page_cursor: "42".into(),
        })
        .map_err(|_| "history")?,
    });
    assert!(store_controls::history_control(&link, 27, 0));
    let crate::combat_input::PlayCommand::ServerMechanic(c) = commands.try_recv()? else {
        return Err("history command".into());
    };
    assert_eq!(
        decode_coin_history_request(&c.payload)
            .map_err(|_| "request")?
            .page_cursor,
        "42"
    );
    outbox.store_result(&oteryn_session::GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: oteryn_session::CommandStatus::Accepted,
        payload: encode_coin_history_page(&CoinHistoryPage {
            entries: vec![],
            next_page_cursor: String::new(),
        })
        .map_err(|_| "history")?,
    });
    assert!(!store_controls::history_control(&link, 27, 0));
    assert!(commands.try_recv().is_err());
    assert!(store_controls::history_control(&link, 13, 0));
    let crate::combat_input::PlayCommand::ServerMechanic(c) = commands.try_recv()? else {
        return Err("history command".into());
    };
    assert!(
        decode_coin_history_request(&c.payload)
            .map_err(|_| "request")?
            .page_cursor
            .is_empty()
    );
    Ok(())
}

#[test]
fn store_retry_uses_retained_selected_quote_across_presentations()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cui3::store::*;
    let (link, mut commands, outbox) = play_channel();
    outbox.store_initialize(true, false);
    let page = StoreCataloguePage {
        catalog_revision: "A".into(),
        offers: vec![
            StoreOffer {
                offer_key: "other".into(),
                title: "Other product".into(),
                price: 1,
                currency: "TIBIA_COIN".into(),
            },
            StoreOffer {
                offer_key: "mount".into(),
                title: "Confirmed mount".into(),
                price: 25,
                currency: "TIBIA_COIN".into(),
            },
        ],
        next_page_cursor: String::new(),
        total_coins: 100,
        transferable_coins: 0,
    };
    let first = {
        let mut m = crate::play::mailbox::lock(&link.mailbox);
        m.store.catalogue = Some(page.clone());
        native::purchase(&mut m.store, 1, true).ok_or("purchase")?
    };
    outbox.store_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_session::CommandStatus::Rejected,
        payload: vec![],
    });
    let quote =
        oteryn_client_ui::store::confirmation_quote(&link.store_panel()).ok_or("retained quote")?;
    assert_eq!(quote.offers.len(), 1);
    assert_eq!(quote.offers[0].title, "Confirmed mount");
    assert!(!link.store_confirm_purchase(&page, "", "", ""));
    assert!(commands.try_recv().is_err());
    assert!(link.store_confirm_purchase(&quote, "", "", ""));
    let crate::combat_input::PlayCommand::ServerMechanic(retry) = commands.try_recv()? else {
        return Err("retry".into());
    };
    assert_eq!(first.payload, retry.payload);
    assert_eq!(link.store_panel().selected_offer, 1);
    let request = decode_purchase_request(&retry.payload).map_err(|_| "request")?;
    outbox.store_result(&oteryn_session::GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: oteryn_session::CommandStatus::Accepted,
        payload: encode_purchase_result(&StorePurchaseResult {
            code: StorePurchaseCode::Accepted,
            order_id: Some(request.purchase_request_id),
            operation_id: Some(request.purchase_request_id),
        })
        .map_err(|_| "result")?,
    });
    assert!(link.store_panel().confirmed_quote.is_none());
    assert!(link.store_panel().confirmed_purchase.is_none());
    Ok(())
}

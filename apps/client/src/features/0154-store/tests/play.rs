use crate::play::play_channel;
#[test]
fn store_panel_opens_and_sends() {
    let (link, mut rx, outbox) = play_channel();
    outbox.store_initialize(true);
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
fn coin_gift_reviews_exact_recipient_amount_and_remains_disabled()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cui3::store::StoreCataloguePage;
    let (link, mut commands, outbox) = play_channel();
    outbox.store_initialize(true);
    crate::play::mailbox::lock(&link.mailbox).store.catalogue = Some(StoreCataloguePage {
        catalog_revision: "r1".into(),
        offers: vec![],
        next_page_cursor: String::new(),
        total_coins: 100,
        transferable_coins: 40,
    });
    link.store_gift_open();
    let action = |code, recipient: &str, amount: &str| {
        crate::play::feature_0154_store::input::gift_submit(
            &mut crate::play::mailbox::lock(&link.mailbox).store,
            code,
            &[recipient.into(), amount.into()],
        )
    };
    assert!(action(0, "Alice", "20"));
    assert!(!action(1, "Bob", "20"));
    assert!(
        link.store_panel()
            .feedback
            .as_deref()
            .is_some_and(|s| s.contains("Review"))
    );
    assert!(!action(0, "Alice", "41"));
    assert!(action(0, "Alice", "20"));
    crate::play::mailbox::lock(&link.mailbox)
        .store
        .catalogue
        .as_mut()
        .ok_or("catalogue")?
        .transferable_coins = 10;
    assert!(!action(1, "Alice", "20"));
    assert!(
        link.store_panel()
            .feedback
            .as_deref()
            .is_some_and(|s| s.contains("exceeds"))
    );
    crate::play::mailbox::lock(&link.mailbox)
        .store
        .catalogue
        .as_mut()
        .ok_or("catalogue")?
        .transferable_coins = 40;
    assert!(action(0, "Alice", "20"));
    assert!(!action(1, "Alice", "20"));
    assert!(
        link.store_panel()
            .feedback
            .as_deref()
            .is_some_and(|s| s.contains("No coins were moved"))
    );
    assert!(commands.try_recv().is_err());
    assert_eq!(
        link.store_panel()
            .catalogue
            .as_ref()
            .map(|p| p.transferable_coins),
        Some(40)
    );
    assert!(!link.store_panel().pending);
    outbox.store_initialize(false);
    assert!(!link.store_panel().gift_open);
    assert!(link.store_panel().gift_reviewed.is_none());
    assert!(link.store_panel().gift_fields.iter().all(String::is_empty));
    assert!(!action(0, "Alice", "1"));
    Ok(())
}

include!("ledger.rs");

#[test]
fn store_category_navigation_sends_server_key_with_fresh_cursor()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::cui3::store::*;
    let (link, mut rx, outbox) = play_channel();
    outbox.store_initialize(true);
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

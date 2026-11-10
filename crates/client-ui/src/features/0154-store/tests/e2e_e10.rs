//! E10 replay ownership:
//! - `apply_result` in `crates/client-ui/src/features/0154-store/protocol.rs` consumes the
//!   catalogue page and the purchase results; `panel_text` in `view.rs` shows them;
//! - `apply_result` in `crates/client-ui/src/features/0285-store-purchase-history/protocol.rs`
//!   consumes the purchase-history page; `panel_text` in its `view.rs` shows it.
use super::*;
use crate::purchase_history;
use oteryn_client_runtime::cui3::store::{
    self as wire, StoreCataloguePage, StoreHistoryEntry, StoreHistoryPage, StoreOffer,
    StorePurchaseCode, StorePurchaseResult,
};
use oteryn_client_runtime::server_mechanic::{
    CommandStatus, ServerMechanicCommand, ServerMechanicResponse,
};

type Error = Box<dyn std::error::Error>;

struct Capture {
    catalogue: Vec<u8>,
    purchase: Vec<u8>,
    stale: Vec<u8>,
    history: Vec<u8>,
    expected: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn capture() -> Result<Capture, Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok(Capture {
            catalogue: std::fs::read(root.join("e10-catalogue.bin"))?,
            purchase: std::fs::read(root.join("e10-purchase.bin"))?,
            stale: std::fs::read(root.join("e10-stale.bin"))?,
            history: std::fs::read(root.join("e10-history.bin"))?,
            expected: std::fs::read_to_string(root.join("e10-expected.txt"))?,
        });
    }
    let mut order = [0x11; 16];
    let mut operation = [0x22; 16];
    for id in [&mut order, &mut operation] {
        id[6] = 0x71;
        id[8] = 0x81;
    }
    let offer = |key: &str, title: &str, price| StoreOffer {
        offer_key: key.into(),
        title: title.into(),
        price,
        currency: "TIBIA_COIN".into(),
    };
    let page = StoreCataloguePage {
        catalog_revision: "r1".into(),
        offers: vec![
            offer("store:exercise:25", "Training Sword", 25),
            offer("store:exercise:90", "Training Axe", 90),
        ],
        next_page_cursor: String::new(),
        total_coins: 120,
        transferable_coins: 120,
    };
    let history = StoreHistoryPage {
        entries: vec![StoreHistoryEntry {
            order_id: order,
            offer_key: "store:exercise:25".into(),
            catalog_revision: "r1".into(),
            price: 25,
            currency: "TIBIA_COIN".into(),
            purchased_at_unix_ms: 1_700_000_000_000,
        }],
        next_page_cursor: String::new(),
    };
    let result = |code, order_id, operation_id| {
        wire::encode_purchase_result(&StorePurchaseResult {
            code,
            order_id,
            operation_id,
        })
    };
    let encoded = |value: Result<Vec<u8>, wire::StoreWireError>| {
        value.map_err(|error| format!("E10 fixture: {error:?}"))
    };
    Ok(Capture {
        catalogue: encoded(wire::encode_catalogue_page(&page))?,
        purchase: encoded(result(
            StorePurchaseCode::Accepted,
            Some(order),
            Some(operation),
        ))?,
        stale: encoded(result(StorePurchaseCode::StaleQuote, None, None))?,
        history: encoded(wire::encode_history_page(&history))?,
        expected: format!(
            "category=consumables.exercise-weapons\nbalance=120\n\
             offer=store:exercise:25|Training Sword|25|TIBIA_COIN\n\
             offer=store:exercise:90|Training Axe|90|TIBIA_COIN\n\
             purchased_index=0\npurchase=Accepted|{}|{}\nstale=StaleQuote\n\
             history={}|store:exercise:25|25|TIBIA_COIN|1700000000000\n",
            hex(&order),
            hex(&operation),
            hex(&order)
        ),
    })
}

fn response(status: CommandStatus, payload: &[u8]) -> ServerMechanicResponse {
    ServerMechanicResponse {
        command_id: 1,
        status,
        server_sequence: 1,
        payload: payload.to_vec(),
    }
}

fn field<'a>(expected: &'a str, key: &str) -> Result<Vec<&'a str>, Error> {
    let prefix = format!("{key}=");
    Ok(expected
        .lines()
        .filter_map(|line| line.strip_prefix(prefix.as_str()))
        .collect())
}

fn one<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    field(expected, key)?
        .first()
        .copied()
        .ok_or_else(|| format!("E10 expected value {key} absent").into())
}

#[test]
fn e10_real_server_payloads_replay_into_store_panels() -> Result<(), Error> {
    let capture = capture()?;
    let expected = capture.expected.as_str();

    let mut panel = StorePanel {
        selected: true,
        category: one(expected, "category")?.into(),
        ..StorePanel::default()
    };
    let command: ServerMechanicCommand = catalogue(&mut panel).ok_or("catalogue request")?;
    assert_eq!(
        command.command_type,
        wire::COMMAND_TYPE_STORE_CATALOGUE_PAGE
    );
    apply_result(
        &mut panel,
        &response(CommandStatus::Accepted, &capture.catalogue),
    );
    let mut want = format!(
        "Store - Catalogue\nBalance: {} Oteryn Coins\nSearch/sort cover the loaded category page. Sort: Catalogue",
        one(expected, "balance")?
    );
    let mut preview = None;
    for (index, offer) in field(expected, "offer")?.into_iter().enumerate() {
        let parts: Vec<&str> = offer.split('|').collect();
        let [key, title, price, currency] = parts.as_slice() else {
            return Err("E10 offer line".into());
        };
        want.push_str(&format!("\n{title}: {price} {currency}"));
        if index == 0 {
            preview = Some((*key, *title, *price, *currency));
        }
    }
    let (key, title, price, currency) = preview.ok_or("E10 first offer")?;
    let revision = &panel
        .catalogue
        .as_ref()
        .ok_or("catalogue absent")?
        .catalog_revision;
    want.push_str(&format!(
        "\nPreview: {title} — {price} {currency}\r\nOffer: {key}; catalogue revision: {revision}"
    ));
    assert_eq!(panel_text(&panel), want);

    // A refused catalogue refresh leaves the shown catalogue untouched.
    let shown = panel.catalogue.clone();
    catalogue(&mut panel).ok_or("second catalogue request")?;
    apply_result(&mut panel, &response(CommandStatus::Rejected, &[]));
    assert_eq!(panel.catalogue, shown);
    assert_eq!(
        panel.feedback.as_deref(),
        Some("Server refused store action: Rejected")
    );

    panel.selected_offer = one(expected, "purchased_index")?.parse()?;
    panel.purchase_request_id = [0x3e; 16];
    panel.purchase_request_id[6] = 0x7e;
    panel.purchase_request_id[8] = 0x8e;
    panel.purchase_confirmed = true;
    purchase(&mut panel).ok_or("purchase request")?;
    apply_result(
        &mut panel,
        &response(CommandStatus::Accepted, &capture.purchase),
    );
    let accepted = panel.purchase.clone().ok_or("purchase result absent")?;
    let wanted = one(expected, "purchase")?.split('|').collect::<Vec<_>>();
    let [code, order, operation] = wanted.as_slice() else {
        return Err("E10 purchase line".into());
    };
    assert_eq!(format!("{:?}", accepted.code), *code);
    assert_eq!(
        accepted.order_id.map(|id| hex(&id)).as_deref(),
        Some(*order)
    );
    assert_eq!(
        accepted.operation_id.map(|id| hex(&id)).as_deref(),
        Some(*operation)
    );
    assert!(panel_text(&panel).ends_with("\nPurchase: Accepted"));

    panel.purchase_confirmed = true;
    purchase(&mut panel).ok_or("stale purchase request")?;
    apply_result(
        &mut panel,
        &response(CommandStatus::Accepted, &capture.stale),
    );
    let stale = panel.purchase.clone().ok_or("stale result absent")?;
    assert_eq!(format!("{:?}", stale.code), one(expected, "stale")?);
    assert_eq!((stale.order_id, stale.operation_id), (None, None));

    let mut history = purchase_history::Panel {
        selected: true,
        ..purchase_history::Panel::default()
    };
    purchase_history::protocol::request(&mut history, false).ok_or("history request")?;
    purchase_history::protocol::apply_result(
        &mut history,
        &response(CommandStatus::Accepted, &capture.history),
    );
    let row = one(expected, "history")?.split('|').collect::<Vec<_>>();
    let [_, key, price, currency, at] = row.as_slice() else {
        return Err("E10 history line".into());
    };
    assert!(purchase_history::panel_text(&history).contains(&format!(
        "\r\n{key} — {price} {currency} — {at} ms since Unix epoch"
    )));

    let before = history.page.clone();
    purchase_history::protocol::request(&mut history, false).ok_or("second history request")?;
    purchase_history::protocol::apply_result(
        &mut history,
        &response(CommandStatus::Accepted, &[0xff]),
    );
    assert_eq!(history.page, before);
    assert_eq!(
        history.feedback,
        "Purchase history refused or invalid reply; refresh"
    );
    Ok(())
}

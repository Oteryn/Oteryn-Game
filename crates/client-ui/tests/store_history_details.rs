use oteryn_client_runtime::discovery::store::{StoreHistoryEntry, StoreHistoryPage};
use oteryn_client_ui::purchase_history::{Panel, panel_text};
#[test]
fn coin_purchase_details_include_order_identity_and_quote_revision() {
    let panel = Panel {
        selected: true,
        page: Some(StoreHistoryPage {
            entries: vec![StoreHistoryEntry {
                order_id: [1; 16],
                offer_key: "mount".into(),
                catalog_revision: "r17".into(),
                price: 73,
                currency: "coins".into(),
                purchased_at_unix_ms: 42,
            }],
            next_page_cursor: "next".into(),
        }),
        ..Panel::default()
    };
    let text = panel_text(&panel);
    assert!(text.contains("Order: 01010101010101010101010101010101"));
    assert!(text.contains("Catalogue revision: r17"));
    assert!(text.contains("Debit: 73 coins"));
    assert!(text.contains("More purchases: Next page"));
}

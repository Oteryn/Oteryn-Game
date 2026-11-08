use oteryn_client_runtime::discovery::store::{StoreHistoryEntry, StoreHistoryPage};
use oteryn_client_ui::purchase_history::{Panel, panel_text, set_filter};
#[test]
fn history_filters_offer_and_inclusive_time_range_without_combining_currencies() {
    let mut p = Panel {
        selected: true,
        ..Panel::default()
    };
    let entries = [
        ("mount", 12, 7, "coins"),
        ("mount", 18, 3, "tokens"),
        ("mount", 5, 100, "coins"),
        ("outfit", 15, 30, "coins"),
    ]
    .into_iter()
    .map(|(key, at, price, currency)| StoreHistoryEntry {
        order_id: [1; 16],
        offer_key: key.into(),
        catalog_revision: "r1".into(),
        price,
        currency: currency.into(),
        purchased_at_unix_ms: at,
    })
    .collect();
    p.page = Some(StoreHistoryPage {
        entries,
        next_page_cursor: "next".into(),
    });
    assert!(set_filter(&mut p, "MOUNT", "10", "20"));
    let text = panel_text(&p);
    assert!(text.contains("Filtered page total: 7 coins"));
    assert!(text.contains("Filtered page total: 3 tokens"));
    assert!(!text.contains("100 coins"));
    assert!(!text.contains("outfit"));
    assert!(!set_filter(&mut p, "", "20", "10"));
    assert_eq!(p.offer_filter, "MOUNT");
    assert!(set_filter(&mut p, "none", "", ""));
    assert!(panel_text(&p).contains("No purchases match filters on this page"));
    assert_eq!(
        p.page.as_ref().map(|page| page.next_page_cursor.as_str()),
        Some("next")
    );
}

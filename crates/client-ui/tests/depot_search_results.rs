use oteryn_client_runtime::cui3::depot_search::{Outcome, Reply, SearchResult, TierCount};
use oteryn_client_ui::depot_search::{DepotSearchPanel, panel_text};
#[test]
fn depot_results_show_the_picker_index_tiers_and_retrieve_scope() {
    let p = DepotSearchPanel {
        selected: true,
        item_id: 42,
        tier: 2,
        reply: Some(Reply {
            outcome: Outcome::More,
            retrieved: 0,
            results: vec![SearchResult {
                client_id: 42,
                depot: u32::MAX,
                stash: 2,
                inbox: 3,
                tiers: vec![TierCount { tier: 2, count: 7 }],
            }],
        }),
        ..DepotSearchPanel::default()
    };
    let text = panel_text(&p);
    assert!(text.contains("Row 0 | Item 42"));
    assert!(text.contains("total 4294967300"));
    assert!(text.contains("Selected tier: 2"));
    assert!(text.contains("Retrieve all includes every tier"));
    assert!(text.contains("More results: Next page"));
}

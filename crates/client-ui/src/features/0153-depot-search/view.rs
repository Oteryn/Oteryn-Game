use std::fmt::Write;
pub fn panel_text(p: &super::DepotSearchPanel) -> String {
    let mut s = lfmt!(
        "Depot Search\nItem id: {}\nSelected tier: {}\nRetrieve one retrieves one unit. Retrieve all includes every tier of the selected item.",
        p.item_id,
        p.tier
    );
    if let Some(r) = &p.reply {
        let _ = lwriteln!(s, "\nResult: {:?}\nRetrieved: {}", r.outcome, r.retrieved);
        if r.outcome == oteryn_client_runtime::cui3::depot_search::Outcome::More {
            s.push_str(tr!("\nMore results: Next page"));
        }
        if r.results.is_empty() {
            s.push_str(tr!("\nNo matching holdings on this page"));
        }
        for (index, x) in r.results.iter().enumerate() {
            let _ = lwrite!(
                s,
                "\nRow {index} | Item {}: depot {}, stash {}, inbox {}; total {}",
                x.client_id,
                x.depot,
                x.stash,
                x.inbox,
                u64::from(x.depot) + u64::from(x.stash) + u64::from(x.inbox),
                index = index
            );
            for t in &x.tiers {
                let _ = lwrite!(s, "; tier {} x{}", t.tier, t.count);
            }
        }
    }
    if let Some(f) = &p.feedback {
        let _ = write!(s, "\n{f}");
    }
    s
}

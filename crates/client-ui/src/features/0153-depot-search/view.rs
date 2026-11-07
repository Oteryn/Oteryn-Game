use std::fmt::Write;
pub fn panel_text(p: &super::DepotSearchPanel) -> String {
    let mut s = format!("Depot Search\nItem id: {}", p.item_id);
    if let Some(r) = &p.reply {
        let _ = writeln!(s, "\nResult: {:?}", r.outcome);
        for x in &r.results {
            let _ = write!(
                s,
                "\nItem {}: depot {}, stash {}, inbox {}",
                x.client_id, x.depot, x.stash, x.inbox
            );
            for t in &x.tiers {
                let _ = write!(s, "; tier {} x{}", t.tier, t.count);
            }
        }
    }
    if let Some(f) = &p.feedback {
        let _ = write!(s, "\n{f}");
    }
    s
}

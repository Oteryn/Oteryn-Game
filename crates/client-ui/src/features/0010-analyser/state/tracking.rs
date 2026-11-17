//! Local tracker selection over confirmed drop facts; recent entries have bounded retention.
use super::Panel;
use std::fmt::Write;
pub const MAX_TRACKED: usize = 128;
pub const MAX_RECENT: usize = 1024;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Drop {
    pub item: u32,
    pub count: u64,
    pub seconds: u64,
}
pub fn select(panel: &mut Panel, value: &str, tracked: bool) -> bool {
    let Some(item) = value.trim().parse::<u32>().ok().filter(|id| *id != 0) else {
        panel.feedback = Some(tr!("Enter an item number to track"));
        return false;
    };
    if tracked && !panel.loot.contains_key(&item) && !panel.item_info.contains_key(&item) {
        panel.feedback = Some(tr!(
            "Choose an item from confirmed loot or resolved item information"
        ));
        return false;
    }
    if tracked {
        if panel.loot_tracked.len() >= MAX_TRACKED && !panel.loot_tracked.contains(&item) {
            panel.feedback = Some(tr!("Track up to 128 item types"));
            return false;
        }
        panel.loot_tracked.insert(item);
    } else {
        panel.loot_tracked.remove(&item);
    }
    panel.feedback = None;
    true
}
pub(super) fn record(panel: &mut Panel, item: u32, count: u64, seconds: u64) {
    if panel.loot_drops.len() == MAX_RECENT {
        panel.loot_drops.pop_front();
        panel.omitted_drops = panel.omitted_drops.saturating_add(1);
    }
    panel.loot_drops.push_back(Drop {
        item,
        count,
        seconds,
    });
}
pub fn text(panel: &Panel, seconds: u64) -> String {
    let mut text = lfmt!(
        "Loot Tracker\r\nShowing {} · {} selected item types\r\nUse the item number field to track or untrack an item.\r\n",
        if panel.loot_only_tracked {
            tr!("tracked items")
        } else {
            tr!("all observed and tracked items")
        },
        panel.loot_tracked.len()
    );
    let ids: std::collections::BTreeSet<_> = panel
        .loot
        .keys()
        .chain(panel.loot_tracked.iter())
        .copied()
        .collect();
    for item in ids {
        if panel.loot_only_tracked && !panel.loot_tracked.contains(&item) {
            continue;
        }
        let name = panel
            .item_info
            .get(&item)
            .map_or(tr!("Name unresolved"), |info| info.name.as_str());
        let marker = if panel.loot_tracked.contains(&item) {
            "tracked"
        } else {
            "observed"
        };
        let _ = write!(text, "\r\n#{item} {name} ({marker})");
        if let Some(row) = panel.loot.get(&item) {
            let _ = lwrite!(
                text,
                " · {} confirmed items\r\n{}\r\n",
                row.count,
                row.history.chart(seconds)
            );
        } else {
            text.push_str(tr!(" · no confirmed drops\r\n"));
        }
    }
    text.push_str(tr!("\r\nRecent confirmed drops · newest first\r\n"));
    let mut visible_drops = false;
    for drop in panel
        .loot_drops
        .iter()
        .rev()
        .filter(|drop| !panel.loot_only_tracked || panel.loot_tracked.contains(&drop.item))
    {
        visible_drops = true;
        let _ = lwrite!(
            text,
            "+{}s · #{} × {}\r\n",
            drop.seconds,
            drop.item,
            drop.count
        );
    }
    if panel.loot_drops.is_empty() {
        text.push_str(tr!("No drops received yet\r\n"));
    } else if !visible_drops {
        text.push_str("No recent drops match the tracked item selection\r\n");
    }
    if panel.omitted_drops != 0 {
        let _ = lwrite!(
            text,
            "Earlier recent entries omitted: {}. Item counters and minute charts remain available.\r\n",
            panel.omitted_drops
        );
    }
    if panel.omitted_items != 0 {
        let _ = lwrite!(text, "Unlisted item quantity: {}\r\n", panel.omitted_items);
    }
    if let Some(feedback) = panel.feedback {
        text.push_str(feedback);
    }
    text
}

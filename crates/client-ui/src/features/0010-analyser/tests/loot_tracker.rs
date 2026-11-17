use super::{
    Panel, Tool, panel_text,
    protocol::{AnalyserFact, apply_at, recover, reset},
};

#[test]
fn loot_tracker_filters_confirmed_drop_history_and_preserves_selection_on_reset()
-> Result<(), &'static str> {
    use oteryn_client_runtime::analyser::AnalyserItemCount;
    let item = |id, count| -> Result<AnalyserItemCount, &'static str> {
        Ok(AnalyserItemCount {
            item_type: std::num::NonZeroU32::new(id).ok_or("item")?,
            count: std::num::NonZeroU16::new(count).ok_or("count")?,
        })
    };
    let mut panel = Panel {
        selected: true,
        tool: Tool::LootTracker,
        ..Default::default()
    };
    apply_at(
        &mut panel,
        &[AnalyserFact::KillLoot {
            race: None,
            corpse_items: vec![item(6, 3)?, item(7, 1)?],
            gold: 0,
        }],
        2,
    );
    apply_at(
        &mut panel,
        &[AnalyserFact::KillLoot {
            race: None,
            corpse_items: vec![item(6, 2)?],
            gold: 0,
        }],
        65,
    );
    assert!(!panel.open);
    assert!(!super::tracking::select(&mut panel, "0", true));
    assert!(!super::tracking::select(&mut panel, "999", true));
    assert!(super::tracking::select(&mut panel, "6", true));
    panel.loot_only_tracked = true;
    let text = super::tracking::text(&panel, 65);
    assert!(text.contains("#6 Name unresolved (tracked) · 5 confirmed items"));
    assert!(text.contains("+65s · #6 × 2"));
    assert!(text.contains("+2s · #6 × 3"));
    assert!(!text.contains("#7"));
    assert_eq!(
        panel.loot.get(&6).ok_or("loot row")?.history.values(65),
        vec![3, 2]
    );
    reset(&mut panel);
    assert!(panel.loot_tracked.contains(&6) && panel.loot_only_tracked);
    assert!(panel.loot_drops.is_empty());
    assert!(panel_text(&panel).contains("no confirmed drops"));
    assert!(super::tracking::select(&mut panel, "6", false));
    assert!(!panel_text(&panel).contains("#6"));
    recover(&mut panel);
    assert!(panel.loot_tracked.is_empty());
    Ok(())
}

#[test]
fn loot_tracker_bounds_recent_entries_without_losing_item_totals() {
    let mut panel = Panel {
        selected: true,
        tool: Tool::LootTracker,
        ..Default::default()
    };
    let fact = AnalyserFact::KillLoot {
        race: None,
        corpse_items: vec![oteryn_client_runtime::analyser::AnalyserItemCount {
            item_type: std::num::NonZeroU32::MIN,
            count: std::num::NonZeroU16::MIN,
        }],
        gold: 0,
    };
    for seconds in 0..=super::tracking::MAX_RECENT as u64 {
        apply_at(&mut panel, std::slice::from_ref(&fact), seconds);
    }
    assert_eq!(panel.loot_drops.len(), super::tracking::MAX_RECENT);
    assert_eq!(panel.omitted_drops, 1);
    assert_eq!(panel.loot_items, super::tracking::MAX_RECENT as u64 + 1);
    assert!(panel_text(&panel).contains("Earlier recent entries omitted: 1"));
}

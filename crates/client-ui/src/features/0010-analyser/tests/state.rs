use super::{Panel, Tool, panel_text, protocol::*, select_tool};
#[test]
fn confirmed_state_is_visible() -> Result<(), &'static str> {
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    apply(
        &mut panel,
        &[
            AnalyserFact::Kill { race: None },
            AnalyserFact::KillLoot {
                race: None,
                corpse_items: vec![],
                gold: 7,
            },
            AnalyserFact::KillLoot {
                race: None,
                corpse_items: vec![],
                gold: 3,
            },
            AnalyserFact::Experience { raw: 8, gained: 0 },
            AnalyserFact::Dropped(std::num::NonZeroU32::new(2).ok_or("count")?),
        ],
    );
    assert_eq!(
        (
            panel.kills,
            panel.gold,
            panel.raw_xp,
            panel.gained_xp,
            panel.dropped
        ),
        (1, 10, 8, 0, 2)
    );
    assert!(panel_text(&panel).contains("Kills: 1"));
    reset(&mut panel);
    assert!(panel.selected);
    assert_eq!(panel.gold, 0);
    panel.selected = false;
    apply(&mut panel, &[AnalyserFact::Kill { race: None }]);
    assert_eq!(panel.kills, 0);
    Ok(())
}

#[test]
fn selector_keeps_confirmed_totals_and_shows_only_the_selected_tool() {
    use super::Tool;
    let mut panel = Panel {
        selected: true,
        ..Panel::default()
    };
    apply(
        &mut panel,
        &[
            AnalyserFact::Kill { race: None },
            AnalyserFact::Experience {
                raw: 25,
                gained: 30,
            },
        ],
    );
    panel.tool = Tool::Experience;
    let text = panel_text(&panel);
    assert!(text.contains("Experience: 25 raw / 30 gained"));
    assert!(!text.contains("Kills:"));
    panel.tool = Tool::Hunting;
    assert!(panel_text(&panel).contains("Kills: 1"));
    assert!(!panel_text(&panel).contains("Experience:"));
    reset(&mut panel);
    assert_eq!(panel.tool, Tool::Hunting);
    assert!(panel_text(&panel).contains("Kills: 0"));
    for tool in Tool::ALL {
        panel.tool = tool;
        assert!(panel_text(&panel).contains(tool.title()));
    }
}

#[test]
fn analytics_selector_opens_independent_tools_over_confirmed_totals() {
    let mut panel = Panel {
        selected: true,
        kills: 2,
        gained_xp: 30,
        ..Default::default()
    };
    assert!(select_tool(&mut panel, "xp"));
    assert!(panel_text(&panel).starts_with("XP Analyser"));
    assert!(!panel_text(&panel).contains("Kills:"));
    assert!(select_tool(&mut panel, "kills"));
    assert!(panel_text(&panel).contains("Kills: 2"));
    assert_eq!(panel.gained_xp, 30);
    assert!(!select_tool(&mut panel, "unknown"));
    assert_eq!(panel.tool, Tool::Kills);
}

#[test]
fn xp_rates_goal_eta_and_minute_chart_follow_confirmed_experience() {
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    assert!(super::set_xp_goal(&mut panel, "1200"));
    apply_at(
        &mut panel,
        &[AnalyserFact::Experience {
            raw: 400,
            gained: 600,
        }],
        0,
    );
    apply_at(
        &mut panel,
        &[AnalyserFact::Experience {
            raw: 100,
            gained: 200,
        }],
        120,
    );
    let text = super::view::xp_text(&panel, 240);
    assert!(text.contains("XP/h: 12000"));
    assert!(text.contains("Raw XP/h: 7500"));
    assert!(text.contains("Level goal remaining: 400 XP"));
    assert!(text.contains("Estimated time to level goal: 120 seconds"));
    assert_eq!(panel.xp_chart.values(120), vec![600, 0, 200]);
    assert_eq!(panel.xp_chart.values(60 * 100).len(), 60);
    assert!(super::view::xp_text(&Panel::default(), 0).contains("Collect experience"));
    assert!(!super::set_xp_goal(&mut panel, "invalid"));
    assert_eq!(panel.goal_xp, Some(1200));
}

#[test]
fn loot_items_reprice_confirmed_counts_rates_and_history() -> Result<(), &'static str> {
    use std::num::{NonZeroU16, NonZeroU32};
    let mut panel = Panel {
        selected: true,
        tool: Tool::Loot,
        ..Default::default()
    };
    let item = oteryn_client_runtime::analyser::AnalyserItemCount {
        item_type: NonZeroU32::new(100).ok_or("item")?,
        count: NonZeroU16::new(2).ok_or("count")?,
    };
    apply_at(
        &mut panel,
        &[AnalyserFact::KillLoot {
            race: None,
            corpse_items: vec![item],
            gold: 10,
        }],
        0,
    );
    assert!(super::view::loot_text(&panel, 60).contains("Unpriced items: 2"));
    panel.item_info.insert(
        100,
        oteryn_client_runtime::analyser::ItemInfo {
            item_type: 100,
            name: "Loot item".into(),
            coin_worth: 0,
        },
    );
    assert!(super::set_item_price(&mut panel, "100", "25"));
    assert_eq!(panel.loot_value(), 60);
    assert_eq!(panel.loot_values(60), vec![60, 0]);
    let text = super::view::loot_text(&panel, 60);
    assert!(text.contains("Loot value/h: 3600"));
    assert!(text.contains("Item 100: 2 | 120/h | unit 25 | value 50"));
    assert!(super::set_item_price(&mut panel, "100", "50"));
    assert_eq!(panel.loot_value(), 110);
    assert_eq!(panel.loot_values(60), vec![110, 0]);
    assert!(!super::set_item_price(&mut panel, "0", "10"));
    reset(&mut panel);
    assert_eq!(panel.loot_value(), 0);
    assert_eq!(panel.loot_prices.get(&100), Some(&50));
    Ok(())
}

#[test]
fn analyser_metadata_reply_is_matched_and_coin_prices_cannot_duplicate_gold()
-> Result<(), &'static str> {
    use oteryn_client_runtime::{
        CommandStatus,
        analyser::{ItemInfo, encode_reply},
    };
    let mut panel = Panel {
        selected: true,
        gold: 100,
        ..Default::default()
    };
    super::state::items::record_item(&mut panel.loot, 3, 1, 0);
    super::state::items::record_item(&mut panel.loot, 9, 2, 0);
    assert!(super::set_item_price(&mut panel, "3", "100"));
    assert!(super::set_item_price(&mut panel, "9", "25"));
    assert_eq!(panel.loot_value(), 100); // Unknown identities never count as non-currency.
    let command = catalogue_request(&mut panel).ok_or("query")?;
    assert_eq!((command.capability, command.kind), (10, 734));
    assert!(catalogue_request(&mut panel).is_none());
    let rows = vec![
        ItemInfo {
            item_type: 3,
            name: "Coin".into(),
            coin_worth: 100,
        },
        ItemInfo {
            item_type: 9,
            name: "Loot item".into(),
            coin_worth: 0,
        },
    ];
    let mut result = oteryn_client_runtime::raw_feature::GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: encode_reply(&rows)?,
    };
    assert!(catalogue_result(&mut panel, &result));
    assert_eq!(panel.loot_value(), 150);
    panel.item_info.clear();
    catalogue_request(&mut panel).ok_or("query")?;
    result.payload = encode_reply(&rows[..1])?;
    assert!(!catalogue_result(&mut panel, &result));
    assert!(panel.item_info.is_empty());
    Ok(())
}

#[test]
fn supply_counts_costs_rates_and_charts_revalue_real_consumption() -> Result<(), &'static str> {
    use oteryn_client_runtime::analyser::{AnalyserItemCount, ItemInfo};
    let mut panel = Panel {
        selected: true,
        tool: Tool::Supply,
        ..Default::default()
    };
    panel.item_info.insert(
        6,
        ItemInfo {
            item_type: 6,
            name: "Supply".into(),
            coin_worth: 0,
        },
    );
    let item = AnalyserItemCount {
        item_type: std::num::NonZeroU32::new(6).ok_or("item")?,
        count: std::num::NonZeroU16::new(3).ok_or("count")?,
    };
    apply_at(&mut panel, &[AnalyserFact::SupplyUsed(item)], 0);
    assert!(super::set_item_price(&mut panel, "6", "7"));
    assert_eq!(panel.supply_cost(), 21);
    assert_eq!(panel.supply_values(60), vec![21, 0]);
    let text = super::view::supply_text(&panel, 60);
    assert!(text.contains("Supply cost/h: 1260"));
    assert!(text.contains("Item 6: 3 | 180/h | unit 7 | cost 21"));
    assert!(super::set_item_price(&mut panel, "6", "11"));
    assert_eq!(panel.supply_cost(), 33);
    reset(&mut panel);
    assert_eq!(panel.supply_cost(), 0);
    assert_eq!(panel.supply_prices.get(&6), Some(&11));
    Ok(())
}

#[test]
fn every_analyser_tool_marks_a_dropped_server_feed_incomplete() {
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    apply(
        &mut panel,
        &[AnalyserFact::Dropped(std::num::NonZeroU32::MIN)],
    );
    for tool in Tool::ALL {
        panel.tool = tool;
        assert!(
            panel_text(&panel).contains("Session feed incomplete: 1 fact(s) dropped"),
            "{}",
            tool.key()
        );
    }
}

#[test]
fn analyser_recovery_discards_generation_scoped_item_settings() {
    let mut panel = Panel {
        selected: true,
        open: true,
        tool: Tool::Supply,
        ..Default::default()
    };
    assert!(super::set_item_price(&mut panel, "6", "7"));
    assert!(super::set_xp_goal(&mut panel, "100"));
    panel.item_info.insert(
        6,
        oteryn_client_runtime::analyser::ItemInfo {
            item_type: 6,
            name: "Old generation".into(),
            coin_worth: 0,
        },
    );
    panel.catalogue_pending.push(6);
    recover(&mut panel);
    assert!(panel.item_info.is_empty());
    assert!(panel.loot_prices.is_empty());
    assert!(panel.supply_prices.is_empty());
    assert!(panel.catalogue_pending.is_empty());
    assert!(panel.goal_xp.is_none());
    assert_eq!(
        (panel.selected, panel.open, panel.tool),
        (true, true, Tool::Supply)
    );
}

#[test]
fn impact_uses_wire_damage_types_healing_rates_gauges_and_charts() -> Result<(), &'static str> {
    let facts = oteryn_client_runtime::analyser::decode_analyser_facts(&[
        10, 8, 34, 6, 8, 1, 16, 120, 24, 2, 10, 6, 34, 4, 8, 2, 16, 60,
    ])
    .map_err(|_| "impact wire")?;
    let mut panel = Panel {
        selected: true,
        tool: Tool::Impact,
        ..Default::default()
    };
    apply_at(&mut panel, &facts, 0);
    let text = super::view::impact_text(&panel, 60);
    assert!(text.contains("DPS: 2.00 | HPS: 1.00"));
    assert!(text.contains("Fire: 120"));
    assert!(text.contains("66.66%"));
    assert_eq!(panel.damage_chart.values(60), vec![120, 0]);
    assert_eq!(panel.healing_chart.values(60), vec![60, 0]);
    assert!(super::view::impact_text(&Panel::default(), 0).contains("No impact yet"));
    Ok(())
}

#[path = "state_additional.rs"]
mod additional;

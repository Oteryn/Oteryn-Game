use super::{Panel, Tool, panel_text, protocol::*};
#[test]
fn xp_rates_goal_eta_and_minute_chart_follow_confirmed_experience() -> Result<(), &'static str> {
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
    Ok(())
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

#[test]
fn input_preserves_wire_sources_elements_rates_and_reset() -> Result<(), &'static str> {
    // Creature race 7, unknown creature race, player, and explicitly unattributed.
    let facts = oteryn_client_runtime::analyser::decode_analyser_facts(&[
        10, 10, 42, 8, 8, 60, 16, 2, 24, 1, 32, 7, 10, 8, 42, 6, 8, 30, 16, 1, 24, 1, 10, 8, 42, 6,
        8, 20, 16, 2, 24, 2, 10, 8, 42, 6, 8, 10, 16, 3, 24, 3,
    ])
    .map_err(|_| "input wire")?;
    let mut panel = Panel {
        selected: true,
        tool: Tool::Input,
        ..Default::default()
    };
    apply_at(&mut panel, &facts, 0);
    let text = super::view::input_text(&panel, 60);
    for expected in [
        "Incoming DPS: 2.00",
        "Fire: 80",
        "Creature race #7: 60",
        "Creature (race unavailable): 30",
        "Player: 20",
        "Unattributed: 10",
        "50.00%",
    ] {
        assert!(text.contains(expected), "{text}");
    }
    assert_eq!(panel.received_chart.values(60), vec![120, 0]);
    reset(&mut panel);
    assert!(panel.input_types.is_empty() && panel.input_sources.is_empty());
    assert_eq!(panel.received_chart.values(0), vec![0]);
    assert!(panel_text(&panel).contains("No damage received yet"));
    Ok(())
}

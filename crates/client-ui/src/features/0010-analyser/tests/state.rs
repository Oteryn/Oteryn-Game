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
fn hunting_session_reports_signed_balance_rates_and_credited_kills() -> Result<(), &'static str> {
    use oteryn_client_runtime::analyser::{AnalyserItemCount, ItemInfo};
    let mut panel = Panel {
        selected: true,
        tool: Tool::Session,
        ..Default::default()
    };
    panel.item_info.insert(
        6,
        ItemInfo {
            item_type: 6,
            name: "Bandage".into(),
            coin_worth: 0,
        },
    );
    panel.supply_prices.insert(6, 7);
    apply_at(
        &mut panel,
        &[
            AnalyserFact::Kill {
                race: std::num::NonZeroU32::new(7),
            },
            AnalyserFact::Kill { race: None },
            AnalyserFact::KillLoot {
                race: None,
                corpse_items: vec![],
                gold: 10,
            },
            AnalyserFact::Experience {
                raw: 50,
                gained: 60,
            },
            AnalyserFact::SupplyUsed(AnalyserItemCount {
                item_type: std::num::NonZeroU32::new(6).ok_or("item")?,
                count: std::num::NonZeroU16::new(3).ok_or("count")?,
            }),
            AnalyserFact::Dropped(std::num::NonZeroU32::MIN),
        ],
        0,
    );
    let text = super::view::session_text_at(&panel, 60);
    for expected in [
        "Elapsed: 60 seconds",
        "XP/h: 3600",
        "Priced balance: -11 gold",
        "Priced balance/h: -660 gold",
        "Kills: 2",
        "Race #7: 1",
        "Race unavailable: 1",
        "Bandage",
        "Dropped facts: 1",
        "resolved, priced items",
    ] {
        assert!(text.contains(expected), "missing {expected}: {text}");
    }
    assert!(!super::view::session_text_at(&panel, 0).contains("NaN"));
    reset(&mut panel);
    assert!(panel.kills_by_race.is_empty());
    assert_eq!(panel.supply_cost(), 0);
    Ok(())
}

#[test]
fn party_split_conserves_positive_negative_and_remainder_balances() -> Result<(), &'static str> {
    use oteryn_client_runtime::analyser::party::Member;
    for values in [
        vec![(100, 10), (0, 20), (5, 0)],
        vec![(0, 100), (0, 11), (0, 0)],
        vec![(0, 0), (0, 0), (1, 0)],
    ] {
        let mut rows: Vec<_> = values
            .iter()
            .enumerate()
            .map(|(i, (loot, cost))| Member {
                handle: i as u64 + 1,
                name: format!("Scout {i}"),
                available: true,
                loot_value: *loot,
                supply_cost: *cost,
                ..Default::default()
            })
            .collect();
        let mut balances: std::collections::BTreeMap<_, _> = rows
            .iter()
            .map(|r| {
                (
                    r.handle,
                    i128::from(r.loot_value) - i128::from(r.supply_cost),
                )
            })
            .collect();
        let original: i128 = balances.values().sum();
        let transfers = super::party::split(&rows).ok_or("complete report")?;
        for transfer in &transfers {
            *balances.get_mut(&transfer.from).ok_or("payer")? -=
                i128::try_from(transfer.gold).map_err(|_| "transfer overflow")?;
            *balances.get_mut(&transfer.to).ok_or("receiver")? +=
                i128::try_from(transfer.gold).map_err(|_| "transfer overflow")?;
        }
        assert_eq!(balances.values().sum::<i128>(), original);
        assert!(balances.values().max().ok_or("max")? - balances.values().min().ok_or("min")? <= 1);
        rows.reverse();
        assert_eq!(super::party::split(&rows), Some(transfers));
        rows[0].unpriced_supplies = 1;
        assert!(super::party::split(&rows).is_none());
        rows[0].unpriced_supplies = 0;
        rows[0].available = false;
        assert!(super::party::split(&rows).is_none());
        rows[0].available = true;
        rows[0].incomplete = true;
        assert!(super::party::split(&rows).is_none());
    }
    Ok(())
}

#[test]
fn party_prices_changed_while_pending_never_publish_stale_settlements() -> Result<(), &'static str>
{
    use oteryn_client_runtime::analyser::party::{Member, encode_reply};
    let mut panel = Panel {
        selected: true,
        tool: Tool::Party,
        ..Default::default()
    };
    let command = super::party::request(&mut panel).ok_or("request")?;
    assert_eq!((command.capability, command.kind), (10, 735));
    assert!(super::party::request(&mut panel).is_none());
    assert!(super::set_item_price(&mut panel, "7", "100"));
    let reply = oteryn_client_runtime::raw_feature::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_client_runtime::CommandStatus::Accepted,
        payload: encode_reply(&[Member {
            handle: 1,
            name: "Scout".into(),
            available: true,
            ..Default::default()
        }])?,
    };
    super::party::result(&mut panel, &reply);
    assert!(panel.party.is_empty());
    assert!(!panel.party_pending);
    assert!(panel_text(&panel).contains("Prices changed"));
    super::party::request(&mut panel).ok_or("retry")?;
    super::party::result(&mut panel, &reply);
    assert_eq!(panel.party.len(), 1);
    super::party::request(&mut panel).ok_or("refresh")?;
    assert!(!panel_text(&panel).contains("Equal balance split"));
    let refused = oteryn_client_runtime::raw_feature::GatedCommandResult {
        status: oteryn_client_runtime::CommandStatus::Rejected,
        payload: vec![],
        ..reply
    };
    super::party::result(&mut panel, &refused);
    assert_eq!(panel.party.len(), 1);
    assert!(!panel_text(&panel).contains("Equal balance split"));
    Ok(())
}

#[test]
fn local_reset_keeps_inflight_party_query_until_its_server_interval_finishes()
-> Result<(), &'static str> {
    let mut panel = Panel {
        selected: true,
        kills: 3,
        ..Default::default()
    };
    super::party::request(&mut panel).ok_or("query")?;
    reset(&mut panel);
    assert_eq!(panel.kills, 0);
    assert!(panel.party_pending);
    assert!(super::party::request(&mut panel).is_none());
    let reply = oteryn_client_runtime::raw_feature::GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: oteryn_client_runtime::CommandStatus::Accepted,
        payload: oteryn_client_runtime::analyser::party::encode_reply(&[
            oteryn_client_runtime::analyser::party::Member {
                handle: 1,
                name: "Scout".into(),
                available: true,
                kills: 3,
                ..Default::default()
            },
        ])?,
    };
    super::party::result(&mut panel, &reply);
    assert_eq!(panel.party[0].kills, 3);
    assert!(!panel.party_pending);
    recover(&mut panel);
    assert!(panel.party.is_empty());
    Ok(())
}

#[test]
fn party_rounding_agrees_for_viewers_with_different_opaque_handles() -> Result<(), &'static str> {
    use oteryn_client_runtime::analyser::party::Member;
    for (payer, receiver) in [(1, 2), (9, 3)] {
        let rows = vec![
            Member {
                handle: payer,
                name: "Zed".into(),
                available: true,
                loot_value: 1,
                ..Default::default()
            },
            Member {
                handle: receiver,
                name: "Ari".into(),
                available: true,
                ..Default::default()
            },
        ];
        assert_eq!(
            super::party::split(&rows).ok_or("split")?,
            vec![super::party::Transfer {
                from: payer,
                to: receiver,
                gold: 1
            }]
        );
    }
    Ok(())
}

use super::*;

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
    let text = super::super::view::input_text(&panel, 60);
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
    let text = super::super::view::session_text_at(&panel, 60);
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
    assert!(!super::super::view::session_text_at(&panel, 0).contains("NaN"));
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
        let transfers = super::super::party::split(&rows).ok_or("complete report")?;
        for transfer in &transfers {
            *balances.get_mut(&transfer.from).ok_or("payer")? -=
                i128::try_from(transfer.gold).map_err(|_| "transfer overflow")?;
            *balances.get_mut(&transfer.to).ok_or("receiver")? +=
                i128::try_from(transfer.gold).map_err(|_| "transfer overflow")?;
        }
        assert_eq!(balances.values().sum::<i128>(), original);
        assert!(balances.values().max().ok_or("max")? - balances.values().min().ok_or("min")? <= 1);
        rows.reverse();
        assert_eq!(super::super::party::split(&rows), Some(transfers));
        rows[0].unpriced_supplies = 1;
        assert!(super::super::party::split(&rows).is_none());
        rows[0].unpriced_supplies = 0;
        rows[0].available = false;
        assert!(super::super::party::split(&rows).is_none());
        rows[0].available = true;
        rows[0].incomplete = true;
        assert!(super::super::party::split(&rows).is_none());
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
    let command = super::super::party::request(&mut panel).ok_or("request")?;
    assert_eq!((command.capability, command.kind), (10, 735));
    assert!(super::super::party::request(&mut panel).is_none());
    assert!(super::super::set_item_price(&mut panel, "7", "100"));
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
    super::super::party::result(&mut panel, &reply);
    assert!(panel.party.is_empty());
    assert!(!panel.party_pending);
    assert!(panel_text(&panel).contains("Prices changed"));
    super::super::party::request(&mut panel).ok_or("retry")?;
    super::super::party::result(&mut panel, &reply);
    assert_eq!(panel.party.len(), 1);
    super::super::party::request(&mut panel).ok_or("refresh")?;
    assert!(!panel_text(&panel).contains("Equal balance split"));
    let refused = oteryn_client_runtime::raw_feature::GatedCommandResult {
        status: oteryn_client_runtime::CommandStatus::Rejected,
        payload: vec![],
        ..reply
    };
    super::super::party::result(&mut panel, &refused);
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
    super::super::party::request(&mut panel).ok_or("query")?;
    reset(&mut panel);
    assert_eq!(panel.kills, 0);
    assert!(panel.party_pending);
    assert!(super::super::party::request(&mut panel).is_none());
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
    super::super::party::result(&mut panel, &reply);
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
            super::super::party::split(&rows).ok_or("split")?,
            vec![super::super::party::Transfer {
                from: payer,
                to: receiver,
                gold: 1
            }]
        );
    }
    Ok(())
}

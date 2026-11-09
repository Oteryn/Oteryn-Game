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

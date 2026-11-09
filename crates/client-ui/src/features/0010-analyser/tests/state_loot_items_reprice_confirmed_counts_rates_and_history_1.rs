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

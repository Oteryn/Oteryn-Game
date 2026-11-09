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

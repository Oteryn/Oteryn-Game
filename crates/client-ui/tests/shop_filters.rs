use oteryn_client_ui::npc_shop::{LiveShop, NpcTradeSide, npc_service};

#[test]
fn filtered_shop_quotes_keep_the_original_server_offer_index()
-> Result<(), Box<dyn std::error::Error>> {
    let mut shop = LiveShop::default();
    shop.window.open = Some((
        oteryn_client_ui::npc_shop::EntityRef {
            identity: [1; 16],
            generation: 1,
        },
        7,
    ));
    shop.window.offers = vec![
        npc_service::NpcOffer {
            item_definition_ref: 42,
            side: NpcTradeSide::Buy,
            count: 1,
            unit_price: 10,
        },
        npc_service::NpcOffer {
            item_definition_ref: 43,
            side: NpcTradeSide::Sell,
            count: 1,
            unit_price: 20,
        },
    ];
    let rows = shop.filtered_quotes(Some(NpcTradeSide::Sell), "43");
    assert_eq!(rows.len(), 1);
    assert_eq!(shop.look(rows[0]).ok_or("missing quote")?.offer_index, 1);
    assert!(shop.filtered_quotes(None, "missing").is_empty());
    assert_eq!(shop.filtered_quotes(None, "").len(), 2);
    shop.window.open = None;
    assert!(shop.filtered_quotes(None, "").is_empty());
    Ok(())
}

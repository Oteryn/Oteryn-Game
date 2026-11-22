//! Capability 501 retains the negotiated server quote metadata in the existing shop owner.
pub fn apply_window(
    shop: &mut crate::npc_shop::LiveShop,
    window: oteryn_client_runtime::npc_shop::npc_service::NpcTradeWindow,
) {
    shop.apply_window(window);
}

#[cfg(test)]
#[test]
fn authoritative_quotes_reach_ui_rows_and_bounds() -> Result<(), String> {
    use oteryn_client_runtime::npc_shop::{EntityRef, NpcTradeSide, npc_service as wire};
    let window = wire::NpcTradeWindow {
        open: Some((
            EntityRef {
                identity: [3; 16],
                generation: 2,
            },
            7,
        )),
        offers: vec![wire::NpcOffer {
            item_definition_ref: 42,
            side: NpcTradeSide::Buy,
            count: 1,
            unit_price: 9,
        }],
        details: Vec::new(),
    };
    // The bytes are the authoritative domain-8 extension, independent of UI state.
    let mut payload = wire::encode_npc_trade_window(&window).map_err(|e| format!("{e:?}"))?;
    payload.extend_from_slice(&[34, 9, 10, 4, b'f', b'i', b's', b'h', 16, 0x90, 0x4e]);
    let decoded =
        wire::decode_npc_trade_window_for(&payload, true).map_err(|e| format!("{e:?}"))?;
    let mut shop = crate::npc_shop::LiveShop {
        selected: true,
        ..Default::default()
    };
    apply_window(&mut shop, decoded);
    let quote = *shop.quotes().first().ok_or("quote")?;
    assert!(shop.quote_text(quote).contains("fish"));
    assert_eq!(shop.maximum_amount(quote), 10000);
    assert!(shop.valid_amount(quote, 10000));
    assert!(!shop.valid_amount(quote, 10001));
    assert_eq!(shop.filtered_quotes(None, "fish"), vec![quote]);
    assert_eq!(shop.trade(quote, 10000).ok_or("trade")?.quantity, 10000);
    let mut high_value = shop.window.clone();
    high_value.offers[0].side = NpcTradeSide::Sell;
    high_value.offers[0].unit_price = 5_400_000;
    high_value.details[0].maximum_quantity = 1;
    let payload =
        wire::encode_npc_trade_window_for(&high_value, true).map_err(|e| format!("{e:?}"))?;
    apply_window(
        &mut shop,
        wire::decode_npc_trade_window_for(&payload, true).map_err(|e| format!("{e:?}"))?,
    );
    let quote = *shop.quotes().first().ok_or("high-value quote")?;
    assert!(shop.valid_amount(quote, 1));
    assert!(!shop.valid_amount(quote, 2));
    Ok(())
}

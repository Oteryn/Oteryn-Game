//! Each purchase captures the retained overflow preference and bag option.
use crate::npc_shop::{LiveShop, NpcTradeSide, Quote, npc_service};
use oteryn_client_runtime::npc_purchase::npc_purchase::NpcPurchaseIntent;
pub fn request(shop: &mut LiveShop, quote: Quote, amount: u32) -> Option<NpcPurchaseIntent> {
    if !shop.selected || !shop.purchase_selected || shop.pending || quote.side != NpcTradeSide::Buy
    {
        return None;
    }
    let index = shop.quotes().iter().position(|live| *live == quote)?;
    let intent = NpcPurchaseIntent {
        offer: npc_service::NpcTradeIntent {
            npc_actor: quote.npc,
            catalogue_revision: quote.revision,
            offer_index: u32::try_from(index).ok()?,
            side: quote.side,
            quantity: 1,
            expected_unit_price: quote.price,
        },
        amount,
        ignore_cap: shop.options.ignore_capacity,
        in_backpacks: shop.options.in_backpacks,
    };
    oteryn_client_runtime::npc_purchase::npc_purchase::encode_npc_purchase(&intent).ok()?;
    shop.pending = true;
    Some(intent)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overflow_defaults_to_ground_and_both_choices_are_sent() {
        let mut shop = LiveShop {
            selected: true,
            purchase_selected: true,
            ..LiveShop::default()
        };
        let npc = oteryn_client_runtime::npc_shop::EntityRef {
            identity: [1; 16],
            generation: 1,
        };
        shop.window = npc_service::NpcTradeWindow {
            open: Some((npc, 1)),
            offers: vec![npc_service::NpcOffer {
                item_definition_ref: 1,
                side: NpcTradeSide::Buy,
                count: 1,
                unit_price: 2,
            }],
        };
        let quote = shop.quotes()[0];
        assert!(shop.options.ignore_capacity);
        let purchase = request(&mut shop, quote, 45);
        assert!(purchase.is_some_and(|purchase| purchase.ignore_cap && !purchase.in_backpacks));
        shop.pending = false;
        shop.options.ignore_capacity = false;
        shop.options.in_backpacks = true;
        assert!(
            request(&mut shop, quote, 45)
                .is_some_and(|purchase| !purchase.ignore_cap && purchase.in_backpacks)
        );
    }
}

use super::*;

use oteryn_protocol_oteryn::npc_shop::encode_npc_shop_result;

fn intent(action: NpcShopAction) -> NpcShopIntent {
    NpcShopIntent {
        npc_actor: oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: [7; 16],
            generation: 3,
        },
        catalogue_revision: 11,
        offer_index: 2,
        side: NpcTradeSide::Sell,
        action,
        expected_unit_price: 10,
    }
}

#[test]
fn a_shop_intent_names_one_unit_of_the_same_offer() {
    let shop = intent(NpcShopAction::SellAll);
    let trade = trade_intent(&shop);
    assert_eq!(trade.npc_actor, shop.npc_actor);
    assert_eq!(trade.catalogue_revision, 11);
    assert_eq!(trade.offer_index, 2);
    assert_eq!(trade.side, NpcTradeSide::Sell);
    assert_eq!(trade.quantity, 1);
    assert_eq!(trade.expected_unit_price, 10);
}

#[test]
fn a_look_text_is_cut_at_a_character_boundary_to_its_bound() {
    assert_eq!(bounded("You see a sword.".into()), "You see a sword.");
    let wide = "\u{17c}".repeat(MAX_ITEM_LOOK_TEXT_BYTES);
    let cut = bounded(wide);
    assert!(cut.len() <= MAX_ITEM_LOOK_TEXT_BYTES);
    assert_eq!(cut.len(), MAX_ITEM_LOOK_TEXT_BYTES);
    let odd = format!("x{}", "\u{17c}".repeat(MAX_ITEM_LOOK_TEXT_BYTES));
    assert_eq!(bounded(odd).len(), MAX_ITEM_LOOK_TEXT_BYTES - 1);
}

#[test]
fn every_step_encodes_its_result() {
    for step in [
        NpcShopStep::Looked("You see a sword.".into()),
        NpcShopStep::Sold {
            units: 111,
            gold: 1_110,
        },
        NpcShopStep::rejected(),
        NpcShopStep::Refused(NpcTradeRefusal::only(NpcIntentDisposition::NotInRange)),
        NpcShopStep::Unknown,
    ] {
        encode_npc_shop_result(&step.result()).expect("encode");
    }
    let sold = NpcShopStep::Sold {
        units: 111,
        gold: 1_110,
    }
    .result();
    assert_eq!(
        (sold.disposition, sold.sold_units, sold.total_gold_units),
        (NpcIntentDisposition::Ok, 111, 1_110)
    );
    assert_eq!(
        NpcShopStep::rejected().result().disposition,
        NpcIntentDisposition::Rejected
    );
}

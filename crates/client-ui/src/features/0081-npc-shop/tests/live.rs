use super::*;
#[test]
fn quote_identity_survives_display_order_but_not_revision() -> Result<(), String> {
    let npc = EntityRef {
        identity: [3; 16],
        generation: 8,
    };
    let offer = npc_service::NpcOffer {
        item_definition_ref: 42,
        side: NpcTradeSide::Sell,
        count: 1,
        unit_price: 17,
    };
    let mut shop = LiveShop {
        selected: true,
        ..Default::default()
    };
    shop.apply_window(npc_service::NpcTradeWindow {
        open: Some((npc, 9)),
        offers: vec![offer],
    });
    let key = shop.quotes()[0];
    assert!(shop.trade(key, 0).is_none());
    let command = shop.trade(key, 3).ok_or("trade")?;
    assert_eq!(
        (
            command.offer_index,
            command.quantity,
            command.expected_unit_price
        ),
        (0, 3, 17)
    );
    assert!(shop.trade(key, 1).is_none());
    shop.result(NpcIntentDisposition::Stale);
    assert!(shop.text().contains("Stale"));
    shop.apply_window(npc_service::NpcTradeWindow {
        open: Some((npc, 10)),
        offers: vec![offer],
    });
    assert!(shop.trade(key, 1).is_none());
    shop.apply_window(Default::default());
    assert!(shop.quotes().is_empty());
    Ok(())
}

#[test]
fn quote_lookup_uses_item_side_count_and_price_not_a_display_position() -> Result<(), String> {
    let npc = EntityRef {
        identity: [4; 16],
        generation: 2,
    };
    let first = npc_service::NpcOffer {
        item_definition_ref: 11,
        side: NpcTradeSide::Buy,
        count: 5,
        unit_price: 19,
    };
    let second = npc_service::NpcOffer {
        item_definition_ref: 99,
        side: NpcTradeSide::Sell,
        count: 1,
        unit_price: 2,
    };
    let mut shop = LiveShop {
        selected: true,
        ..Default::default()
    };
    shop.apply_window(npc_service::NpcTradeWindow {
        open: Some((npc, 7)),
        offers: vec![first, second],
    });
    let key = shop.quotes()[0];
    shop.window.offers.reverse();
    let request = shop.trade(key, 4).ok_or("trade")?;
    assert_eq!(request.offer_index, 1);
    assert_eq!(request.side, NpcTradeSide::Buy);
    assert_eq!(shop.total(key, 4), "20 items; 76 gold");
    shop.result(NpcIntentDisposition::Rejected);
    shop.window.offers[1].unit_price = 20;
    assert!(shop.trade(key, 1).is_none());
    shop.window.open = None;
    assert!(shop.trade(key, 1).is_none());
    Ok(())
}

#[test]
fn sellable_counts_aggregate_carried_stacks_by_server_item_identity() -> Result<(), String> {
    use oteryn_client_runtime::npc_shop::{CharacterInventory, ItemEntry};
    let entry = |handle, item, count| -> Result<ItemEntry, String> {
        Ok(ItemEntry {
            handle: std::num::NonZeroU64::new(handle).ok_or("handle")?,
            item_definition_ref: std::num::NonZeroU32::new(item).ok_or("item")?,
            count: std::num::NonZeroU32::new(count).ok_or("count")?,
            sub_type: 0,
        })
    };
    let mut shop = LiveShop {
        inventory: Some(CharacterInventory {
            entries: vec![entry(1, 42, 6)?, entry(2, 42, 7)?, entry(3, 99, 100)?],
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(shop.owned(42), 13);
    shop.inventory = Some(Default::default());
    assert_eq!(shop.owned(42), 0);
    assert!(shop.text().contains("Free slots: unknown"));
    Ok(())
}

#[test]
fn trade_rejects_quantity_101_before_sending() {
    let mut shop = LiveShop {
        selected: true,
        ..Default::default()
    };
    shop.apply_window(npc_service::NpcTradeWindow {
        open: Some((
            EntityRef {
                identity: [2; 16],
                generation: 1,
            },
            1,
        )),
        offers: vec![npc_service::NpcOffer {
            item_definition_ref: 42,
            side: NpcTradeSide::Sell,
            count: 1,
            unit_price: 1,
        }],
    });
    let quote = shop.quotes()[0];
    assert!(shop.trade(quote, 101).is_none());
    assert!(!shop.pending);
    assert!(shop.trade(quote, 100).is_some());
}

#[test]
fn gold_projection_distinguishes_negotiation_and_live_balances() {
    use oteryn_client_runtime::npc_shop::resources::ResourceBalances;
    let mut shop = LiveShop {
        selected: true,
        ..Default::default()
    };
    assert!(
        shop.text()
            .contains("unavailable (capability not selected)")
    );
    shop.resources_selected = true;
    assert!(!shop.text().contains("unavailable"));
    shop.balances = Some(ResourceBalances {
        carried_gold: 75,
        bank_gold: 100,
        ..Default::default()
    });
    assert!(
        shop.text()
            .contains("Carried gold: 75; Bank gold: 100; Total: 175")
    );
}
#[test]
fn side_totals_and_backpack_free_slots_follow_server_limits() -> Result<(), String> {
    use oteryn_client_runtime::npc_shop::{
        CharacterInventory, ItemEntry, container_tree::ContainerView,
    };
    let mut shop = LiveShop::default();
    let quote = Quote {
        npc: EntityRef {
            identity: [1; 16],
            generation: 1,
        },
        revision: 1,
        item: 42,
        side: NpcTradeSide::Sell,
        count: 1,
        price: 1_009_999,
    };
    assert!(shop.valid_amount(quote, 1));
    assert!(!shop.valid_amount(quote, 2));
    assert!(shop.valid_amount(
        Quote {
            side: NpcTradeSide::Buy,
            price: u32::MAX,
            ..quote
        },
        100
    ));
    let backpack = ItemEntry {
        handle: std::num::NonZeroU64::new(1).ok_or("handle")?,
        item_definition_ref: std::num::NonZeroU32::new(10).ok_or("item")?,
        count: std::num::NonZeroU32::new(1).ok_or("count")?,
        sub_type: 0,
    };
    shop.inventory = Some(CharacterInventory {
        main_backpack: Some(backpack),
        entries: vec![ItemEntry {
            handle: std::num::NonZeroU64::new(2).ok_or("entry")?,
            ..backpack
        }],
        ..Default::default()
    });
    assert_eq!(shop.free_slots(), None);
    shop.containers.views.push(ContainerView {
        view_id: 0,
        container_handle: backpack.handle,
        parent_handle: None,
        capacity: 8,
        entries: vec![],
        browse: None,
    });
    assert_eq!(shop.free_slots(), Some(7));
    if let Some(inventory) = shop.inventory.as_mut() {
        inventory.entries.clear();
    }
    assert_eq!(shop.free_slots(), Some(8));
    Ok(())
}

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
        details: Vec::new(),
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
        details: Vec::new(),
        open: Some((npc, 10)),
        offers: vec![offer],
    });
    assert!(shop.trade(key, 1).is_none());
    shop.apply_window(Default::default());
    assert!(shop.quotes().is_empty());
    Ok(())
}

#[test]
fn quote_keeps_its_server_index_after_display_reordering() -> Result<(), String> {
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
        details: Vec::new(),
        open: Some((npc, 7)),
        offers: vec![first, second],
    });
    let mut display = shop.quotes();
    display.reverse();
    let key = display[1];
    let request = shop.trade(key, 4).ok_or("trade")?;
    assert_eq!(request.offer_index, 0);
    assert_eq!(request.side, NpcTradeSide::Buy);
    assert_eq!(shop.total(key, 4), "20 items; 76 gold");
    shop.result(NpcIntentDisposition::Rejected);
    shop.window.offers[0].unit_price = 20;
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
            appearance_id: 0,
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
        details: Vec::new(),
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
        offer_index: 0,
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
        appearance_id: 0,
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

#[test]
fn npc_shop_identical_offers_keep_the_later_server_index() -> Result<(), String> {
    let mut shop = LiveShop {
        selected: true,
        ..Default::default()
    };
    let offer = npc_service::NpcOffer {
        item_definition_ref: 42,
        side: NpcTradeSide::Buy,
        count: 1,
        unit_price: 2,
    };
    shop.apply_window(npc_service::NpcTradeWindow {
        open: Some((
            EntityRef {
                identity: [3; 16],
                generation: 1,
            },
            8,
        )),
        offers: vec![offer, offer],
        details: vec![
            npc_service::NpcQuoteDetails {
                name: "beer".into(),
                maximum_quantity: 1,
            },
            npc_service::NpcQuoteDetails {
                name: "wine".into(),
                maximum_quantity: 2,
            },
        ],
    });
    let quote = shop.quotes()[1];
    assert_eq!(shop.quote_name(quote), Some("wine"));
    assert_eq!(shop.maximum_amount(quote), 2);
    assert_eq!(
        shop.filtered_quotes(Some(NpcTradeSide::Buy), "wine"),
        vec![quote]
    );
    assert_eq!(shop.look(quote).ok_or("look")?.offer_index, 1);
    assert_eq!(shop.trade(quote, 2).ok_or("buy")?.offer_index, 1);
    Ok(())
}

#[test]
fn npc_shop_full_backpack_can_pay_with_its_last_coin_stack()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::npc_shop::{
        CharacterInventory, ItemEntry, container_tree::ContainerView, resources::ResourceBalances,
    };
    let backpack = ItemEntry {
        appearance_id: 0,
        handle: 1.try_into()?,
        item_definition_ref: 10.try_into()?,
        count: 1.try_into()?,
        sub_type: 0,
    };
    let mut shop = LiveShop {
        selected: true,
        inventory: Some(CharacterInventory {
            main_backpack: Some(backpack),
            entries: vec![ItemEntry {
                appearance_id: 3031,
                handle: 2.try_into()?,
                item_definition_ref: 11.try_into()?,
                count: 2.try_into()?,
                ..backpack
            }],
            ..Default::default()
        }),
        balances: Some(ResourceBalances {
            carried_gold: 2,
            ..Default::default()
        }),
        ..Default::default()
    };
    shop.containers.views.push(ContainerView {
        view_id: 0,
        container_handle: backpack.handle,
        parent_handle: None,
        capacity: 1,
        entries: vec![],
        browse: None,
    });
    shop.apply_window(npc_service::NpcTradeWindow {
        open: Some((
            EntityRef {
                identity: [3; 16],
                generation: 1,
            },
            8,
        )),
        offers: vec![npc_service::NpcOffer {
            item_definition_ref: 42,
            side: NpcTradeSide::Buy,
            count: 1,
            unit_price: 2,
        }],
        details: vec![],
    });
    let quote = shop.quotes()[0];
    assert_eq!(shop.free_slots(), Some(0));
    assert!(!shop.options.ignore_capacity);
    assert!(shop.can_trade(quote, 1));
    assert!(shop.trade(quote, 1).is_some());
    Ok(())
}

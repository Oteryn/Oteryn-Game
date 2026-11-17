#[test]
fn npc_shop_options_filter_owned_quotes_and_open_the_live_backpack()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_ui::npc_shop::{ShopOptions, npc_service};
    use oteryn_session::{CharacterInventory, ItemEntry};
    let item = |handle: u64, definition: u32| -> Result<ItemEntry, Box<dyn std::error::Error>> {
        Ok(ItemEntry {
            appearance_id: 0,
            handle: handle.try_into()?,
            item_definition_ref: definition.try_into()?,
            count: 1.try_into()?,
            sub_type: 0,
        })
    };
    let backpack = item(10, 100)?;
    let inventory = CharacterInventory {
        main_backpack: Some(backpack),
        entries: vec![item(11, 42)?],
        equipment: vec![oteryn_session::item_movement::EquippedItem {
            slot: oteryn_session::item_movement::EquipmentSlot::Head,
            item: item(12, 99)?,
        }],
    };
    let (link, mut commands, outbox) = play_channel();
    outbox.container_initialize(Some(Default::default()));
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.npc_shop.live.selected = true;
        mailbox.npc_shop.live.inventory = Some(inventory);
        mailbox
            .npc_shop
            .live
            .apply_window(npc_service::NpcTradeWindow {
                open: Some((
                    EntityRef {
                        identity: [9; 16],
                        generation: 2,
                    },
                    7,
                )),
                offers: vec![42, 99]
                    .into_iter()
                    .map(|item| npc_service::NpcOffer {
                        item_definition_ref: item,
                        side: NpcTradeSide::Sell,
                        count: 1,
                        unit_price: 12,
                    })
                    .collect(),
            });
    }
    assert!(link.npc_shop_options(ShopOptions {
        in_backpacks: false,
        ignore_capacity: true,
        ignore_equipped: true,
        only_owned: true
    }));
    let panel = link.npc_shop_panel();
    let quotes = panel.live.filtered_quotes(Some(NpcTradeSide::Sell), "");
    assert_eq!(quotes.len(), 1);
    assert_eq!(quotes[0].item, 42);
    assert!(link.npc_shop_options(ShopOptions {
        in_backpacks: false,
        ignore_capacity: false,
        ignore_equipped: false,
        only_owned: true
    }));
    assert_eq!(
        link.npc_shop_panel()
            .live
            .filtered_quotes(Some(NpcTradeSide::Sell), "")
            .len(),
        2
    );
    link.npc_trade_request(quotes[0], 1);
    let crate::combat_input::PlayCommand::NpcTrade(intent) = commands.try_recv()? else {
        return Err("trade command".into());
    };
    let bytes = npc_service::encode_npc_trade_intent(&intent).map_err(|e| format!("{e:?}"))?;
    assert_eq!(
        npc_service::decode_npc_trade_intent(&bytes)
            .map_err(|e| format!("{e:?}"))?
            .offer_index,
        0
    );
    crate::play::mailbox::lock(&link.mailbox)
        .npc_shop
        .live
        .pending = false;
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        let shop = &mut mailbox.npc_shop.live;
        shop.containers.views = vec![oteryn_session::container_tree::ContainerView {
            view_id: 0,
            container_handle: backpack.handle,
            parent_handle: None,
            capacity: 1,
            entries: shop.inventory.as_ref().ok_or("inventory")?.entries.clone(),
            browse: None,
        }];
        shop.window.offers[0].side = NpcTradeSide::Buy;
    }
    let buy = link.npc_shop_panel().live.quotes()[0];
    link.npc_trade_request(buy, 1);
    assert!(commands.try_recv().is_err());
    assert!(link.npc_shop_panel().live.feedback.contains("capacity"));
    assert!(link.npc_shop_options(ShopOptions {
        in_backpacks: false,
        ignore_capacity: true,
        ignore_equipped: true,
        only_owned: false
    }));
    link.npc_trade_request(buy, 1);
    assert!(
        matches!(commands.try_recv()?, crate::combat_input::PlayCommand::NpcTrade(intent) if intent.side == NpcTradeSide::Buy)
    );
    crate::play::mailbox::lock(&link.mailbox)
        .npc_shop
        .live
        .pending = false;
    assert!(link.npc_shop_open_backpack());
    assert_eq!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::ContainerView(
            oteryn_session::container_tree::ContainerViewIntent::Open {
                handle: backpack.handle,
                replace_view: None
            }
        )
    );
    Ok(())
}

#[test]
fn npc_buy_captures_overflow_preference_and_bag_flag_each_time()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_ui::npc_shop::{ShopOptions, npc_service};
    let (link, mut commands, _outbox) = play_channel();
    {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        mailbox.npc_shop.live.selected = true;
        mailbox.npc_shop.live.purchase_selected = true;
        mailbox
            .npc_shop
            .live
            .apply_window(npc_service::NpcTradeWindow {
                open: Some((
                    EntityRef {
                        identity: [9; 16],
                        generation: 2,
                    },
                    7,
                )),
                offers: vec![npc_service::NpcOffer {
                    item_definition_ref: 42,
                    side: NpcTradeSide::Buy,
                    count: 1,
                    unit_price: 2,
                }],
            });
    }
    let quote = link.npc_shop_panel().live.quotes()[0];
    link.npc_trade_request(quote, 45);
    let crate::combat_input::PlayCommand::NpcPurchase(first) = commands.try_recv()? else {
        return Err("bulk purchase command missing".into());
    };
    assert_eq!(first.amount, 45);
    assert!(first.ignore_cap);
    assert!(!first.in_backpacks);
    crate::play::mailbox::lock(&link.mailbox)
        .npc_shop
        .live
        .pending = false;
    assert!(link.npc_shop_options(ShopOptions {
        ignore_capacity: false,
        in_backpacks: true,
        ..ShopOptions::default()
    }));
    link.npc_trade_request(quote, 45);
    let crate::combat_input::PlayCommand::NpcPurchase(second) = commands.try_recv()? else {
        return Err("second bulk purchase command missing".into());
    };
    assert!(!second.ignore_cap);
    assert!(second.in_backpacks);
    assert_eq!(
        oteryn_session::npc_purchase::decode_npc_purchase(
            &oteryn_session::npc_purchase::encode_npc_purchase(&second)
                .map_err(|error| format!("{error:?}"))?
        )
        .map_err(|error| format!("{error:?}"))?,
        second
    );
    Ok(())
}

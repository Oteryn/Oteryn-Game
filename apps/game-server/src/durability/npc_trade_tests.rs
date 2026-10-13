#![allow(clippy::expect_used, clippy::unwrap_used)]
use super::*;
use crate::durability::item_transfer::ItemStackClass;
use crate::foundation::{CommandId, GameSessionId};

fn id(tag: u8) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[5] = 1;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = tag;
    bytes
}

fn definition(family: &str, key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: family.into(),
        production_key: key.into(),
        revision_ref: "oteryn:rev.1".into(),
    }
}

fn coin(coin: Coin) -> TypedDefinitionRef {
    definition(COIN_DEFINITION_FAMILY, coin.production_key())
}

fn facts(definition: TypedDefinitionRef, stack: ItemStackClass) -> ItemDefinitionFacts {
    ItemDefinitionFacts {
        definition,
        stack,
        container_capacity: None,
        container_slot_equip_pattern: false,
    }
}

fn sword() -> TypedDefinitionRef {
    definition("Item", "oteryn:item.tibia.i3264")
}

fn request(side: NpcTradeSide, quantity: u32, unit_price: u64) -> NpcTradeRequest {
    NpcTradeRequest {
        cause: NpcTradeCause {
            npc_key: "oteryn:npc.tibia.sam".into(),
            catalogue_revision: 1,
            offer_index: 0,
            side,
            occurrence: id(70),
        },
        command: CommandRef::new(
            GameSessionId::decode(&id(50)).expect("session"),
            CommandId::new(9).expect("command"),
        ),
        quantity,
        unit_price,
        offer_count: 1,
        item: facts(
            sword(),
            ItemStackClass::Stackable {
                proven_maximum: None,
            },
        ),
        transaction_id: id(80),
        event_id: id(81),
        occurred_at_unix_ms: 1_790_000_000_000,
        output_item_instance_ids: [id(90), id(91), id(92)],
        coins: NpcTradeCoins {
            gold: coin(Coin::Gold),
            platinum: coin(Coin::Platinum),
            crystal: coin(Coin::Crystal),
        },
        backpack: ItemDefinitionFacts {
            container_capacity: Some(20),
            ..facts(
                definition("Item", "oteryn:item.tibia.i2854"),
                ItemStackClass::NonStackable,
            )
        },
    }
}

fn entry(tag: u8, ordinal: u64, definition: TypedDefinitionRef, quantity: u32) -> Entry {
    Entry {
        item_instance_id: id(tag),
        placement_ordinal: ordinal,
        definition,
        quantity,
        live: true,
        not_default: false,
    }
}

#[test]
fn a_request_outside_its_shape_is_invalid() {
    assert_eq!(
        validate(&request(NpcTradeSide::Buy, 2, 50)).unwrap(),
        (100, 2)
    );
    let mut cases: Vec<(&str, NpcTradeRequest)> = vec![
        ("zero quantity", request(NpcTradeSide::Buy, 0, 50)),
        ("quantity 101", request(NpcTradeSide::Buy, 101, 1)),
        (
            "BUY above 20 crystal stacks",
            request(NpcTradeSide::Buy, 1, 20_000_001),
        ),
        (
            "SELL above 3 stacks",
            request(NpcTradeSide::Sell, 1, 1_010_000),
        ),
        ("price overflow", request(NpcTradeSide::Buy, 2, u64::MAX)),
    ];
    let mut value = request(NpcTradeSide::Buy, 1, 1);
    value.output_item_instance_ids[2] = value.event_id;
    cases.push(("a repeated identity", value));
    let mut value = request(NpcTradeSide::Buy, 1, 1);
    value.item.definition = coin(Coin::Gold);
    cases.push(("a coin offer", value));
    let mut value = request(NpcTradeSide::Buy, 1, 1);
    value.coins.gold = coin(Coin::Platinum);
    cases.push(("a coin at another key", value));
    let mut value = request(NpcTradeSide::Buy, 1, 1);
    value.backpack.container_capacity = None;
    cases.push(("a backpack without capacity", value));
    let mut value = request(NpcTradeSide::Buy, 1, 1);
    value.cause.npc_key = String::new();
    cases.push(("an empty NPC key", value));
    let mut value = request(NpcTradeSide::Buy, 1, 1);
    value.occurred_at_unix_ms = 0;
    cases.push(("time 0", value));
    let mut value = request(NpcTradeSide::Buy, 2, 1);
    value.item.stack = ItemStackClass::NonStackable;
    cases.push(("two of a non-stackable", value));
    let mut value = request(NpcTradeSide::Buy, 20, 1);
    value.offer_count = 6;
    cases.push(("more units than a stack", value));
    let mut value = request(NpcTradeSide::Buy, 1, 1);
    value.item.stack = ItemStackClass::Unknown;
    cases.push(("an unknown stack class", value));
    for (name, value) in cases {
        assert!(
            matches!(validate(&value), Err(NpcTradeError::InvalidInput)),
            "{name}"
        );
    }
    let mut value = request(NpcTradeSide::Buy, 1, 1);
    value.item.stack = ItemStackClass::NonStackable;
    assert_eq!(validate(&value).unwrap(), (1, 1));
    // NPC-0 §5.3 and §5.4: a price of 0 is a valid trade on either side.
    for side in [NpcTradeSide::Buy, NpcTradeSide::Sell] {
        assert_eq!(validate(&request(side, 3, 0)).unwrap(), (0, 3));
    }
    let mut value = request(NpcTradeSide::Buy, 20, 1);
    value.offer_count = 5;
    assert_eq!(item_units(&value).unwrap(), 100);
}

#[test]
fn a_buy_burns_the_fee_plan_and_mints_change_then_the_item() {
    // Entries highest ordinal first: 3 platinum (6), a sword (5), 10 gold (4), 1 crystal (2).
    let entries = [
        entry(1, 6, coin(Coin::Platinum), 3),
        entry(2, 5, sword(), 1),
        entry(3, 4, coin(Coin::Gold), 10),
        entry(4, 2, coin(Coin::Crystal), 1),
    ];
    let value = request(NpcTradeSide::Buy, 1, 235);
    let plan = plan_buy(&value, 235, 1, 20, &entries).unwrap();
    // Gold whole (10), platinum 3 whole (300): change 75 as 75 gold, then the item.
    assert_eq!(
        plan.lines,
        vec![
            NpcTradeLine {
                kind: NpcTradeLineKind::CoinBurn,
                item_instance_id: id(3),
                placement_ordinal: 4,
                coin: Some(Coin::Gold),
                quantity_before: 10,
                quantity_after: 0,
            },
            NpcTradeLine {
                kind: NpcTradeLineKind::CoinBurn,
                item_instance_id: id(1),
                placement_ordinal: 6,
                coin: Some(Coin::Platinum),
                quantity_before: 3,
                quantity_after: 0,
            },
            NpcTradeLine {
                kind: NpcTradeLineKind::CoinMint,
                item_instance_id: id(91),
                placement_ordinal: 7,
                coin: Some(Coin::Gold),
                quantity_before: 0,
                quantity_after: 75,
            },
            NpcTradeLine {
                kind: NpcTradeLineKind::ItemMint,
                item_instance_id: id(92),
                placement_ordinal: 8,
                coin: None,
                quantity_before: 0,
                quantity_after: 1,
            },
        ]
    );
    assert_eq!(
        (
            plan.burned_gold_units,
            plan.minted_gold_units,
            plan.item_instance_id
        ),
        (310, 75, id(92))
    );
    // Exactly enough: no change line.
    let plan = plan_buy(&value, 10, 1, 20, &entries).unwrap();
    assert_eq!(plan.lines.len(), 2);
    assert_eq!(plan.lines[1].placement_ordinal, 7);
    // A crystal paying 9,999 mints 99 platinum and 99 gold.
    let plan = plan_buy(&value, 311, 1, 20, &entries).unwrap();
    assert_eq!(plan.minted_gold_units, 9_999);
    assert_eq!(
        plan.lines
            .iter()
            .filter(|line| line.kind == NpcTradeLineKind::CoinMint)
            .map(|line| (line.coin, line.quantity_after, line.item_instance_id))
            .collect::<Vec<_>>(),
        vec![
            (Some(Coin::Platinum), 99, id(90)),
            (Some(Coin::Gold), 99, id(91))
        ]
    );
    assert!(matches!(
        plan_buy(&value, 10_311, 1, 20, &entries),
        Err(NpcTradeError::InsufficientFunds)
    ));
    // 4 entries, none burned whole by a 1-gold price (partial), change 0, the item: 5 > 4.
    assert!(matches!(
        plan_buy(&value, 1, 1, 4, &entries),
        Err(NpcTradeError::DoesNotFit)
    ));
    assert!(plan_buy(&value, 1, 1, 5, &entries).is_ok());
    // Price 0: no BURN and no change, only the item.
    let plan = plan_buy(&value, 0, 1, 20, &entries).unwrap();
    assert_eq!(
        plan.lines
            .iter()
            .map(|line| (line.kind, line.placement_ordinal))
            .collect::<Vec<_>>(),
        vec![(NpcTradeLineKind::ItemMint, 7)]
    );
    assert_eq!((plan.burned_gold_units, plan.minted_gold_units), (0, 0));
    assert!(matches!(
        plan_buy(&value, 0, 1, 4, &entries),
        Err(NpcTradeError::DoesNotFit)
    ));
    // A coin stack at another revision is never paid from.
    let mut stale = entries.clone();
    stale[2].definition.revision_ref = "oteryn:rev.0".into();
    assert!(matches!(
        plan_buy(&value, 1, 1, 20, &stale),
        Err(NpcTradeError::InvalidInput)
    ));
    // 21 one-gold stacks for 21 gold: more than 20 inputs.
    let many: Vec<Entry> = (0..21)
        .map(|n| entry(100 + n, 30 - u64::from(n), coin(Coin::Gold), 1))
        .collect();
    assert!(matches!(
        plan_buy(&value, 21, 1, 30, &many),
        Err(NpcTradeError::CapacityExceeded)
    ));
}

#[test]
fn a_sell_burns_the_first_eligible_entry_and_mints_the_price() {
    let mut contained = entry(1, 9, sword(), 5);
    contained.not_default = true;
    let entries = [
        contained,
        entry(2, 8, sword(), 1),
        entry(3, 7, sword(), 5),
        entry(4, 3, coin(Coin::Gold), 4),
    ];
    let value = request(NpcTradeSide::Sell, 2, 12_345);
    let plan = plan_sell(&value, 24_690, 2, 20, &entries).unwrap();
    assert_eq!(
        plan.lines
            .iter()
            .map(|line| (
                line.kind,
                line.item_instance_id,
                line.placement_ordinal,
                line.coin,
                line.quantity_before,
                line.quantity_after
            ))
            .collect::<Vec<_>>(),
        vec![
            (NpcTradeLineKind::ItemBurn, id(3), 7, None, 5, 3),
            (
                NpcTradeLineKind::CoinMint,
                id(90),
                10,
                Some(Coin::Crystal),
                0,
                2
            ),
            (
                NpcTradeLineKind::CoinMint,
                id(91),
                11,
                Some(Coin::Platinum),
                0,
                46
            ),
            (
                NpcTradeLineKind::CoinMint,
                id(92),
                12,
                Some(Coin::Gold),
                0,
                90
            ),
        ]
    );
    assert_eq!(
        (plan.burned_gold_units, plan.minted_gold_units),
        (0, 24_690)
    );
    // A whole burn frees its entry: 4 - 1 + 3 = 6 fits 6.
    let value = request(NpcTradeSide::Sell, 5, 12_345);
    assert!(plan_sell(&value, 61_725, 5, 6, &entries).is_ok());
    let value = request(NpcTradeSide::Sell, 2, 12_345);
    assert!(matches!(
        plan_sell(&value, 24_690, 2, 6, &entries),
        Err(NpcTradeError::DoesNotFit)
    ));
    assert!(matches!(
        plan_sell(&value, 24_690, 6, 20, &entries),
        Err(NpcTradeError::ItemNotFound)
    ));
    let mut other = request(NpcTradeSide::Sell, 1, 1);
    other.item.definition.revision_ref = "oteryn:rev.2".into();
    assert!(matches!(
        plan_sell(&other, 1, 1, 20, &entries),
        Err(NpcTradeError::ItemNotFound)
    ));
    // Price 0: only the item BURN.
    let free = request(NpcTradeSide::Sell, 2, 0);
    let plan = plan_sell(&free, 0, 2, 20, &entries).unwrap();
    assert_eq!(plan.lines.len(), 1);
    assert_eq!(plan.lines[0].kind, NpcTradeLineKind::ItemBurn);
    assert_eq!((plan.burned_gold_units, plan.minted_gold_units), (0, 0));
    assert_eq!(first_mint_ordinal(&[]).unwrap(), 1);
    assert!(matches!(
        first_mint_ordinal(&[entry(1, u64::MAX, sword(), 1)]),
        Err(NpcTradeError::DoesNotFit)
    ));
}

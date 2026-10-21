#![allow(clippy::expect_used, clippy::unwrap_used)]
use super::*;

#[test]
fn coin_outputs_follow_the_decision() {
    assert_eq!(
        sell_coins(1_009_999),
        Some(vec![
            (Coin::Crystal, 100),
            (Coin::Platinum, 99),
            (Coin::Gold, 99)
        ])
    );
    assert_eq!(sell_coins(1_010_000), None);
    assert_eq!(sell_coins(10_000), Some(vec![(Coin::Crystal, 1)]));
    assert_eq!(sell_coins(7), Some(vec![(Coin::Gold, 7)]));
    assert_eq!(
        buy_change_coins(9_999),
        vec![(Coin::Platinum, 99), (Coin::Gold, 99)]
    );
    assert_eq!(buy_change_coins(200), vec![(Coin::Platinum, 2)]);
    assert!(buy_change_coins(0).is_empty());
}

use oteryn_gameplay_domain::domain::item_definition::ItemStackClass;

#[derive(Clone, Copy)]
enum Type2EventTuple {
    V1,
    V2,
}

struct Request {
    item: ItemDefinitionFacts,
    coins: NpcTradeCoins,
    output_item_instance_ids: [[u8; 16]; 3],
}
fn input(request: &Request) -> PlanInput<'_> {
    PlanInput {
        item: &request.item,
        coins: &request.coins,
        output_item_instance_ids: request.output_item_instance_ids,
    }
}
fn plan_buy(
    request: &Request,
    total: u64,
    units: u32,
    capacity: usize,
    entries: &[Entry],
    tuple: Type2EventTuple,
    junior: bool,
) -> Result<Plan, PlanError> {
    super::plan_buy(
        &input(request),
        total,
        units,
        capacity,
        entries,
        matches!(tuple, Type2EventTuple::V2) && !junior,
    )
}
fn plan_sell(
    request: &Request,
    total: u64,
    units: u32,
    capacity: usize,
    entries: &[Entry],
) -> Result<Plan, PlanError> {
    super::plan_sell(&input(request), total, units, capacity, entries)
}
#[derive(Clone, Copy)]
enum NpcTradeSide {
    Buy,
    Sell,
}
fn request(_side: NpcTradeSide, _quantity: u32, _unit_price: u64) -> Request {
    Request {
        item: facts(
            sword(),
            ItemStackClass::Stackable {
                proven_maximum: None,
            },
        ),
        output_item_instance_ids: [id(90), id(91), id(92)],
        coins: NpcTradeCoins {
            gold: coin(Coin::Gold),
            platinum: coin(Coin::Platinum),
            crystal: coin(Coin::Crystal),
        },
    }
}
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
        weight: None,
        definition,
        stack,
        container_capacity: None,
        container_slot_equip_pattern: false,
    }
}

fn sword() -> TypedDefinitionRef {
    definition("Item", "oteryn:item.tibia.i3264")
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
fn a_buy_burns_the_fee_plan_and_mints_change_then_the_item() {
    // Entries highest ordinal first: 3 platinum (6), a sword (5), 10 gold (4), 1 crystal (2).
    let entries = [
        entry(1, 6, coin(Coin::Platinum), 3),
        entry(2, 5, sword(), 1),
        entry(3, 4, coin(Coin::Gold), 10),
        entry(4, 2, coin(Coin::Crystal), 1),
    ];
    let value = request(NpcTradeSide::Buy, 1, 235);
    let plan = plan_buy(&value, 235, 1, 20, &entries, Type2EventTuple::V1, false).unwrap();
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
    let plan = plan_buy(&value, 10, 1, 20, &entries, Type2EventTuple::V1, false).unwrap();
    assert_eq!(plan.lines.len(), 2);
    assert_eq!(plan.lines[1].placement_ordinal, 7);
    // A crystal paying 9,999 mints 99 platinum and 99 gold.
    let plan = plan_buy(&value, 311, 1, 20, &entries, Type2EventTuple::V1, false).unwrap();
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
        plan_buy(&value, 10_311, 1, 20, &entries, Type2EventTuple::V1, false),
        Err(PlanError::InsufficientFunds)
    ));
    // 4 entries, none burned whole by a 1-gold price (partial), change 0, the item: 5 > 4.
    assert!(matches!(
        plan_buy(&value, 1, 1, 4, &entries, Type2EventTuple::V1, false),
        Err(PlanError::DoesNotFit)
    ));
    assert!(plan_buy(&value, 1, 1, 5, &entries, Type2EventTuple::V1, false).is_ok());
    // Price 0: no BURN and no change, only the item.
    let plan = plan_buy(&value, 0, 1, 20, &entries, Type2EventTuple::V1, false).unwrap();
    assert_eq!(
        plan.lines
            .iter()
            .map(|line| (line.kind, line.placement_ordinal))
            .collect::<Vec<_>>(),
        vec![(NpcTradeLineKind::ItemMint, 7)]
    );
    assert_eq!((plan.burned_gold_units, plan.minted_gold_units), (0, 0));
    assert!(matches!(
        plan_buy(&value, 0, 1, 4, &entries, Type2EventTuple::V1, false),
        Err(PlanError::DoesNotFit)
    ));
    // A coin stack at another revision is never paid from.
    let mut stale = entries.clone();
    stale[2].definition.revision_ref = "oteryn:rev.0".into();
    assert!(matches!(
        plan_buy(&value, 1, 1, 20, &stale, Type2EventTuple::V1, false),
        Err(PlanError::InvalidInput)
    ));
    // A coin stack with contents or a timed state is never paid from, nor skipped.
    let mut held = entries.clone();
    held[2].not_default = true;
    assert!(matches!(
        plan_buy(&value, 1, 1, 20, &held, Type2EventTuple::V1, false),
        Err(PlanError::InvalidInput)
    ));
    // A free BUY pays from no coin: neither stack blocks it.
    for blocked in [&stale, &held] {
        let plan = plan_buy(&value, 0, 1, 20, blocked, Type2EventTuple::V1, false).unwrap();
        assert_eq!(plan.lines.len(), 1);
    }
    // 21 one-gold stacks for 21 gold: more than 20 inputs.
    let many: Vec<Entry> = (0..21)
        .map(|n| entry(100 + n, 30 - u64::from(n), coin(Coin::Gold), 1))
        .collect();
    assert!(matches!(
        plan_buy(&value, 21, 1, 30, &many, Type2EventTuple::V1, false),
        Err(PlanError::CapacityExceeded)
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
        Err(PlanError::DoesNotFit)
    ));
    assert!(matches!(
        plan_sell(&value, 24_690, 6, 20, &entries),
        Err(PlanError::ItemNotFound)
    ));
    let mut other = request(NpcTradeSide::Sell, 1, 1);
    other.item.definition.revision_ref = "oteryn:rev.2".into();
    assert!(matches!(
        plan_sell(&other, 1, 1, 20, &entries),
        Err(PlanError::ItemNotFound)
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
        Err(PlanError::DoesNotFit)
    ));
}

#[test]
fn buy_bank_fallback_burns_every_coin_and_never_changes_junior_or_v1_rules() {
    let value = request(NpcTradeSide::Buy, 1, 250);
    let entries = vec![
        entry(10, 3, coin(Coin::Gold), 10),
        entry(11, 2, coin(Coin::Platinum), 2),
    ];
    let plan = plan_buy(&value, 250, 1, 20, &entries, Type2EventTuple::V2, false)
        .expect("coins then bank");
    assert_eq!(plan.bank_debit_gold_units, 40);
    assert_eq!(plan.burned_gold_units, 210);
    assert_eq!(plan.minted_gold_units, 0);
    assert!(
        plan.lines
            .iter()
            .filter(|l| l.kind == NpcTradeLineKind::CoinBurn)
            .all(|l| l.quantity_after == 0)
    );
    for (tuple, junior) in [(Type2EventTuple::V1, false), (Type2EventTuple::V2, true)] {
        assert!(matches!(
            plan_buy(&value, 250, 1, 20, &entries, tuple, junior),
            Err(PlanError::InsufficientFunds)
        ));
    }
    let bank_only = plan_buy(&value, 250, 1, 1, &[], Type2EventTuple::V2, false)
        .expect("bank only still fits bag");
    assert_eq!(bank_only.bank_debit_gold_units, 250);
    assert_eq!(bank_only.lines.len(), 1);
    assert_eq!(bank_only.burned_gold_units, 0);
    assert!(matches!(
        plan_buy(&value, 250, 1, 0, &[], Type2EventTuple::V2, false),
        Err(PlanError::DoesNotFit)
    ));
}

#[test]
fn scalar_price_and_item_units_keep_the_request_limits() {
    for buy in [true, false] {
        assert_eq!(trade_total(buy, 3, 0), Ok(0));
        assert_eq!(trade_total(buy, 100, 1), Ok(100));
        for (quantity, price) in [(0, 1), (101, 1), (2, u64::MAX)] {
            assert_eq!(
                trade_total(buy, quantity, price),
                Err(PlanError::InvalidInput)
            );
        }
    }
    for (buy, cap) in [
        (true, NPC_TRADE_BUY_GOLD_UNITS_MAX),
        (false, NPC_TRADE_SELL_GOLD_UNITS_MAX),
    ] {
        assert_eq!(trade_total(buy, 1, cap), Ok(cap));
        assert_eq!(trade_total(buy, 1, cap + 1), Err(PlanError::InvalidInput));
    }
    assert_eq!(item_units(20, 5, 100, true), Ok(100));
    assert_eq!(item_units(1, 1, 1, false), Ok(1));
    for (quantity, count, maximum, stackable) in [
        (2, 1, 1, false),
        (20, 6, 100, true),
        (0, 1, 100, true),
        (u32::MAX, 2, 100, true),
    ] {
        assert_eq!(
            item_units(quantity, count, maximum, stackable),
            Err(PlanError::InvalidInput)
        );
    }
}

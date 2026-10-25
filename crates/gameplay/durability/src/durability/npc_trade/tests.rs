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
            weight: None,
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

/// The priced (non-sell-all) shape of a request.
fn priced(request: &NpcTradeRequest) -> Result<(u64, u32)> {
    validate(request, false).map(|priced| priced.expect("a priced trade"))
}

#[test]
fn a_sell_all_names_one_unit_of_a_count_one_sell_offer() {
    assert!(validate(&request(NpcTradeSide::Sell, 1, 99), true)
        .unwrap()
        .is_none());
    let mut count = request(NpcTradeSide::Sell, 1, 1);
    count.offer_count = 2;
    for (name, value) in [
        ("a BUY", request(NpcTradeSide::Buy, 1, 1)),
        ("two units", request(NpcTradeSide::Sell, 2, 1)),
        ("a count of 2", count),
        (
            "a price above the SELL ceiling",
            request(NpcTradeSide::Sell, 1, rules::NPC_TRADE_SELL_GOLD_UNITS_MAX + 1),
        ),
    ] {
        assert!(
            matches!(validate(&value, true), Err(NpcTradeError::InvalidInput)),
            "{name}"
        );
    }
}

/// A sell-all of 3 stacks (100, 100 and 7 units) at 99 each: every stack is one whole BURN, the
/// audited quantity is the 207 units and the binding differs from the trade's.
#[test]
fn a_sell_all_audits_every_burned_stack_and_its_units() {
    use crate::durability::npc_trade_audit::{check_npc_trade, decode_npc_trade_envelope};
    use crate::foundation::{
        ChannelId, ConnectionGeneration, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    };

    let world = WorldId::decode(&id(1)).unwrap();
    let channel = ChannelId::decode(&id(2)).unwrap();
    let fence = CurrentCharacterItemFence {
        character_id: CharacterId::from_bytes(id(3)).unwrap(),
        game_session_id: GameSessionId::decode(&id(50)).unwrap(),
        connection_generation: ConnectionGeneration::new(1).unwrap(),
        character_lease_generation: 1,
        runtime_scope: RuntimeScopeRefV1::channel(world, channel),
        scope_ownership_generation: ScopeOwnershipGeneration::new(1).unwrap(),
    };
    let value = request(NpcTradeSide::Sell, 1, 99);
    let entries = [
        entry(10, 5, sword(), 7),
        entry(11, 4, coin(Coin::Gold), 3),
        entry(12, 3, sword(), 100),
        entry(13, 1, sword(), 100),
    ];
    let (plan, total, units) = plan_sell_all(&value, 20, &entries).unwrap();
    assert_eq!((total, units), (20_493, 207));
    let message = trade_message(
        &fence,
        world,
        channel,
        &id(4),
        &value,
        (units, total, true),
        &plan,
    );
    assert!(message.sell_all);
    assert_eq!(message.quantity, 207);
    check_npc_trade(&message).unwrap();
    let wire = encode_npc_trade_event(
        NpcTradeEventIdentity {
            event_id: value.event_id,
            transaction_id: value.transaction_id,
            occurred_at_unix_ms: value.occurred_at_unix_ms,
            server_build_id: SERVER_BUILD_ID,
        },
        message.clone(),
        Type2EventTuple::V2,
    )
    .unwrap();
    assert_eq!(decode_npc_trade_envelope(&wire).unwrap().1, message);
    assert_ne!(
        request_binding(&fence, world, &value, true),
        request_binding(&fence, world, &value, false)
    );
    // The same lines without the marker are a SELL of three stacks: refused.
    let mut unmarked = message;
    unmarked.sell_all = false;
    assert_eq!(check_npc_trade(&unmarked), Err(AuditError::InvalidInput));
}

#[test]
fn a_request_outside_its_shape_is_invalid() {
    assert_eq!(
        priced(&request(NpcTradeSide::Buy, 2, 50)).unwrap(),
        (100, 2)
    );
    let mut cases: Vec<(&str, NpcTradeRequest)> = vec![
        ("zero quantity", request(NpcTradeSide::Buy, 0, 50)),
        ("quantity 101", request(NpcTradeSide::Buy, 101, 1)),
        (
            "BUY above bank plus coin ceiling",
            request(NpcTradeSide::Buy, 1, NPC_TRADE_BUY_GOLD_UNITS_MAX + 1),
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
            matches!(priced(&value), Err(NpcTradeError::InvalidInput)),
            "{name}"
        );
    }
    let mut value = request(NpcTradeSide::Buy, 1, 1);
    value.item.stack = ItemStackClass::NonStackable;
    assert_eq!(priced(&value).unwrap(), (1, 1));
    // NPC-0 §5.3 and §5.4: a price of 0 is a valid trade on either side.
    for side in [NpcTradeSide::Buy, NpcTradeSide::Sell] {
        assert_eq!(priced(&request(side, 3, 0)).unwrap(), (0, 3));
    }
    let mut value = request(NpcTradeSide::Buy, 20, 1);
    value.offer_count = 5;
    assert_eq!(item_units(&value).unwrap(), 100);
}

/// Exercise the actual producer and codec together: quantity is purchase count, not
/// delivered stack units. These typed fence fields are audit test data, not live authority.
#[test]
fn bundled_buy_and_sell_plans_encode_purchase_quantity_and_actual_item_units() {
    use crate::durability::npc_trade_audit::{check_npc_trade, decode_npc_trade_envelope};
    use crate::foundation::{
        ChannelId, ConnectionGeneration, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    };

    let world = WorldId::decode(&id(1)).unwrap();
    let channel = ChannelId::decode(&id(2)).unwrap();
    let fence = CurrentCharacterItemFence {
        character_id: CharacterId::from_bytes(id(3)).unwrap(),
        game_session_id: GameSessionId::decode(&id(50)).unwrap(),
        connection_generation: ConnectionGeneration::new(1).unwrap(),
        character_lease_generation: 1,
        runtime_scope: RuntimeScopeRefV1::channel(world, channel),
        scope_ownership_generation: ScopeOwnershipGeneration::new(1).unwrap(),
    };
    for (quantity, count, expected_units) in [(2, 5, 10), (20, 5, 100)] {
        for side in [NpcTradeSide::Buy, NpcTradeSide::Sell] {
            let mut value = request(side, quantity, 3);
            value.offer_count = count;
            let (total, units) = priced(&value).unwrap();
            assert_eq!((total, units), (u64::from(quantity) * 3, expected_units));
            let plan = match side {
                NpcTradeSide::Buy => plan_buy(
                    &value,
                    total,
                    units,
                    20,
                    &[entry(10, 1, coin(Coin::Gold), 100)],
                    Type2EventTuple::V1,
                    false,
                )
                .unwrap(),
                NpcTradeSide::Sell => plan_sell(
                    &value,
                    total,
                    units,
                    20,
                    &[entry(10, 1, sword(), expected_units)],
                )
                .unwrap(),
            };
            let message = trade_message(&fence, world, channel, &id(4), &value, (quantity, total, false), &plan);
            assert_eq!(
                (
                    message.quantity,
                    message.unit_price,
                    message.total_gold_units
                ),
                (quantity, 3, total)
            );
            let moved_units = match side {
                NpcTradeSide::Buy => {
                    message
                        .lines
                        .last()
                        .unwrap()
                        .after
                        .as_ref()
                        .unwrap()
                        .quantity
                }
                NpcTradeSide::Sell => {
                    let line = message.lines.first().unwrap();
                    line.before.as_ref().unwrap().quantity - line.after.as_ref().unwrap().quantity
                }
            };
            assert_eq!(moved_units, expected_units);
            for tuple in [Type2EventTuple::V1, Type2EventTuple::V2] {
                let wire = encode_npc_trade_event(
                    NpcTradeEventIdentity {
                        event_id: value.event_id,
                        transaction_id: value.transaction_id,
                        occurred_at_unix_ms: value.occurred_at_unix_ms,
                        server_build_id: SERVER_BUILD_ID,
                    },
                    message.clone(),
                    tuple,
                )
                .unwrap();
                assert_eq!(decode_npc_trade_envelope(&wire).unwrap().1, message);
            }
            // Change only the item custody quantity, retaining valid price/conservation
            // and all other states: no integral positive offer count can explain it.
            let mut broken = message;
            match side {
                NpcTradeSide::Buy => {
                    broken
                        .lines
                        .last_mut()
                        .unwrap()
                        .after
                        .as_mut()
                        .unwrap()
                        .quantity = expected_units - 1
                }
                NpcTradeSide::Sell => {
                    broken
                        .lines
                        .first_mut()
                        .unwrap()
                        .before
                        .as_mut()
                        .unwrap()
                        .quantity = expected_units - 1
                }
            }
            assert_eq!(check_npc_trade(&broken), Err(AuditError::InvalidInput));
        }
    }
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
            Err(NpcTradeError::InsufficientFunds)
        ));
    }
    let bank_only = plan_buy(&value, 250, 1, 1, &[], Type2EventTuple::V2, false)
        .expect("bank only still fits bag");
    assert_eq!(bank_only.bank_debit_gold_units, 250);
    assert_eq!(bank_only.lines.len(), 1);
    assert_eq!(bank_only.burned_gold_units, 0);
    assert!(matches!(
        plan_buy(&value, 250, 1, 0, &[], Type2EventTuple::V2, false),
        Err(NpcTradeError::DoesNotFit)
    ));
}

#![allow(clippy::expect_used, clippy::unwrap_used)]
use super::super::item_fee_burn_audit::decode_fee_burn_payload;
use super::super::item_mint_audit::{OneItemTypedDefinitionRevisionV1, decode_payload};
use super::*;
use serde_json::Value;

const RESOURCE_REGISTRY: &str =
    oteryn_protocol_oteryn::RESOURCE_LIMITS_REGISTRY_JSON;

fn uuid(tag: u8) -> Vec<u8> {
    let mut bytes = vec![0; 16];
    bytes[5] = 1;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = tag;
    bytes
}

fn fixed(tag: u8) -> [u8; 16] {
    uuid(tag).try_into().unwrap()
}

fn definition(family: &str, key: &str, revision: &str) -> OneItemTypedDefinitionRevisionV1 {
    OneItemTypedDefinitionRevisionV1 {
        family: family.into(),
        production_key: key.into(),
        revision_ref: revision.into(),
    }
}

fn state(item: u8, definition: OneItemTypedDefinitionRevisionV1, quantity: u32) -> OneItemStateV1 {
    OneItemStateV1 {
        item_instance_id: uuid(item),
        world_id: uuid(1),
        definition: Some(definition),
        quantity,
        lifecycle: if quantity == 0 {
            ITEM_LIFECYCLE_RETIRED
        } else {
            ITEM_LIFECYCLE_LIVE
        },
        key_number: 0,
    }
}

fn coin(coin: Coin, revision: &str) -> OneItemTypedDefinitionRevisionV1 {
    definition(COIN_DEFINITION_FAMILY, coin.production_key(), revision)
}

fn sword(revision: &str) -> OneItemTypedDefinitionRevisionV1 {
    definition("Item", "oteryn:item.tibia.i3264", revision)
}

fn burn(
    kind: u32,
    item: u8,
    definition: OneItemTypedDefinitionRevisionV1,
    ordinal: u64,
    before: u32,
    after: u32,
) -> OneItemNpcTradeLineV1 {
    OneItemNpcTradeLineV1 {
        kind,
        before: Some(state(item, definition.clone(), before)),
        after: Some(state(item, definition, after)),
        placement_ordinal: ordinal,
    }
}

fn mint_line(
    kind: u32,
    item: u8,
    definition: OneItemTypedDefinitionRevisionV1,
    ordinal: u64,
    quantity: u32,
) -> OneItemNpcTradeLineV1 {
    OneItemNpcTradeLineV1 {
        kind,
        before: None,
        after: Some(state(item, definition, quantity)),
        placement_ordinal: ordinal,
    }
}

fn trade(side: u32, quantity: u32, unit_price: u64) -> OneItemNpcTradeV1 {
    OneItemNpcTradeV1 {
        cause: Some(OneItemNpcTradeCauseV1 {
            npc_key: "oteryn:npc.tibia.sam".into(),
            catalogue_revision: 3,
            offer_index: 2,
            side,
            occurrence: uuid(70),
        }),
        command: Some(OneItemCommandRefV1 {
            game_session_id: uuid(71),
            command_id: 9,
        }),
        character_id: uuid(41),
        world_id: uuid(1),
        channel_id: uuid(2),
        runtime_scope_ownership_generation: 1,
        backpack_item_instance_id: uuid(50),
        quantity,
        unit_price,
        total_gold_units: unit_price * u64::from(quantity),
        burned_gold_units: 0,
        minted_gold_units: 0,
        lines: Vec::new(),
        bank_debit: None,
        sell_all: false,
    }
}

/// A BUY of one sword for 235 from 10 gold (ordinal 4, whole) and 3 platinum (ordinal 2, 3 of
/// 5 burned): 75 change as 0 platinum and 75 gold, then the sword.
fn buy() -> OneItemNpcTradeV1 {
    let mut value = trade(SIDE_BUY, 1, 235);
    value.lines = vec![
        burn(LINE_COIN_BURN, 10, coin(Coin::Gold, "r"), 4, 10, 0),
        burn(LINE_COIN_BURN, 11, coin(Coin::Platinum, "r"), 2, 5, 2),
        mint_line(LINE_COIN_MINT, 20, coin(Coin::Gold, "r"), 5, 75),
        mint_line(LINE_ITEM_MINT, 21, sword("s"), 6, 1),
    ];
    value.burned_gold_units = 310;
    value.minted_gold_units = 75;
    value
}

/// A SELL of 2 of 5 swords at 12,345 each: 24,690 as 2 crystal, 46 platinum and 90 gold.
fn sell() -> OneItemNpcTradeV1 {
    let mut value = trade(SIDE_SELL, 2, 12_345);
    value.lines = vec![
        burn(LINE_ITEM_BURN, 10, sword("s"), 3, 5, 3),
        mint_line(LINE_COIN_MINT, 20, coin(Coin::Crystal, "r"), 4, 2),
        mint_line(LINE_COIN_MINT, 21, coin(Coin::Platinum, "r"), 5, 46),
        mint_line(LINE_COIN_MINT, 22, coin(Coin::Gold, "r"), 6, 90),
    ];
    value.minted_gold_units = 24_690;
    value
}

fn identity() -> NpcTradeEventIdentity<'static> {
    NpcTradeEventIdentity {
        event_id: fixed(80),
        transaction_id: fixed(81),
        occurred_at_unix_ms: 1_790_000_000_000,
        server_build_id: "oteryn-game-server-test",
    }
}

#[test]
fn a_buy_and_a_sell_round_trip() {
    for (value, usage) in [
        (
            buy(),
            TransactionResourceUsage {
                touched_item_instances: 4,
                location_custody_lines: 3,
                value_lines: 0,
                transform_lines: 0,
                container_expansion: 0,
                participants: 4,
                effect_work_units: 9,
                events: 1,
            },
        ),
        (
            sell(),
            TransactionResourceUsage {
                touched_item_instances: 4,
                location_custody_lines: 3,
                value_lines: 0,
                transform_lines: 0,
                container_expansion: 0,
                participants: 4,
                effect_work_units: 8,
                events: 1,
            },
        ),
    ] {
        for tuple in [Type2EventTuple::V1, Type2EventTuple::V2] {
            let wire = encode_npc_trade_event(identity(), value.clone(), tuple).unwrap();
            let (envelope, decoded) = decode_npc_trade_envelope(&wire).unwrap();
            assert_eq!(decoded, value);
            assert_eq!(envelope.event_schema_revision, tuple.schema_revision());
            assert_eq!(envelope.game_session_id, None);
            assert_eq!(envelope.command_id, None);
            // Other operations' gates refuse the trade payload.
            assert_eq!(
                decode_payload(&envelope.payload),
                Err(AuditError::InvalidInput)
            );
            assert_eq!(
                decode_fee_burn_payload(&envelope.payload),
                Err(AuditError::InvalidInput)
            );
        }
        assert_eq!(npc_trade_usage(&value), usage);
    }
}

/// Price 0 (NPC-0 §5.3, §5.4): a BUY mints only the item, a SELL burns only the item.
fn free_buy() -> OneItemNpcTradeV1 {
    let mut value = trade(SIDE_BUY, 1, 0);
    value.lines = vec![mint_line(LINE_ITEM_MINT, 21, sword("s"), 6, 1)];
    value
}

fn free_sell() -> OneItemNpcTradeV1 {
    let mut value = trade(SIDE_SELL, 2, 0);
    value.lines = vec![burn(LINE_ITEM_BURN, 10, sword("s"), 3, 5, 3)];
    value
}

#[test]
fn a_free_buy_and_sell_round_trip() {
    for value in [free_buy(), free_sell()] {
        let wire = encode_npc_trade_event(identity(), value.clone(), Type2EventTuple::V2).unwrap();
        assert_eq!(decode_npc_trade_envelope(&wire).unwrap().1, value);
    }
    let mut value = free_buy();
    value
        .lines
        .insert(0, burn(LINE_COIN_BURN, 10, coin(Coin::Gold, "r"), 4, 10, 9));
    value.lines[1].placement_ordinal = 5;
    value.burned_gold_units = 1;
    value.minted_gold_units = 0;
    assert_eq!(check_npc_trade(&value), Err(AuditError::InvalidInput));
    // A priced BUY burns: the item alone does not pay 235.
    let mut value = free_buy();
    value.unit_price = 235;
    value.total_gold_units = 235;
    assert_eq!(check_npc_trade(&value), Err(AuditError::InvalidInput));
    let mut value = free_sell();
    value.minted_gold_units = 1;
    assert_eq!(check_npc_trade(&value), Err(AuditError::InvalidInput));
}

#[test]
fn every_broken_invariant_is_rejected() {
    let mut cases: Vec<(&str, OneItemNpcTradeV1)> = Vec::new();
    let mut value = buy();
    value.cause.as_mut().unwrap().side = 3;
    cases.push(("unknown side", value));
    let mut value = buy();
    value.total_gold_units = 236;
    cases.push(("total other than price x quantity", value));
    let mut value = buy();
    value.quantity = 0;
    value.total_gold_units = 0;
    cases.push(("zero quantity", value));
    let mut value = buy();
    value.burned_gold_units = 309;
    cases.push(("burned summary other than the lines", value));
    let mut value = buy();
    value.lines.swap(0, 1);
    cases.push(("plan order broken", value));
    let mut value = buy();
    value.lines[0].after = Some(state(10, coin(Coin::Gold, "r"), 5));
    cases.push(("a partial coin burn before the last", value));
    let mut value = buy();
    value.lines[2].placement_ordinal = 4;
    cases.push(("a MINT inside the burned entries", value));
    let mut value = buy();
    value.lines[3].placement_ordinal = 7;
    cases.push(("MINT entries not consecutive", value));
    let mut value = buy();
    value.lines.swap(2, 3);
    cases.push(("the item MINT before the change", value));
    let mut value = buy();
    value.lines.pop();
    cases.push(("a BUY without its item", value));
    let mut value = buy();
    value.lines[3].after = Some(state(21, coin(Coin::Crystal, "r"), 1));
    cases.push(("a coin bought as the item", value));
    let mut value = buy();
    value.lines[2].after = Some(state(20, coin(Coin::Gold, "r"), 74));
    value.minted_gold_units = 74;
    value.burned_gold_units = 309;
    cases.push(("change other than burned minus price", value));
    let mut value = buy();
    value.lines[3].after = Some(state(10, sword("s"), 1));
    cases.push(("a repeated item", value));
    let mut value = buy();
    value.lines[0].before = None;
    cases.push(("a BURN without its before state", value));
    let mut value = buy();
    value.lines[2].before = Some(state(20, coin(Coin::Gold, "r"), 1));
    cases.push(("a MINT with a before state", value));
    let mut value = sell();
    value.lines[0].kind = LINE_COIN_BURN;
    cases.push(("a SELL burning coins", value));
    let mut value = sell();
    value.lines[0].before = Some(state(10, coin(Coin::Gold, "r"), 5));
    value.lines[0].after = Some(state(10, coin(Coin::Gold, "r"), 3));
    cases.push(("a coin sold as the item", value));
    let mut value = sell();
    value.lines.swap(1, 2);
    value.lines[1].placement_ordinal = 4;
    value.lines[2].placement_ordinal = 5;
    cases.push(("proceeds out of order", value));
    let mut value = sell();
    value.lines.pop();
    cases.push(("proceeds short of the price", value));
    let mut value = sell();
    value.burned_gold_units = 1;
    cases.push(("a SELL burning gold units", value));
    let mut value = sell();
    value
        .lines
        .push(mint_line(LINE_ITEM_MINT, 30, sword("s"), 7, 1));
    cases.push(("a SELL minting an item", value));
    let mut value = trade(SIDE_SELL, 1, 1_010_000);
    value.lines = sell().lines;
    cases.push(("a SELL above three stacks", value));
    let mut value = sell();
    value.lines[0].after.as_mut().unwrap().lifecycle = ITEM_LIFECYCLE_RETIRED;
    cases.push(("a partial burn retired", value));
    let mut value = buy();
    value.command.as_mut().unwrap().command_id = 0;
    cases.push(("command 0", value));
    let mut value = buy();
    value.cause.as_mut().unwrap().npc_key = String::new();
    cases.push(("empty NPC key", value));
    for (name, value) in cases {
        assert_eq!(
            check_npc_trade(&value),
            Err(AuditError::InvalidInput),
            "{name}"
        );
        assert!(
            encode_npc_trade_event(identity(), value, Type2EventTuple::V1).is_err(),
            "{name}"
        );
    }
}

fn registered(id: &str) -> u64 {
    let registry: Value = serde_json::from_str(RESOURCE_REGISTRY).unwrap();
    let rows: Vec<&Value> = registry["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["id"] == id)
        .collect();
    assert_eq!(rows.len(), 1, "exactly one registry row {id}");
    assert!(
        rows[0]["consumers"]
            .as_array()
            .unwrap()
            .iter()
            .any(|consumer| consumer.as_str().unwrap()
                == "apps/game-server/src/durability/npc_trade_audit.rs"),
        "{id} names the trade runtime consumer"
    );
    rows[0]["hard_maximum"].as_u64().unwrap()
}

#[test]
fn trade_rows_equal_the_registry() {
    for (id, value) in [
        (
            "DUR03-RL-01-NPC-TRADE-BUY",
            BUY_RL01_TOUCHED_ITEM_INSTANCES_MAX,
        ),
        (
            "DUR03-RL-02-NPC-TRADE-BUY",
            BUY_RL02_LOCATION_CUSTODY_LINES_MAX,
        ),
        (
            "DUR03-RL-06-NPC-TRADE-BUY-PARTICIPANTS",
            BUY_RL06_PARTICIPANTS_MAX,
        ),
        (
            "DUR03-RL-06-NPC-TRADE-BUY-EFFECT-WORK-UNITS",
            BUY_RL06_EFFECT_WORK_UNITS_MAX,
        ),
        (
            "DUR03-RL-01-NPC-TRADE-SELL",
            SELL_RL01_TOUCHED_ITEM_INSTANCES_MAX,
        ),
        (
            "DUR03-RL-02-NPC-TRADE-SELL",
            SELL_RL02_LOCATION_CUSTODY_LINES_MAX,
        ),
        (
            "DUR03-RL-06-NPC-TRADE-SELL-PARTICIPANTS",
            SELL_RL06_PARTICIPANTS_MAX,
        ),
        (
            "DUR03-RL-06-NPC-TRADE-SELL-EFFECT-WORK-UNITS",
            SELL_RL06_EFFECT_WORK_UNITS_MAX,
        ),
        // BANK-FEE-0 §4.3/§5: one bank value line on a BUY, none on a SELL.
        ("DUR03-RL-03-NPC-TRADE-BUY", 1),
        ("DUR03-RL-07-NPC-TRADE-EVENTS", NPC_TRADE_RL07_EVENTS_MAX),
        (
            "DUR03-RL-07-NPC-TRADE-PAYLOAD-BYTES",
            NPC_TRADE_RL07_PAYLOAD_BYTES_MAX as u64,
        ),
        (
            "DUR03-RL-07-NPC-TRADE-ENVELOPE-BYTES",
            NPC_TRADE_RL07_ENVELOPE_BYTES_MAX as u64,
        ),
        // NPC-TRADE-SELLALL-1.
        (
            "DUR03-RL-01-NPC-SELL-ALL",
            SELL_ALL_RL01_TOUCHED_ITEM_INSTANCES_MAX,
        ),
        (
            "DUR03-RL-02-NPC-SELL-ALL",
            SELL_ALL_RL02_LOCATION_CUSTODY_LINES_MAX,
        ),
        (
            "DUR03-RL-06-NPC-SELL-ALL-PARTICIPANTS",
            SELL_ALL_RL06_PARTICIPANTS_MAX,
        ),
        (
            "DUR03-RL-06-NPC-SELL-ALL-EFFECT-WORK-UNITS",
            SELL_ALL_RL06_EFFECT_WORK_UNITS_MAX,
        ),
        (
            "DUR03-RL-07-NPC-SELL-ALL-PAYLOAD-BYTES",
            SELL_ALL_RL07_PAYLOAD_BYTES_MAX as u64,
        ),
        (
            "DUR03-RL-07-NPC-SELL-ALL-ENVELOPE-BYTES",
            SELL_ALL_RL07_ENVELOPE_BYTES_MAX as u64,
        ),
    ] {
        assert_eq!(registered(id), value, "{id}");
    }
    // NPC-0 §5.5.
    assert_eq!(
        (BUY_RL06_PARTICIPANTS_MAX, BUY_RL06_EFFECT_WORK_UNITS_MAX),
        (23, 66)
    );
    assert_eq!(
        (SELL_RL06_PARTICIPANTS_MAX, SELL_RL06_EFFECT_WORK_UNITS_MAX),
        (4, 9)
    );
}

/// A BUY of 20 whole crystal stacks for 19,990,001: the largest change (99 platinum, 99 gold)
/// and the item, every key and revision at 512 bytes, maximal ordinals and integers.
fn worst_case(burns: usize) -> OneItemNpcTradeV1 {
    let long = "r".repeat(mint::RL07_CONTENT_KEY_BYTES_MAX);
    let family = "f".repeat(mint::RL07_TECHNICAL_FIELD_BYTES_MAX);
    let burned = u64::try_from(burns).unwrap() * 100 * Coin::Crystal.worth();
    let mut value = trade(SIDE_BUY, 1, burned - 9_999);
    value.cause = Some(OneItemNpcTradeCauseV1 {
        npc_key: "n".repeat(mint::RL07_CONTENT_KEY_BYTES_MAX),
        catalogue_revision: u64::MAX,
        offer_index: u32::MAX,
        side: SIDE_BUY,
        occurrence: uuid(70),
    });
    value.command.as_mut().unwrap().command_id = u64::MAX;
    value.runtime_scope_ownership_generation = u64::MAX;
    value.lines = (0..burns)
        .map(|n| {
            let tag = u8::try_from(n).unwrap() + 100;
            burn(
                LINE_COIN_BURN,
                tag,
                coin(Coin::Crystal, &long),
                u64::MAX - 3 - n as u64,
                100,
                0,
            )
        })
        .collect();
    value.lines.extend([
        mint_line(
            LINE_COIN_MINT,
            200,
            coin(Coin::Platinum, &long),
            u64::MAX - 2,
            99,
        ),
        mint_line(
            LINE_COIN_MINT,
            201,
            coin(Coin::Gold, &long),
            u64::MAX - 1,
            99,
        ),
        mint_line(
            LINE_ITEM_MINT,
            202,
            definition(&family, &long, &long),
            u64::MAX,
            1,
        ),
    ]);
    value.burned_gold_units = burned;
    value.minted_gold_units = 9_999;
    value
}

// Within the ANL-01 ceilings (payload 196,608 B, envelope 262,144 B).
const _: () = assert!(NPC_TRADE_RL07_PAYLOAD_BYTES_MAX <= 196_608);
const _: () = assert!(NPC_TRADE_RL07_ENVELOPE_BYTES_MAX <= 262_144);

#[test]
fn worst_case_payload_and_envelope_are_the_registered_rows() {
    let value = worst_case(FEE_INPUTS_MAX);
    check_npc_trade(&value).unwrap();
    assert_eq!(
        npc_trade_usage(&value).effect_work_units,
        BUY_RL06_EFFECT_WORK_UNITS_MAX
    );
    let payload = OneItemTransactionV1 {
        interpretation_revision: mint::INTERPRETATION_REVISION,
        operation: Some(OneItemOperationV1::NpcTrade(value.clone())),
    }
    .encoded_len();
    assert_eq!(payload, NPC_TRADE_RL07_PAYLOAD_BYTES_MAX);
    let identity = NpcTradeEventIdentity {
        occurred_at_unix_ms: i64::MAX,
        server_build_id: &"b".repeat(mint::RL07_TECHNICAL_FIELD_BYTES_MAX),
        ..identity()
    };
    let wire = encode_npc_trade_event(identity, value, Type2EventTuple::V2).unwrap();
    assert_eq!(wire.len(), NPC_TRADE_RL07_ENVELOPE_BYTES_MAX);
    // A 21st coin line is rejected before encoding.
    let over = worst_case(FEE_INPUTS_MAX + 1);
    assert!(encode_npc_trade_event(identity, over, Type2EventTuple::V2).is_err());
    // One byte over either ceiling is refused before decoding.
    let mut long = wire.clone();
    long.push(0);
    assert_eq!(
        decode_npc_trade_envelope(&long),
        Err(AuditError::CapacityExceeded)
    );
    assert_eq!(
        decode_npc_trade_payload(&vec![0; NPC_TRADE_RL07_PAYLOAD_BYTES_MAX + 1]),
        Err(AuditError::CapacityExceeded)
    );
}

#[test]
fn npc_buy_bank_value_line_is_v2_only_and_conserves_the_single_trade() {
    let mut value = trade(SIDE_BUY, 1, 250);
    value.lines = vec![mint_line(LINE_ITEM_MINT, 21, sword("s"), 1, 1)];
    value.bank_debit = Some(super::super::item_fee_burn_audit::OneItemFeeBankDebitV1 {
        entry_id: uuid(90),
        asset: "gold".into(),
        account_id: uuid(91),
        world_id: value.world_id.clone(),
        kind: 5,
        line_class: 3,
        debit_gold_units: 250,
        balance_before_gold_units: 300,
        balance_after_gold_units: 50,
    });
    assert!(check_npc_trade(&value).is_ok());
    assert_eq!(npc_trade_usage(&value).value_lines, 1);
    assert_eq!(npc_trade_usage(&value).participants, 2);
    let mut over_value_lines = npc_trade_usage(&value);
    over_value_lines.value_lines = 2;
    assert_eq!(
        check_npc_trade_usage(SIDE_BUY, false, &over_value_lines),
        Err(AuditError::InvalidInput)
    );
    // Prost accepts a repeated singular message by merging it. The canonical native gate
    // must reject the extra raw field even when the decoded last value is valid and equal.
    let mut duplicate = value.encode_to_vec();
    let debit = value.bank_debit.as_ref().unwrap().encode_to_vec();
    prost::encoding::encode_varint((14 << 3) | 2, &mut duplicate);
    prost::encoding::encode_varint(debit.len() as u64, &mut duplicate);
    duplicate.extend_from_slice(&debit);
    assert_eq!(
        OneItemNpcTradeV1::decode(duplicate.as_slice()).unwrap(),
        value
    );
    let mut duplicate_payload = OneItemTransactionV1 {
        interpretation_revision: mint::INTERPRETATION_REVISION,
        operation: None,
    }
    .encode_to_vec();
    prost::encoding::encode_varint((9 << 3) | 2, &mut duplicate_payload);
    prost::encoding::encode_varint(duplicate.len() as u64, &mut duplicate_payload);
    duplicate_payload.extend_from_slice(&duplicate);
    assert_eq!(
        decode_npc_trade_payload(&duplicate_payload),
        Err(AuditError::InvalidInput)
    );
    assert!(encode_npc_trade_event(identity(), value.clone(), Type2EventTuple::V1).is_err());
    let wire = encode_npc_trade_event(identity(), value.clone(), Type2EventTuple::V2).unwrap();
    assert_eq!(decode_npc_trade_envelope(&wire).unwrap().1, value);
    for mutate in [0, 1, 2, 3] {
        let mut broken = value.clone();
        match mutate {
            0 => broken.bank_debit.as_mut().unwrap().balance_after_gold_units += 1,
            1 => broken.bank_debit.as_mut().unwrap().debit_gold_units -= 1,
            2 => broken.bank_debit.as_mut().unwrap().kind = 1,
            _ => broken.cause.as_mut().unwrap().side = SIDE_SELL,
        }
        assert!(check_npc_trade(&broken).is_err());
    }
}

#[test]
fn largest_bank_remainder_fits_existing_measured_trade_envelope_bounds() {
    let mut value = worst_case(FEE_INPUTS_MAX);
    value.lines.retain(|line| line.kind != LINE_COIN_MINT);
    value.lines.last_mut().unwrap().placement_ordinal = u64::MAX - 2;
    value.minted_gold_units = 0;
    value.total_gold_units = NPC_TRADE_BUY_GOLD_UNITS_MAX;
    value.unit_price = value.total_gold_units;
    let maximum = super::super::bank_audit::BANK_BALANCE_MAX;
    value.bank_debit = Some(super::super::item_fee_burn_audit::OneItemFeeBankDebitV1 {
        entry_id: uuid(90),
        asset: "gold".into(),
        account_id: uuid(91),
        world_id: value.world_id.clone(),
        kind: 5,
        line_class: 3,
        debit_gold_units: maximum,
        balance_before_gold_units: maximum,
        balance_after_gold_units: 0,
    });
    assert!(check_npc_trade(&value).is_ok());
    assert_eq!(npc_trade_usage(&value).participants, 22);
    assert_eq!(npc_trade_usage(&value).effect_work_units, 64);
    let payload = OneItemTransactionV1 {
        interpretation_revision: mint::INTERPRETATION_REVISION,
        operation: Some(OneItemOperationV1::NpcTrade(value.clone())),
    }
    .encoded_len();
    let build = "b".repeat(mint::RL07_TECHNICAL_FIELD_BYTES_MAX);
    let wire = encode_npc_trade_event(
        NpcTradeEventIdentity {
            occurred_at_unix_ms: i64::MAX,
            server_build_id: &build,
            ..identity()
        },
        value,
        Type2EventTuple::V2,
    )
    .unwrap();
    eprintln!(
        "NPC-BUY-BANK-D50 payload={payload} envelope={} participants=22 work=64",
        wire.len()
    );
    assert!(payload <= NPC_TRADE_RL07_PAYLOAD_BYTES_MAX);
    assert!(wire.len() <= NPC_TRADE_RL07_ENVELOPE_BYTES_MAX);
}

/// A sell-all of two sword stacks (7 at ordinal 5, 100 at ordinal 3) at 99 each: 10,593 as 1
/// crystal, 5 platinum and 93 gold.
fn sell_all() -> OneItemNpcTradeV1 {
    let mut value = trade(SIDE_SELL, 107, 99);
    value.sell_all = true;
    value.lines = vec![
        burn(LINE_ITEM_BURN, 10, sword("s"), 5, 7, 0),
        burn(LINE_ITEM_BURN, 11, sword("s"), 3, 100, 0),
        mint_line(LINE_COIN_MINT, 20, coin(Coin::Crystal, "r"), 6, 1),
        mint_line(LINE_COIN_MINT, 21, coin(Coin::Platinum, "r"), 7, 5),
        mint_line(LINE_COIN_MINT, 22, coin(Coin::Gold, "r"), 8, 93),
    ];
    value.minted_gold_units = 10_593;
    value
}

#[test]
fn a_sell_all_burns_whole_stacks_of_one_item_in_display_order() {
    let value = sell_all();
    check_npc_trade(&value).unwrap();
    let wire = encode_npc_trade_event(identity(), value.clone(), Type2EventTuple::V1).unwrap();
    assert_eq!(decode_npc_trade_envelope(&wire).unwrap().1, value);
    let usage = npc_trade_usage(&value);
    assert_eq!(
        (usage.participants, usage.effect_work_units),
        (5, 3 * 2 + 2 * 3)
    );
    let mut cases: Vec<(&str, OneItemNpcTradeV1)> = Vec::new();
    let mut value = sell_all();
    value.cause.as_mut().unwrap().side = SIDE_BUY;
    cases.push(("a sell-all BUY", value));
    let mut value = sell_all();
    value.lines[1].after = Some(state(11, sword("s"), 1));
    value.quantity = 106;
    value.total_gold_units = 106 * 99;
    cases.push(("a partial stack", value));
    let mut value = sell_all();
    value.lines.swap(0, 1);
    cases.push(("stacks out of display order", value));
    let mut value = sell_all();
    value.lines[1].before = Some(state(11, sword("t"), 100));
    value.lines[1].after = Some(OneItemStateV1 {
        lifecycle: ITEM_LIFECYCLE_RETIRED,
        ..state(11, sword("t"), 0)
    });
    cases.push(("another item revision", value));
    let mut value = sell_all();
    value.quantity = 108;
    value.total_gold_units = 108 * 99;
    cases.push(("a quantity other than the units", value));
    let mut value = sell_all();
    value.lines.drain(0..2);
    cases.push(("no stack", value));
    let mut value = sell_all();
    value.quantity = SELL_ALL_UNITS_MAX + 1;
    value.unit_price = 0;
    value.total_gold_units = 0;
    cases.push(("more units than the bound", value));
    for (name, value) in cases {
        assert_eq!(
            check_npc_trade(&value),
            Err(AuditError::InvalidInput),
            "{name}"
        );
    }
}

/// A sell-all of `stacks` whole stacks (the first of `first` units, then 100 each) at `price`,
/// every key and revision at 512 bytes, maximal ordinals and integers.
fn worst_case_sell_all(stacks: usize, first: u32, price: u64) -> OneItemNpcTradeV1 {
    let long = "r".repeat(mint::RL07_CONTENT_KEY_BYTES_MAX);
    let family = "f".repeat(mint::RL07_TECHNICAL_FIELD_BYTES_MAX);
    let units = first + 100 * (u32::try_from(stacks).unwrap() - 1);
    let mut value = trade(SIDE_SELL, units, price);
    value.sell_all = true;
    value.cause = Some(OneItemNpcTradeCauseV1 {
        npc_key: "n".repeat(mint::RL07_CONTENT_KEY_BYTES_MAX),
        catalogue_revision: u64::MAX,
        offer_index: u32::MAX,
        side: SIDE_SELL,
        occurrence: uuid(70),
    });
    value.command.as_mut().unwrap().command_id = u64::MAX;
    value.runtime_scope_ownership_generation = u64::MAX;
    value.lines = (0..stacks)
        .map(|n| {
            let tag = u8::try_from(n).unwrap() + 100;
            let quantity = if n == 0 { first } else { 100 };
            burn(
                LINE_ITEM_BURN,
                tag,
                definition(&family, &long, &long),
                u64::MAX - 3 - n as u64,
                quantity,
                0,
            )
        })
        .collect();
    let total = u64::from(units) * price;
    let coins = sell_coins(total).unwrap();
    for (index, (coin_kind, quantity)) in coins.into_iter().enumerate() {
        value.lines.push(mint_line(
            LINE_COIN_MINT,
            200 + u8::try_from(index).unwrap(),
            coin(coin_kind, &long),
            u64::MAX - 2 + index as u64,
            quantity,
        ));
    }
    value.minted_gold_units = total;
    value
}

/// The worst case of `stacks` single items at `price`.
fn singles(stacks: usize, price: u64) -> OneItemNpcTradeV1 {
    let mut value = worst_case_sell_all(stacks, 1, 0);
    value.unit_price = price;
    for line in value.lines.iter_mut().take(stacks) {
        line.before.as_mut().unwrap().quantity = 1;
    }
    let units = u32::try_from(stacks).unwrap();
    value.quantity = units;
    value.total_gold_units = u64::from(units) * price;
    value.lines.truncate(stacks);
    let long = "r".repeat(mint::RL07_CONTENT_KEY_BYTES_MAX);
    for (index, (coin_kind, quantity)) in sell_coins(value.total_gold_units)
        .unwrap()
        .into_iter()
        .enumerate()
    {
        value.lines.push(mint_line(
            LINE_COIN_MINT,
            200 + u8::try_from(index).unwrap(),
            coin(coin_kind, &long),
            u64::MAX - 2 + index as u64,
            quantity,
        ));
    }
    value.minted_gold_units = value.total_gold_units;
    value
}

const _: () = assert!(SELL_ALL_RL07_PAYLOAD_BYTES_MAX <= 196_608);
const _: () = assert!(SELL_ALL_RL07_ENVELOPE_BYTES_MAX <= 262_144);
const _: () = assert!(SELL_ALL_RL07_PAYLOAD_BYTES_MAX >= NPC_TRADE_RL07_PAYLOAD_BYTES_MAX);
const _: () = assert!(SELL_ALL_RL07_ENVELOPE_BYTES_MAX >= NPC_TRADE_RL07_ENVELOPE_BYTES_MAX);

#[test]
fn worst_case_sell_all_payload_and_envelope_are_the_registered_rows() {
    let identity = NpcTradeEventIdentity {
        occurred_at_unix_ms: i64::MAX,
        server_build_id: &"b".repeat(mint::RL07_TECHNICAL_FIELD_BYTES_MAX),
        ..identity()
    };
    let measure = |value: &OneItemNpcTradeV1| {
        let payload = OneItemTransactionV1 {
            interpretation_revision: mint::INTERPRETATION_REVISION,
            operation: Some(OneItemOperationV1::NpcTrade(value.clone())),
        };
        let bytes = payload.encode_to_vec();
        let envelope = EventEnvelopeV1 {
            envelope_revision: mint::ENVELOPE_REVISION,
            event_id: identity.event_id.to_vec(),
            event_type_id: mint::EVENT_TYPE_ID,
            event_schema_revision: Type2EventTuple::V2.schema_revision(),
            durability_class: mint::DURABLE_AUDIT,
            privacy_class: mint::RESTRICTED_PLAYER_LINKED,
            retention_profile_id: Type2EventTuple::V2.retention_profile_id().into(),
            occurred_at_unix_ms: identity.occurred_at_unix_ms,
            world_id: Some(value.world_id.clone()),
            channel_id: Some(value.channel_id.clone()),
            transaction_event: Some(TransactionEventRefV1 {
                transaction_id: identity.transaction_id.to_vec(),
                ordinal: 1,
                count: 1,
            }),
            server_build_id: identity.server_build_id.into(),
            payload_sha256: Sha256::digest(&bytes).to_vec(),
            payload: bytes.clone(),
            ..EventEnvelopeV1::default()
        };
        (bytes.len(), envelope.encoded_len())
    };
    // 1,999 units at 505 (100 crystal, 94 platinum, 95 gold), 2,000 at 504 and 20 single items
    // at 50,499 (100 crystal, 99 platinum, 80 gold): the largest unit count and price.
    let mut largest = (0, 0);
    for value in [
        worst_case_sell_all(FEE_INPUTS_MAX, 99, 505),
        worst_case_sell_all(FEE_INPUTS_MAX, 100, 504),
        singles(FEE_INPUTS_MAX, 50_499),
    ] {
        check_npc_trade(&value).unwrap();
        let (payload, envelope) = measure(&value);
        largest = (largest.0.max(payload), largest.1.max(envelope));
        // Above the rows: refused before any write.
        assert_eq!(
            encode_npc_trade_event(identity, value, Type2EventTuple::V2),
            Err(AuditError::CapacityExceeded)
        );
    }
    assert_eq!(
        largest,
        (
            SELL_ALL_WORST_CASE_PAYLOAD_BYTES,
            SELL_ALL_WORST_CASE_ENVELOPE_BYTES
        )
    );
    // The rows are the outbox ceiling and that ceiling less the measured overhead.
    assert_eq!(SELL_ALL_RL07_ENVELOPE_BYTES_MAX, 34_816);
    assert_eq!(
        SELL_ALL_RL07_ENVELOPE_BYTES_MAX - SELL_ALL_RL07_PAYLOAD_BYTES_MAX,
        SELL_ALL_WORST_CASE_ENVELOPE_BYTES - SELL_ALL_WORST_CASE_PAYLOAD_BYTES
    );
    // 20 stacks under production-sized keys fit and round trip.
    let mut value = singles(FEE_INPUTS_MAX, 50_499);
    let key = "oteryn:item.tibia.i3264";
    for line in &mut value.lines {
        for state in [line.before.as_mut(), line.after.as_mut()].into_iter().flatten() {
            let definition = state.definition.as_mut().unwrap();
            definition.revision_ref = "r".repeat(64);
            if definition.family != COIN_DEFINITION_FAMILY {
                definition.family = "Item".into();
                definition.production_key = key.into();
            }
        }
    }
    value.cause.as_mut().unwrap().npc_key = "oteryn:npc.tibia.sam".into();
    check_npc_trade(&value).unwrap();
    let wire = encode_npc_trade_event(identity, value.clone(), Type2EventTuple::V2).unwrap();
    assert!(wire.len() <= SELL_ALL_RL07_ENVELOPE_BYTES_MAX);
    assert_eq!(decode_npc_trade_envelope(&wire).unwrap().1, value);
    assert_eq!(
        npc_trade_usage(&value).effect_work_units,
        SELL_ALL_RL06_EFFECT_WORK_UNITS_MAX
    );
    // The worst case with 64-byte revisions exceeds the trade row but fits the sell-all row.
    let mut wide = singles(FEE_INPUTS_MAX, 50_499);
    for line in &mut wide.lines {
        for state in [line.before.as_mut(), line.after.as_mut()].into_iter().flatten() {
            state.definition.as_mut().unwrap().revision_ref = "r".repeat(64);
        }
    }
    check_npc_trade(&wide).unwrap();
    let wire = encode_npc_trade_event(identity, wide.clone(), Type2EventTuple::V2).unwrap();
    assert!(wire.len() > NPC_TRADE_RL07_ENVELOPE_BYTES_MAX);
    assert!(wire.len() <= SELL_ALL_RL07_ENVELOPE_BYTES_MAX);
    assert_eq!(decode_npc_trade_envelope(&wire).unwrap().1, wide);
    assert_eq!(
        decode_npc_trade_payload(&vec![0; SELL_ALL_RL07_PAYLOAD_BYTES_MAX + 1]),
        Err(AuditError::CapacityExceeded)
    );
    // A 21st stack is refused before encoding.
    let over = singles(FEE_INPUTS_MAX + 1, 1);
    assert_eq!(check_npc_trade(&over), Err(AuditError::CapacityExceeded));
}

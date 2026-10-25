//! Fixed wire fixtures captured before DUR-ONEOF-1 changed either oneof block.
//! These are codec fixtures, not semantically valid gameplay transactions.
use super::*;
use crate::durability::item_fee_burn_audit::{
    FeeBurnCauseV1, OneItemAreaDonationV1, OneItemCharmUnassignV1, OneItemFeeBurnCauseV1,
    OneItemImbuementV1, OneItemMarketFeeV1, OneItemNpcPurchaseV1, OneItemTimedRepairV1,
};

fn operation_fixtures() -> Vec<(u32, &'static str, OneItemOperationV1)> {
    vec![
        (
            2,
            "Mint",
            OneItemOperationV1::Mint(OneItemMintV1 {
                after: Some(OneItemStateV1 {
                    quantity: 7,
                    ..Default::default()
                }),
                before_semantically_absent: true,
                ..Default::default()
            }),
        ),
        (
            3,
            "Transfer",
            OneItemOperationV1::Transfer(
                crate::durability::item_transfer_audit::OneItemTransferV1 {
                    before: Some(OneItemStateV1 {
                        quantity: 7,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ),
        ),
        (
            4,
            "RewardClaimMint",
            OneItemOperationV1::RewardClaimMint(
                crate::durability::reward_claim_mint_audit::OneItemRewardClaimMintV1 {
                    before_semantically_absent: true,
                    ..Default::default()
                },
            ),
        ),
        (
            5,
            "DecayRetire",
            OneItemOperationV1::DecayRetire(
                crate::durability::item_decay_retire_audit::OneItemDecayRetireV1 {
                    runtime_scope_ownership_generation: 129,
                    ..Default::default()
                },
            ),
        ),
        (
            6,
            "FeeBurn",
            OneItemOperationV1::FeeBurn(crate::durability::item_fee_burn_audit::OneItemFeeBurnV1 {
                fee_gold_units: 129,
                ..Default::default()
            }),
        ),
        (
            7,
            "TimedExpiry",
            OneItemOperationV1::TimedExpiry(
                crate::durability::item_timed_state_audit::OneItemTimedExpiryV1 {
                    runtime_scope_ownership_generation: 129,
                    equipment_slot: Some(0),
                    ..Default::default()
                },
            ),
        ),
        (
            8,
            "MapItemMint",
            OneItemOperationV1::MapItemMint(
                crate::durability::map_item_mint_audit::OneItemMapItemMintV1 {
                    before_semantically_absent: true,
                    connection_generation: 129,
                    ..Default::default()
                },
            ),
        ),
        (
            9,
            "NpcTrade",
            OneItemOperationV1::NpcTrade(crate::durability::npc_trade_audit::OneItemNpcTradeV1 {
                quantity: 7,
                unit_price: 129,
                ..Default::default()
            }),
        ),
        (
            10,
            "ItemUse",
            OneItemOperationV1::ItemUse(crate::durability::item_use_audit::OneItemUseV1 {
                expected_session_generation: 129,
                flask_missing: true,
                ..Default::default()
            }),
        ),
        (
            11,
            "HouseTransfer",
            OneItemOperationV1::HouseTransfer(
                crate::durability::house_interior_audit::OneItemHouseTransferV1 {
                    connection_generation: 129,
                    character_lease_generation: 7,
                    ..Default::default()
                },
            ),
        ),
        (
            13,
            "ForgeDustLimit",
            OneItemOperationV1::ForgeDustLimit(
                crate::durability::character_forge_dust_audit::OneItemForgeDustLimitV1 {
                    dust_burned: 129,
                    character_revision: 7,
                    ..Default::default()
                },
            ),
        ),
        (
            14,
            "PlayerTrade",
            OneItemOperationV1::PlayerTrade(
                crate::durability::player_trade::audit::OneItemPlayerTradeV1 {
                    request_binding: vec![0x81, 0x07],
                    ..Default::default()
                },
            ),
        ),
        (
            15,
            "Craft",
            OneItemOperationV1::Craft(crate::durability::item_craft::audit::OneItemCraftV1 {
                proof: vec![0x81, 0x07],
            }),
        ),
    ]
}

fn fee_fixtures() -> Vec<(u32, &'static str, FeeBurnCauseV1)> {
    vec![
        (
            1,
            "CharmUnassign",
            FeeBurnCauseV1::CharmUnassign(OneItemCharmUnassignV1 {
                charm_key: "charm".into(),
                occurrence: vec![0x81, 0x07],
            }),
        ),
        (
            2,
            "PromotionPurchase",
            FeeBurnCauseV1::PromotionPurchase(OneItemNpcPurchaseV1 {
                service_key: "promotion".into(),
                occurrence: vec![0x81, 0x07],
            }),
        ),
        (
            3,
            "BlessingPurchase",
            FeeBurnCauseV1::BlessingPurchase(OneItemNpcPurchaseV1 {
                service_key: "blessing".into(),
                occurrence: vec![0x81, 0x07],
            }),
        ),
        (
            4,
            "AreaDonation",
            FeeBurnCauseV1::AreaDonation(OneItemAreaDonationV1 {
                world: vec![0x81, 0x07],
                area: 7,
                occurrence: vec![0x08, 0x09],
            }),
        ),
        (
            5,
            "NpcRepair",
            FeeBurnCauseV1::NpcRepair(Box::new(OneItemTimedRepairV1 {
                occurrence: vec![0x81, 0x07],
                placement_ordinal: 129,
                ..Default::default()
            })),
        ),
        (
            6,
            "MarketFee",
            FeeBurnCauseV1::MarketFee(OneItemMarketFeeV1 {
                offer: vec![0x81, 0x07],
                occurrence: vec![0x08, 0x09],
            }),
        ),
        (
            6,
            "MarketFee",
            FeeBurnCauseV1::MarketFee(OneItemMarketFeeV1 {
                offer: vec![0x81, 0x07],
                occurrence: vec![0x08, 0x09],
            }),
        ),
        (
            7,
            "Imbuement",
            FeeBurnCauseV1::Imbuement(Box::new(OneItemImbuementV1 {
                occurrence: vec![0x81, 0x07],
                placement_ordinal: 129,
                ..Default::default()
            })),
        ),
    ]
}

fn assert_wire<M: Message + Default + PartialEq>(name: &str, value: M) {
    // Captured at 1fcf3ded6, with only this test module mounted, before sorting.
    let goldens = [
        ("Mint", "080112060a0220072001"),
        ("Transfer", "08011a040a022007"),
        ("RewardClaimMint", "080122022001"),
        ("DecayRetire", "08012a03308101"),
        ("FeeBurn", "08013203108101"),
        ("TimedExpiry", "08013a053081016000"),
        ("MapItemMint", "080142052001288101"),
        ("NpcTrade", "08014a054007488101"),
        ("ItemUse", "080152054881016001"),
        ("HouseTransfer", "08015a054881015007"),
        ("ForgeDustLimit", "08016a06488101900107"),
        ("PlayerTrade", "080172040a028107"),
        ("Craft", "08017a040a028107"),
        ("CharmUnassign", "0a0b0a05636861726d12028107"),
        ("PromotionPurchase", "120f0a0970726f6d6f74696f6e12028107"),
        ("BlessingPurchase", "1a0e0a08626c657373696e6712028107"),
        ("AreaDonation", "220a0a02810710071a020809"),
        ("NpcRepair", "2a070a028107308101"),
        ("MarketFee", "32080a02810712020809"),
        ("MarketFee", "32080a02810712020809"),
        ("Imbuement", "3a070a028107208101"),
        ("EventId", "0a028107"),
        ("Command", "12070a028107108101"),
        ("OperationId", "1a028107"),
        ("TransactionId", "22028107"),
    ];
    let hex = goldens
        .iter()
        .find(|(variant, _)| *variant == name)
        .unwrap()
        .1;
    let expected: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect();
    let bytes = value.encode_to_vec();
    assert_eq!(bytes, expected, "{name}");
    assert!(M::decode(bytes.as_slice()).unwrap() == value, "{name}");
}

#[test]
fn every_operation_matches_pre_structure_wire_bytes() {
    for (_, name, operation) in operation_fixtures() {
        assert_wire(
            name,
            OneItemTransactionV1 {
                interpretation_revision: 1,
                operation: Some(operation),
            },
        );
    }
}

#[test]
fn every_fee_cause_matches_pre_structure_wire_bytes() {
    for (_, name, cause) in fee_fixtures() {
        assert_wire(name, OneItemFeeBurnCauseV1 { cause: Some(cause) });
    }
}

#[test]
fn every_causation_matches_pre_structure_wire_bytes() {
    for (name, cause) in [
        ("EventId", CausationV1::EventId(vec![0x81, 0x07])),
        (
            "Command",
            CausationV1::Command(EnvelopeCommandRefV1 {
                game_session_id: vec![0x81, 0x07],
                command_id: 129,
            }),
        ),
        ("OperationId", CausationV1::OperationId(vec![0x81, 0x07])),
        (
            "TransactionId",
            CausationV1::TransactionId(vec![0x81, 0x07]),
        ),
    ] {
        assert_wire(name, CausationRefV1 { cause: Some(cause) });
    }
}

fn assert_tag_block(source: &str, oneof: &str, golden: &str, fixtures: Vec<(u32, &str)>) {
    let start = "// Keep sorted by tag; insert in order, never append.";
    let body = source
        .split_once(&format!("pub enum {oneof} {{"))
        .unwrap()
        .1;
    let body = body.split_once("\n}").unwrap().0;
    let (before, body) = body.split_once(start).unwrap();
    assert!(
        before.trim().is_empty(),
        "{oneof}: variant before sorted block"
    );
    let (body, after) = body.split_once("// End tag-sorted variants.").unwrap();
    assert!(
        after.trim().is_empty(),
        "{oneof}: variant after sorted block"
    );
    let lines: Vec<_> = body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .collect();
    let (pairs, remainder) = lines.as_chunks::<2>();
    assert!(remainder.is_empty(), "{oneof}: unpaired tag/variant");
    let variants: Vec<_> = pairs
        .iter()
        .map(|pair| {
            let tag = pair[0]
                .strip_prefix("#[prost(message, tag = \"")
                .unwrap()
                .strip_suffix("\")]")
                .unwrap()
                .parse::<u32>()
                .unwrap();
            let variant = pair[1].split_once('(').unwrap().0;
            (tag, variant)
        })
        .collect();
    let tags: Vec<_> = variants.iter().map(|(tag, _)| *tag).collect();
    let unique: std::collections::BTreeSet<_> = tags.iter().collect();
    assert_eq!(unique.len(), tags.len(), "{oneof}: duplicate tags");
    assert!(
        tags.windows(2).all(|pair| pair[0] < pair[1]),
        "{oneof}: tag order"
    );
    let expected: Vec<u32> = golden.lines().map(|line| line.parse().unwrap()).collect();
    assert_eq!(tags, expected, "{oneof}: golden tag list");
    assert_eq!(
        variants, fixtures,
        "{oneof}: every variant needs a wire fixture"
    );

    let metadata = source
        .split_once(&format!("oneof = \"{oneof}\""))
        .unwrap()
        .1;
    let metadata = metadata
        .split_once("tags(")
        .unwrap()
        .1
        .split_once(')')
        .unwrap()
        .0;
    let declared: Vec<u32> = metadata
        .lines()
        .map(|line| line.split("//").next().unwrap().trim())
        .filter(|line| !line.is_empty())
        .map(|line| line.strip_suffix(',').unwrap().parse().unwrap())
        .collect();
    assert_eq!(declared, expected, "{oneof}: prost dispatch tag list");
}

#[test]
fn operation_tags_are_unique_sorted_and_match_golden_and_wire_fixtures() {
    assert_tag_block(
        include_str!("../../item_mint_audit.rs"),
        "OneItemOperationV1",
        include_str!("../../item_mint_audit.tags"),
        operation_fixtures()
            .into_iter()
            .map(|(tag, name, _)| (tag, name))
            .collect(),
    );
}

#[test]
fn fee_cause_tags_are_unique_sorted_and_match_golden_and_wire_fixtures() {
    assert_tag_block(
        include_str!("../../item_fee_burn_audit.rs"),
        "FeeBurnCauseV1",
        include_str!("../../item_fee_burn_audit.tags"),
        fee_fixtures()
            .into_iter()
            .map(|(tag, name, _)| (tag, name))
            .collect(),
    );
}

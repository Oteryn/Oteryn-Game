use super::*;
use imbuement_cause::{
    IMBUEMENT_ACTION_APPLY, IMBUEMENT_ACTION_CLEAR, IMBUEMENT_FEE_INPUTS_MAX,
    IMBUEMENT_MATERIALS_MAX,
};

fn item(id: u8, key: &str, quantity: u32) -> OneItemStateV1 {
    OneItemStateV1 {
        item_instance_id: uuid(id),
        world_id: uuid(1),
        definition: Some(OneItemTypedDefinitionRevisionV1 {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "r".repeat(512),
        }),
        quantity,
        lifecycle: if quantity == 0 {
            ITEM_LIFECYCLE_RETIRED
        } else {
            ITEM_LIFECYCLE_LIVE
        },
        key_number: 0,
    }
}

fn apply() -> OneItemImbuementV1 {
    OneItemImbuementV1 {
        occurrence: uuid(70),
        action: IMBUEMENT_ACTION_APPLY,
        item: Some(item(90, "oteryn:item.tibia.i3264", 1)),
        placement_ordinal: u64::MAX,
        slot: 2,
        category: 19,
        imbuement_key: "k".repeat(512),
        imbuement_revision: "r".repeat(512),
        tier: 3,
        materials: (0..IMBUEMENT_MATERIALS_MAX as u8)
            .map(|n| OneItemImbuementMaterialV1 {
                before: Some(item(91 + n, "oteryn:item.tibia.i11444", 100)),
                after: Some(item(91 + n, "oteryn:item.tibia.i11444", u32::from(n) * 40)),
                placement_ordinal: u64::MAX - 1 - u64::from(n),
            })
            .collect(),
    }
}

/// The widest fee: every allowed coin line whole, two change mints, every revision at its
/// registered bound.
fn fee(imbuement: OneItemImbuementV1) -> OneItemFeeBurnV1 {
    let wide = |state: &mut Option<OneItemStateV1>| {
        state
            .as_mut()
            .unwrap()
            .definition
            .as_mut()
            .unwrap()
            .revision_ref = "r".repeat(512);
    };
    let fee = imbuement.fee_gold_units().unwrap();
    let lines = IMBUEMENT_FEE_INPUTS_MAX as u8;
    let crystals = u32::try_from(fee / 10_000 + 1).unwrap();
    let mut value = burn(
        (0..lines)
            .map(|n| {
                let mut line = coin_line(
                    if n + 1 == lines {
                        Coin::Crystal
                    } else {
                        Coin::Gold
                    },
                    100 + n,
                    100 - u64::from(n),
                    if n + 1 == lines { crystals } else { 1 },
                    0,
                );
                wide(&mut line.before);
                wide(&mut line.after);
                line
            })
            .collect(),
        fee,
    );
    let burned = 10_000 * u64::from(crystals) + u64::from(lines) - 1;
    let change = burned - fee;
    value.burned_gold_units = burned;
    value.change_gold_units = change;
    value.change = [
        (Coin::Platinum, change / 100),
        (Coin::Gold, change % 100),
    ]
    .into_iter()
    .filter(|(_, count)| *count > 0)
    .enumerate()
    .map(|(n, (coin, count))| {
        let mut mint = minted(coin, 120 + n as u8, count as u32, 200 + n as u64);
        wide(&mut mint.after);
        mint
    })
    .collect();
    value.cause = Some(OneItemFeeBurnCauseV1 {
        cause: Some(FeeBurnCauseV1::Imbuement(Box::new(imbuement))),
    });
    value.committed_character_revision = 1;
    value
}

fn clear() -> OneItemImbuementV1 {
    OneItemImbuementV1 {
        action: IMBUEMENT_ACTION_CLEAR,
        tier: 1,
        materials: Vec::new(),
        ..apply()
    }
}

#[test]
fn imbuement_audit_maximum_coin_inputs_materials_and_change_fit() {
    for cause in [
        OneItemImbuementV1 {
            tier: 1,
            ..apply()
        },
        clear(),
    ] {
        let value = fee(cause);
        assert_eq!(value.change.len(), 2);
        assert_eq!(check_fee_burn(&value), Ok(()));
        let envelope =
            encode_fee_burn_event_with(identity(), value.clone(), Type2EventTuple::V1).unwrap();
        assert!(envelope.len() <= FEE_RL07_ENVELOPE_BYTES_MAX);
        assert_eq!(decode_fee_burn_envelope(&envelope).unwrap().1, value);
        let usage = fee_burn_usage(&value);
        assert!(usage.touched_item_instances <= FEE_RL01_TOUCHED_ITEM_INSTANCES_MAX);
        assert!(usage.participants <= FEE_RL06_PARTICIPANTS_MAX);
        assert!(usage.effect_work_units <= FEE_RL06_EFFECT_WORK_UNITS_MAX);
        let mut too_many = value;
        too_many.lines.insert(0, coin_line(Coin::Gold, 125, 120, 1, 0));
        too_many.burned_gold_units += 1;
        too_many.change_gold_units += 1;
        too_many.change[1].after.as_mut().unwrap().quantity += 1;
        assert!(check_fee_burn(&too_many).is_err());
    }
}

#[test]
fn imbuement_audit_counts_the_item_and_each_material() {
    let usage = imbuement_cause::usage(&apply());
    // Item plus three materials, the first burned whole.
    assert_eq!(usage.touched_item_instances, 4);
    assert_eq!(usage.participants, 4);
    assert_eq!(usage.location_custody_lines, 1);
    assert_eq!(usage.effect_work_units, 2 + 3 + 2 * 2);
    let usage = imbuement_cause::usage(&clear());
    assert_eq!(usage.touched_item_instances, 1);
    assert_eq!(usage.effect_work_units, 2);
}

#[test]
fn imbuement_audit_prices_by_action_and_tier() {
    for (tier, gold) in [(1, 5_000), (2, 30_000), (3, 200_000)] {
        assert_eq!(
            OneItemImbuementV1 { tier, ..apply() }.fee_gold_units(),
            Some(gold)
        );
    }
    assert_eq!(clear().fee_gold_units(), Some(15_000));
    assert_eq!(OneItemImbuementV1 { tier: 4, ..apply() }.fee_gold_units(), None);
    assert_eq!(OneItemImbuementV1 { action: 3, ..apply() }.fee_gold_units(), None);
    // A fee that differs from the cause's price is refused.
    let mut value = fee(clear());
    value.fee_gold_units = 5_000;
    value.change_gold_units = value.burned_gold_units - 5_000;
    assert!(check_fee_burn(&value).is_err());
}

#[test]
fn imbuement_audit_refuses_each_malformed_cause() {
    assert_eq!(imbuement_cause::check(&apply()), Ok(()));
    assert_eq!(imbuement_cause::check(&clear()), Ok(()));
    let broken: [fn(&mut OneItemImbuementV1); 14] = [
        |v| v.occurrence = vec![0; 16],
        |v| v.action = 0,
        |v| v.item = None,
        |v| v.item.as_mut().unwrap().quantity = 2,
        |v| v.placement_ordinal = 0,
        |v| v.slot = 3,
        |v| v.category = 20,
        |v| v.tier = 0,
        |v| v.imbuement_key = String::new(),
        |v| v.materials.clear(),
        |v| {
            let extra = v.materials[0].clone();
            v.materials.push(extra);
        },
        // A material that is the imbued item itself.
        |v| {
            let item = v.item.clone();
            v.materials[0].before = item.clone();
            v.materials[0].after = item.map(|mut state| {
                state.quantity = 0;
                state.lifecycle = ITEM_LIFECYCLE_RETIRED;
                state
            });
        },
        // A material whose count does not fall.
        |v| v.materials[1].after = v.materials[1].before.clone(),
        |v| v.materials[2].placement_ordinal = v.placement_ordinal,
    ];
    for (index, breaks) in broken.iter().enumerate() {
        let mut value = apply();
        breaks(&mut value);
        assert_eq!(
            imbuement_cause::check(&value),
            Err(AuditError::InvalidInput),
            "case {index}"
        );
    }
    let mut value = clear();
    value.materials = apply().materials;
    assert_eq!(imbuement_cause::check(&value), Err(AuditError::InvalidInput));
}

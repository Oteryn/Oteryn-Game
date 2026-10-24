use super::*;
use crate::durability::item_timed_state_audit::OneItemTimedRowV1;

fn repair() -> OneItemTimedRepairV1 {
    let state = |key: &str| OneItemStateV1 {
        item_instance_id: uuid(80),
        world_id: uuid(1),
        definition: Some(OneItemTypedDefinitionRevisionV1 {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "r".repeat(512),
        }),
        quantity: 1,
        lifecycle: 1,
        key_number: 0,
    };
    OneItemTimedRepairV1 {
        occurrence: uuid(70),
        before: Some(state("oteryn:item.tibia.i6530")),
        after: Some(state("oteryn:item.tibia.i6529")),
        row_before: Some(OneItemTimedRowV1 {
            charges: None,
            remaining_ms: None,
            revision: u64::MAX - 1,
        }),
        row_after: Some(OneItemTimedRowV1 {
            charges: None,
            remaining_ms: Some(14_400_000),
            revision: u64::MAX,
        }),
        placement_ordinal: u64::MAX,
    }
}
fn fee(bank: bool) -> OneItemFeeBurnV1 {
    let mut value = burn(
        (0..19)
            .map(|n| {
                let mut line = coin_line(
                    if bank { Coin::Gold } else { Coin::Platinum },
                    100 + n,
                    100 - u64::from(n),
                    if bank {
                        100
                    } else if n == 18 {
                        10
                    } else {
                        5
                    },
                    0,
                );
                for state in [&mut line.before, &mut line.after] {
                    state
                        .as_mut()
                        .unwrap()
                        .definition
                        .as_mut()
                        .unwrap()
                        .revision_ref = "r".repeat(512);
                }
                line
            })
            .collect(),
        10_000,
    );
    value.cause = Some(OneItemFeeBurnCauseV1 {
        cause: Some(FeeBurnCauseV1::NpcRepair(Box::new(repair()))),
    });
    if bank {
        value.burned_gold_units = 1900;
        value.bank_debit = Some(debit(8100, 8100));
    }
    value.committed_character_revision = 1;
    value
}
#[test]
fn timed_repair_audit_maximum_nineteen_coin_inputs_and_bank_fit() {
    for bank in [false, true] {
        let value = fee(bank);
        assert_eq!(check_fee_burn(&value), Ok(()));
        let envelope = encode_fee_burn_event_with(
            identity(),
            value.clone(),
            if bank {
                Type2EventTuple::V2
            } else {
                Type2EventTuple::V1
            },
        )
        .unwrap();
        assert!(envelope.len() <= FEE_RL07_ENVELOPE_BYTES_MAX);
        assert_eq!(decode_fee_burn_envelope(&envelope).unwrap().1, value);
        let usage = fee_burn_usage(&value);
        assert_eq!(usage.transform_lines, 1);
        assert!(usage.touched_item_instances <= 22);
        assert!(usage.effect_work_units <= 68);
        let mut too_many = value;
        too_many.lines.push(coin_line(Coin::Gold, 125, 50, 1, 0));
        assert!(check_fee_burn(&too_many).is_err());
    }
    let mut usage = fee_burn_usage(&fee(false));
    usage.participants = 22;
    usage.effect_work_units = 68;
    usage.touched_item_instances = 22;
    assert_eq!(repair_cause::check_usage(&usage), Ok(()));
    for field in 0..3 {
        let mut over = usage;
        match field {
            0 => over.participants += 1,
            1 => over.effect_work_units += 1,
            _ => over.touched_item_instances += 1,
        };
        assert_eq!(
            repair_cause::check_usage(&over),
            Err(AuditError::CapacityExceeded)
        );
    }
}

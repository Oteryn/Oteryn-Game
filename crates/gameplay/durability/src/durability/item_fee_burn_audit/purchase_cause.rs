use super::*;
use crate::durability::item_mint_audit::check_content_key;

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemNpcPurchaseV1 {
    #[prost(string, tag = "1")]
    pub service_key: String,
    #[prost(bytes = "vec", tag = "2")]
    pub occurrence: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub atelier_proof: Vec<u8>,
}

pub(super) fn check(value: &OneItemFeeBurnV1) -> Result<(), AuditError> {
    if let Some(
        FeeBurnCauseV1::PromotionPurchase(cause)
        | FeeBurnCauseV1::BlessingPurchase(cause)
        | FeeBurnCauseV1::NpcTravel(cause)
        | FeeBurnCauseV1::GemSwitchDomain(cause)
        | FeeBurnCauseV1::PreyListReroll(cause)
        | FeeBurnCauseV1::TaskBoardTalisman(cause),
    ) = value.cause.as_ref().and_then(|v| v.cause.as_ref())
        && !cause.atelier_proof.is_empty()
    {
        return Err(AuditError::InvalidInput);
    }
    match value.cause.as_ref().and_then(|value| value.cause.as_ref()) {
        Some(FeeBurnCauseV1::TaskBoardTalisman(cause))
            if cause.service_key == "oteryn:taskboard.talisman"
                && value.fee_gold_units == oteryn_gameplay_quests::quest::task_board::full::TALISMAN_PURCHASE_GOLD =>
        {
            check_uuid_v7(&cause.occurrence)
        }
        Some(FeeBurnCauseV1::Forge(cause)) => {
            let proof=crate::durability::character_forge_dust::live::audit::decode(&cause.proof)?;
            if !(1..=4).contains(&proof.operation) || proof.character!=value.character_id || proof.world!=value.world_id || proof.gold_spent!=value.fee_gold_units {
                return Err(AuditError::InvalidInput);
            }
            Ok(())
        }
        Some(FeeBurnCauseV1::PreyListReroll(cause)) if ["oteryn:prey.list_reroll.1","oteryn:prey.list_reroll.2","oteryn:prey.list_reroll.3"].contains(&cause.service_key.as_str()) && value.fee_gold_units>0 && value.fee_gold_units.is_multiple_of(200) => check_uuid_v7(&cause.occurrence),
        Some(FeeBurnCauseV1::MarketFee(cause))
            if (20..=1_000_000).contains(&value.fee_gold_units)
                && value.lines.is_empty()
                && value.change.is_empty()
                && value.bank_debit.is_some() =>
        {
            check_uuid_v7(&cause.offer)?;
            check_uuid_v7(&cause.occurrence)
        }
        Some(FeeBurnCauseV1::AreaDonation(cause))
            if cause.area == 1
                && cause.world == value.world_id
                && (1..=10_000_000).contains(&value.fee_gold_units) =>
        {
            check_uuid_v7(&cause.world)?;
            check_uuid_v7(&cause.occurrence)
        }
        Some(FeeBurnCauseV1::NpcRepair(repair))
            if value.fee_gold_units == 10_000
                && value.lines.len() <= 19
                && repair.before.as_ref().is_some_and(|item| {
                    item.world_id == value.world_id
                        && value.lines.iter().all(|line| {
                            line.before
                                .as_ref()
                                .is_none_or(|coin| coin.item_instance_id != item.item_instance_id)
                        })
                        && value.change.iter().all(|line| {
                            line.after
                                .as_ref()
                                .is_none_or(|coin| coin.item_instance_id != item.item_instance_id)
                        })
                }) =>
        {
            super::repair_cause::check(repair)
        }
        Some(FeeBurnCauseV1::Imbuement(imbuement))
            // Immutable V1 events retain their original tier price. New commands are
            // checked against the current fee by item_imbuement and migration 0298.
            if (imbuement.fee_gold_units() == Some(value.fee_gold_units)
                || matches!(
                    (imbuement.action, imbuement.tier, value.fee_gold_units),
                    (super::imbuement_cause::IMBUEMENT_ACTION_APPLY, 1, 5_000)
                        | (super::imbuement_cause::IMBUEMENT_ACTION_APPLY, 2, 30_000)
                        | (super::imbuement_cause::IMBUEMENT_ACTION_APPLY, 3, 200_000)
                ))
                && value.lines.len() <= super::imbuement_cause::IMBUEMENT_FEE_INPUTS_MAX
                && imbuement
                    .item
                    .as_ref()
                    .is_some_and(|item| item.world_id == value.world_id)
                && imbuement.item_ids().all(|id| {
                    value.lines.iter().all(|line| {
                        line.before
                            .as_ref()
                            .is_none_or(|coin| coin.item_instance_id != id)
                    }) && value.change.iter().all(|line| {
                        line.after
                            .as_ref()
                            .is_none_or(|coin| coin.item_instance_id != id)
                    })
                }) =>
        {
            super::imbuement_cause::check(imbuement)
        }
        Some(FeeBurnCauseV1::CharmUnassign(cause)) => {
            CharmKey::new(cause.charm_key.clone()).map_err(|_| AuditError::InvalidInput)?;
            check_uuid_v7(&cause.occurrence)
        }
        Some(FeeBurnCauseV1::PromotionPurchase(cause))
            if cause.service_key.starts_with("oteryn:service.promotion.")
                && value.bank_debit.is_none()
                && value.fee_gold_units
                    == oteryn_gameplay_progression::promotion::PROMOTION_PRICE =>
        {
            check_content_key(&cause.service_key)?;
            check_uuid_v7(&cause.occurrence)
        }
        Some(FeeBurnCauseV1::NpcTravel(cause))
            if cause.service_key.starts_with("oteryn:service.travel.")
                && value.fee_gold_units <= 1_000_000 =>
        {
            check_content_key(&cause.service_key)?;
            check_uuid_v7(&cause.occurrence)
        }
        Some(FeeBurnCauseV1::BlessingPurchase(cause))
            if cause.service_key.starts_with("oteryn:service.blessing.") =>
        {
            check_content_key(&cause.service_key)?;
            check_uuid_v7(&cause.occurrence)
        }
        Some(FeeBurnCauseV1::GemReveal(cause))
            if cause.service_key == "oteryn:atelier.reveal" && [125_000, 1_000_000, 6_000_000].contains(&value.fee_gold_units) => check_atelier(cause,value,"Reveal"),
        Some(FeeBurnCauseV1::GemSwitchDomain(cause))
            if cause.service_key == "oteryn:atelier.switch_domain" && [125_000, 250_000, 500_000].contains(&value.fee_gold_units) => check_uuid_v7(&cause.occurrence),
        Some(FeeBurnCauseV1::GemGradeUp(cause))
            if cause.service_key == "oteryn:atelier.grade_up" && [2_000_000, 5_000_000, 30_000_000, 12_000_000, 75_000_000].contains(&value.fee_gold_units) => check_atelier(cause,value,"GradeUp"),
        _ => Err(AuditError::InvalidInput),
    }
}

fn check_atelier(
    cause: &OneItemNpcPurchaseV1,
    value: &OneItemFeeBurnV1,
    action: &str,
) -> Result<(), AuditError> {
    check_uuid_v7(&cause.occurrence)?;
    let proof: serde_json::Value =
        serde_json::from_slice(&cause.atelier_proof).map_err(|_| AuditError::InvalidInput)?;
    if proof["occurrence"] != serde_json::json!(cause.occurrence)
        || proof["character"] != serde_json::json!(value.character_id)
        || proof["mutation"].get(action).is_none()
        || proof["outputs"].as_array().is_none_or(|v| !v.is_empty())
        || proof["inputs"]
            .as_array()
            .is_none_or(|v| v.is_empty() || v.len() > 20)
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

pub(super) fn atelier_usage(cause: &OneItemNpcPurchaseV1) -> TransactionResourceUsage {
    let count = serde_json::from_slice::<serde_json::Value>(&cause.atelier_proof)
        .ok()
        .and_then(|p| p["inputs"].as_array().map(|a| a.len() as u64))
        .unwrap_or(0);
    TransactionResourceUsage {
        touched_item_instances: count,
        location_custody_lines: count,
        value_lines: 0,
        transform_lines: 0,
        container_expansion: 0,
        participants: count,
        effect_work_units: 3 * count,
        events: 0,
    }
}

#[cfg(test)]
mod task_board_tests {
    use super::*;

    #[test]
    fn task_board_fee_accepts_only_the_donor_talisman_price_and_closed_cause() {
        let mut id = vec![1; 16];
        id[6] = 0x71;
        id[8] = 0x81;
        let cause = OneItemNpcPurchaseV1 {
            service_key: "oteryn:taskboard.talisman".into(),
            occurrence: id,
            atelier_proof: Vec::new(),
        };
        let value = |cause, price| OneItemFeeBurnV1 {
            cause: Some(OneItemFeeBurnCauseV1 {
                cause: Some(FeeBurnCauseV1::TaskBoardTalisman(cause)),
            }),
            fee_gold_units: price,
            ..Default::default()
        };
        assert_eq!(check(&value(cause.clone(), 5_000)), Ok(()));
        for price in [0, 4_999, 5_001, 50_000] {
            assert_eq!(
                check(&value(cause.clone(), price)),
                Err(AuditError::InvalidInput)
            );
        }
        for broken in [
            OneItemNpcPurchaseV1 {
                service_key: "oteryn:taskboard.expansion".into(),
                ..cause.clone()
            },
            OneItemNpcPurchaseV1 {
                occurrence: vec![0; 16],
                ..cause.clone()
            },
            OneItemNpcPurchaseV1 {
                atelier_proof: vec![1],
                ..cause
            },
        ] {
            assert_eq!(check(&value(broken, 5_000)), Err(AuditError::InvalidInput));
        }
    }
}

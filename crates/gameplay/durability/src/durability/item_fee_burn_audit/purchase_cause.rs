use super::*;
use crate::durability::item_mint_audit::check_content_key;

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemNpcPurchaseV1 {
    #[prost(string, tag = "1")]
    pub service_key: String,
    #[prost(bytes = "vec", tag = "2")]
    pub occurrence: Vec<u8>,
}

pub(super) fn check(value: &OneItemFeeBurnV1) -> Result<(), AuditError> {
    match value.cause.as_ref().and_then(|value| value.cause.as_ref()) {
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
            if imbuement.fee_gold_units() == Some(value.fee_gold_units)
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
        Some(FeeBurnCauseV1::BlessingPurchase(cause))
            if cause.service_key.starts_with("oteryn:service.blessing.") =>
        {
            check_content_key(&cause.service_key)?;
            check_uuid_v7(&cause.occurrence)
        }
        _ => Err(AuditError::InvalidInput),
    }
}

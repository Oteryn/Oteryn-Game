//! Closed fee-source bindings. The Charm bytes remain unchanged.
use super::*;
use crate::durability::item_fee_burn_audit::OneItemNpcPurchaseV1;

impl FeeBurnCause {
    pub(super) fn kind(&self) -> i16 {
        match self {
            Self::QuestAccessPurchase { .. } => 13,
            Self::PreyListReroll { .. } => 12,
            Self::GemReveal { .. } => 9,
            Self::GemSwitchDomain { .. } => 10,
            Self::GemGradeUp { .. } => 11,
            Self::MarketFee { .. } => 6,
            Self::NpcTravel { .. } => 8,
            Self::AreaDonation { .. } => 4,
            Self::CharmUnassign { .. } => CAUSE_CHARM_UNASSIGN,
            Self::NpcRepair { .. } => 5,
            Self::Imbuement { .. } => 7,
            Self::PromotionPurchase { .. } => 2,
            Self::BlessingPurchase { .. } => 3,
        }
    }
    pub(super) fn occurrence(&self) -> &[u8; 16] {
        match self {
            Self::QuestAccessPurchase { occurrence }
            | Self::PreyListReroll { occurrence, .. }
            | Self::GemReveal { occurrence, .. }
            | Self::GemSwitchDomain { occurrence }
            | Self::GemGradeUp { occurrence, .. } => occurrence,
            Self::MarketFee { occurrence, .. } | Self::AreaDonation { occurrence, .. } => {
                occurrence
            }
            Self::CharmUnassign { occurrence, .. } => occurrence.as_bytes(),
            Self::NpcRepair { occurrence, .. } | Self::Imbuement { occurrence, .. } => occurrence,
            Self::NpcTravel { occurrence, .. }
            | Self::PromotionPurchase { occurrence, .. }
            | Self::BlessingPurchase { occurrence, .. } => occurrence,
        }
    }
    pub(super) fn key(&self) -> &str {
        match self {
            Self::QuestAccessPurchase { .. } => "oteryn:service.quest.rotten-blood.access",
            Self::PreyListReroll { slot, .. } => match slot {
                1 => "oteryn:prey.list_reroll.1",
                2 => "oteryn:prey.list_reroll.2",
                _ => "oteryn:prey.list_reroll.3",
            },
            Self::GemReveal { .. } => "oteryn:atelier.reveal",
            Self::GemSwitchDomain { .. } => "oteryn:atelier.switch_domain",
            Self::GemGradeUp { .. } => "oteryn:atelier.grade_up",
            Self::MarketFee { .. } => "oteryn:market.offer",
            Self::AreaDonation { .. } => "oteryn:area.entry_environs",
            Self::CharmUnassign { charm, .. } => charm.as_str(),
            Self::NpcRepair { .. } => "oteryn:service.repair.aldo",
            Self::Imbuement { .. } => "oteryn:service.imbuement",
            Self::NpcTravel { service, .. }
            | Self::PromotionPurchase { service, .. }
            | Self::BlessingPurchase { service, .. } => service,
        }
    }
    pub(super) fn charm_key(&self) -> Option<&str> {
        matches!(self, Self::CharmUnassign { .. }).then(|| self.key())
    }
    pub(super) fn purchase_key(&self) -> Option<&str> {
        (!matches!(self, Self::CharmUnassign { .. })).then(|| self.key())
    }
    pub(super) fn validate(&self) -> Result<()> {
        check_uuid_v7(self.occurrence()).map_err(|_| FeeBurnError::InvalidInput)?;
        match self {
            Self::QuestAccessPurchase { .. } => Ok(()),
            Self::PreyListReroll { slot: 1..=3, .. } => Ok(()),
            Self::GemReveal { .. } | Self::GemSwitchDomain { .. } | Self::GemGradeUp { .. } => {
                Ok(())
            }
            Self::MarketFee { offer, .. } => {
                check_uuid_v7(offer).map_err(|_| FeeBurnError::InvalidInput)
            }
            Self::CharmUnassign { .. } => Ok(()),
            Self::NpcTravel { service, .. } if service.starts_with("oteryn:service.travel.") => {
                Ok(())
            }
            Self::AreaDonation { area: 1, .. } => Ok(()),
            Self::NpcRepair { occurrence, repair }
                if repair.occurrence.as_slice() == occurrence =>
            {
                super::super::item_fee_burn_audit::repair_cause::check(repair)
                    .map_err(|_| FeeBurnError::InvalidInput)
            }
            Self::Imbuement {
                occurrence,
                imbuement,
            } if imbuement.occurrence.as_slice() == occurrence => {
                super::super::item_fee_burn_audit::imbuement_cause::check(imbuement)
                    .map_err(|_| FeeBurnError::InvalidInput)
            }
            Self::PromotionPurchase { service, .. }
                if service.starts_with("oteryn:service.promotion.") =>
            {
                Ok(())
            }
            Self::BlessingPurchase { service, .. }
                if service.starts_with("oteryn:service.blessing.") =>
            {
                Ok(())
            }
            _ => Err(FeeBurnError::InvalidInput),
        }
    }
    pub(in super::super) fn audit(&self) -> FeeBurnCauseV1 {
        match self {
            Self::QuestAccessPurchase { occurrence } => FeeBurnCauseV1::QuestAccessPurchase(OneItemNpcPurchaseV1 {
                service_key:self.key().into(),occurrence:occurrence.to_vec(),atelier_proof:Vec::new(),
            }),
            Self::PreyListReroll { occurrence, .. } => {
                FeeBurnCauseV1::PreyListReroll(OneItemNpcPurchaseV1 {
                    service_key: self.key().into(),
                    occurrence: occurrence.to_vec(),
                    atelier_proof: Vec::new(),
                })
            }
            Self::GemReveal { occurrence, proof } => {
                FeeBurnCauseV1::GemReveal(OneItemNpcPurchaseV1 {
                    atelier_proof: proof.clone(),
                    service_key: self.key().into(),
                    occurrence: occurrence.to_vec(),
                })
            }
            Self::GemSwitchDomain { occurrence } => {
                FeeBurnCauseV1::GemSwitchDomain(OneItemNpcPurchaseV1 {
                    atelier_proof: Vec::new(),
                    service_key: self.key().into(),
                    occurrence: occurrence.to_vec(),
                })
            }
            Self::GemGradeUp { occurrence, proof } => {
                FeeBurnCauseV1::GemGradeUp(OneItemNpcPurchaseV1 {
                    atelier_proof: proof.clone(),
                    service_key: self.key().into(),
                    occurrence: occurrence.to_vec(),
                })
            }
            Self::MarketFee { offer, occurrence } => {
                FeeBurnCauseV1::MarketFee(super::super::item_fee_burn_audit::OneItemMarketFeeV1 {
                    offer: offer.to_vec(),
                    occurrence: occurrence.to_vec(),
                })
            }
            Self::AreaDonation {
                world,
                area,
                occurrence,
            } => FeeBurnCauseV1::AreaDonation(
                super::super::item_fee_burn_audit::OneItemAreaDonationV1 {
                    world: world.as_bytes().to_vec(),
                    area: *area,
                    occurrence: occurrence.to_vec(),
                },
            ),
            Self::NpcRepair { repair, .. } => FeeBurnCauseV1::NpcRepair(repair.clone()),
            Self::Imbuement { imbuement, .. } => FeeBurnCauseV1::Imbuement(imbuement.clone()),
            Self::CharmUnassign { charm, occurrence } => {
                FeeBurnCauseV1::CharmUnassign(OneItemCharmUnassignV1 {
                    charm_key: charm.as_str().into(),
                    occurrence: occurrence.as_bytes().to_vec(),
                })
            }
            Self::NpcTravel {
                service,
                occurrence,
            } => FeeBurnCauseV1::NpcTravel(OneItemNpcPurchaseV1 {
                atelier_proof: Vec::new(),
                service_key: service.clone(),
                occurrence: occurrence.to_vec(),
            }),
            Self::PromotionPurchase {
                service,
                occurrence,
            } => FeeBurnCauseV1::PromotionPurchase(OneItemNpcPurchaseV1 {
                atelier_proof: Vec::new(),
                service_key: service.clone(),
                occurrence: occurrence.to_vec(),
            }),
            Self::BlessingPurchase {
                service,
                occurrence,
            } => FeeBurnCauseV1::BlessingPurchase(OneItemNpcPurchaseV1 {
                atelier_proof: Vec::new(),
                service_key: service.clone(),
                occurrence: occurrence.to_vec(),
            }),
        }
    }
}

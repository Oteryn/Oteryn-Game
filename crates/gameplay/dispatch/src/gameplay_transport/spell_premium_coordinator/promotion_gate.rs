//! Promotion's pre-delivery exception is selected inside the native gate's borrowed status.
use super::*;
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::npc_purchase::{NpcPurchaseGate, PurchaseService};
use crate::durability::premium_activation_latch::with_purchase_latch;

impl NpcPurchaseGate for SpellPremiumCoordinator {
    async fn authorize(
        &self,
        tx: &mut Type2Transaction<'_>,
        account: [u8; 16],
        service: &PurchaseService,
    ) -> Result<bool, DurabilityError> {
        let decide = |status: PremiumStatus<'_>| match service {
            PurchaseService::Promotion(_) => {
                oteryn_gameplay_progression::promotion::purchase_permitted(&status)
            }
            PurchaseService::Blessing(_) => status.gate().premium_current(),
        };
        let now = self.read_now();
        if self.delivery.held.get().is_some() {
            return Ok(decide(self.delivered_status(account, now)));
        }
        let before = match (&self.delivery.activation, now) {
            (None, _) => true,
            (Some(activation), Some(now)) => now.upper_us() < activation.switch_over_us,
            _ => false,
        };
        if !before {
            let Some(root) = &self.latch else {
                return Ok(false);
            };
            let empty = match now {
                Some(now) => EmptyLatch::Latch { now },
                None => EmptyLatch::NotCurrent,
            };
            let delivered = self
                .after_read(&empty, root.read_premium_activation::<()>().await)
                .await;
            return Ok(decide(if delivered {
                self.delivered_status(account, now)
            } else {
                PremiumStatus::NotCurrent
            }));
        }
        if self.latch.is_none() {
            return Ok(false);
        }
        with_purchase_latch(tx, |latch, held| {
            if let Some(latch) = latch {
                self.delivery.hold(latch);
                decide(self.delivered_status(account, now))
            } else if let Some(held) = held {
                decide(PremiumStatus::NotActivated(PreDelivery::inside_gate(held)))
            } else {
                false
            }
        })
        .await
    }
}

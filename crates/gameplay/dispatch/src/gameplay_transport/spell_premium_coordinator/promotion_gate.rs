//! Promotion and Wheel share the native gate's transaction-scoped Premium decision.
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
        self.transaction_permission(tx, account, |status| match service {
            PurchaseService::Promotion(_) => {
                oteryn_gameplay_progression::promotion::purchase_permitted(&status)
            }
            PurchaseService::Blessing(_) => status.gate().premium_current(),
        })
        .await
    }
}

impl crate::durability::character_wheel::WheelPremiumGate for SpellPremiumCoordinator {
    async fn allocation_permitted(
        &self,
        tx: &mut PremiumGateTx<'_>,
        account: [u8; 16],
    ) -> Result<bool, DurabilityError> {
        self.transaction_permission(tx, account, |status| {
            oteryn_gameplay_progression::promotion::purchase_permitted(&status)
        })
        .await
    }
}

impl SpellPremiumCoordinator {
    async fn transaction_permission(
        &self,
        tx: &mut PremiumGateTx<'_>,
        account: [u8; 16],
        decide: impl FnOnce(PremiumStatus<'_>) -> bool + Send,
    ) -> Result<bool, DurabilityError> {
        let now = self.read_now();
        if self.delivery.held.get().is_some() {
            return Ok(decide(self.delivered_status(account, now)));
        }
        let before = match (&self.delivery.activation, now) {
            (None, _) => true,
            (Some(activation), Some(now)) => now.upper_us() < activation.switch_over_us,
            _ => false,
        };
        // Delivery is settled by the outer Premium gate. A durable participant reads
        // the latch in its own transaction: borrowing another root holder here can fail
        // while this transaction holds the only ready holder. An empty latch after the
        // configured start is NotCurrent; never upgrade a shared latch lock to activate.
        if self.latch.is_none() {
            return Ok(false);
        }
        with_purchase_latch(tx, |latch, held| {
            if let Some(latch) = latch {
                self.delivery.hold(latch);
                decide(self.delivered_status(account, now))
            } else if let Some(held) = held.filter(|_| before) {
                decide(PremiumStatus::NotActivated(PreDelivery::inside_gate(held)))
            } else {
                false
            }
        })
        .await
    }
}

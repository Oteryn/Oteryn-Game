//! Focused M03 PostgreSQL target reuses the authoritative server admission case.
#![cfg(feature = "test-support")]
use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
use oteryn_gameplay_domain::domain;
use oteryn_gameplay_durability::durability;
use oteryn_gameplay_entities as foundation;

#[allow(
    dead_code,
    reason = "shared native fixture; this target uses only world admission"
)]
#[path = "../../../../apps/game-server/tests/support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
#[path = "../../../../apps/game-server/tests/support/qworld_live_admission_cases.rs"]
mod world_admission;

#[path = "../../../../apps/game-server/tests/support/authority_cases/character_promotion_fixture/participant.rs"]
mod promotion_participant;
mod character_promotion_fixture {
    use super::promotion_participant as participant;
    struct BeforeDelivery;
    impl crate::durability::npc_purchase::NpcPurchaseGate for BeforeDelivery {
        async fn authorize(
            &self,
            tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
            _: [u8; 16],
            _: &crate::durability::npc_purchase::PurchaseService,
        ) -> Result<bool, crate::durability::DurabilityError> {
            crate::durability::premium_activation_latch::with_purchase_latch(tx, |latch, held| {
                latch.is_none() && held.is_some()
            })
            .await
        }
    }
    pub(crate) async fn purchase(
        h: &crate::bestiary_postgres_harness::Harness,
        revision: u64,
    ) -> crate::bestiary_postgres_harness::TestResult {
        participant::purchase_with_gate(h, revision, &BeforeDelivery).await
    }
}

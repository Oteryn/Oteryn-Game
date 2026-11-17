//! Promotion fixtures use the same durable purchase participant in each test target.
#[path = "character_promotion_fixture/participant.rs"]
mod participant;
use crate::bestiary_postgres_harness::{Harness, TestResult};
// The standalone promotion target calls prepare directly; lifted targets use purchase.
#[allow(dead_code)]
pub(crate) async fn prepare(
    h: &crate::bestiary_postgres_harness::Harness,
) -> crate::bestiary_postgres_harness::TestResult<crate::durability::npc_purchase::NpcPurchaseRequest>
{
    participant::prepare(h).await
}
pub(crate) async fn purchase(h: &Harness, revision: u64) -> TestResult {
    let coordinator =
        crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
            Default::default(),
            None,
            None,
            Some(h.root.clone()),
        );
    participant::purchase_with_gate(h, revision, &coordinator).await
}

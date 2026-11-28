//! Premium is resolved before any House lock. Pre-delivery writes retain the
//! native latch transaction, with a savepoint so a refused pass cannot commit.
use super::*;
use crate::domain::premium::PremiumStatus;
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use crate::durability::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::durability::premium_activation_latch::PremiumGateOwner;
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use oteryn_gameplay_character_recovery_fence::CharacterRecoveryFenceV1;
use sqlx::Acquire;

impl DurabilityRoot {
    /// Places a bid and opens a qualified vacant auction in the same transaction.
    pub async fn house_bid<G: PremiumGateOwner>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        gate: &G,
        fence: CurrentCharacterItemFence,
        request: HouseBidRequest,
    ) -> Result<HouseOwnershipOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseOwnershipError::AuthorityRejected)?;
        let character = fence.character_id;
        let account=self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            let text: String=sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            super::super::character_progression::uuid_text(&text)
        })).await?;
        if request.kind == "guildhall" {
            return run(
                self,
                None,
                &recovery,
                node,
                (fence, request),
                false,
            )
            .await;
        }
        let root = self.clone();
        let node = node.clone();
        gate.with_premium_gate(account, move |status, tx| {
            Box::pin(async move {
                run(
                    &root, tx, &recovery, &node, (fence, request),
                    !matches!(status, PremiumStatus::NotCurrent),
                )
                .await
            })
        })
        .await?
    }

}

async fn run(
    root: &DurabilityRoot,
    gate_tx: Option<&mut Tx<'_>>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    bid: (CurrentCharacterItemFence, HouseBidRequest),
    allowed: bool,
) -> Result<HouseOwnershipOutcome> {
    if let Some(gate_tx) = gate_tx {
        let mut savepoint = gate_tx.begin().await?;
        let result = execute(&mut savepoint, recovery, node, bid, allowed).await?;
        savepoint.commit().await?;
        return Ok(result);
    }
    let recovery = recovery.clone();
    let node = node.clone();
    root.try_issue_semantic_pass()?
        .run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                let result = execute(&mut tx, &recovery, &node, bid, allowed).await;
                if result.is_ok() {
                    commit_semantic_transaction(tx, deadline).await?;
                }
                Ok(result)
            })
        })
        .await?
}

async fn execute(
    tx: &mut Tx<'_>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    bid: (CurrentCharacterItemFence, HouseBidRequest),
    allowed: bool,
) -> Result<HouseOwnershipOutcome> {
    assert_recovery_fence(tx, recovery).await?;
    lock_admission_relations(tx).await?;
    bid::bid_in_transaction(tx, node, &bid.0, &bid.1, allowed).await
}

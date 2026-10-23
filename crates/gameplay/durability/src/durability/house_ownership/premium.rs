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
use std::collections::BTreeSet;

enum Work {
    Bid(CurrentCharacterItemFence, HouseBidRequest),
    Settle(HouseSettlementRequest),
}

impl DurabilityRoot {
    /// HOUSE-1a backend pass. A VACANT anchor cannot open an auction; D1383's
    /// qualified catalogue and runtime acceptance remain separate activation gates.
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
        let root = self.clone();
        let node = node.clone();
        gate.with_premium_gate(account, move |status, tx| {
            Box::pin(async move {
                let allowed = if matches!(status, PremiumStatus::NotCurrent) {
                    BTreeSet::new()
                } else {
                    BTreeSet::from([account])
                };
                run(
                    &root,
                    tx,
                    &recovery,
                    &node,
                    Work::Bid(fence, request),
                    &[account],
                    &allowed,
                )
                .await
            })
        })
        .await?
    }

    /// One idempotent election/refund step. The accepted packet deliberately
    /// assigns loop scheduling and protocol dispatch to HOUSE-WIRE-1.
    pub async fn house_settlement<G: PremiumGateOwner>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        gate: &G,
        request: HouseSettlementRequest,
    ) -> Result<HouseOwnershipOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseOwnershipError::AuthorityRejected)?;
        let auction = request.auction;
        let accounts = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    settlement::candidates(&mut tx, &auction)
                        .await
                        .map_err(|e| match e {
                            HouseOwnershipError::Unavailable(e) => e,
                            _ => DurabilityError::InvalidStoredState,
                        })
                })
            })
            .await?;
        let captured = accounts.clone();
        let root = self.clone();
        let node = node.clone();
        gate.with_premium_batch_gate(&accounts, move |statuses, tx| {
            Box::pin(async move {
                if statuses.len() != captured.len() {
                    return Err(HouseOwnershipError::AuthorityRejected);
                }
                let allowed = captured
                    .iter()
                    .zip(statuses)
                    .filter_map(|(a, s)| (!matches!(s, PremiumStatus::NotCurrent)).then_some(*a))
                    .collect();
                run(
                    &root,
                    tx,
                    &recovery,
                    &node,
                    Work::Settle(request),
                    &captured,
                    &allowed,
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
    work: Work,
    accounts: &[[u8; 16]],
    allowed: &BTreeSet<[u8; 16]>,
) -> Result<HouseOwnershipOutcome> {
    if let Some(gate_tx) = gate_tx {
        let mut savepoint = gate_tx.begin().await?;
        let result = execute(&mut savepoint, recovery, node, work, accounts, allowed).await?;
        savepoint.commit().await?;
        return Ok(result);
    }
    let recovery = recovery.clone();
    let node = node.clone();
    let accounts = accounts.to_vec();
    let allowed = allowed.clone();
    root.try_issue_semantic_pass()?
        .run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                let result = execute(&mut tx, &recovery, &node, work, &accounts, &allowed).await;
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
    work: Work,
    accounts: &[[u8; 16]],
    allowed: &BTreeSet<[u8; 16]>,
) -> Result<HouseOwnershipOutcome> {
    assert_recovery_fence(tx, recovery).await?;
    lock_admission_relations(tx).await?;
    match work {
        // The opaque bypass witness never leaves the Premium gate: its shared
        // transaction is retained by run(). Eligibility uses the resolved account.
        Work::Bid(fence, request) => {
            bid::bid_in_transaction(tx, node, &fence, &request, !allowed.is_empty()).await
        }
        Work::Settle(request) => {
            settlement::settle_in_transaction(tx, node, &request, accounts, allowed).await
        }
    }
}

//! One checkout for the chest's independently committed DUR-03 phases.

use super::*;
use crate::durability::db::IssuedSemanticPass;
use crate::durability::item_mint_audit::Type2Transaction;
use crate::durability::item_transfer::ItemWeightLookup;
use oteryn_error_codes::{FieldValue, Level, Line};
use std::time::Duration;
use tokio::time::Instant;

const MINT_HOLDER_WINDOW: Duration = Duration::from_secs(1);
const RECEIPT_HOLDER_WINDOW: Duration = Duration::from_secs(2);

struct ChestMintPass<'a> {
    admission: ClaimMintAdmission<'a>,
    // Filled only after the reservation and charge have both committed.
    charged_transaction: Option<[u8; 16]>,
}

impl DurabilityRoot {
    /// Settle one native reward chest. A checkout refusal is retryable before
    /// BEGIN; after issue, this executes each phase once on the same holder.
    /// An unacknowledged mint can only be recovered by reading its receipt.
    pub async fn mint_bundle_chest_reward(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: RewardClaimMintRequest,
        catalogue_len: usize,
        weights: &dyn ItemWeightLookup,
    ) -> Result<(RewardClaimMintOutcome, GrantNotice)> {
        if !contents::weight_pin_matches(&request, weights) {
            return Err(RewardClaimMintError::InvalidInput);
        }
        let binding = validate_claim_intent(&request, fence, Some(weights))?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| RewardClaimMintError::AuthorityRejected)?;
        let command = request.command;
        let mut context = ChestMintPass {
            admission: ClaimMintAdmission {
                recovery: recovery.clone(),
                node: node.clone(),
                fence,
                weights: Some(weights),
                catalogue_len: Some(catalogue_len),
            },
            charged_transaction: None,
        };
        let result = self
            .chest_reward_holder("mint", MINT_HOLDER_WINDOW)
            .await?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let admission = &context.admission;
                    let candidate =
                        match reserve_reward_claim(holder, deadline, admission, binding, request)
                            .await?
                        {
                            Ok(candidate) => candidate,
                            Err(error) => return Ok(Err(error)),
                        };
                    let transaction = candidate.transaction_id;
                    if let Err(error) = charge_reward_claim(
                        holder,
                        deadline,
                        admission.recovery.clone(),
                        command,
                        transaction,
                    )
                    .await?
                    {
                        return Ok(Err(error));
                    }
                    context.charged_transaction = Some(transaction);
                    commit_reward_claim(holder, deadline, admission, FrozenClaim::of(&candidate))
                        .await
                })
            })
            .await;

        let error = match result {
            Ok(outcome) => return outcome,
            Err(error) => error,
        };
        if let Some(transaction) = context.charged_transaction
            && matches!(
                error,
                DurabilityError::CommitOutcomeUnknown
                    | DurabilityError::RootPassDeadlineExceeded
                    | DurabilityError::InvalidStoredState
            )
            && let Ok(holder) = self
                .chest_reward_holder("receipt", RECEIPT_HOLDER_WINDOW)
                .await
            && let Ok(Ok(Some(receipt))) = holder
                .run(move |holder, deadline| {
                    Box::pin(async move {
                        if let Err(error) =
                            charge_reward_claim(holder, deadline, recovery.clone(), command, transaction)
                                .await?
                        {
                            return Ok(Err(error));
                        }
                        reconcile_reward_claim(holder, deadline, recovery, command, binding).await
                    })
                })
                .await
        {
            return Ok((
                RewardClaimMintOutcome::AlreadyCommitted(receipt),
                GrantNotice::Unknown,
            ));
        }
        Err(error.into())
    }

    async fn chest_reward_holder(
        &self,
        purpose: &'static str,
        window: Duration,
    ) -> Result<IssuedSemanticPass> {
        let started = Instant::now();
        let deadline = started + window;
        let mut observed = None;
        let busy = |error: &RewardClaimMintError| {
            matches!(
                error,
                RewardClaimMintError::Refused(RewardClaimRefusal::HolderBusyBeforeIssue)
            )
        };
        let result = self
            .retry_foreground_holder(deadline, busy, async || {
                match self.try_issue_semantic_pass() {
                    Ok(pass) => Ok(pass),
                    // The retry runs ONLY checkout. No transaction exists yet.
                    Err(DurabilityError::RootUnavailable) => {
                        observed.get_or_insert_with(|| self.checked_out_semantic_pass());
                        if self.checked_out_semantic_pass().is_none() {
                            // A lost COMMIT may have retired its holder. Only the
                            // root maintenance path may replace it.
                            let _ = tokio::time::timeout_at(deadline, self.await_ready()).await;
                        }
                        Err(RewardClaimMintError::Refused(
                            RewardClaimRefusal::HolderBusyBeforeIssue,
                        ))
                    }
                    Err(error) => Err(error.into()),
                }
            })
            .await;
        if let Some(owner) = observed {
            let (owner, held) = owner.map_or(("none".to_owned(), Duration::ZERO), |(site, held)| {
                (site.to_string(), held)
            });
            let millis = |time: Duration| u64::try_from(time.as_millis()).unwrap_or(u64::MAX);
            let line = Line::new(Level::Info, "durability", "reward_chest_holder_wait")
                .field("purpose", FieldValue::Token(purpose))
                .field("owner", FieldValue::Owned(owner))
                .field("held_ms", FieldValue::U64(millis(held)))
                .field("waited_ms", FieldValue::U64(millis(started.elapsed())))
                .field(
                    "outcome",
                    FieldValue::Token(if result.is_ok() { "acquired" } else { "unavailable" }),
                );
            oteryn_error_codes::emit("oteryn-game-server", &line);
        }
        result
    }
}

pub(super) async fn commit_reward_transaction(
    tx: Type2Transaction<'_>,
    deadline: Instant,
) -> std::result::Result<(), DurabilityError> {
    #[cfg(any(test, feature = "test-support"))]
    if STALL_REWARD_COMMIT.try_with(|stage| stage.replace(StalledRewardCommit::Started))
        == Ok(StalledRewardCommit::Armed)
    {
        let (result, stalled) =
            crate::durability::with_stalled_commit_response(tx.commit(deadline)).await;
        if stalled {
            STALL_REWARD_COMMIT.with(|stage| stage.set(StalledRewardCommit::Observed));
        }
        return result;
    }
    tx.commit(deadline).await
}

#[cfg(any(test, feature = "test-support"))]
tokio::task_local! {
    static STALL_REWARD_COMMIT: std::cell::Cell<StalledRewardCommit>;
}

#[cfg(any(test, feature = "test-support"))]
#[derive(Clone, Copy, PartialEq, Eq)]
enum StalledRewardCommit {
    Armed,
    Started,
    Observed,
}

/// Withhold the new reward's COMMIT response only, leaving preparation and
/// receipt reconciliation on the real storage path.
#[cfg(any(test, feature = "test-support"))]
pub async fn with_stalled_reward_chest_commit<F: std::future::Future>(
    operation: F,
) -> (F::Output, bool) {
    STALL_REWARD_COMMIT
        .scope(std::cell::Cell::new(StalledRewardCommit::Armed), async {
            let result = operation.await;
            let stalled = STALL_REWARD_COMMIT.with(|stage| stage.get() == StalledRewardCommit::Observed);
            (result, stalled)
        })
        .await
}

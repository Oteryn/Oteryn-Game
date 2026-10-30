//! Complete DAILY transaction: Character then Account/World serialization, all
//! ordered v2 reward lines and their value ledgers in one commit.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::daily_reward::{load_receipt, load_state};
use super::daily_reward_claim::{
    DailyRewardClaimError, DailyRewardClaimOutcome, DailyRewardClaimRequest, write_claim,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::{CurrentCharacterItemFence, character_item_fence_is_current};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{CommandRef, RuntimeScopeRefV1};
use sqlx::{Postgres, Transaction};
use std::future::Future;

/// The existing Premium owner resolves current status inside the claim transaction.
pub trait DailyRewardPremiumGate: Sync {
    fn current(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        account: [u8; 16],
    ) -> impl Future<Output = Result<bool, DurabilityError>> + Send;
}

#[derive(Debug, Clone)]
pub struct DailyRewardDeliveryPlan {
    pub content_revision: String,
}

#[derive(Debug, Clone)]
pub struct DailyRewardView {
    pub epoch: u64,
    pub lane_day: u8,
    pub streak: u32,
    pub jokers: u8,
    pub receipt: Option<super::daily_reward::DailyRewardReceipt>,
    pub settled: Option<super::daily_reward::DailyRewardReceipt>,
}

impl DurabilityRoot {
    /// Load the durable streak for a fresh session projection. This is character data,
    /// not current authority; the admission owner revalidates its session after this read.
    pub async fn read_daily_reward_streak(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: crate::domain::CharacterId,
        world_id: crate::foundation::WorldId,
    ) -> Result<u32, DailyRewardClaimError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DailyRewardClaimError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let streak: Option<i32> = sqlx::query_scalar(
                "SELECT streak FROM public.game_character_daily_rewards WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid",
            )
            .bind(character_id.as_bytes().as_slice())
            .bind(world_id.as_bytes().as_slice())
            .fetch_optional(&mut *tx)
            .await?;
            Ok(Ok(u32::try_from(streak.unwrap_or(0)).map_err(|_| DurabilityError::InvalidStoredState)?))
        })).await?
    }

    /// A current-session read; neither a caller epoch nor a local clock opens a window.
    pub async fn read_daily_reward_view(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
    ) -> Result<DailyRewardView, DailyRewardClaimError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DailyRewardClaimError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !current(&mut tx, &node, fence, command).await? {
                return Ok(Err(DailyRewardClaimError::AuthorityRejected));
            }
            let epoch: Option<String> = sqlx::query_scalar("SELECT h.reset_epoch::text FROM public.game_world_reset_history h WHERE h.world_id=encode($1,'hex')::uuid ORDER BY h.reset_epoch DESC LIMIT 1")
                .bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let Some(epoch) = epoch else { return Ok(Err(DailyRewardClaimError::ResetWindowUnavailable)); };
            let epoch = epoch.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
            let state = load_state(&mut tx, fence.character_id.as_bytes(), fence.runtime_scope.world_id().as_bytes()).await?;
            let receipt = load_receipt(&mut tx, fence.character_id.as_bytes(), fence.runtime_scope.world_id().as_bytes(), epoch).await?;
            let jokers: Option<i16> = sqlx::query_scalar("SELECT least(3,coalesce(j.jokers,0)+(SELECT count(*) FROM (SELECT DISTINCT to_char(to_timestamp(h.reset_at_unix_ms/1000.0) AT TIME ZONE 'UTC','YYYY-MM') AS month FROM public.game_world_reset_history h WHERE h.world_id=r.world_id AND h.reset_epoch<=$2::text::numeric AND to_char(to_timestamp(h.reset_at_unix_ms/1000.0) AT TIME ZONE 'UTC','YYYY-MM')>coalesce(j.last_grant_month,'') ORDER BY month DESC LIMIT 3) pending))::smallint FROM public.game_character_roots r LEFT JOIN public.game_account_daily_reward_jokers j ON r.account_id=j.account_id AND r.world_id=j.world_id WHERE r.character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).bind(epoch.to_string()).fetch_optional(&mut *tx).await?;
            let saved: Option<String> = sqlx::query_scalar("SELECT c.reset_epoch::text FROM public.game_daily_reward_commands d JOIN public.game_character_daily_reward_claims c ON c.transaction_id=d.claim_id WHERE d.game_session_id=encode($1,'hex')::uuid AND d.command_id=$2::text::numeric AND c.character_id=encode($3,'hex')::uuid AND c.world_id=encode($4,'hex')::uuid")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
                .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let settled = match saved { Some(epoch) => load_receipt(&mut tx, fence.character_id.as_bytes(), fence.runtime_scope.world_id().as_bytes(), epoch.parse().map_err(|_| DurabilityError::InvalidStoredState)?).await?, None => None };
            let jokers = u8::try_from(jokers.unwrap_or(0)).map_err(|_| DurabilityError::InvalidStoredState)?;
            let lane_day = if receipt.is_none() {
                oteryn_gameplay_progression::daily_reward::advance_daily_reward(state, epoch, jokers)
                    .map_err(|_| DurabilityError::InvalidStoredState)?.claimed_lane_day
            } else { state.lane_day };
            Ok(Ok(DailyRewardView { epoch, lane_day, streak: state.streak, jokers, receipt, settled }))
        })).await?
    }

    /// Every v2 reward shares the native recovery/admission/fence transaction.
    pub async fn commit_daily_reward_claim<G: DailyRewardPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: DailyRewardClaimRequest,
        plan: DailyRewardDeliveryPlan,
        gate: &G,
    ) -> Result<DailyRewardClaimOutcome, DailyRewardClaimError> {
        use DailyRewardClaimError as Error;
        if request.reset_epoch == 0
            || request.transaction_id[6] >> 4 != 7
            || request.transaction_id[8] >> 6 != 2
            || oteryn_gameplay_content::content::daily_reward::DailyRewardContent::catalogue()
                .is_none_or(|content| content.revision != plan.content_revision)
        {
            return Err(Error::InvalidRequest);
        }
        // Recheck current authority even for a historical claim; it grants no new value.
        let view = self
            .read_daily_reward_view(authority, node, fence, request.command)
            .await?;
        if let Some(receipt) = view.settled {
            return if receipt.request_binding == request.request_binding {
                Ok(DailyRewardClaimOutcome::AlreadyClaimed(receipt))
            } else {
                Err(Error::BindingMismatch)
            };
        }
        if request.reset_epoch != view.epoch {
            return Err(Error::ResetWindowUnavailable);
        }
        if let Some(receipt) = view.receipt {
            return if receipt.request_binding == request.request_binding {
                Ok(DailyRewardClaimOutcome::AlreadyClaimed(receipt))
            } else {
                Err(Error::BindingMismatch)
            };
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| Error::AuthorityRejected)?;
        let node = node.clone();
        let mut gate = gate;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut gate, move |holder, deadline, gate| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !current(&mut tx, &node, fence, request.command).await? {
                        return Ok(Err(Error::AuthorityRejected));
                    }
                    let outcome = apply(&mut tx, fence, request, *gate).await;
                    if outcome.is_ok() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(outcome)
                })
            })
            .await?
    }
}

async fn current(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
) -> Result<bool, DurabilityError> {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    character_item_fence_is_current(
        tx,
        node,
        &fence,
        command,
        fence.character_id,
        *world_id.as_bytes(),
        *channel_id.as_bytes(),
    )
    .await
}

async fn apply<G: DailyRewardPremiumGate>(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    request: DailyRewardClaimRequest,
    gate: &G,
) -> Result<DailyRewardClaimOutcome, DailyRewardClaimError> {
    let world = fence.runtime_scope.world_id();
    // A competing claim must not retain the mint performed before this callback.
    if let Some(receipt) = load_receipt(
        tx,
        fence.character_id.as_bytes(),
        world.as_bytes(),
        request.reset_epoch,
    )
    .await?
    {
        return if receipt.request_binding == request.request_binding {
            Ok(DailyRewardClaimOutcome::AlreadyClaimed(receipt))
        } else {
            Err(DailyRewardClaimError::BindingMismatch)
        };
    }
    let account: String = sqlx::query_scalar("SELECT account_id::text FROM public.game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let premium = gate
        .current(tx, super::item_mint::uuid_text(&account)?)
        .await?;
    let joker = super::daily_reward_jokers::grant(tx, fence, request).await?;
    let outcome = write_claim(tx, fence, request, joker.balance, premium).await?;
    let DailyRewardClaimOutcome::Committed(receipt) = &outcome else {
        return Err(DailyRewardClaimError::InvalidRequest);
    };
    super::daily_reward_jokers::spend(tx, fence, request, joker, receipt.jokers_spent).await?;
    sqlx::query("INSERT INTO public.game_daily_reward_commands VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid)")
        .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .bind(request.transaction_id.as_slice()).execute(&mut **tx).await?;
    super::daily_reward_effects::grant(tx, fence, request, receipt.lane_day, premium).await?;
    Ok(outcome)
}

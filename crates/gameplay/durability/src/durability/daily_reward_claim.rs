//! Fenced DAILY claim/state core. This remains uncomposed until content, shrine,
//! joker ledger, ordered lines and available sinks join this same transaction.
//! It is not an enabled reward path and cannot mint or bind an item.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::daily_reward::{DailyRewardReceipt, load_receipt, load_state};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::{CurrentCharacterItemFence, character_item_fence_is_current};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{CommandRef, RuntimeScopeRefV1};
use oteryn_gameplay_progression::daily_reward::{advance_daily_reward, daily_reward_reset_day};
use sqlx::Row;

/// Frozen by the eventual source owner before the first attempt. No client-
/// supplied epoch, binding or transaction identity is current authority evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DailyRewardClaimRequest {
    pub command: CommandRef,
    pub reset_epoch: u64,
    pub transaction_id: [u8; 16],
    pub request_binding: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DailyRewardClaimOutcome {
    Committed(DailyRewardReceipt),
    AlreadyClaimed(DailyRewardReceipt),
}

#[derive(Debug)]
pub enum DailyRewardClaimError {
    InvalidRequest,
    AuthorityRejected,
    ResetWindowUnavailable,
    BindingMismatch,
    Durability(DurabilityError),
}

impl From<DurabilityError> for DailyRewardClaimError {
    fn from(error: DurabilityError) -> Self {
        Self::Durability(error)
    }
}

impl DurabilityRoot {
    /// Atomically record a Free claim and its lane/streak projection. Run 1c
    /// deliberately supplies zero jokers until their native ledger is composed.
    /// Every refusal rolls back; reconnect retries recover the stored receipt.
    pub async fn commit_daily_reward_claim(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: DailyRewardClaimRequest,
    ) -> Result<DailyRewardClaimOutcome, DailyRewardClaimError> {
        use DailyRewardClaimError as Error;
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Err(Error::AuthorityRejected);
        };
        if request.reset_epoch == 0
            || request.transaction_id[6] >> 4 != 7
            || request.transaction_id[8] >> 6 != 2
        {
            return Err(Error::InvalidRequest);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| Error::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            // Current authority is required even for duplicate claims. The helper
            // locks the native Character root and proves Account/World ownership.
            if !character_item_fence_is_current(&mut tx, &node, &fence, request.command,
                fence.character_id, *world_id.as_bytes(), *channel_id.as_bytes()).await? {
                return Ok(Err(Error::AuthorityRejected));
            }
            if let Some(receipt) = load_receipt(&mut tx, fence.character_id.as_bytes(), world_id.as_bytes(), request.reset_epoch).await? {
                if receipt.request_binding != request.request_binding {
                    return Ok(Err(Error::BindingMismatch));
                }
                // No new receipt, projection update, or commit is needed for a refusal.
                return Ok(Ok(DailyRewardClaimOutcome::AlreadyClaimed(receipt)));
            }
            // Latest committed epoch, never wall-clock-derived. The World owner
            // will publish this history after its durable reset protocol commits.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:world-reset:' || (encode($1,'hex')::uuid)::text,0))")
                .bind(world_id.as_bytes().as_slice()).execute(&mut *tx).await?;
            let reset = sqlx::query("SELECT reset_epoch::text, reset_day, (local_time::date - DATE '1970-01-01')::bigint AS civil_day, floor(extract(epoch FROM local_time::time))::bigint AS local_seconds FROM (SELECT reset_epoch,reset_day,to_timestamp(reset_at_unix_ms/1000.0) AT TIME ZONE 'Europe/Berlin' AS local_time FROM public.game_world_reset_history WHERE world_id=encode($1,'hex')::uuid ORDER BY reset_epoch DESC LIMIT 1) h")
                .bind(world_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let Some(reset) = reset else { return Ok(Err(Error::ResetWindowUnavailable)); };
            let epoch: u64 = reset.try_get::<String,_>("reset_epoch")?.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
            if epoch != request.reset_epoch { return Ok(Err(Error::ResetWindowUnavailable)); }
            let day: i64 = reset.try_get("reset_day")?;
            let seconds = u32::try_from(reset.try_get::<i64,_>("local_seconds")?).map_err(|_| DurabilityError::InvalidStoredState)?;
            if daily_reward_reset_day(reset.try_get("civil_day")?, seconds).map_err(|_| DurabilityError::InvalidStoredState)? != day {
                return Err(DurabilityError::InvalidStoredState);
            }
            let before = load_state(&mut tx, fence.character_id.as_bytes(), world_id.as_bytes()).await?;
            let advance = advance_daily_reward(before, epoch, 0).map_err(|_| DurabilityError::InvalidStoredState)?;
            let receipt = DailyRewardReceipt { transaction_id: request.transaction_id,
                request_binding: request.request_binding, reset_epoch: epoch, reset_day: day,
                lane_day: advance.claimed_lane_day, streak_before: before.streak,
                streak_after: advance.next.streak, jokers_spent: advance.jokers_spent };
            sqlx::query("INSERT INTO public.game_character_daily_rewards(character_id,world_id,lane_day,streak,last_claim_epoch,last_claim_day) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5::text::numeric(20,0),$6) ON CONFLICT(character_id,world_id) DO UPDATE SET lane_day=EXCLUDED.lane_day,streak=EXCLUDED.streak,last_claim_epoch=EXCLUDED.last_claim_epoch,last_claim_day=EXCLUDED.last_claim_day")
                .bind(fence.character_id.as_bytes().as_slice()).bind(world_id.as_bytes().as_slice())
                .bind(i16::from(advance.next.lane_day)).bind(i32::try_from(advance.next.streak).map_err(|_| DurabilityError::InvalidStoredState)?)
                .bind(epoch.to_string()).bind(day).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO public.game_character_daily_reward_claims(character_id,world_id,reset_epoch,reset_day,transaction_id,request_binding,lane_day,streak_before,streak_after,jokers_spent,premium) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric(20,0),$4,encode($5,'hex')::uuid,$6,$7,$8,$9,$10,FALSE)")
                .bind(fence.character_id.as_bytes().as_slice()).bind(world_id.as_bytes().as_slice())
                .bind(epoch.to_string()).bind(day).bind(receipt.transaction_id.as_slice()).bind(receipt.request_binding.as_slice())
                .bind(i16::from(receipt.lane_day)).bind(i32::try_from(receipt.streak_before).map_err(|_| DurabilityError::InvalidStoredState)?).bind(i32::try_from(receipt.streak_after).map_err(|_| DurabilityError::InvalidStoredState)?)
                .bind(i16::from(receipt.jokers_spent)).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(DailyRewardClaimOutcome::Committed(receipt)))
        })).await?
    }
}

#[cfg(test)]
#[path = "daily_reward_tests.rs"]
mod tests;

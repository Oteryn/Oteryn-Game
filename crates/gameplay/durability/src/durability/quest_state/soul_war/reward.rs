use super::*;
use crate::domain::CharacterId;
use sqlx::{Postgres, Row, Transaction};

pub const SOUL_WAR_REWARD_CLAIM: &str = "oteryn:reward-claim.soul-war-final";
pub const SOUL_WAR_REWARD_REVISION: &str = "soul-war-crystal-201189f-v1";
pub const SOUL_WAR_REWARD_PLACEMENT: &str = "oteryn:quest-callback/soul-war-final-reward";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingSoulWarReward {
    pub item_source_id: u32,
    pub claim_key: String,
    pub content_revision: String,
}

/// The first verified final completion chooses once. Retry, a full backpack and
/// reconnect cannot reroll this entitlement. Crystal's reward pool has18 entries.
pub(super) async fn earn(
    tx: &mut Transaction<'_, Postgres>, character: CharacterId, occurrence: &[u8; 32],
) -> StoredResult<()> {
    let item = reward_source_item(*character.as_bytes(), *occurrence)?;
    sqlx::query("INSERT INTO game_character_soul_war_reward_entitlements(character_id,earned_occurrence,item_source_id,claim_key,content_revision) SELECT character_id,$2,$4,$3,content_revision FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id) DO NOTHING")
        .bind(character.as_bytes().as_slice()).bind(occurrence.as_slice())
        .bind(SOUL_WAR_REWARD_CLAIM).bind(item as i32).execute(&mut **tx).await?;
    Ok(())
}

/// The existing deterministic primitive derives each draw from the trusted,
/// immutable native-death occurrence and recipient. Rejection removes modulo
/// bias; a transaction rollback produces the same next accepted draw on retry.
fn reward_source_item(character: [u8; 16], occurrence: [u8; 32]) -> StoredResult<u32> {
    use oteryn_simulation_determinism::{GameplayDecisionRoot, DecisionOccurrenceId, deterministic_decision_u64};
    use oteryn_gameplay_quests::quest::soul_war::FINAL_REWARD_ITEM_IDS;
    let root = GameplayDecisionRoot::from_bytes(occurrence);
    let occurrence = DecisionOccurrenceId::from_bytes(character);
    let count = FINAL_REWARD_ITEM_IDS.len() as u64;
    let reject_below = count.wrapping_neg() % count;
    let mut ordinal = 0_u64;
    loop {
        let draw = deterministic_decision_u64(&root, occurrence, "soul-war.final-reward.v1", ordinal)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if draw >= reject_below { return Ok(FINAL_REWARD_ITEM_IDS[(draw % count) as usize]); }
        ordinal = ordinal.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
    }
}

impl DurabilityRoot {
    pub async fn pending_soul_war_reward(
        &self, authority: &ReconciledCharacterAuthority<'_, '_>, character: CharacterId,
    ) -> Result<Option<PendingSoulWarReward>> {
        let recovery = authority.record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let row = sqlx::query("SELECT item_source_id,claim_key,content_revision FROM game_character_soul_war_reward_entitlements WHERE character_id=encode($1,'hex')::uuid AND state='PENDING'")
                .bind(character.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let pending = row.map(|row| -> StoredResult<_> { Ok(PendingSoulWarReward {
                item_source_id: u32::try_from(row.try_get::<i32, _>("item_source_id")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                claim_key: row.try_get("claim_key")?,
                content_revision: row.try_get("content_revision")?,
            }) }).transpose()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(pending))
        })).await?
    }
}

/// Only the existing MINT owner calls this, after inserting its same-transaction
/// item/claim receipt. A client-selected flag is never accepted as delivery proof.
pub(crate) async fn confirm_soul_war_reward_in_transaction(
    tx: &mut Transaction<'_, Postgres>, node: &NodeIncarnationProof,
    fence: CurrentCharacterGameplayFence, command: CommandRef,
) -> Result<()> {
    let delivered: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_reward_claim_mint_receipts r JOIN game_reward_claims c ON c.character_id=r.character_id AND c.claim_family=r.claim_family AND c.claim_production_key=r.claim_production_key AND c.claimed_transaction_id=r.transaction_id WHERE r.character_id=encode($1,'hex')::uuid AND r.game_session_id=encode($2,'hex')::uuid AND r.command_id=$3::text::numeric AND r.claim_family='RewardClaim' AND r.claim_production_key=$4)")
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
        .bind(SOUL_WAR_REWARD_CLAIM).fetch_one(&mut **tx).await.map_err(DurabilityError::from)?;
    if !delivered { return Err(CharacterProgressionError::AuthorityRejected); }
    let root = assert_gameplay_fence(tx, &fence, node).await??;
    let occurrence: [u8; 32] = Sha256::new()
        .chain_update(b"oteryn:soul-war:reward-claim:v1")
        .chain_update(fence.character_id.as_bytes()).chain_update(command.game_session_id().as_bytes())
        .chain_update(command.command_id().get().to_be_bytes()).finalize().into();
    let outcome = storage::commit_locked(tx, fence.character_id, &root, &occurrence, SoulWarTransition::ClaimReward, None, None).await?;
    if outcome.result.is_err() { return Err(CharacterProgressionError::AuthorityRejected); }
    let updated = sqlx::query("UPDATE game_character_soul_war_reward_entitlements SET state='DELIVERED',claim_session_id=encode($2,'hex')::uuid,claim_command_id=$3::text::numeric WHERE character_id=encode($1,'hex')::uuid AND state='PENDING'")
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
        .execute(&mut **tx).await.map_err(DurabilityError::from)?;
    if updated.rows_affected() != 1 { return Err(DurabilityError::InvalidStoredState.into()); }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soul_war_reward_choice_is_stable_for_retry_and_uses_the_source_pool() -> std::result::Result<(), String> {
        assert_eq!(reward_source_item([1; 16], [2; 32]).map_err(|e| format!("{e:?}"))?, 34095);
        let mut chosen = std::collections::BTreeSet::new();
        for seed in 0..=u8::MAX {
            let first = reward_source_item([seed; 16], [2; 32]).map_err(|e| format!("{e:?}"))?;
            assert!((34082..=34099).contains(&first));
            assert_eq!(reward_source_item([seed; 16], [2; 32]).map_err(|e| format!("{e:?}"))?, first);
            chosen.insert(first);
        }
        assert_eq!(chosen.len(), 18);
        Ok(())
    }
}

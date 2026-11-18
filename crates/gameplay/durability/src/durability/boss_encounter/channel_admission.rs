//! Single-player source rooms use the same Character cooldown and admission policy as lever rooms.
use super::super::activity_handoff::ActivityRefusal;
use super::super::character_authority::ReconciledCharacterAuthority;
use super::super::character_progression::{assert_item_only_fence, assert_runtime_scope_fence};
use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::item_transfer::CurrentCharacterItemFence;
use super::super::spell_owner_commit::SpellLanePermit;
use super::cooldown_storage::*;
use super::*;
use oteryn_gameplay_encounter_rules::boss_room::{
    AdmissionRefusal, BossRoomPolicy, ParticipantFact, plan_fresh_room,
};

pub struct ChannelBossEntry<'a> {
    pub boss_key: &'a str,
    pub policy: &'a BossRoomPolicy,
    pub trigger: (i32, i32, i16),
    pub occurrence: [u8; 16],
    pub reset_epoch: u64,
}

/// Exact committed spend retained until the native owner installs the entry.
pub struct ChannelBossCooldown {
    fence: CurrentCharacterItemFence,
    key: BossRewardKey,
    occurrence: [u8; 16],
}

impl super::super::DurabilityRoot {
    /// Spend the shared cooldown only after the Channel owner qualifies the physical entry.
    pub async fn admit_channel_boss(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        permit: &SpellLanePermit,
        entry: ChannelBossEntry<'_>,
    ) -> std::result::Result<
        std::result::Result<ChannelBossCooldown, ActivityRefusal>,
        DurabilityError,
    > {
        if !valid_scope(permit, fence) {
            return Ok(Err(ActivityRefusal::AuthorityRejected));
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let key = BossRewardKey::new(fence.character_id, entry.boss_key)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let policy = entry.policy.clone();
        let admissions =
            super::super::fresh_admission::FreshAdmissionStore::from_root(self.clone());
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let character = crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if !admissions.committed_session_use(&mut tx, character, fence.game_session_id).await?
                || assert_item_only_fence(&mut tx, &fence, &node).await?.is_err()
            {
                return Ok(Err(ActivityRefusal::AuthorityRejected));
            }
            let now: i64 = sqlx::query_scalar(
                "SELECT floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint",
            ).fetch_one(&mut *tx).await?;
            let reset: Option<String> = sqlx::query_scalar(
                "SELECT reset_epoch::text FROM game_world_reset_read_locked(encode($1,'hex')::uuid)",
            ).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
                .fetch_optional(&mut *tx).await?;
            if reset.and_then(|value| value.parse::<u64>().ok()) != Some(entry.reset_epoch) {
                return Ok(Err(ActivityRefusal::ContentChanged));
            }
            let spend = ChannelBossCooldown { fence, key, occurrence: entry.occurrence };
            let old = read_boss_cooldown_data(&mut tx, &spend.key).await
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if old.as_ref().is_some_and(|old| old.admission == entry.occurrence && old.available_at > now) {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(spend));
            }
            let level: Option<i64> = sqlx::query_scalar(
                "SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid",
            ).bind(character.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let Some(level) = level else {
                return Ok(Err(ActivityRefusal::AuthorityRejected));
            };
            let fact = ParticipantFact {
                character: *character.as_bytes(),
                position: entry.trigger,
                level: u32::try_from(level).map_err(|_| DurabilityError::InvalidStoredState)?,
                cooldown_until_ms: old.map_or(0, |old| old.available_at as u64),
            };
            if let Err(reason) = plan_fresh_room(&policy, &[fact], now as u64) {
                return Ok(Err(match reason {
                    AdmissionRefusal::Level => ActivityRefusal::Level,
                    AdmissionRefusal::Cooldown => ActivityRefusal::Cooldown,
                    _ => ActivityRefusal::InvalidInput,
                }));
            }
            let result = set_boss_cooldowns_data(&mut tx, std::slice::from_ref(&spend.key), &BossCooldownAdmissionData {
                admission: entry.occurrence,
                now,
                reset_epoch: entry.reset_epoch,
                cooldown_s: policy.cooldown_ms / 1000,
                boosted: false,
            }).await.map_err(|_| DurabilityError::Unavailable)?;
            if result != BossCooldownChange::Applied {
                return Ok(Err(ActivityRefusal::Cooldown));
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(spend))
        })).await
    }

    /// Undo an exact spend only while the same current Channel owner still holds its lane.
    pub async fn release_channel_boss_cooldown(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &SpellLanePermit,
        spend: ChannelBossCooldown,
    ) -> std::result::Result<(), DurabilityError> {
        if !valid_scope(permit, spend.fence) {
            return Err(DurabilityError::Unavailable);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();

        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !assert_runtime_scope_fence(
                        &mut tx,
                        spend.fence.runtime_scope,
                        spend.fence.scope_ownership_generation,
                        &node,
                    )
                    .await?
                    {
                        return Err(DurabilityError::Unavailable);
                    }
                    super::cooldown_storage::compensate_boss_cooldowns_data(
                        &mut tx,
                        &[spend.key],
                        spend.occurrence,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    commit_semantic_transaction(tx, deadline).await
                })
            })
            .await
    }
}

fn valid_scope(permit: &SpellLanePermit, fence: CurrentCharacterItemFence) -> bool {
    permit.runtime_scope() == fence.runtime_scope
        && matches!(
            fence.runtime_scope,
            crate::foundation::RuntimeScopeRefV1::Channel { .. }
        )
}

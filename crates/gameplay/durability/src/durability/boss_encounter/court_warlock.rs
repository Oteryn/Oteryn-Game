//! Source lever admission uses the shared Character boss cooldown owner.
use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::character_progression::assert_item_only_fence;
use super::super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::super::item_transfer::CurrentCharacterItemFence;
use super::super::runtime_scope_assignment::NodeIncarnationProof;
use super::super::{DurabilityError, DurabilityRoot};
use super::cooldown_storage::{self, BossCooldownAdmissionData, BossCooldownChange, BossRewardKey};
use oteryn_gameplay_quests::quest::world_callbacks::court_warlock as source;

impl DurabilityRoot {
    /// The runtime freezes the exact staging party before this call. All current
    /// fences and source quest gates are checked before spending any cooldown.
    /// A failed physical publication refunds only this exact admission.
    pub async fn court_warlock_admission(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        mut party: Vec<CurrentCharacterItemFence>,
        admission: [u8; 16],
        release: bool,
    ) -> Result<bool, DurabilityError> {
        if !(1..=5).contains(&party.len()) || !cooldown_storage::reward_uuid(&admission) {
            return Ok(false);
        }
        party.sort_by_key(|f| *f.character_id.as_bytes());
        if party
            .windows(2)
            .any(|pair| pair[0].character_id == pair[1].character_id)
            || party.iter().any(|f| {
                f.runtime_scope != party[0].runtime_scope
                    || f.scope_ownership_generation != party[0].scope_ownership_generation
            })
        {
            return Ok(false);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                let mut keys = Vec::new();
                for fence in &party {
                    if assert_item_only_fence(&mut tx, fence, &node).await?.is_err() { return Ok(false); }
                    keys.push(BossRewardKey::new(fence.character_id, source::BOSS).map_err(|_| DurabilityError::InvalidStoredState)?);
                }
                cooldown_storage::lock_reward_roots(&mut tx, &keys).await.map_err(|_| DurabilityError::InvalidStoredState)?;
                if release {
                    for key in &keys {
                        sqlx::query("DELETE FROM game_character_boss_cooldowns WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2 AND admission=encode($3,'hex')::uuid")
                            .bind(key.character.as_bytes().as_slice()).bind(source::BOSS).bind(admission.as_slice()).execute(&mut *tx).await?;
                    }
                } else {
                    for fence in &party {
                        let stage: Option<i64> = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND track_key=$3")
                            .bind(fence.character_id.as_bytes().as_slice()).bind(source::QUEST).bind(source::TRACK).fetch_optional(&mut *tx).await?;
                        if stage.is_none_or(|s| s < 11) { return Ok(false); }
                        if sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid)")
                            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await? { return Ok(false); }
                    }
                    let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut *tx).await?;
                    // An exact command replay retains its originally committed time.
                    let mut replay = 0;
                    for key in &keys {
                        if cooldown_storage::read_boss_cooldown_data(&mut tx, key).await.map_err(|_| DurabilityError::InvalidStoredState)?
                            .is_some_and(|old| old.admission == admission) { replay += 1; }
                    }
                    if replay != 0 && replay != keys.len() { return Ok(false); }
                    if replay == 0 {
                        let change = cooldown_storage::set_boss_cooldowns_data(&mut tx, &keys, &BossCooldownAdmissionData {
                            admission, now, reset_epoch:0, cooldown_s:72_000, boosted:false,
                        }).await.map_err(|_| DurabilityError::InvalidStoredState)?;
                        if change == BossCooldownChange::OnCooldown { return Ok(false); }
                    }
                }
                commit_semantic_transaction(tx, deadline).await?;
                Ok(true)
            })
        }).await
    }
}

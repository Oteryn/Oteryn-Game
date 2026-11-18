//! Exact read for a native boss staging portal, using the shared Character cooldown owner.
use super::{
    DurabilityError, NodeIncarnationProof, assert_recovery_fence,
    cooldown_storage::{BossRewardKey, read_boss_cooldown_data},
    lock_admission_relations,
};
use crate::durability::{
    DurabilityRoot,
    character_authority::ReconciledCharacterAuthority,
    character_progression::assert_item_only_fence,
    db::{begin_semantic_transaction, commit_semantic_transaction},
    item_transfer::CurrentCharacterItemFence,
    spell_owner_commit::SpellLanePermit,
};

impl DurabilityRoot {
    /// Returns remaining seconds at the transaction clock, or no decision for a stale fence.
    pub async fn boss_portal_cooldown_seconds(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &SpellLanePermit,
        fence: CurrentCharacterItemFence,
        boss: &str,
    ) -> Result<Option<u64>, DurabilityError> {
        if permit.runtime_scope() != fence.runtime_scope || permit.has_unresolved() {
            return Ok(None);
        }
        let key = BossRewardKey::new(fence.character_id, boss)
            .map_err(|_| DurabilityError::CommitRejected)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::CommitRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if assert_item_only_fence(&mut tx, &fence, &node)
                        .await?
                        .is_err()
                    {
                        return Ok(None);
                    }
                    let row = read_boss_cooldown_data(&mut tx, &key)
                        .await
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let now: i64 = sqlx::query_scalar(
                        "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    let remaining =
                        row.map_or(0, |row| row.available_at.saturating_sub(now).max(0));
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some(remaining.unsigned_abs().div_ceil(1000)))
                })
            })
            .await
    }
}

impl DurabilityRoot {
    /// Crystal 201189f8 actions_bosses_levers.lua:277-282 writes the cooldown
    /// for every character admitted by the lever, using config.lua.dist:658.
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_quest_encounter_cooldown(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        permit: &SpellLanePermit,
        command: crate::foundation::CommandRef,
        cooldown: oteryn_gameplay_quests::quest::world_callbacks::EncounterCooldown,
        reset_epoch: u64,
        mut party: Vec<CurrentCharacterItemFence>,
    ) -> Result<bool, DurabilityError> {
        use super::cooldown_storage::{
            BossCooldownAdmissionData, BossCooldownChange, set_boss_cooldowns_data,
        };
        use sha2::{Digest, Sha256};

        let Some(first) = party.first().copied() else {
            return Ok(false);
        };
        let scope = first.runtime_scope;
        if party.len() > 15
            || permit.runtime_scope() != scope
            || permit.has_unresolved()
            || !party
                .iter()
                .any(|fence| fence.game_session_id == command.game_session_id())
            || party.iter().any(|fence| {
                fence.runtime_scope != scope
                    || fence.scope_ownership_generation != first.scope_ownership_generation
            })
        {
            return Ok(false);
        }
        party.sort_unstable_by_key(|fence| fence.character_id);
        if party
            .windows(2)
            .any(|pair| pair[0].character_id == pair[1].character_id)
        {
            return Ok(false);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::CommitRejected)?;
        let node = node.clone();
        let digest = Sha256::new()
            .chain_update(b"oteryn:quest-encounter-cooldown:v1")
            .chain_update(scope.world_id().as_bytes())
            .chain_update(command.game_session_id().as_bytes())
            .chain_update(command.command_id().get().to_be_bytes())
            .chain_update(reset_epoch.to_be_bytes())
            .chain_update(cooldown.boss.as_bytes())
            .finalize();
        let mut admission = [0; 16];
        admission.copy_from_slice(&digest[..16]);
        admission[6] = (admission[6] & 0x0f) | 0x70;
        admission[8] = (admission[8] & 0x3f) | 0x80;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                let epoch: Option<String> = sqlx::query_scalar(
                    "SELECT reset_epoch::text FROM game_world_reset_read_locked(encode($1,'hex')::uuid)",
                )
                .bind(scope.world_id().as_bytes().as_slice())
                .fetch_optional(&mut *tx)
                .await?;
                let epoch = epoch.map(|value| value.parse::<u64>()).transpose()
                    .map_err(|_| DurabilityError::InvalidStoredState)?.unwrap_or(0);
                if epoch != reset_epoch {
                    return Ok(false);
                }
                let mut keys = Vec::with_capacity(party.len());
                for fence in party {
                    if assert_item_only_fence(&mut tx, &fence, &node).await?.is_err() {
                        return Ok(false);
                    }
                    keys.push(BossRewardKey::new(fence.character_id, &cooldown.boss)
                        .map_err(|_| DurabilityError::CommitRejected)?);
                }
                let now: i64 = sqlx::query_scalar(
                    "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
                ).fetch_one(&mut *tx).await?;
                // Reconcile an uncertain write without advancing time or admitting a subset.
                let prior_count: i64 = sqlx::query_scalar(
                    "SELECT count(*) FROM game_character_boss_cooldowns
                     WHERE boss_key=$1 AND admission=encode($2,'hex')::uuid",
                )
                .bind(&cooldown.boss).bind(admission.as_slice()).fetch_one(&mut *tx).await?;
                if prior_count != 0 {
                    if usize::try_from(prior_count) != Ok(keys.len()) {
                        return Ok(false);
                    }
                    for key in &keys {
                        if !read_boss_cooldown_data(&mut tx, key).await
                            .map_err(|_| DurabilityError::InvalidStoredState)?
                            .is_some_and(|row| row.admission == admission && row.available_at > now)
                        {
                            return Ok(false);
                        }
                    }
                } else if set_boss_cooldowns_data(&mut tx, &keys, &BossCooldownAdmissionData {
                    admission, now, reset_epoch, cooldown_s: u64::from(cooldown.seconds), boosted: false,
                }).await.map_err(|_| DurabilityError::InvalidStoredState)? != BossCooldownChange::Applied {
                    return Ok(false);
                }
                commit_semantic_transaction(tx, deadline).await?;
                Ok(true)
            }))
            .await
    }
}

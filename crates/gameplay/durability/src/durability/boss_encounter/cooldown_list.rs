//! Bounded, current-session-fenced read of the shared cooldown owner.
use super::super::admission_authority_guards::AdmissionGuardStore;
use super::super::character_authority::ReconciledCharacterAuthority;
use super::super::character_progression::assert_item_only_fence;
use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::item_transfer::CurrentCharacterItemFence;
use super::*;
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
};
use oteryn_protocol_oteryn::boss_encounter::{
    BossCooldownEntry, BossRewardChestEntry, BosstiaryProgressEntry, COOLDOWN_PAGE_SIZE,
    REWARD_CHEST_PAGE_SIZE, decode_cooldown_query, encode_bosstiary_list, encode_reward_chest,
};

impl super::super::DurabilityRoot {
    /// Reads the admitted Character's own unexpired personal reward groups. This uses the same
    /// current session/lease/scope proof as the cooldown view; an expired row is deliberately
    /// invisible even if the bounded expiry worker has not retired its items yet.
    pub async fn read_boss_reward_chest(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
    ) -> std::result::Result<Option<Vec<u8>>, DurabilityError> {
        self.read_boss_character_view(authority, node, fence, BossView::Rewards)
            .await
    }

    /// Read only this admitted Character's page under retained session identity use,
    /// retained eligibility/presence publications, current recovery, session, lease,
    /// assignment, node and live Character-root interpretation proofs. This is an item-only
    /// read: unrelated CharacterRevision advances do not invalidate it. The semantic-pass
    /// deadline bounds every database wait.
    pub async fn read_boss_cooldown_list(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        query: &[u8],
    ) -> std::result::Result<Option<Vec<u8>>, DurabilityError> {
        let Ok(after) = decode_cooldown_query(query) else {
            return Ok(None);
        };
        let after = after.to_owned();
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let guards = AdmissionGuardStore::from_root(self.clone());
        let admissions =
            super::super::fresh_admission::FreshAdmissionStore::from_root(self.clone());
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let character_id =
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(|_| DurabilityError::InvalidStoredState)?;
                    // A materialized live session is insufficient after a partial
                    // restore loses issuance history. Prove retained identity use
                    // before the independent participant/root/runtime checks.
                    if !admissions
                        .committed_session_use(&mut tx, character_id, fence.game_session_id)
                        .await?
                    {
                        return Ok(None);
                    }
                    // Eligibility, lease and presence mirrors also need their
                    // publishers' retained decisions. Resolve the account through
                    // the validated Character publication; the participant fence
                    // below still compares it to the exact session and root.
                    let character = match guards
                        .load_locked(
                            &mut tx,
                            &AdmissionAuthorityGuardKeyV1::Character(character_id),
                        )
                        .await
                    {
                        Ok(Some(character)) => character,
                        Ok(None) | Err(DurabilityError::InvalidStoredState) => return Ok(None),
                        Err(error) => return Err(error),
                    };
                    let AdmissionAuthorityGuardStateV1::Character { account_id, .. } =
                        character.state
                    else {
                        return Ok(None);
                    };
                    // Reuse the owner validator for account presence and runtime
                    // readiness under the same relation locks and deadline.
                    for key in [
                        AdmissionAuthorityGuardKeyV1::Account { account_id },
                        AdmissionAuthorityGuardKeyV1::Runtime(fence.runtime_scope),
                    ] {
                        match guards.load_locked(&mut tx, &key).await {
                            Ok(Some(_)) => {}
                            Ok(None) | Err(DurabilityError::InvalidStoredState) => return Ok(None),
                            Err(error) => return Err(error),
                        }
                    }
                    if assert_item_only_fence(&mut tx, &fence, &node)
                        .await?
                        .is_err()
                    {
                        return Ok(None);
                    }
                    // C collation is also the primary-key collation, matching Rust key ordering.
                    // One extra row establishes continuation without a count or an unbounded scan.
                    let rows: Vec<(String, i64)> = sqlx::query_as(
                        "SELECT boss_key,available_at FROM game_character_boss_cooldowns \
                 WHERE character_id=encode($1,'hex')::uuid AND boss_key > $2 COLLATE \"C\" \
                 ORDER BY boss_key LIMIT 65",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(&after)
                    .fetch_all(&mut *tx)
                    .await?;
                    let now: i64 = sqlx::query_scalar(
                        "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    let more = rows.len() > COOLDOWN_PAGE_SIZE;
                    let entries = rows
                        .into_iter()
                        .take(COOLDOWN_PAGE_SIZE)
                        .map(|(boss_key, at)| {
                            Ok(BossCooldownEntry {
                                boss_key,
                                available_at_unix_ms: u64::try_from(at)
                                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                            })
                        })
                        .collect::<std::result::Result<Vec<_>, DurabilityError>>()?;
                    // Progress belongs to the same Character authority and transaction as the
                    // cooldown page, so the client cannot observe a cross-session mixture.
                    let boss_points: i64 = sqlx::query_scalar(
                        "SELECT COALESCE(sum(boss_points),0)::bigint FROM game_character_bosstiary_progress \
                         WHERE character_id=encode($1,'hex')::uuid",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .fetch_one(&mut *tx)
                    .await?;
                    let boss_points = u64::try_from(boss_points)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let progress_rows: Vec<(String, i64)> = sqlx::query_as(
                        "SELECT boss_key,kill_count FROM game_character_bosstiary_progress \
                         WHERE character_id=encode($1,'hex')::uuid AND boss_key > $2 COLLATE \"C\" \
                         ORDER BY boss_key LIMIT 65",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(&after)
                    .fetch_all(&mut *tx)
                    .await?;
                    let progress_more = progress_rows.len() > COOLDOWN_PAGE_SIZE;
                    let progress = progress_rows
                        .into_iter()
                        .take(COOLDOWN_PAGE_SIZE)
                        .map(|(boss_key, kills)| {
                            Ok(BosstiaryProgressEntry {
                                boss_key,
                                kill_count: u64::try_from(kills)
                                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                            })
                        })
                        .collect::<std::result::Result<Vec<_>, DurabilityError>>()?;
                    let payload = encode_bosstiary_list(
                        u64::try_from(now).map_err(|_| DurabilityError::InvalidStoredState)?,
                        &entries,
                        more,
                        &progress,
                        boss_points,
                        progress_more,
                    )
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some(payload))
                })
            })
            .await
    }
}

#[derive(Clone, Copy)]
enum BossView {
    Rewards,
}

impl super::super::DurabilityRoot {
    async fn read_boss_character_view(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        _view: BossView,
    ) -> std::result::Result<Option<Vec<u8>>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let guards = AdmissionGuardStore::from_root(self.clone());
        let admissions =
            super::super::fresh_admission::FreshAdmissionStore::from_root(self.clone());
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let character_id = crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if !admissions.committed_session_use(&mut tx, character_id, fence.game_session_id).await? {
                return Ok(None);
            }
            let character = match guards.load_locked(&mut tx, &AdmissionAuthorityGuardKeyV1::Character(character_id)).await {
                Ok(Some(character)) => character,
                Ok(None) | Err(DurabilityError::InvalidStoredState) => return Ok(None),
                Err(error) => return Err(error),
            };
            let AdmissionAuthorityGuardStateV1::Character { account_id, .. } = character.state else { return Ok(None); };
            for key in [AdmissionAuthorityGuardKeyV1::Account { account_id }, AdmissionAuthorityGuardKeyV1::Runtime(fence.runtime_scope)] {
                match guards.load_locked(&mut tx, &key).await {
                    Ok(Some(_)) => {}
                    Ok(None) | Err(DurabilityError::InvalidStoredState) => return Ok(None),
                    Err(error) => return Err(error),
                }
            }
            if assert_item_only_fence(&mut tx, &fence, &node).await?.is_err() { return Ok(None); }
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut *tx).await?;
            let rows: Vec<(Vec<u8>, String, i64, i64, i32)> = sqlx::query_as(
                "SELECT decode(replace(death_id::text,'-',''),'hex'),boss_key,minted_at,expires_at,item_count FROM game_character_boss_reward_chests WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND expires_at>$3 ORDER BY minted_at,death_id LIMIT 64"
            ).bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(now).fetch_all(&mut *tx).await?;
            let entries = rows.into_iter().take(REWARD_CHEST_PAGE_SIZE).map(|(death,boss_key,minted,expires,item_count)| Ok(BossRewardChestEntry {
                death: death.try_into().map_err(|_| DurabilityError::InvalidStoredState)?, boss_key,
                minted_at_unix_ms: u64::try_from(minted).map_err(|_| DurabilityError::InvalidStoredState)?,
                expires_at_unix_ms: u64::try_from(expires).map_err(|_| DurabilityError::InvalidStoredState)?,
                item_count: u32::try_from(item_count).map_err(|_| DurabilityError::InvalidStoredState)?,
            })).collect::<std::result::Result<Vec<_>, DurabilityError>>()?;
            let payload = encode_reward_chest(u64::try_from(now).map_err(|_| DurabilityError::InvalidStoredState)?, &entries).map_err(|_| DurabilityError::InvalidStoredState)?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Some(payload))
        })).await
    }
}

//! Source lever admission uses the shared Character boss cooldown owner.
use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::character_progression::{assert_item_only_fence, assert_runtime_scope_fence};
use super::super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::super::item_transfer::CurrentCharacterItemFence;
use super::super::runtime_scope_assignment::{NodeIncarnationProof, assignment_scope_key};
use super::super::{DurabilityError, DurabilityRoot};
use super::cooldown_storage::{self, BossCooldownAdmissionData, BossCooldownChange, BossRewardKey};
use crate::domain::CharacterId;
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_quests::quest::world_callbacks::court_warlock as source;
use sqlx::{Postgres, Transaction};

const RESERVED: i16 = 1;
const PUBLISHED: i16 = 2;
const REFUNDED: i16 = 3;

#[derive(Clone, Copy)]
pub enum SourceBossLever {
    CourtWarlock,
    Arbaziloth,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SourceBossAdmission {
    Reserve,
    Reconcile,
    Publish,
    Refund,
}

impl SourceBossLever {
    fn boss(self) -> &'static str {
        match self {
            Self::CourtWarlock => source::BOSS,
            Self::Arbaziloth => "oteryn:boss.arbaziloth",
        }
    }

    fn cooldown_seconds(self) -> u64 {
        match self {
            Self::CourtWarlock => 72_000,
            // Crystal 201189f8 arbaziloth_lever.lua, timeToFightAgain.
            Self::Arbaziloth => 20 * 60 * 60,
        }
    }
}

impl DurabilityRoot {
    /// Reserve the exact staging party before its native projection. Publication
    /// and compensation are mutually exclusive decisions of the same admission.
    #[allow(
        clippy::too_many_arguments,
        reason = "party, cooldown owner and exact admission are independently fenced"
    )]
    pub async fn source_boss_lever_admission(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        lever: SourceBossLever,
        mut party: Vec<CurrentCharacterItemFence>,
        admission: [u8; 16],
        reset_epoch: u64,
        operation: SourceBossAdmission,
    ) -> Result<bool, DurabilityError> {
        if !(1..=5).contains(&party.len()) || !cooldown_storage::reward_uuid(&admission) {
            return Ok(false);
        }
        party.sort_by_key(|fence| *fence.character_id.as_bytes());
        let scope = party[0].runtime_scope;
        let generation = party[0].scope_ownership_generation;
        if party
            .windows(2)
            .any(|pair| pair[0].character_id == pair[1].character_id)
            || party.iter().any(|fence| {
                fence.runtime_scope != scope || fence.scope_ownership_generation != generation
            })
        {
            return Ok(false);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !assert_runtime_scope_fence(&mut tx, scope, generation, &node).await? {
                return Ok(false);
            }
            let keys = party.iter().map(|fence| {
                BossRewardKey::new(fence.character_id, lever.boss())
                    .map_err(|_| DurabilityError::InvalidStoredState)
            }).collect::<Result<Vec<_>, _>>()?;
            let party_bytes: Vec<u8> = keys.iter()
                .flat_map(|key| key.character.as_bytes().iter().copied()).collect();
            let scope_key = assignment_scope_key(scope);
            let row: Option<(bool, i16)> = sqlx::query_as(
                "SELECT scope_key=$2 AND scope_generation=$3::text::numeric AND boss_key=$4 AND party=$5,
                        state FROM game_source_boss_admissions
                  WHERE admission=encode($1,'hex')::uuid FOR UPDATE",
            )
            .bind(admission.as_slice()).bind(scope_key.as_slice()).bind(generation.get().to_string())
            .bind(lever.boss()).bind(&party_bytes).fetch_optional(&mut *tx).await?;

            if row.is_some_and(|(exact, _)| !exact) {
                return Ok(false);
            }
            if operation == SourceBossAdmission::Refund {
                if row == Some((true, RESERVED)) {
                    refund_source_admission(&mut tx, &keys, &admission).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                }
                return Ok(true);
            }
            // Reconcile a lost publication acknowledgement even if a participant
            // moved after the durable decision. This branch cannot perform writes.
            if operation == SourceBossAdmission::Publish && row == Some((true, PUBLISHED)) {
                return Ok(true);
            }
            for fence in &party {
                if assert_item_only_fence(&mut tx, fence, &node).await?.is_err() {
                    return Ok(false);
                }
            }
            if row == Some((true, RESERVED)) {
                if !exact_cooldowns(&mut tx, &keys, &admission).await? {
                    return Ok(false);
                }
                if operation == SourceBossAdmission::Publish {
                    transition_source_admission(&mut tx, &admission, PUBLISHED).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                }
                return Ok(true);
            }
            if row.is_some() || operation != SourceBossAdmission::Reserve {
                return Ok(false);
            }

            for fence in &party {
                if matches!(lever, SourceBossLever::CourtWarlock) {
                    let stage: Option<i64> = sqlx::query_scalar(
                        "SELECT value FROM game_character_quest_tracks
                          WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND track_key=$3",
                    ).bind(fence.character_id.as_bytes().as_slice()).bind(source::QUEST)
                        .bind(source::TRACK).fetch_optional(&mut *tx).await?;
                    if stage.is_none_or(|stage| stage < 11) {
                        return Ok(false);
                    }
                }
                let pending: bool = sqlx::query_scalar(
                    "SELECT EXISTS(SELECT 1 FROM game_character_pending_respawns
                                    WHERE character_id=encode($1,'hex')::uuid)",
                ).bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                if pending {
                    return Ok(false);
                }
            }
            cooldown_storage::lock_reward_roots(&mut tx, &keys).await
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&mut *tx).await?;
            let change = cooldown_storage::set_boss_cooldowns_data(&mut tx, &keys, &BossCooldownAdmissionData {
                admission, now, reset_epoch, cooldown_s: lever.cooldown_seconds(), boosted: false,
            }).await.map_err(|_| DurabilityError::InvalidStoredState)?;
            // A pre-migration decision has no new publication authority.
            if change != BossCooldownChange::Applied {
                return Ok(false);
            }
            sqlx::query(
                "INSERT INTO game_source_boss_admissions(admission,scope_key,scope_generation,boss_key,party,state)
                 VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,$4,$5,$6)",
            ).bind(admission.as_slice()).bind(scope_key.as_slice()).bind(generation.get().to_string())
                .bind(lever.boss()).bind(party_bytes).bind(RESERVED).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(true)
        })).await
    }

    /// Called while holding the current native owner. Fence the ready generation
    /// and compensate up to one abandoned reservation, including older owners'.
    pub async fn refund_abandoned_source_boss_lever(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
    ) -> Result<(), DurabilityError> {
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
                    if !assert_runtime_scope_fence(&mut tx, scope, generation, &node).await? {
                        return Ok(());
                    }
                    let row: Option<(Vec<u8>, String, Vec<u8>)> = sqlx::query_as(
                "SELECT uuid_send(admission),boss_key,party FROM game_source_boss_admissions
                  WHERE scope_key=$1 AND scope_generation<=$2::text::numeric AND state=1
                  ORDER BY scope_generation,admission LIMIT 1 FOR UPDATE",
            ).bind(assignment_scope_key(scope).as_slice()).bind(generation.get().to_string())
                .fetch_optional(&mut *tx).await?;
                    let Some((admission, boss, party)) = row else {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(());
                    };
                    let admission = admission
                        .try_into()
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    let keys = party
                        .chunks_exact(16)
                        .map(|id| {
                            let id = id
                                .try_into()
                                .map_err(|_| DurabilityError::InvalidStoredState)?;
                            let character = CharacterId::from_bytes(id)
                                .map_err(|_| DurabilityError::InvalidStoredState)?;
                            BossRewardKey::new(character, &boss)
                                .map_err(|_| DurabilityError::InvalidStoredState)
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    refund_source_admission(&mut tx, &keys, &admission).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(())
                })
            })
            .await
    }
}

async fn exact_cooldowns(
    tx: &mut Transaction<'_, Postgres>,
    keys: &[BossRewardKey],
    admission: &[u8; 16],
) -> Result<bool, DurabilityError> {
    cooldown_storage::lock_reward_roots(tx, keys)
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    for key in keys {
        if !cooldown_storage::read_boss_cooldown_data(tx, key)
            .await
            .map_err(|_| DurabilityError::InvalidStoredState)?
            .is_some_and(|state| &state.admission == admission)
        {
            return Ok(false);
        }
    }
    Ok(true)
}

async fn refund_source_admission(
    tx: &mut Transaction<'_, Postgres>,
    keys: &[BossRewardKey],
    admission: &[u8; 16],
) -> Result<(), DurabilityError> {
    cooldown_storage::lock_reward_roots(tx, keys)
        .await
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    for key in keys {
        sqlx::query(
            "DELETE FROM game_character_boss_cooldowns WHERE character_id=encode($1,'hex')::uuid
              AND boss_key=$2 AND admission=encode($3,'hex')::uuid",
        )
        .bind(key.character.as_bytes().as_slice())
        .bind(key.boss.as_str())
        .bind(admission.as_slice())
        .execute(&mut **tx)
        .await?;
    }
    transition_source_admission(tx, admission, REFUNDED).await
}

async fn transition_source_admission(
    tx: &mut Transaction<'_, Postgres>,
    admission: &[u8; 16],
    state: i16,
) -> Result<(), DurabilityError> {
    let changed = sqlx::query(
        "UPDATE game_source_boss_admissions SET state=$2
          WHERE admission=encode($1,'hex')::uuid AND state=$3",
    )
    .bind(admission.as_slice())
    .bind(state)
    .bind(RESERVED)
    .execute(&mut **tx)
    .await?;
    if changed.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(())
}

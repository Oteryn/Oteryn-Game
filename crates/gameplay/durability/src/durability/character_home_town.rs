//! C20d-W23: durable citizenship in the global Character revision chain.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_death::PlayerDeathOccurrence;
use super::character_last_position::{
    FinalLastPosition, LastPositionKey, LastPositionWrite, upsert_last_position,
};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence, numeric_u64,
    state_matches_root,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::npc_travel::{ArrivalConsume, TravelOccurrence};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};

/// P20's actual first-entry placement and the obligation it supersedes, committed with citizenship.
pub struct HomeTownRelocation {
    pub placed: FinalLastPosition,
    pub respawn: Option<PlayerDeathOccurrence>,
    pub arrival: Option<TravelOccurrence>,
}

impl DurabilityRoot {
    /// Initialize an absent town from the pinned World, or set citizenship. Every read/write
    /// proves the current session, lease, scope, node and root under the same transaction.
    /// The dispatch caller serializes this with the Character's revision sequencer.
    pub async fn character_home_town(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        default_town: u32,
        citizenship: Option<u32>,
        relocation: Option<HomeTownRelocation>,
    ) -> Result<u32, CharacterProgressionError> {
        if relocation.as_ref().is_some_and(|r| {
            citizenship.is_none()
                || (r.respawn.is_some() && r.arrival.is_some())
                || !matches!(fence.runtime_scope,
                    crate::foundation::RuntimeScopeRefV1::Channel { world_id, .. }
                        if world_id == r.placed.record.world_id)
        }) {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let default_town = i32::try_from(default_town)
            .ok()
            .filter(|id| *id > 0)
            .ok_or(CharacterProgressionError::InvalidInput)?;
        let citizenship = citizenship
            .map(|id| {
                i32::try_from(id)
                    .ok()
                    .filter(|id| *id > 0)
                    .ok_or(CharacterProgressionError::InvalidInput)
            })
            .transpose()?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let root = match assert_gameplay_fence(&mut tx, &fence, &node).await? {
                Ok(root) => root,
                Err(error) => return Ok(Err(error)),
            };
            // Crystal/Canary citizen.lua: Svargrond requires Barbarian Test questline 8.
            let citizenship = if citizenship == Some(16) {
                let progress: Option<i64> = sqlx::query_scalar(
                    "SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid \
                     AND track_key='oteryn:quest-progress/storage/41174'"
                ).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                citizenship.filter(|_| progress.is_some_and(|value| value >= 8))
            } else { citizenship };
            let before: Option<i32> = sqlx::query_scalar(
                "SELECT town_id FROM game_character_home_towns WHERE character_id=encode($1,'hex')::uuid FOR UPDATE"
            ).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let before = before.unwrap_or(default_town);
            let town = citizenship.unwrap_or(before);
            if let Some(relocation) = relocation {
                // The existing owners consume the exact obligation before the position owner
                // writes. Any missing obligation or superseding write rolls back citizenship too.
                if let Some(occurrence) = relocation.respawn
                    && !super::character_death::consume_relocated_respawn(&mut tx, fence, occurrence).await?
                {
                    return Err(DurabilityError::InvalidStoredState);
                }
                let written = if let Some(occurrence) = relocation.arrival {
                    super::npc_travel::consume_in_transaction(
                        &mut tx, &recovery, &node, fence, occurrence, relocation.placed,
                    ).await.map_err(|_| DurabilityError::InvalidStoredState)? == ArrivalConsume::Consumed
                } else {
                    upsert_last_position(
                        &mut tx, fence.character_id.as_bytes(),
                        LastPositionKey {
                            character_lease_generation: fence.character_lease_generation,
                            write_sequence: relocation.placed.write_sequence,
                        }, &relocation.placed.record,
                    ).await? == LastPositionWrite::Written
                };
                if !written {
                    return Err(DurabilityError::InvalidStoredState);
                }
            }
            // An absent row already means the pinned default. Materialization and repeated
            // citizenship are no-ops; an actual choice is one Character semantic transaction.
            if town != before {
                let state = sqlx::query(
                    "SELECT character_revision::text,profile_revision,ruleset_revision,content_revision \
                     FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE"
                ).bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?
                    .ok_or(DurabilityError::InvalidStoredState)?;
                if numeric_u64(&state, "character_revision")? != root.revision || !state_matches_root(&state, &root)
                { return Err(DurabilityError::InvalidStoredState); }
                let next = root.revision.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
                if sqlx::query(
                    "INSERT INTO game_character_home_town_receipts(character_id,game_session_id,connection_generation,character_lease_generation,scope_ownership_generation,town_before,town_after,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision) \
                     SELECT s.character_id,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,$6,$7,s.character_revision,$8::text::numeric,s.level,s.level,s.total_experience,s.total_experience,s.profile_revision,s.ruleset_revision,s.content_revision,s.simulation_revision,s.evidence_revision,s.declaration_revision,s.policy_revision,s.reward_revision FROM game_character_progression_state s WHERE s.character_id=encode($1,'hex')::uuid"
                ).bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice())
                    .bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
                    .bind(fence.scope_ownership_generation.get().to_string()).bind(before).bind(town).bind(next.to_string())
                    .execute(&mut *tx).await?.rows_affected() != 1
                { return Err(DurabilityError::InvalidStoredState); }
                for query in [
                    "UPDATE game_character_roots SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
                    "UPDATE game_character_progression_state SET character_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid AND character_revision=$3::text::numeric",
                ] {
                    if sqlx::query(query).bind(fence.character_id.as_bytes().as_slice()).bind(next.to_string())
                        .bind(root.revision.to_string()).execute(&mut *tx).await?.rows_affected() != 1
                    { return Err(DurabilityError::InvalidStoredState); }
                }
            }
            let town: i32 = sqlx::query_scalar(
                "INSERT INTO game_character_home_towns(character_id,town_id) \
                 VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT(character_id) DO UPDATE \
                 SET town_id=EXCLUDED.town_id RETURNING town_id"
            ).bind(fence.character_id.as_bytes().as_slice())
                .bind(town).fetch_one(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(town as u32))
        })).await?
    }
}

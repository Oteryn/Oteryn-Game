use super::*;
use crate::domain::CharacterId;
use crate::foundation::{GameSessionId, RuntimeScopeRefV1};

pub struct ArenaProjection {
    pub character: CharacterId,
    pub session: GameSessionId,
    pub state: MatchState,
    pub relocation_pending: bool,
    pub legacy_victory_pit: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RelocationKind {
    Entry,
    Advance,
    Victory,
    Exit,
}

impl RelocationKind {
    pub(super) fn committed(before: &MatchState, after: &MatchState) -> Option<Self> {
        match after.phase {
            1 if before.rank == 0 => Some(Self::Entry),
            1 if before.pit != after.pit => Some(Self::Advance),
            3 if before.phase != 3 => Some(Self::Victory),
            // Leave, timeout, departure and stale-run resets all project idle.
            0 if before.rank != 0 => Some(Self::Exit),
            _ => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Entry => "ENTRY",
            Self::Advance => "ADVANCE",
            Self::Victory => "VICTORY",
            Self::Exit => "EXIT",
        }
    }

    #[cfg(test)]
    const ALL: [Self; 4] = [Self::Entry, Self::Advance, Self::Victory, Self::Exit];
}

impl DurabilityRoot {
    /// The active-pit census and unacknowledged relocations in this Channel.
    /// A reconnectable session can resume its carrier during the grace window.
    pub async fn read_arena_projection_leases(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        scope: RuntimeScopeRefV1,
        generation: u64,
    ) -> Result<(i64, Vec<ArenaProjection>), DurabilityError> {
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = scope
        else {
            return Ok((0, Vec::new()));
        };
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let states = sqlx::query_as::<_, (sqlx::types::Uuid, sqlx::types::Uuid, Option<sqlx::types::Uuid>, serde_json::Value, Option<i16>, bool)>(
                "WITH pending AS (
                    SELECT p.character_id,p.game_session_id,NULL::uuid AS run_id,r.after_state AS body,p.legacy_victory_pit
                    FROM game_arena_relocations p
                    JOIN game_arena_match_receipts r USING(character_id,game_session_id,command_id)
                    JOIN game_character_arena_matches s ON s.character_id=p.character_id AND s.body=r.after_state
                    JOIN game_durability_reconnect_sessions a ON a.game_session_id=p.game_session_id
                    WHERE p.world_id=encode($1,'hex')::uuid AND p.channel_id=encode($2,'hex')::uuid
                      AND p.scope_generation=$3::text::numeric AND p.relocated_at IS NULL AND p.superseded_at IS NULL
                      AND a.session_state IN (1,2)
                    ORDER BY p.character_id LIMIT 10
                ), active AS (
                    SELECT l.character_id,l.game_session_id,l.run_id,s.body FROM game_svargrond_pit_leases l
                    JOIN game_character_arena_matches s USING(character_id)
                    WHERE l.world_id=encode($1,'hex')::uuid AND l.channel_id=encode($2,'hex')::uuid
                      AND l.scope_generation=$3::text::numeric AND s.body->>'phase' IN ('1','2')
                      AND s.body->>'pit'=l.pit::text AND s.body->>'deadline_ms'=l.expires_at_ms::text
                      AND NOT EXISTS (SELECT 1 FROM pending p WHERE p.character_id=l.character_id)
                    ORDER BY l.pit LIMIT 10
                )
                SELECT *,true FROM pending UNION ALL SELECT *,NULL::smallint,false FROM active"
            ).bind(world_id.as_bytes().as_slice())
             .bind(channel_id.as_bytes().as_slice())
             .bind(generation.to_string())
             .fetch_all(&mut *tx).await?;
            let now_ms = now(&mut tx).await?;
            let states = states.into_iter().map(|(character, session, run, body, legacy_victory_pit, relocation_pending)| {
                let state: MatchState = serde_json::from_value(body).map_err(|_| DurabilityError::InvalidStoredState)?;
                if state.phase != 0 && (&state.session != session.as_bytes() || state.scope_generation != generation)
                    || run.is_some_and(|run| run.as_bytes() != &state.run) {
                    return Err(DurabilityError::InvalidStoredState);
                }
                Ok(ArenaProjection {
                    character: CharacterId::from_bytes(*character.as_bytes()).map_err(|_| DurabilityError::InvalidStoredState)?,
                    session: GameSessionId::decode(session.as_bytes()).map_err(|_| DurabilityError::InvalidStoredState)?,
                    state,
                    relocation_pending,
                    legacy_victory_pit: legacy_victory_pit.map(u8::try_from).transpose()
                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                })
            }).collect::<Result<Vec<_>, _>>()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok((now_ms, states))
        })).await
    }

    /// Check a census row again after taking the Channel spell lane.
    pub async fn arena_projection_current(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        state: &MatchState,
    ) -> Result<bool, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let state = state.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if assert_gameplay_fence(&mut tx, &fence, &node).await?.is_err() {
                return Ok(false);
            }
            let current = load_state(&mut tx, fence).await? == state;
            let dead = state.rank > 0 && sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM game_character_death_receipts
                 WHERE character_id=encode($1,'hex')::uuid AND committed_character_revision>$2::text::numeric)"
            )
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(state.entry_revision.to_string())
            .fetch_one(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(current && !dead)
        })).await
    }

    /// Acknowledge the originating transition only after the same fenced session projects it.
    pub async fn acknowledge_arena_relocation(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        state: &MatchState,
    ) -> Result<(), DurabilityError> {
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Err(DurabilityError::CommitRejected);
        };
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let body = serde_json::to_value(state).map_err(|_| DurabilityError::InvalidStoredState)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            crate::durability::character_progression::assert_item_only_fence(&mut tx, &fence.item_fence(), &node).await?
                .map_err(|_| DurabilityError::CommitRejected)?;
            sqlx::query(
                "UPDATE game_arena_relocations p SET relocated_at=statement_timestamp()
                 FROM game_arena_match_receipts r,game_character_arena_matches s
                 WHERE (p.character_id,p.game_session_id,p.command_id)=(r.character_id,r.game_session_id,r.command_id)
                   AND p.character_id=s.character_id AND r.after_state=s.body AND s.body=$6
                   AND p.character_id=encode($1,'hex')::uuid AND p.game_session_id=encode($2,'hex')::uuid
                   AND p.world_id=encode($3,'hex')::uuid AND p.channel_id=encode($4,'hex')::uuid
                   AND p.scope_generation=$5::text::numeric AND p.relocated_at IS NULL AND p.superseded_at IS NULL"
            )
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(fence.game_session_id.as_bytes().as_slice())
            .bind(world_id.as_bytes().as_slice())
            .bind(channel_id.as_bytes().as_slice())
            .bind(fence.scope_ownership_generation.get().to_string())
            .bind(body)
            .execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await
        })).await
    }
}

pub(super) async fn record_relocation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    cause: MatchCause,
    kind: RelocationKind,
) -> Result<(), DurabilityError> {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(DurabilityError::CommitRejected);
    };
    // The Character root lock serializes transitions. A newer destination
    // replaces an unresolved one, for example a timeout after an advance.
    sqlx::query(
        "UPDATE game_arena_relocations SET superseded_at=statement_timestamp()
         WHERE character_id=encode($1,'hex')::uuid AND relocated_at IS NULL AND superseded_at IS NULL"
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .execute(&mut **tx).await?;
    sqlx::query(
        "INSERT INTO game_arena_relocations(character_id,game_session_id,command_id,world_id,channel_id,scope_generation,kind)
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7)"
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(cause.ordinal())
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(kind.label())
    .execute(&mut **tx).await?;
    Ok(())
}

#[cfg(test)]
#[path = "relocation_tests.rs"]
mod tests;

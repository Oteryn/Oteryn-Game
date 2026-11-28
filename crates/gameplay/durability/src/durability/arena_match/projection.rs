use super::*;
use crate::domain::CharacterId;
use crate::durability::db::commit_semantic_read;
use crate::foundation::{GameSessionId, RuntimeScopeRefV1};

pub struct ArenaProjection {
    pub character: CharacterId,
    pub session: GameSessionId,
    pub state: MatchState,
    pub relocation_pending: bool,
    pub legacy_victory_pit: Option<u8>,
}

macro_rules! relocation_kinds {
    ($($kind:ident => $label:literal),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub(super) enum RelocationKind { $($kind),+ }
        impl RelocationKind {
            fn label(self) -> &'static str {
                match self { $(Self::$kind => $label),+ }
            }

            #[cfg(test)]
            const ALL: &'static [Self] = &[$(Self::$kind),+];
        }
    };
}

relocation_kinds! {
    Entry => "ENTRY",
    Advance => "ADVANCE",
    Victory => "VICTORY",
    Exit => "EXIT",
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
}

impl DurabilityRoot {
    /// The active-pit census and unacknowledged relocations in this Channel.
    /// Only admitted and positioned sessions can consume the pending batch.
    /// A cursor advances the pending batch independently of the active pits.
    pub async fn read_arena_projection_leases(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        scope: RuntimeScopeRefV1,
        generation: u64,
        positioned: &[GameSessionId],
        after_character: Option<CharacterId>,
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
        let positioned: Vec<_> = positioned
            .iter()
            .map(|session| sqlx::types::Uuid::from_bytes(*session.as_bytes()))
            .collect();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let states = sqlx::query_as::<_, (sqlx::types::Uuid, sqlx::types::Uuid, Option<sqlx::types::Uuid>, serde_json::Value, Option<i16>, bool)>(
                "WITH pending AS (
                    SELECT p.character_id,a.game_session_id,NULL::uuid AS run_id,s.body,p.legacy_victory_pit
                    FROM game_arena_relocations p
                    JOIN game_arena_match_receipts r USING(character_id,game_session_id,command_id)
                    JOIN game_character_arena_matches s ON s.character_id=p.character_id
                      AND s.body - ARRAY['session','scope_generation']=r.after_state - ARRAY['session','scope_generation']
                    JOIN game_durability_reconnect_sessions a ON a.character_id=p.character_id
                      AND a.runtime_scope_kind=1 AND a.runtime_scope_world_id=p.world_id
                      AND a.runtime_scope_channel_id=p.channel_id AND a.runtime_scope_house_key IS NULL
                      AND a.scope_ownership_generation=$3::text::numeric
                    WHERE p.world_id=encode($1,'hex')::uuid AND p.channel_id=encode($2,'hex')::uuid
                      AND p.scope_generation<=$3::text::numeric AND p.relocated_at IS NULL AND p.superseded_at IS NULL
                      AND a.session_state=2 AND a.current_transport_ref IS NOT NULL AND a.game_session_id=ANY($4)
                      AND ($6::uuid IS NULL OR p.character_id>$6)
                    ORDER BY p.character_id LIMIT $5
                ), active AS (
                    SELECT l.character_id,l.game_session_id,l.run_id,s.body FROM game_svargrond_pit_leases l
                    JOIN game_character_arena_matches s USING(character_id)
                    WHERE l.world_id=encode($1,'hex')::uuid AND l.channel_id=encode($2,'hex')::uuid
                      AND l.scope_generation=$3::text::numeric AND s.body->>'phase' IN ('1','2')
                      AND s.body->>'pit'=l.pit::text AND s.body->>'deadline_ms'=l.expires_at_ms::text
                      AND NOT EXISTS (SELECT 1 FROM pending p WHERE p.character_id=l.character_id)
                    ORDER BY l.pit LIMIT $5
                )
                SELECT *,true FROM pending UNION ALL SELECT *,NULL::smallint,false FROM active"
            ).bind(world_id.as_bytes().as_slice())
             .bind(channel_id.as_bytes().as_slice())
             .bind(generation.to_string())
             .bind(positioned)
             .bind(i64::try_from(donor::CENTERS.len()).map_err(|_| DurabilityError::InvalidStoredState)?)
             .bind(after_character.map(|character| sqlx::types::Uuid::from_bytes(*character.as_bytes())))
             .fetch_all(&mut *tx).await?;
            let now_ms = now(&mut tx).await?;
            let states = states.into_iter().map(|(character, session, run, body, legacy_victory_pit, relocation_pending)| {
                let state: MatchState = serde_json::from_value(body).map_err(|_| DurabilityError::InvalidStoredState)?;
                if !relocation_pending && state.phase != 0
                    && (&state.session != session.as_bytes() || state.scope_generation != generation)
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
            commit_semantic_read(tx, deadline).await?;
            Ok((now_ms, states))
        })).await
    }

    /// Re-prove the pending transition under the current admitted owner. When
    /// ownership changed, record the new session/scope and reacquire its pit
    /// through the same custody writer. The original relocation key stays put
    /// unless a committed death has ended the fight.
    pub async fn prepare_arena_relocation(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        snapshot: &MatchState,
    ) -> Result<Option<MatchState>, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let snapshot = snapshot.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if super::super::character_progression::assert_item_only_fence(&mut tx, &fence.item_fence(), &node).await?.is_err() {
                return Ok(None);
            }
            let admitted: bool = sqlx::query_scalar(
                "SELECT session_state=2 AND current_transport_ref IS NOT NULL
                 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid"
            ).bind(fence.game_session_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            if !admitted {
                return Ok(None);
            }
            let current = load_state(&mut tx, fence).await?;
            if current != snapshot
                || !pending_relocation(&mut tx, fence, &current).await?
            {
                return Ok(None);
            }
            if dead_since_entry(&mut tx, fence.character_id, &current).await? {
                pit_lease::release(&mut tx, fence).await?;
                supersede_pending_relocation(&mut tx, fence.character_id).await?;
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(None);
            }
            let mut rebound = current.clone();
            if current.rank > 0 {
                rebound.session = *fence.game_session_id.as_bytes();
                rebound.scope_generation = fence.scope_ownership_generation.get();
            }
            if rebound != current {
                let now_ms = now(&mut tx).await?;
                if rebound.projection_remaining_ms(now_ms).is_some()
                    && !pit_lease::claim(&mut tx, fence, &rebound, now_ms).await?
                {
                    return Ok(None);
                }
                let reply = oteryn_protocol_oteryn::arena::encode_match_result(&ArenaResult {
                    disposition: ArenaDisposition::Applied,
                    state: rebound.wire(now_ms, claimed_masks(&mut tx, fence).await?),
                }).map_err(|_| DurabilityError::InvalidStoredState)?;
                // Wire commands and native callback causes occupy the first
                // two u64 ranges. Each admitted session can adopt once per generation.
                let ordinal = 2 * u128::from(u64::MAX) + u128::from(rebound.scope_generation);
                let payload = oteryn_protocol_oteryn::arena::encode_match_request(
                    oteryn_protocol_oteryn::arena::ArenaAction::Status,
                ).map_err(|_| DurabilityError::InvalidStoredState)?;
                save_receipt(&mut tx, fence, fence.game_session_id, ordinal.to_string(), &payload,
                    &current, &rebound, &reply, None).await?;
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Some(rebound))
        })).await
    }

    /// A freshly attached pit actor can be kicked only after its current
    /// owner proves it has neither a pending relocation nor live custody.
    pub async fn arena_orphan_can_leave(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<bool, DurabilityError> {
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
                    if super::super::character_progression::assert_item_only_fence(
                        &mut tx,
                        &fence.item_fence(),
                        &node,
                    )
                    .await?
                    .is_err()
                    {
                        return Ok(false);
                    }
                    let state = load_state(&mut tx, fence).await?;
                    let now_ms = now(&mut tx).await?;
                    let can_leave = !pending_relocation(&mut tx, fence, &state).await?
                        && !pit_lease::owned(&mut tx, fence, &state, now_ms).await?;
                    commit_semantic_read(tx, deadline).await?;
                    Ok(can_leave)
                })
            })
            .await
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
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if assert_gameplay_fence(&mut tx, &fence, &node)
                        .await?
                        .is_err()
                    {
                        return Ok(false);
                    }
                    let current = load_state(&mut tx, fence).await? == state;
                    let dead = dead_since_entry(&mut tx, fence.character_id, &state).await?;
                    commit_semantic_read(tx, deadline).await?;
                    Ok(current && !dead)
                })
            })
            .await
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
                   AND p.character_id=s.character_id AND s.body=$6
                   AND s.body - ARRAY['session','scope_generation']=r.after_state - ARRAY['session','scope_generation']
                   AND p.character_id=encode($1,'hex')::uuid
                   AND (s.body->>'phase'='0' OR (s.body->'session'=$2 AND s.body->>'scope_generation'=$5))
                   AND p.world_id=encode($3,'hex')::uuid AND p.channel_id=encode($4,'hex')::uuid
                   AND p.scope_generation<=$5::text::numeric AND p.relocated_at IS NULL AND p.superseded_at IS NULL"
            )
            .bind(fence.character_id.as_bytes().as_slice())
            .bind(serde_json::json!(fence.game_session_id.as_bytes()))
            .bind(world_id.as_bytes().as_slice())
            .bind(channel_id.as_bytes().as_slice())
            .bind(fence.scope_ownership_generation.get().to_string())
            .bind(body)
            .execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await
        })).await
    }
}

pub(super) async fn pending_relocation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    state: &MatchState,
) -> Result<bool, DurabilityError> {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    let body = serde_json::to_value(state).map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_arena_relocations p
            JOIN game_arena_match_receipts r USING(character_id,game_session_id,command_id)
            WHERE p.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid
              AND p.channel_id=encode($3,'hex')::uuid AND p.scope_generation<=$4::text::numeric
              AND p.relocated_at IS NULL AND p.superseded_at IS NULL
              AND r.after_state - ARRAY['session','scope_generation']=$5::jsonb - ARRAY['session','scope_generation'])"
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(body)
    .fetch_one(&mut **tx).await?)
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
    supersede_pending_relocation(tx, fence.character_id).await?;
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

async fn supersede_pending_relocation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: CharacterId,
) -> Result<(), DurabilityError> {
    sqlx::query(
        "UPDATE game_arena_relocations SET superseded_at=statement_timestamp()
         WHERE character_id=encode($1,'hex')::uuid AND relocated_at IS NULL AND superseded_at IS NULL"
    )
    .bind(character.as_bytes().as_slice())
    .execute(&mut **tx).await?;
    Ok(())
}

#[cfg(test)]
#[path = "relocation_tests.rs"]
mod tests;

//! M51 admission for physical pit reset. Crystal201189f
//! lib/quests/svargrond_arena.lua:193-246; movements_arena_enter.lua:22-30.
//! A successful read is a snapshot, never a retained write grant. Retirement
//! must call the same validation again in its deciding transaction.
use super::*;

impl DurabilityRoot {
    pub async fn read_arena_pit_reset(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        expected: &MatchState,
    ) -> Result<bool, CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let expected = expected.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Err(error) = assert_gameplay_fence(&mut tx, &fence, &node).await? {
                        return Ok(Err(error));
                    }
                    let accepted = lock_reset_custody(&mut tx, fence, &expected).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(accepted))
                })
            })
            .await?
    }
}

/// Caller first holds the current Character/node/scope fence and root lock.
/// Keep the exact arena and pit rows locked through physical retirement; this
/// helper never claims, refreshes or releases custody. A detached read is not
/// authority for a later transaction.
pub(in crate::durability) async fn lock_reset_custody(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    expected: &MatchState,
) -> Result<bool, DurabilityError> {
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(false);
    };
    let state = load_state(tx, fence).await?;
    if &state != expected
        || state.phase != 1
        || !(1..=3).contains(&state.rank)
        || !(1..=10).contains(&state.pit)
        || state.entry_revision == 0
        || state.session != *fence.game_session_id.as_bytes()
        || state.scope_generation != fence.scope_ownership_generation.get()
    {
        return Ok(false);
    }
    let held: Option<i32> = sqlx::query_scalar("SELECT 1 FROM game_svargrond_pit_leases WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND pit=$3 AND character_id=encode($4,'hex')::uuid AND game_session_id=encode($5,'hex')::uuid AND run_id=encode($6,'hex')::uuid AND scope_generation=$7::text::numeric AND expires_at_ms=$8 FOR UPDATE")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(i16::from(state.pit)).bind(fence.character_id.as_bytes().as_slice())
        .bind(state.session.as_slice()).bind(state.run.as_slice())
        .bind(state.scope_generation.to_string()).bind(state.deadline_ms)
        .fetch_optional(&mut **tx).await?;
    if held.is_none() {
        return Ok(false);
    }
    let dead: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_death_receipts WHERE character_id=encode($1,'hex')::uuid AND committed_character_revision>$2::text::numeric)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(state.entry_revision.to_string())
        .fetch_one(&mut **tx).await?;
    // Read the deciding clock after acquiring both rows, never before a lock wait.
    Ok(!dead && state.deadline_ms > now(tx).await?)
}

#[cfg(test)]
#[path = "pit_reset_tests.rs"]
mod tests;

//! M51 terminal session custody cleanup. Crystal201189f
//! data-global/lib/quests/svargrond_arena.lua:259-313.
use super::*;
use crate::durability::item_transfer::CurrentCharacterItemFence;

pub(in crate::durability) async fn release_terminal_pit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterItemFence,
) -> Result<(), DurabilityError> {
    let crate::foundation::RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Ok(());
    };
    // Called after the deciding session UPDATE, in that same transaction. The
    // terminal row is the guard; a historical snapshot alone cannot release it.
    sqlx::query("DELETE FROM game_svargrond_pit_leases p USING game_durability_reconnect_sessions s WHERE p.game_session_id=s.game_session_id AND p.character_id=s.character_id AND p.world_id=s.world_id AND p.channel_id=s.runtime_scope_channel_id AND p.scope_generation=s.scope_ownership_generation AND s.game_session_id=encode($1,'hex')::uuid AND s.character_id=encode($2,'hex')::uuid AND s.world_id=encode($3,'hex')::uuid AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=s.world_id AND s.runtime_scope_channel_id=encode($4,'hex')::uuid AND s.runtime_scope_house_key IS NULL AND s.current_generation=$5::text::numeric AND s.character_lease_generation=$6::text::numeric AND s.scope_ownership_generation=$7::text::numeric AND s.session_state=3")
        .bind(fence.game_session_id.as_bytes().as_slice())
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(fence.connection_generation.get().to_string())
        .bind(fence.character_lease_generation.to_string())
        .bind(fence.scope_ownership_generation.get().to_string())
        .execute(&mut **tx).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bestiary_postgres_harness as h;

    #[test]
    fn m51_postgres_terminal_release_is_atomic_and_cannot_clear_a_new_session() -> h::TestResult {
        let Some(admin) = h::configured_admin() else {
            return Ok(());
        };
        h::runtime()?.block_on(async move {
            let started = std::time::Instant::now();
            let fixture = h::Harness::create(admin, "srvenc_m51_terminal", true).await?;
            let result: h::TestResult = async {
                let fence = h::fence(1)?;
                let mut tx = fixture.pool.begin().await?;
                let now = super::super::now(&mut tx).await?;
                let state = MatchState {
                    rank: 1, pit: 1, phase: 1, deadline_ms: now + 600_000,
                    session: *fence.game_session_id.as_bytes(),
                    scope_generation: fence.scope_ownership_generation.get(),
                    run: h::id(230), ..MatchState::default()
                };
                assert!(super::super::pit_lease::claim(&mut tx, fence, &state, now).await?);
                tx.commit().await?;
                let mut active = fixture.pool.begin().await?;
                release_terminal_pit(&mut active, fence.item_fence()).await?;
                assert!(super::super::pit_lease::owned(&mut active, fence, &state, now).await?, "an active session cannot be cleaned up");
                active.rollback().await?;
                for commit in [false, true] {
                    let mut tx = fixture.pool.begin().await?;
                    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3,current_transport_ref=NULL,prepared_attempt_ref=NULL WHERE game_session_id=encode($1,'hex')::uuid")
                        .bind(state.session.as_slice()).execute(&mut *tx).await?;
                    release_terminal_pit(&mut tx, fence.item_fence()).await?;
                    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_svargrond_pit_leases").fetch_one(&mut *tx).await?;
                    assert_eq!(count, 0, "terminal state and custody release share one transaction");
                    if commit { tx.commit().await?; } else {
                        tx.rollback().await?;
                        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_svargrond_pit_leases").fetch_one(&fixture.pool).await?;
                        assert_eq!(count, 1, "rollback must retain custody");
                    }
                }
                let mut next_fence = fence;
                next_fence.game_session_id = crate::foundation::GameSessionId::decode(&h::id(51)).map_err(h::debug)?;
                let mut next = state.clone();
                next.session = *next_fence.game_session_id.as_bytes();
                next.run = h::id(231);
                let mut tx = fixture.pool.begin().await?;
                assert!(super::super::pit_lease::claim(&mut tx, next_fence, &next, now).await?);
                release_terminal_pit(&mut tx, fence.item_fence()).await?;
                assert!(super::super::pit_lease::owned(&mut tx, next_fence, &next, now).await?, "old terminal replay cannot clear a different session/run");
                tx.commit().await?;
                Ok(())
            }.await;
            fixture.cleanup().await?;
            result?;
            eprintln!("M51 REAL POSTGRES terminal custody cleanup duration={:?}", started.elapsed());
            assert!(started.elapsed().as_secs_f64() > 0.01);
            Ok(())
        })
    }
}

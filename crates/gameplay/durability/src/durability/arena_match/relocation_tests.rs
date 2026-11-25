use super::*;
use crate::bestiary_postgres_harness as h;
use oteryn_protocol_oteryn::arena::{self as wire, ArenaAction};

#[test]
fn arena_recovery_covers_every_committed_transition_kind() -> h::TestResult {
    let Some(admin) = h::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            h::runtime()
                .and_then(|runtime| runtime.block_on(scenario(admin)))
                .map_err(h::debug)
        })?
        .join()
        .map_err(|_| "arena recovery kinds scenario panicked")??;
    Ok(())
}

async fn scenario(admin: String) -> h::TestResult {
    let fixture = h::Harness::create(admin, "arena_relocation_kinds", true).await?;
    eprintln!("arenapkg-marker: arena_recovery_covers_every_committed_transition_kind db ready");
    let result = async {
        let seal = fixture.recovery.seal_current().map_err(h::debug)?;
        let authority = fixture
            .root
            .open_character_authority(&seal)
            .await
            .map_err(h::debug)?;
        let fence = h::fence(1)?;
        let transport: Vec<u8> = sqlx::query_scalar(
            "SELECT current_transport_ref FROM game_durability_reconnect_sessions
             WHERE game_session_id=encode($1,'hex')::uuid",
        )
        .bind(fence.game_session_id.as_bytes().as_slice())
        .fetch_one(&fixture.pool)
        .await?;
        for (offset, kind) in RelocationKind::ALL.into_iter().enumerate() {
            let mut tx = fixture.pool.begin().await?;
            let clock = now(&mut tx).await?;
            let fighting = MatchState {
                rank: 1,
                pit: 1,
                phase: 1,
                run: h::id(230),
                deadline_ms: clock + i64::from(donor::PIT_SECONDS) * 1000,
                entry_revision: 1,
                session: *fence.game_session_id.as_bytes(),
                scope_generation: fence.scope_ownership_generation.get(),
                ..MatchState::default()
            };
            let ready = MatchState {
                phase: 2,
                ..fighting.clone()
            };
            let (action, before, after) = match kind {
                RelocationKind::Entry => (ArenaAction::Enter, MatchState::default(), fighting),
                RelocationKind::Advance => (
                    ArenaAction::Advance,
                    ready,
                    MatchState { pit: 2, ..fighting },
                ),
                RelocationKind::Victory => (
                    ArenaAction::Advance,
                    MatchState {
                        pit: u8::try_from(donor::CENTERS.len())?,
                        ..ready
                    },
                    MatchState {
                        completed: 1,
                        pit: 0,
                        phase: 3,
                        deadline_ms: 0,
                        ..fighting
                    },
                ),
                RelocationKind::Exit => (ArenaAction::Leave, fighting.clone(), fighting.idle()),
            };
            assert_eq!(RelocationKind::committed(&before, &after), Some(kind));
            let command = |id| {
                Ok::<_, Box<dyn std::error::Error>>(CommandRef::new(
                    fence.game_session_id,
                    crate::foundation::CommandId::new(id).map_err(h::debug)?,
                ))
            };
            let id = u64::try_from(offset)? * 2 + 1;
            let predecessor = wire::encode_match_result(&ArenaResult {
                disposition: ArenaDisposition::Applied,
                state: before.wire(clock, [0; 3]),
            })
            .map_err(h::debug)?;
            let reply = wire::encode_match_result(&ArenaResult {
                disposition: ArenaDisposition::Applied,
                state: after.wire(clock, [0; 3]),
            })
            .map_err(h::debug)?;
            sqlx::query("DELETE FROM game_character_arena_matches")
                .execute(&mut *tx)
                .await?;
            save_origin(
                &mut tx,
                fence,
                command(id)?,
                &wire::encode_match_request(ArenaAction::Status).map_err(h::debug)?,
                &before,
                &before,
                &predecessor,
                None,
            )
            .await?;
            save_origin(
                &mut tx,
                fence,
                command(id + 1)?,
                &wire::encode_match_request(action).map_err(h::debug)?,
                &before,
                &after,
                &reply,
                None,
            )
            .await?;
            tx.commit().await?;

            let read = || {
                fixture.root.read_arena_projection_leases(
                    &authority,
                    fence.runtime_scope,
                    fence.scope_ownership_generation.get(),
                )
            };
            let set_session_state = |state: i16| {
                sqlx::query(
                    "UPDATE game_durability_reconnect_sessions
                     SET session_state=$2,current_transport_ref=CASE WHEN $2=1 THEN $3::bytea END
                     WHERE game_session_id=encode($1,'hex')::uuid",
                )
                .bind(fence.game_session_id.as_bytes().as_slice())
                .bind(state)
                .bind(&transport)
                .execute(&fixture.pool)
            };
            for session_state in [1, 2] {
                assert_eq!(set_session_state(session_state).await?.rows_affected(), 1);
                let (_, pending) = read().await?;
                assert_eq!(pending.len(), 1, "{kind:?} without a relocation receipt");
                assert_eq!(pending[0].character, fence.character_id);
                assert_eq!(pending[0].session, fence.game_session_id);
                assert!(pending[0].relocation_pending);
                assert_eq!(pending[0].state, after);
            }
            assert_eq!(set_session_state(1).await?.rows_affected(), 1);
            fixture
                .root
                .acknowledge_arena_relocation(&authority, &fixture.node, fence, &after)
                .await?;
            assert!(
                read().await?.1.is_empty(),
                "{kind:?} with a relocation receipt"
            );
        }
        Ok(())
    }
    .await;
    fixture.cleanup().await?;
    result
}

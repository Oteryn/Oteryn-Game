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
        for (offset, kind) in RelocationKind::ALL.iter().copied().enumerate() {
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
                    std::slice::from_ref(&fence.game_session_id),
                    None,
                )
            };
            let set_session_state = |state: i16| {
                sqlx::query(
                    "UPDATE game_durability_reconnect_sessions
                     SET session_state=$2,current_transport_ref=CASE WHEN $2=2 THEN $3::bytea END
                     WHERE game_session_id=encode($1,'hex')::uuid",
                )
                .bind(fence.game_session_id.as_bytes().as_slice())
                .bind(state)
                .bind(&transport)
                .execute(&fixture.pool)
            };
            assert_eq!(set_session_state(1).await?.rows_affected(), 1);
            assert!(read().await?.1.is_empty(), "{kind:?} waits for readmission");
            assert_eq!(set_session_state(2).await?.rows_affected(), 1);
            let (_, pending) = read().await?;
            assert_eq!(pending.len(), 1, "{kind:?} without a relocation receipt");
            assert_eq!(pending[0].character, fence.character_id);
            assert_eq!(pending[0].session, fence.game_session_id);
            assert!(pending[0].relocation_pending);
            assert_eq!(pending[0].state, after);
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

#[test]
fn arena_recovery_reaches_admitted_relocation_past_grace_sessions() -> h::TestResult {
    let Some(admin) = h::configured_admin() else {
        return Ok(());
    };
    h::runtime()?.block_on(async {
        let fixture = h::Harness::create(admin, "arena_relocation_grace", true).await?;
        eprintln!("arenapkg-marker: arena_recovery_reaches_admitted_relocation_past_grace_sessions db ready");
        let result = grace_sessions(&fixture).await;
        fixture.cleanup().await?;
        result
    })
}

async fn grace_sessions(fixture: &h::Harness) -> h::TestResult {
    let fence = h::fence(1)?;
    let seal = fixture.recovery.seal_current().map_err(h::debug)?;
    let authority = fixture
        .root
        .open_character_authority(&seal)
        .await
        .map_err(h::debug)?;
    let mut tx = fixture.pool.begin().await?;
    let clock = now(&mut tx).await?;
    let mut positioned = vec![fence.game_session_id];
    sqlx::query(
        "UPDATE game_durability_reconnect_sessions SET session_state=2
        WHERE game_session_id=encode($1,'hex')::uuid",
    )
    .bind(fence.game_session_id.as_bytes().as_slice())
    .execute(&mut *tx)
    .await?;
    // The additional accounts only supply persisted grace sessions to the
    // census. The admitted Character still uses the standard fenced fixture.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for seed in 1..=u8::try_from(donor::CENTERS.len())? {
        let character = h::id(seed);
        let account = h::id(seed + 60);
        let session = h::id(seed + 100);
        sqlx::query("INSERT INTO game_character_roots
            (character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name,sex)
            SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,world_id,lifecycle,character_revision,
                profile_revision,ruleset_revision,content_revision,starter_template_revision,$3,sex
            FROM game_character_roots WHERE character_id=encode($4,'hex')::uuid")
            .bind(character.as_slice()).bind(account.as_slice())
            .bind(format!("Grace Arena {}", char::from(b'A' + seed - 1)))
            .bind(fence.character_id.as_bytes().as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_durability_reconnect_sessions
            (game_session_id,account_id,character_id,world_id,runtime_scope_kind,runtime_scope_world_id,runtime_scope_channel_id,
             control_loss_epoch,original_grace_deadline,predecessor_generation,character_lease_generation,scope_ownership_generation,current_generation,session_state)
            SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,world_id,runtime_scope_kind,
                runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,original_grace_deadline,predecessor_generation,
                character_lease_generation,scope_ownership_generation,current_generation,1
            FROM game_durability_reconnect_sessions WHERE game_session_id=encode($4,'hex')::uuid")
            .bind(session.as_slice()).bind(account.as_slice()).bind(character.as_slice())
            .bind(fence.game_session_id.as_bytes().as_slice()).execute(&mut *tx).await?;
        positioned.push(GameSessionId::decode(&session).map_err(h::debug)?);
    }
    sqlx::query("SET LOCAL session_replication_role=origin")
        .execute(&mut *tx)
        .await?;
    for (offset, &session) in positioned.iter().enumerate() {
        let target = if offset == 0 {
            fence
        } else {
            CurrentCharacterGameplayFence {
                game_session_id: session,
                character_id: CharacterId::from_bytes(h::id(u8::try_from(offset)?))
                    .map_err(h::debug)?,
                ..fence
            }
        };
        let before = MatchState::default();
        let after = MatchState {
            rank: 1,
            pit: 1,
            phase: 1,
            run: *session.as_bytes(),
            deadline_ms: clock + 1000,
            entry_revision: 1,
            session: *session.as_bytes(),
            scope_generation: target.scope_ownership_generation.get(),
            ..before.clone()
        };
        let payload = wire::encode_match_request(ArenaAction::Enter).map_err(h::debug)?;
        let reply = wire::encode_match_result(&ArenaResult {
            disposition: ArenaDisposition::Applied,
            state: after.wire(clock, [0; 3]),
        })
        .map_err(h::debug)?;
        save_origin(
            &mut tx,
            target,
            CommandRef::new(
                session,
                crate::foundation::CommandId::new(1).map_err(h::debug)?,
            ),
            &payload,
            &before,
            &after,
            &reply,
            None,
        )
        .await?;
    }
    tx.commit().await?;
    let (_, pending) = fixture
        .root
        .read_arena_projection_leases(
            &authority,
            fence.runtime_scope,
            fence.scope_ownership_generation.get(),
            &positioned,
            None,
        )
        .await?;
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].session, fence.game_session_id);
    assert!(pending[0].relocation_pending);
    Ok(())
}

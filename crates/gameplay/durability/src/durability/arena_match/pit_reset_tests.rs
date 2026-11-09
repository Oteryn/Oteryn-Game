use super::*;
use crate::bestiary_postgres_harness as h;

#[test]
fn m51_postgres_pit_reset_requires_existing_current_custody_without_reclaiming() -> h::TestResult {
    let Some(admin) = h::configured_admin() else {
        return Ok(());
    };
    h::runtime()?.block_on(async move {
        let started = std::time::Instant::now();
        let fixture = h::Harness::create(admin, "srvenc_m51_reset_custody", true).await?;
        let result: h::TestResult = async {
            let seal = fixture.recovery.seal_current().map_err(h::debug)?;
            let authority = fixture
                .root
                .open_character_authority(&seal)
                .await
                .map_err(h::debug)?;
            let fence = h::fence(1)?;
            let mut tx = fixture.pool.begin().await?;
            let clock = now(&mut tx).await?;
            let state = MatchState {
                rank: 1,
                pit: 1,
                phase: 1,
                run: h::id(230),
                deadline_ms: clock + 5000,
                entry_revision: 1,
                session: *fence.game_session_id.as_bytes(),
                scope_generation: fence.scope_ownership_generation.get(),
                ..MatchState::default()
            };
            let command = CommandRef::new(
                fence.game_session_id,
                crate::foundation::CommandId::new(93).map_err(h::debug)?,
            );
            let payload = oteryn_protocol_oteryn::arena::encode_match_request(
                oteryn_protocol_oteryn::arena::ArenaAction::Status,
            )
            .map_err(h::debug)?;
            let reply = oteryn_protocol_oteryn::arena::encode_match_result(&ArenaResult {
                disposition: ArenaDisposition::Applied,
                state: state.wire(clock, [0; 3]),
            })
            .map_err(h::debug)?;
            save_origin(
                &mut tx, fence, command, &payload, &state, &state, &reply, None,
            )
            .await?;
            tx.commit().await?;
            assert!(
                !fixture
                    .root
                    .read_arena_pit_reset(&authority, &fixture.node, fence, &state)
                    .await
                    .map_err(h::debug)?,
                "a saved phase alone must not create or substitute for pit custody"
            );
            let mut tx = fixture.pool.begin().await?;
            assert!(pit_lease::claim(&mut tx, fence, &state, clock).await?);
            tx.commit().await?;
            assert!(
                fixture
                    .root
                    .read_arena_pit_reset(&authority, &fixture.node, fence, &state)
                    .await
                    .map_err(h::debug)?
            );
            let mut wrong = state.clone();
            wrong.run = h::id(231);
            assert!(
                !fixture
                    .root
                    .read_arena_pit_reset(&authority, &fixture.node, fence, &wrong)
                    .await
                    .map_err(h::debug)?
            );
            wrong = state.clone();
            wrong.deadline_ms += 1;
            assert!(
                !fixture
                    .root
                    .read_arena_pit_reset(&authority, &fixture.node, fence, &wrong)
                    .await
                    .map_err(h::debug)?
            );
            sqlx::query("SELECT pg_sleep(GREATEST(0,($1-floor(extract(epoch FROM clock_timestamp())*1000))/1000.0)+0.01)")
                .bind(state.deadline_ms).execute(&fixture.pool).await?;
            assert!(!fixture.root.read_arena_pit_reset(&authority, &fixture.node, fence, &state).await.map_err(h::debug)?,
                "an exact retained lease cannot authorize cleanup after its original deadline");
            let mut tx = fixture.pool.begin().await?;
            pit_lease::release(&mut tx, fence).await?;
            tx.commit().await?;
            assert!(
                !fixture
                    .root
                    .read_arena_pit_reset(&authority, &fixture.node, fence, &state)
                    .await
                    .map_err(h::debug)?,
                "a released lease must not be reclaimed by reset preflight"
            );
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_svargrond_pit_leases")
                .fetch_one(&fixture.pool)
                .await?;
            assert_eq!(count, 0);
            Ok(())
        }
        .await;
        fixture.cleanup().await?;
        result?;
        eprintln!(
            "M51 REAL POSTGRES pit reset custody duration={:?}",
            started.elapsed()
        );
        assert!(started.elapsed().as_secs_f64() > 0.01);
        Ok(())
    })
}

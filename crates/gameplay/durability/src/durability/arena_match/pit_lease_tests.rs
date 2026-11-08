//! M51: real PostgreSQL empty-pit race and reservation rollback.
use super::*;
use crate::bestiary_postgres_harness as h;

#[test]
fn m51_postgres_pit_reservation_serializes_contenders_and_releases_atomically() -> h::TestResult {
    let Some(admin) = h::configured_admin() else {
        return Ok(());
    };
    h::runtime()?.block_on(async move {
        let started = std::time::Instant::now();
        let fixture = h::Harness::create(admin, "srvenc_m51_pit_lease", true).await?;
        let result: h::TestResult = async {
            let fence = h::fence(1)?;
            let mut first = fixture.pool.begin().await?;
            let clock = super::super::now(&mut first).await?;
            let state = MatchState {
                rank: 1,
                pit: 1,
                phase: 1,
                deadline_ms: clock + 600_000,
                session: *fence.game_session_id.as_bytes(),
                scope_generation: fence.scope_ownership_generation.get(),
                run: h::id(230),
                ..MatchState::default()
            };
            assert!(claim(&mut first, fence, &state, clock).await?);
            let mut contender = state.clone();
            contender.run = h::id(231);
            let pool = fixture.pool.clone();
            let task = tokio::spawn(async move {
                let mut tx = pool.begin().await?;
                let accepted = claim(&mut tx, fence, &contender, clock).await?;
                tx.commit().await?;
                Ok::<_, DurabilityError>(accepted)
            });
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            assert!(
                !task.is_finished(),
                "empty-pit admission must retain its transaction lock"
            );
            first.commit().await?;
            assert!(
                !task.await??,
                "another live run cannot steal committed custody"
            );
            let owner: sqlx::types::Uuid =
                sqlx::query_scalar("SELECT run_id FROM game_svargrond_pit_leases")
                    .fetch_one(&fixture.pool)
                    .await?;
            assert_eq!(*owner.as_bytes(), state.run);
            let mut move_tx = fixture.pool.begin().await?;
            let mut next = state.clone();
            next.pit = 2;
            assert!(claim(&mut move_tx, fence, &next, clock).await?);
            move_tx.rollback().await?;
            let pit: i16 = sqlx::query_scalar("SELECT pit FROM game_svargrond_pit_leases")
                .fetch_one(&fixture.pool)
                .await?;
            assert_eq!(pit, 1, "failed composition must retain the old pit");
            let mut leave = fixture.pool.begin().await?;
            release(&mut leave, fence).await?;
            leave.commit().await?;
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
            "M51 REAL POSTGRES pit custody race/rollback/release duration={:?}",
            started.elapsed()
        );
        assert!(started.elapsed().as_secs_f64() > 0.01);
        Ok(())
    })
}

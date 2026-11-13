// M51: the actual fenced death writer must release the pit in its own commit.
// Crystal201189f data-global/lib/quests/svargrond_arena.lua:259-313.
#[test]
fn m51_postgres_character_death_releases_pit_without_clearing_a_later_run() -> TestResult {
    pubproj_threaded(|| {
        run("srvenc_m51_death_cleanup", async |h| {
            let started = std::time::Instant::now();
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            async fn reserve(h: &Harness, run: u8) -> TestResult {
                sqlx::query("INSERT INTO game_svargrond_pit_leases(world_id,channel_id,pit,character_id,game_session_id,run_id,scope_generation,expires_at_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,1,floor(extract(epoch FROM clock_timestamp())*1000)::bigint+600000)")
                    .bind(id(harness::WORLD).as_slice()).bind(id(harness::CHANNEL).as_slice())
                    .bind(id(harness::CHARACTER).as_slice()).bind(id(harness::SESSION).as_slice())
                    .bind(id(run).as_slice()).execute(&h.pool).await?;
                Ok(())
            }
            reserve(h, 130).await?;
            let death = request(131)?;
            assert!(matches!(
                h.root
                    .commit_character_death(&authority, &h.node, fence(2)?, death.clone())
                    .await,
                Err(CharacterProgressionError::CharacterRevisionMismatch)
            ));
            assert_eq!(
                h.count("game_svargrond_pit_leases").await?,
                1,
                "a refused death cannot release custody"
            );
            assert!(matches!(
                h.root
                    .commit_character_death(&authority, &h.node, fence(1)?, death.clone())
                    .await
                    .map_err(debug)?,
                CharacterDeathOutcome::Committed(_)
            ));
            assert_eq!(
                h.count("game_svargrond_pit_leases").await?,
                0,
                "committed death must release its pit immediately"
            );
            reserve(h, 132).await?;
            assert!(matches!(
                h.root
                    .commit_character_death(&authority, &h.node, fence(1)?, death)
                    .await
                    .map_err(debug)?,
                CharacterDeathOutcome::AlreadyCommitted(_)
            ));
            let retained: sqlx::types::Uuid =
                sqlx::query_scalar("SELECT run_id FROM game_svargrond_pit_leases")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(
                *retained.as_bytes(),
                id(132),
                "old death replay cannot clear a later run"
            );
            assert_eq!(h.count("game_character_death_receipts").await?, 1);
            eprintln!(
                "M51 REAL POSTGRES character death pit cleanup duration={:?}",
                started.elapsed()
            );
            Ok(())
        })
    })
}

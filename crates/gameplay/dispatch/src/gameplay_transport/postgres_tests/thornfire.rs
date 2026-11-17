use super::*;
use crate::foundation::{ChannelId, RuntimeScopeRefV1, ScopeOwnershipGeneration, WorldId};
use oteryn_gameplay_world_change::thornfire::COMPLETE_PROGRESS;

#[path = "world_event/reset_fixture.rs"]
mod reset_fixture;

#[test]
fn thornfire_postgres_server_save_consumes_complete_world_progress_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(admin).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Thornfire save thread panicked")??;
    eprintln!(
        "THORNFIRE_SAVE_PG_EXECUTED elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

fn scenario(admin: String) -> TestResult {
    run(async |_| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "thornfire_save", true, false).await?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let (_controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        // Seed the completed cohort in the first epoch. The second real save, including
        // its commit replay in the fixture, must consume it and keep the same decision.
        let seed = format!(
            "CREATE FUNCTION seed_thornfire() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
                INSERT INTO game_thornfire_progress VALUES(NEW.world_id,NEW.reset_epoch,{COMPLETE_PROGRESS},false);
                RETURN NEW;
            END $$"
        );
        sqlx::query(sqlx::AssertSqlSafe(seed))
            .execute(&h.pool)
            .await?;
        sqlx::query("CREATE TRIGGER seed_thornfire AFTER INSERT ON game_world_event_epochs FOR EACH ROW WHEN (NEW.reset_epoch=1) EXECUTE FUNCTION seed_thornfire()")
            .execute(&h.pool)
            .await?;
        reset_fixture::reset_epochs(&h, &authority, pin.qualified_room(), world, channel, 2)
            .await?;
        assert!(!h.root.thornfire_requested_at_save(world, 1).await?);
        assert!(h.root.thornfire_requested_at_save(world, 2).await?);
        let progress: i64 = sqlx::query_scalar("SELECT elf_homes FROM game_thornfire_progress")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(progress, 0);

        reset_fixture::admit(&h, 3).await?;
        let scope = RuntimeScopeRefV1::channel(world, channel);
        let generation = ScopeOwnershipGeneration::new(3).map_err(debug)?;
        for epoch in [1, 2, 2] {
            h.root
                .credit_thornfire_elf(&authority, &h.node, scope, generation, epoch, 1)
                .await?;
            let progress: i64 = sqlx::query_scalar("SELECT elf_homes FROM game_thornfire_progress")
                .fetch_one(&h.pool)
                .await?;
            assert_eq!(progress, i64::from(epoch == 2));
        }
        assert!(h.root.thornfire_requested_at_save(world, 2).await?);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

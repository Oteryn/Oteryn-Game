use super::*;

#[test]
fn s7_soulpit_core_entry_accepts_level_one_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("soulpit-low-level", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "soulpit_low_level", false, false)
                .await?;
            super::super::reset();
            let evidence = prepare_source(&h).await?;
            let deps = Dependencies::new()?;
            let admitted = admit_inner(&h, &deps, &evidence, true).await?;
            let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(admitted.destination.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                level, 1,
                "ordinary Core-funded entry must preserve the starting level"
            );
            h.cleanup().await
        })
    })
}

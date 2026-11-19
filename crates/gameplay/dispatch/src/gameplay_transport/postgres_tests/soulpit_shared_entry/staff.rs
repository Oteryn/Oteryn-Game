use super::*;

#[test]
fn soulpit_shared_entry_uses_current_trusted_gm_group_not_forged_hint_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    super::super::kill_reward_e2e_fixtures::scenario_thread::run("soulpit-gm-level", || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "soulpit_gm_level", false, false)
                .await?;
            super::super::reset();
            let evidence = prepare_source(&h).await?;
            let deps = Dependencies::new()?;
            let admitted = admit_inner(&h, &deps, &evidence, true).await?;
            let level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
            .bind(admitted.destination.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                level, 1,
                "trusted GM entry must preserve the real low level"
            );
            h.cleanup().await
        })
    })
}

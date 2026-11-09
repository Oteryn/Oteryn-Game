//! The node's online timer pass uses the restricted runtime login, including on an empty World.
use crate::bestiary_postgres_harness as fixture;

#[test]
fn node_boot_quest_privileges_allow_the_runtime_timer_pass() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(|error| error.to_string())?
                .block_on(async {
                    let h = fixture::Harness::create(admin, "node_quest_privileges", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "node quest privilege thread failed")??;
    Ok(())
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    for table in [
        "game_floor_assembly_recipes",
        "game_floor_assembly_receipts",
        "game_item_fluid_reward_mints",
        "game_floor_transform_recipes",
        "game_floor_transform_receipts",
        "game_floor_transform_deadlines",
        "game_floor_transform_expiry_receipts",
        "game_floor_transform_expiry_audit",
        "game_carried_step_recipes",
        "game_carried_step_receipts",
        "game_carried_step_audit",
        "game_shared_switch_recipes",
        "game_shared_switch_leases",
        "game_shared_switch_audit",
        "game_quest_timer_rules",
        "game_quest_timer_observations",
        "game_quest_timer_audit",
    ] {
        for role in ["oteryn_game_runtime", "oteryn_game_control"] {
            let privileges: (bool, bool, bool, bool) = sqlx::query_as(
                "SELECT has_table_privilege($1,$2,'SELECT'), \
                 has_table_privilege($1,$2,'INSERT'), \
                 has_table_privilege($1,$2,'UPDATE'), \
                 has_table_privilege($1,$2,'DELETE')",
            )
            .bind(role)
            .bind(table)
            .fetch_one(&h.pool)
            .await?;
            let insert = role == "oteryn_game_runtime"
                && !table.ends_with("_recipes")
                && !table.ends_with("_rules")
                && !table.ends_with("_deadlines");
            assert_eq!(privileges, (true, insert, false, false), "{role}: {table}");
        }
    }
    for function in [
        "game_shared_switch_lock(uuid,text,numeric)",
        "game_shared_switch_count(uuid,text,numeric,text,bigint)",
        "game_shared_switch_complete(uuid)",
        "game_quest_timer_source_current(uuid,text,uuid,uuid,uuid,uuid,numeric)",
        "game_quest_timer_envelope(uuid,text)",
        "game_quest_timer_complete(uuid,text)",
    ] {
        let grants: (bool, bool, bool) = sqlx::query_as(
            "SELECT has_function_privilege('oteryn_game_runtime',$1,'EXECUTE'), \
             has_function_privilege('oteryn_game_control',$1,'EXECUTE'), \
             EXISTS(SELECT 1 FROM pg_proc p, \
               LATERAL aclexplode(coalesce(p.proacl,acldefault('f',p.proowner))) a \
               WHERE p.oid=$1::regprocedure AND a.grantee=0 AND a.privilege_type='EXECUTE')",
        )
        .bind(function)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(grants, (true, false, false), "{function}");
    }
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let mut connection = h.root.try_acquire_ready().map_err(fixture::debug)?;
    sqlx::query("SET ROLE oteryn_game_runtime")
        .execute(&mut *connection)
        .await?;
    let returned = connection
        .return_to_pool_observed_until(
            std::time::Instant::now() + std::time::Duration::from_secs(5),
        )
        .await;
    assert!(matches!(
        returned,
        sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle
    ));
    assert!(
        h.root
            .read_due_quest_timers(&authority, &h.node, fixture::fence(1)?, [0; 16])
            .await?
            .is_empty()
    );
    Ok(())
}

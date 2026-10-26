//! ECHORAID durable first-Warden reward through the real PostgreSQL migrator.
use crate::bestiary_postgres_harness::{
    CHARACTER, Harness, SESSION, TestResult, configured_admin, id, runtime,
};

#[test]
fn echo_raid_warden_reward_is_fenced_idempotent_and_durable() -> TestResult {
    let Some(admin) = configured_admin() else { return Ok(()) };
    runtime()?.block_on(async {
        let h = Harness::create(admin, "echo_raid_warden", true).await?;
        let result = async {
            sqlx::query("INSERT INTO game_durability_fresh_admission_receipts SELECT \
                '\\x01'::bytea || decode(repeat('31',32),'hex'), game_session_id, account_id, \
                character_id, world_id, runtime_scope_channel_id, character_lease_generation, \
                scope_ownership_generation, 1, current_transport_ref, 1, '{}', 1 FROM \
                game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
                .bind(id(SESSION).as_slice()).execute(&h.pool).await?;
            let insert = "INSERT INTO game_echo_raid_warden_rewards VALUES \
                (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,\
                 'oteryn:creature.rat',true,1,1,50,pg_current_xact_id()) ON CONFLICT DO NOTHING";
            for _ in 0..2 {
                sqlx::query(insert).bind(id(91).as_slice()).bind(id(CHARACTER).as_slice())
                    .bind(id(SESSION).as_slice()).bind(1_i64).execute(&h.pool).await?;
            }
            let retained: (i64, Option<i16>, Option<i16>, Option<i16>) = sqlx::query_as(
                "SELECT count(*),min(imbuement_scroll_count),min(catalyst_count),min(forge_dust_count) \
                 FROM game_echo_raid_warden_rewards")
                .fetch_one(&h.pool).await?;
            assert_eq!(retained, (1, Some(1), Some(1), Some(50)));
            let stale = sqlx::query(insert).bind(id(92).as_slice()).bind(id(CHARACTER).as_slice())
                .bind(id(SESSION).as_slice()).bind(2_i64).execute(&h.pool).await;
            assert!(stale.is_err());
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}

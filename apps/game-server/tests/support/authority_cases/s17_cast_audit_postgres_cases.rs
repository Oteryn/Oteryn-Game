use crate::bestiary_postgres_harness::{Harness, TestResult, configured_admin, runtime};

#[test]
fn s17_cast_audit_postgres_installs_outbox() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(|e| e.to_string())?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = Harness::create(admin, "s17_audit_schema", true).await?;
                    let result = async {
                        let count: i64 =
                            sqlx::query_scalar("SELECT count(*) FROM game_cast_audit_outbox")
                                .fetch_one(&h.pool)
                                .await?;
                        assert_eq!(count, 0);
                        let grants:(bool,bool,bool,bool)=sqlx::query_as("SELECT has_table_privilege('oteryn_game_runtime','game_cast_audit_outbox','UPDATE'),has_table_privilege('oteryn_game_runtime','game_cast_audit_outbox','DELETE'),has_function_privilege('oteryn_game_runtime','game_cast_audit_expire(integer)','EXECUTE'),has_function_privilege('oteryn_game_control','game_cast_audit_expire(integer)','EXECUTE')").fetch_one(&h.pool).await?;
                        assert_eq!(grants,(false,false,false,true));
                        let removed:i32=sqlx::query_scalar("SELECT game_cast_audit_expire(128)").fetch_one(&h.pool).await?;
                        assert_eq!(removed,0);
                        assert!(sqlx::query("SELECT game_cast_audit_expire(129)").execute(&h.pool).await.is_err());
                        Ok::<(), Box<dyn std::error::Error>>(())
                    }
                    .await;
                    h.cleanup().await?;
                    eprintln!("S17 real PostgreSQL audit schema: {:.3}s", started.elapsed().as_secs_f64());
                    result
                })
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "S17 audit thread panicked")?
        .map_err(Into::into)
}

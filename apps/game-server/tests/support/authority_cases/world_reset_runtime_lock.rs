//! MAINRED1108: runtime reset reads retain row locks without reset-write authority.
use crate::bestiary_postgres_harness as fixture;

#[test]
fn world_reset_runtime_lock_postgres() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .name("world-reset-runtime-lock".into())
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime().map_err(|e| e.to_string())?.block_on(async {
                let h = fixture::Harness::create(admin, "reset_runtime_lock", false).await?;
                let result: fixture::TestResult = async {
                    let world = fixture::scope()?.world_id();
                    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(123),'1970-01')")
                        .bind(world.as_bytes().as_slice()).bind([1u8;32].as_slice()).execute(&h.pool).await?;
                    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
                        .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
                    let mut tx = h.pool.begin().await?;
                    sqlx::query("SET LOCAL ROLE oteryn_game_runtime").execute(&mut *tx).await?;
                    let row: (String, i64) = sqlx::query_as("SELECT reset_epoch::text, activated_at_unix_ms FROM game_world_reset_read_locked(encode($1,'hex')::uuid)")
                        .bind(world.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                    assert_eq!(row, ("0".into(), 123000));
                    let admission_epoch: String = sqlx::query_scalar("SELECT reset_epoch::text FROM game_world_reset_read_locked(encode($1,'hex')::uuid)")
                        .bind(world.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                    assert_eq!(admission_epoch, "0");
                    let missing: Option<(String, i64)> = sqlx::query_as("SELECT reset_epoch::text, activated_at_unix_ms FROM game_world_reset_read_locked(encode($1,'hex')::uuid)")
                        .bind(fixture::id(90).as_slice()).fetch_optional(&mut *tx).await?;
                    assert!(missing.is_none());
                    let broad_update: bool = sqlx::query_scalar("SELECT has_table_privilege(current_user,'game_world_reset_active','UPDATE') OR has_table_privilege(current_user,'game_world_reset_epochs','UPDATE') OR has_any_column_privilege(current_user,'game_world_reset_epochs','UPDATE')")
                        .fetch_one(&mut *tx).await?;
                    assert!(!broad_update);
                    // The definer read must not grant runtime even a no-op reset mutation.
                    for (statement, code) in [
                        ("UPDATE game_world_reset_epochs SET reset_epoch=reset_epoch WHERE world_id=encode($1,'hex')::uuid", "42501"),
                        ("UPDATE game_world_reset_active SET reset_epoch=reset_epoch WHERE world_id=encode($1,'hex')::uuid", "42501"),
                    ] {
                        sqlx::query("SAVEPOINT rejected_write").execute(&mut *tx).await?;
                        let rejected = sqlx::query(statement).bind(world.as_bytes().as_slice()).execute(&mut *tx).await;
                        assert_eq!(rejected.err().and_then(|e| e.as_database_error().and_then(|e| e.code()).map(|c| c.into_owned())).as_deref(), Some(code));
                        sqlx::query("ROLLBACK TO SAVEPOINT rejected_write").execute(&mut *tx).await?;
                    }
                    // An independent reset owner cannot replace the pointer while the read is held.
                    let mut writer = h.pool.begin().await?;
                    sqlx::query("SET LOCAL lock_timeout='100ms'").execute(&mut *writer).await?;
                    let blocked = sqlx::query("SELECT 1 FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid FOR UPDATE")
                        .bind(world.as_bytes().as_slice()).execute(&mut *writer).await;
                    assert_eq!(blocked.err().and_then(|e| e.as_database_error().and_then(|e| e.code()).map(|c| c.into_owned())).as_deref(), Some("55P03"));
                    writer.rollback().await?;
                    tx.rollback().await?;
                    eprintln!("MAINRED1108 PG EXECUTED: runtime reset read, missing world, write denial and reset-owner lock conflict");
                    Ok(())
                }.await;
                h.cleanup().await?;
                result
            }).map_err(|e| e.to_string())
        })?
        .join().map_err(|_| "reset runtime thread failed")??;
    Ok(())
}

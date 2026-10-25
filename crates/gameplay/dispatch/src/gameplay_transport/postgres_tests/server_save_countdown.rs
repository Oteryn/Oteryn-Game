//! PARITY-EVENT-CALLBACKS-1: the countdown reads Control's planned reset and never writes it.
use super::*;
use crate::foundation::WorldId;

#[test]
fn server_save_countdown_reads_the_next_berlin_save_and_stops_while_retiring() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "server save countdown scenario panicked")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "server_save_countdown", true, false)
                .await?;
        let world = WorldId::decode(&id(77)).map_err(debug)?;
        // An uninitialized World has no planned reset to announce.
        assert_eq!(harness.root.planned_world_reset(world).await?, None);
        // Control's activation fixture: epoch 0 on the Tibia server-save calendar.
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_10',1,clock_timestamp(),to_char((clock_timestamp() AT TIME ZONE 'Europe/Berlin')-interval '10 hours','YYYY-MM'))")
            .bind(world.as_bytes().as_slice()).bind([7_u8; 32].as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
            .bind(world.as_bytes().as_slice())
            .execute(&harness.pool)
            .await?;
        let planned = harness
            .root
            .planned_world_reset(world)
            .await?
            .ok_or("planned reset")?;
        assert_eq!(planned.ending_epoch, 0);
        let ahead = planned.reset_at_unix_ms - planned.database_now_unix_ms;
        assert!(ahead > 0 && ahead <= 25 * 3_600_000, "{planned:?}");
        let berlin: String = sqlx::query_scalar("SELECT to_char(to_timestamp($1::double precision/1000) AT TIME ZONE 'Europe/Berlin','HH24:MI:SS.MS')")
            .bind(planned.reset_at_unix_ms).fetch_one(&harness.pool).await?;
        assert_eq!(berlin, "10:00:00.000");
        // The read is pure: nothing in Control's reset state moved.
        let jobs: i64 = sqlx::query_scalar("SELECT count(*) FROM game_world_reset_jobs")
            .fetch_one(&harness.pool)
            .await?;
        assert_eq!(jobs, 0);
        // Once Control is retiring the epoch, the countdown has nothing left to announce.
        sqlx::query("INSERT INTO game_world_reset_jobs(world_id,reset_epoch,target_digest,retirement_binding,target_map_revision,target_server_digest,target_client_digest,target_frame_digest,target_house_tiles,calendar_revision,phase) VALUES(encode($1,'hex')::uuid,0,$2,sha256(convert_to('oteryn:world-reset-retirement:v1','UTF8') || $2),'map-2',$2,$2,$2,'[]',1,'RETIRING')")
            .bind(world.as_bytes().as_slice()).bind([7_u8; 32].as_slice()).execute(&harness.pool).await?;
        assert_eq!(harness.root.planned_world_reset(world).await?, None);
        Ok(())
    })
}

use super::fixture::Harness;

use super::TestResult;

use crate::durability::world_event::changes;

pub(super) async fn day(h: &Harness, world: crate::foundation::WorldId, epoch: u64) -> TestResult {
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01') ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind([7u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active(world_id,reset_epoch) VALUES(encode($1,'hex')::uuid,$2::text::numeric) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=excluded.reset_epoch")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&h.pool).await?;
    // Isolate the daily callback from unrelated live phase controllers. Legacy immutable
    // daily decisions have world_counters=false; all actual selection draws stay unchanged.
    let mut expected = changes::roll(world, epoch, 0)?;
    expected.world_counters = false;
    sqlx::query("INSERT INTO game_world_change_days(world_id,reset_epoch,decision) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3) ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(serde_json::to_value(expected)?).execute(&h.pool).await?;
    let loaded = h
        .root
        .world_changes(world, epoch)
        .await?
        .ok_or("daily snapshot")?;
    assert_eq!(
        (loaded.reset_epoch, loaded.oriental_trader),
        (epoch, expected.oriental_trader)
    );
    Ok(())
}

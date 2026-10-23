//! One shared test-only RETIRING row fixture; callers use the production sealer.
use crate::foundation::WorldId;
type TestResult = Result<(), Box<dyn std::error::Error>>;

pub async fn seed_retiring(pool: &sqlx::PgPool, world: WorldId) -> TestResult {
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_10',1,clock_timestamp(),to_char((clock_timestamp() AT TIME ZONE 'Europe/Berlin')-interval '10 hours','YYYY-MM'))")
            .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).execute(pool).await?;
    sqlx::query("INSERT INTO game_world_reset_jobs(world_id,reset_epoch,target_digest,retirement_binding,target_map_revision,target_server_digest,target_client_digest,target_frame_digest,target_house_tiles,calendar_revision,phase) VALUES(encode($1,'hex')::uuid,0,$2,sha256(convert_to('oteryn:world-reset-retirement:v1','UTF8') || $2),'map-2',$2,$2,$2,'[]',1,'RETIRING')")
            .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).execute(pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
        .bind(world.as_bytes().as_slice())
        .execute(pool)
        .await?;
    sqlx::query("INSERT INTO game_world_reset_channels SELECT world_id,0,channel_id,ownership_generation FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND scope_kind=1")
            .bind(world.as_bytes().as_slice()).execute(pool).await?;
    Ok(())
}

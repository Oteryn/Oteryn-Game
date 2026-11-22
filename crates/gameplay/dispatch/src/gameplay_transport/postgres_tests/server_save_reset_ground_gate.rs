//! Actual World-reset activation requires an empty Ground ledger after the drain.
use super::{Harness, TestResult, debug, id, planned_world_reset_fixtures::reset_fixture};
use crate::foundation::{ChannelId, WorldId};
use sqlx::{Connection, PgConnection};

#[test]
fn server_save_sweep_world_reset_activation_waits_for_empty_ground_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?
                .block_on(scenario(admin))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "reset Ground gate scenario panicked")?
        .map_err(Into::into)
}

async fn scenario(admin: String) -> TestResult {
    let started = std::time::Instant::now();
    let harness = Harness::create_with_admission_seed(admin, "reset_ground_gate", true, false).await?;
    let outcome = qualify(&harness).await;
    let cleanup = harness.cleanup().await;
    outcome.and(cleanup)?;
    eprintln!("SERVER SAVE RESET GROUND GATE PG {:?}", started.elapsed());
    Ok(())
}

async fn qualify(harness: &Harness) -> TestResult {
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let channel = ChannelId::decode(&id(43)).map_err(debug)?;
    let digest = [7_u8; 32];
    sqlx::query("INSERT INTO game_content_activations
      (world_id,channel_id,activation_sequence,server_artifact_digest,client_artifact_digest,frame_binding_digest,issued_by,issued_at)
      VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,$3,$3,$3,session_user,1)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(digest.as_slice())
        .execute(&harness.pool).await?;
    reset_fixture::seed_retiring(&harness.pool, world).await?;
    reset_fixture::replace(harness, world, channel, &harness.node).await?;
    let mut tx = harness.pool.begin().await?;
    // Pre-existing ordinary Ground. The assertion below runs all activation
    // guards under the exact current replacement assignment.
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,
      definition_revision_ref,quantity,lifecycle,minted_transaction_id)
      VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3031','definition-r1',1,1,encode($1,'hex')::uuid)")
        .bind(id(101).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,
      runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,
      native_room_placement_context,stack_ordinal)
      VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$1,'map-1','content-1',$5,1)")
        .bind(id(101).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind([0_u8,0,0,101,0,0,0,42,0,7].as_slice()).bind(digest.as_slice())
        .execute(&mut *tx).await?;
    tx.commit().await?;

    let mut fresh = PgConnection::connect_with(&harness.pool.connect_options()).await?;
    assert!(sqlx::query_scalar::<_, bool>("SELECT game_world_reset_drained(encode($1,'hex')::uuid,0)")
        .bind(id(42).as_slice()).fetch_one(&mut fresh).await?);
    assert_eq!(activate(harness, &mut fresh).await?, None);
    assert_eq!(harness.root.active_world_reset_epoch(world).await?, Some(0));
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_ground_locations")
        .fetch_one(&mut fresh).await?, 1);

    let seal = harness.recovery.seal_current().map_err(debug)?;
    let recovery = harness.root.open_character_authority(&seal).await.map_err(debug)?;
    let job = harness.root.retiring_world_reset(world).await?.ok_or("retiring World reset")?;
    assert!(harness.root.retire_world_reset_ground(&recovery,&harness.node,&job).await?);
    assert_eq!(sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_ground_locations")
        .fetch_one(&mut fresh).await?, 0);
    assert_eq!(activate(harness, &mut fresh).await?.as_deref(), Some("NEW_EPOCH"));
    assert_eq!(harness.root.active_world_reset_epoch(world).await?, Some(1));
    Ok(())
}

async fn activate(harness: &Harness, db: &mut PgConnection) -> TestResult<Option<String>> {
    let node = harness.node.fact();
    Ok(sqlx::query_scalar("SELECT game_world_reset_activate(world_id,reset_epoch,retirement_binding,target_digest,
      target_map_revision,target_server_digest,target_client_digest,target_frame_digest,
      encode($2,'hex')::uuid,$3::text::numeric) FROM game_world_reset_jobs WHERE world_id=encode($1,'hex')::uuid")
       .bind(id(42).as_slice()).bind(node.node_id().as_bytes().as_slice())
       .bind(node.registration_revision().to_string()).fetch_one(db).await?)
}

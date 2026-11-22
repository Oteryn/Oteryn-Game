//! A lost publication is reconciled by the next real countdown owner pass.
use super::{
    Dependencies, Harness, TestResult, debug, id, planned_world_reset_fixtures::reset_fixture,
};
use crate::durability::server_save_sweep::{GroundSweepPolicy, ServerSaveCleanup};
use crate::foundation::{ChannelContentPin, ChannelId, WorldId};
use crate::gameplay_transport::{SeamRooms, loop_move::loop_ground};

#[test]
fn server_save_sweep_dispatch_rebuilds_the_ground_projection_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
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
        .map_err(|_| "server-save projection scenario panicked")?
        .map_err(Into::into)
}

async fn scenario(admin: String) -> TestResult {
    let started = std::time::Instant::now();
    let harness =
        Harness::create_with_admission_seed(admin, "save_sweep_projection", true, false).await?;
    let outcome = qualify(&harness).await;
    let cleanup = harness.cleanup().await;
    outcome.and(cleanup)?;
    eprintln!("SERVER SAVE SWEEP PROJECTION PG {:?}", started.elapsed());
    Ok(())
}

async fn qualify(h: &Harness) -> TestResult {
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let channel = ChannelId::decode(&id(43)).map_err(debug)?;
    let (controller, entry) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(
            world,
            Some(include_bytes!(
                "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            )),
        );
    let active = controller.active().ok_or("active bundle content")?;
    let native = active.native_gameplay().ok_or("native Item policy")?;
    let index = native.item_index().ok_or("active Item definition index")?;
    // A different tile in this bundle is a House. Its presence must not make
    // the qualified public drop's House status unknown.
    let bundle = super::house_wire_bundle::house_bundle(world, channel)?;
    let start = crate::content::NativeEntryStart {
        x: 2,
        y: 0,
        floor: 7,
    };
    let entry = entry.qualified_room();
    let room = entry
        .bundle_view(
            bundle.movement_cells(entry.movement_cells())?,
            bundle.frame_binding_digest(world),
            bundle.map_revision_digest(),
            start,
        )
        .ok_or("qualified bundle frame")?;
    let pin = ChannelContentPin::from_activation(
        world,
        1,
        room.compiled().server_digest(),
        room.compiled().client_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let deps = Dependencies::with_content(pin, room.clone(), native.spell_book().clone())?;
    reset_fixture::publish_ready(h, 1).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let recovery = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut owner = deps.owner(h, &recovery);
    owner.active_generation = Some(active);
    owner.movement_cells = room.movement_cells().into();
    owner.rooms = SeamRooms::new(None, Some(&room));
    let base = owner.bundle_map().ok_or("bundle map")?.overlay.base();
    seed(h, &room, crate::map::overlay::map_revision(base)).await?;

    let before = loop_ground(&owner, base, Some(index))
        .await
        .ok_or("initial Ground projection")?;
    assert!(before.ground_entry(&id(101)).is_some());
    let cleanup = ServerSaveCleanup {
        enabled: true,
        policy: GroundSweepPolicy {
            clean_protection_zones: true,
        },
    };
    let mut worker = deps.owner(h, &recovery);
    worker.active_generation = Some(active);
    worker.movement_cells = room.movement_cells().into();
    worker.rooms = SeamRooms::new(None, Some(&room));
    // The committing worker has its own projection. Losing that owner's result
    // leaves the observing owner on the previously published snapshot.
    worker.states = Default::default();
    let mut worker_publication = None;
    worker
        .advance_server_save(cleanup, &mut worker_publication)
        .await?;
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT disposition FROM game_server_save_ground")
            .fetch_one(&h.pool)
            .await?,
        "pending"
    );
    let _lost = worker
        .advance_server_save(cleanup, &mut worker_publication)
        .await;
    drop(worker);
    let stale = loop_ground(&owner, base, Some(index))
        .await
        .ok_or("still cached Ground projection")?;
    assert!(std::sync::Arc::ptr_eq(&before, &stale));
    assert!(stale.ground_entry(&id(101)).is_some());
    let mut replacement_publication = None;
    owner
        .advance_server_save(cleanup, &mut replacement_publication)
        .await?;
    assert!(replacement_publication.is_some());
    let rebuilt = loop_ground(&owner, base, Some(index))
        .await
        .ok_or("rebuilt Ground projection")?;
    assert!(rebuilt.ground_entry(&id(101)).is_none());

    use sqlx::Connection;
    let mut fresh = sqlx::PgConnection::connect_with(&h.pool.connect_options()).await?;
    let proof: (i16, i64, i64, i64, String) = sqlx::query_as(
        "SELECT lifecycle,
       (SELECT count(*) FROM game_item_ground_locations),
       (SELECT count(*) FROM game_spell_item_receipts WHERE cause_kind=7),
       (SELECT count(*) FROM game_spell_item_audit_outbox),
       (SELECT state FROM game_server_save_channels)
       FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(101).as_slice())
    .fetch_one(&mut fresh)
    .await?;
    assert_eq!(proof, (2, 0, 1, 1, "complete".into()));
    Ok(())
}

async fn seed(
    h: &Harness,
    room: &crate::content::QualifiedNativeEntryRoom,
    map: String,
) -> TestResult {
    let world = id(42);
    let channel = id(43);
    let server = room.compiled().server_digest();
    let client = room.compiled().client_digest();
    let frame = room.frame_binding().digest();
    let epoch_map = room.movement_cells().scope().map_revision.as_str();
    let start = room.entry_start();
    let position = crate::durability::map_item_mint_audit::map_item_spatial_position(
        u16::try_from(start.x)?,
        u16::try_from(start.y)?,
        i8::try_from(-start.floor)?,
    );
    let mut tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_content_activations
        (world_id,channel_id,activation_sequence,server_artifact_digest,client_artifact_digest,frame_binding_digest,issued_by,issued_at)
        VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,$3,$4,$5,session_user,1)")
        .bind(world.as_slice()).bind(channel.as_slice()).bind(server.as_slice()).bind(client.as_slice()).bind(frame.as_slice())
        .execute(&mut *tx).await?;
    sqlx::query("SELECT game_world_reset_initialize(encode($1,'hex')::uuid,$2,$3,$2,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1)")
        .bind(world.as_slice()).bind(server.as_slice()).bind(epoch_map).bind(client.as_slice()).bind(frame.as_slice())
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_server_save_occurrences
        (world_id,reset_epoch,scheduled_at,freeze_until,map_revision,server_digest,frame_digest,cleanup_enabled,clean_protection_zones)
        SELECT world_id,reset_epoch,clock_timestamp()-interval '1 second',clock_timestamp()+interval '10 minutes',
          map_revision,server_digest,frame_digest,true,true FROM game_world_reset_epochs").execute(&mut *tx).await?;
    // Provision a typed, non-authored Item and the ordinary player drop codec with the signed bundle floor.
    // All retirement and readback proofs are enabled in subsequent transactions.
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,
        definition_revision_ref,quantity,lifecycle,minted_transaction_id)
        VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3031','definition-r1',1,1,encode($1,'hex')::uuid)")
        .bind(id(101).as_slice()).bind(world.as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_starter_grants(character_id,template_key,template_revision,world_id,definition_family,
        definition_production_key,definition_revision_ref,quantity,outcome,item_instance_id,transaction_id,event_id,grant_envelope,grant_envelope_sha256)
        SELECT encode($2,'hex')::uuid,'ground-sweep-gold','starter-1',world_id,definition_family,
         definition_production_key,definition_revision_ref,quantity,1,item_instance_id,minted_transaction_id,uuidv7(),$1,sha256($1)
        FROM game_item_instances WHERE item_instance_id=encode($3,'hex')::uuid")
        .bind(b"ground-sweep-test starter coin".as_slice()).bind(id(41).as_slice()).bind(id(101).as_slice())
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,
        spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal)
        VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$6,'sha256:'||encode($7,'hex'),$8,1)")
        .bind(id(101).as_slice()).bind(world.as_slice()).bind(channel.as_slice())
        .bind(position)
        .bind([crate::durability::item_loop_move::ITEM_LOOP_DROP_REF_PREFIX,id(101).as_slice()].concat())
        .bind(map).bind(server.as_slice()).bind(frame.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

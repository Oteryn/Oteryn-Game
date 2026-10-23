//! Reset finalization and source remint through the composed dispatch owner.
use super::*;
use crate::content::QualifiedNativeEntryRoom;
use crate::foundation::{ChannelId, WorldId};
use planned_world_reset::reset_fixture;
use reset_fixture::{publish_ready, set_runtime};

#[test]
fn world_reset_atomic_activation_calendar_house_refusal_and_dispatch_remint() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "reset_activation", true, false).await?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                world,
                Some(include_bytes!(
                    "../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
                )),
            );
        let room = pin.qualified_room();
        assert!(room.movement_cells().house_tiles().is_known_empty());
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        deps.room = room.clone();
        set_runtime(&mut deps, world, channel, &pin, 1)?;
        publish_ready(&harness, 1).await?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = controller.active();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(room), Some(room));
        assert!(owner.ensure_source_map_initialized().await);
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_native_map_item_receipts")
            .fetch_one(&harness.pool)
            .await?;
        assert!(count > 0);
        assert_eq!(harness.root.active_world_reset_epoch(world).await?, None);
        sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&harness.pool).await?;
        assert!(
            harness
                .root
                .record_content_activation(
                    &crate::durability::content_activation::ContentActivationRequest {
                        world_id: world,
                        channel_id: channel,
                        activation_sequence: 1,
                        previous_sequence: None,
                        server_artifact_digest: room.compiled().server_digest(),
                        client_artifact_digest: room.compiled().client_digest(),
                        frame_binding_digest: room.frame_binding().digest(),
                    }
                )
                .await?
        );
        let init = "SELECT game_world_reset_initialize(encode($1,'hex')::uuid,$2,$3,$2,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_10',1)";
        for expected in [true, false] {
            assert_eq!(
                sqlx::query_scalar::<_, bool>(init)
                    .bind(world.as_bytes().as_slice())
                    .bind(room.compiled().server_digest().as_slice())
                    .bind(room.movement_cells().scope().map_revision.as_str())
                    .bind(room.compiled().client_digest().as_slice())
                    .bind(room.frame_binding().digest().as_slice())
                    .fetch_one(&harness.pool)
                    .await?,
                expected
            );
        }
        let calendar: bool = sqlx::query_scalar("SELECT calendar='TIBIA_SERVER_SAVE_BERLIN_10' AND calendar_revision=1 AND reset_month=to_char((activated_at AT TIME ZONE 'Europe/Berlin')-interval '10 hours','YYYY-MM') FROM game_world_reset_epochs WHERE reset_epoch=0")
            .fetch_one(&harness.pool).await?;
        assert!(calendar);
        let months: Vec<String> = sqlx::query_scalar("SELECT to_char((instant AT TIME ZONE 'Europe/Berlin')-interval '10 hours','YYYY-MM') FROM (VALUES (1,'2026-04-01 07:59:59+00'::timestamptz),(2,'2026-04-01 08:00:00+00'),(3,'2026-11-01 08:59:59+00'),(4,'2026-11-01 09:00:00+00')) v(n,instant) ORDER BY n LIMIT 4")
            .fetch_all(&harness.pool).await?;
        assert_eq!(months, ["2026-03", "2026-04", "2026-10", "2026-11"]);
        // No global role counts: this slice installs no reset capability grants.
        for role in ["oteryn_game_runtime", "oteryn_game_control"] {
            let privileges: (bool,bool,bool) = sqlx::query_as("SELECT has_function_privilege($1,'game_world_reset_initialize(uuid,bytea,text,bytea,bytea,bytea,text,numeric)','EXECUTE'),has_function_privilege($1,'game_world_reset_publish(uuid,numeric,bytea,text,bytea,bytea,bytea,jsonb,numeric)','EXECUTE'),has_function_privilege($1,'game_world_reset_activate(uuid,numeric,bytea,bytea,text,bytea,bytea,bytea,uuid,numeric)','EXECUTE')")
                .bind(role).fetch_one(&harness.pool).await?;
            assert_eq!(privileges, (false, false, false));
        }
        assert!(
            publish(&harness, world, room, "[{\"house\":\"unqualified\"}]")
                .await
                .is_err(),
            "a reset that would touch a House tile must refuse before publication"
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_world_reset_jobs")
                .fetch_one(&harness.pool)
                .await?,
            0
        );
        assert!(publish(&harness, world, room, "[]").await?);
        assert!(!publish(&harness, world, room, "[]").await?);
        let reset = harness
            .root
            .retiring_world_reset(world)
            .await?
            .ok_or("reset")?;
        // Unknown House metadata also refuses before any item retirement.
        let unknown = crate::content::qualify_native_entry_room(world).map_err(debug)?;
        assert!(
            harness
                .root
                .finish_world_reset(&authority, &harness.node, &reset, &unknown, room)
                .await
                .is_err()
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_decay_retire_receipts")
                .fetch_one(&harness.pool)
                .await?,
            0
        );
        assert_eq!(
            harness
                .root
                .finish_world_reset(&authority, &harness.node, &reset, room, room)
                .await
                .map_err(debug)?,
            None
        );
        reset_fixture::replace(&harness, world, channel, &harness.node).await?;
        // Failure after the epoch/phase writes must roll those writes back with
        // the issuance. Already retired Ground remains durable for recovery.
        sqlx::query("CREATE FUNCTION reject_reset_issuance() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.activation_sequence>1 THEN RAISE EXCEPTION 'injected reset issuance failure' USING ERRCODE='23514'; END IF; RETURN NEW; END $$")
            .execute(&harness.pool).await?;
        sqlx::query("CREATE TRIGGER reject_reset_issuance BEFORE INSERT ON game_content_activations FOR EACH ROW EXECUTE FUNCTION reject_reset_issuance()")
            .execute(&harness.pool).await?;
        assert!(
            harness
                .root
                .finish_world_reset(&authority, &harness.node, &reset, room, room)
                .await
                .is_err()
        );
        assert_eq!(harness.root.active_world_reset_epoch(world).await?, Some(0));
        assert!(harness.root.retiring_world_reset(world).await?.is_some());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_content_activations")
                .fetch_one(&harness.pool)
                .await?,
            1
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_decay_retire_receipts")
                .fetch_one(&harness.pool)
                .await?,
            count
        );
        sqlx::query("DROP TRIGGER reject_reset_issuance ON game_content_activations")
            .execute(&harness.pool)
            .await?;
        for _ in 0..2 {
            assert_eq!(
                harness
                    .root
                    .finish_world_reset(&authority, &harness.node, &reset, room, room)
                    .await
                    .map_err(debug)?,
                Some(1)
            );
        }
        assert!(harness.root.retiring_world_reset(world).await?.is_none());
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_world_reset_epochs")
                .fetch_one(&harness.pool)
                .await?,
            2
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_content_activations")
                .fetch_one(&harness.pool)
                .await?,
            2
        );
        let calendar: bool = sqlx::query_scalar("SELECT reset_month=to_char((activated_at AT TIME ZONE 'Europe/Berlin')-interval '10 hours','YYYY-MM') FROM game_world_reset_epochs WHERE reset_epoch=1")
            .fetch_one(&harness.pool).await?;
        assert!(calendar);
        let issuance = harness
            .root
            .read_current_content_activation(world, channel)
            .await?
            .ok_or("issuance")?;
        assert_eq!(issuance.activation_sequence, 2);
        assert_eq!(
            issuance.server_artifact_digest,
            room.compiled().server_digest()
        );
        assert_eq!(
            issuance.client_artifact_digest,
            room.compiled().client_digest()
        );
        assert_eq!(issuance.frame_binding_digest, room.frame_binding().digest());
        drop(owner);
        set_runtime(&mut deps, world, channel, &pin, 2)?;
        publish_ready(&harness, 2).await?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = controller.active();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(room), Some(room));
        owner.reset_epoch = harness
            .root
            .active_world_reset_epoch(world)
            .await?
            .ok_or("epoch")?;
        assert!(owner.ensure_source_map_initialized().await);
        assert_eq!(
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_native_map_item_receipts WHERE reset_epoch=1"
            )
            .fetch_one(&harness.pool)
            .await?,
            count
        );
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_item_decay_retire_receipts")
                .fetch_one(&harness.pool)
                .await?,
            count
        );
        let db: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&harness.pool)
            .await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&harness.pool)
            .await?;
        eprintln!(
            "WORLD-RESET-ACTIVATION-PG: database={db} PostgreSQL={version} retired={count} reminted={count} epoch=1"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn publish(
    harness: &Harness,
    world: WorldId,
    room: &QualifiedNativeEntryRoom,
    houses: &str,
) -> TestResult<bool> {
    Ok(sqlx::query_scalar("SELECT game_world_reset_publish(encode($1,'hex')::uuid,0,$2,$3,$2,$4,$5,$6::text::jsonb,1)")
        .bind(world.as_bytes().as_slice()).bind(room.compiled().server_digest().as_slice())
        .bind(room.movement_cells().scope().map_revision.as_str()).bind(room.compiled().client_digest().as_slice())
        .bind(room.frame_binding().digest().as_slice()).bind(houses).fetch_one(&harness.pool).await?)
}

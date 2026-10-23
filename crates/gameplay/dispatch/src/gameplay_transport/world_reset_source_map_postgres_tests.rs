//! Source placement initialization through the actual composed dispatch owner.
use super::*;
use crate::foundation::{ChannelId, NodeId, WorldId};

#[test]
fn world_reset_source_map_receipt_and_audit_bind_the_active_epoch() -> TestResult {
    scenario(1)
}

#[test]
fn world_reset_source_map_epoch_zero_keeps_the_legacy_intent_bytes() -> TestResult {
    scenario(0)
}

fn scenario(epoch: u64) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "reset_source_epoch", true, false).await?;
        let privileges:(bool,bool,bool)=sqlx::query_as("SELECT prosecdef,has_table_privilege('oteryn_game_runtime','game_world_reset_active','UPDATE'),has_function_privilege('oteryn_game_runtime','game_world_reset_map_epoch_current(uuid,numeric)','EXECUTE') FROM pg_proc WHERE oid='game_world_reset_map_epoch_current(uuid,numeric)'::regprocedure")
            .fetch_one(&harness.pool).await?;
        assert_eq!(
            privileges,
            (true, false, true),
            "runtime locks reset state without an UPDATE grant"
        );
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                world,
                Some(include_bytes!(
                    "../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
                )),
            );
        let mut deps = Dependencies::new()?;
        deps.room = pin.qualified_room().clone();
        let start = deps.room.entry_start();
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            deps.room.map_revision_digest(),
            (start.x, start.y, start.floor),
        );
        deps.runtime = Mutex::new(
            ChannelRuntimeV1::from_committed_assignment(
                world,
                channel,
                NodeId::decode(&id(1)).map_err(debug)?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                2,
                channel_pin,
            )
            .map_err(debug)?,
        );
        // The active pointer is a fixture of a completed reset. This scenario
        // qualifies the source owner, not the still-unimplemented step-4 writer.
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$3::text::numeric,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_10',1,clock_timestamp(),'2026-10')")
            .bind(world.as_bytes().as_slice()).bind([7_u8;32].as_slice()).bind(epoch.to_string()).execute(&harness.pool).await?;
        sqlx::query(
            "INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2::text::numeric)",
        )
        .bind(world.as_bytes().as_slice())
        .bind(epoch.to_string())
        .execute(&harness.pool)
        .await?;
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = controller.active();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&deps.room), Some(&deps.room));
        owner.reset_epoch = harness
            .root
            .active_world_reset_epoch(world)
            .await?
            .ok_or("active epoch")?;
        assert!(owner.ensure_source_map_initialized().await);
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_native_map_item_receipts")
            .fetch_one(&harness.pool)
            .await?;
        assert!(
            count > 0,
            "the real qualified map must mint at least one placement"
        );
        let bad:i64=sqlx::query_scalar("SELECT count(*) FROM game_native_map_item_receipts r LEFT JOIN game_native_map_item_audit a USING(transaction_id) WHERE r.reset_epoch<>$1::text::numeric OR ($1='0' AND convert_from(r.source_intent,'UTF8')::jsonb ? 'reset_epoch') OR ($1<>'0' AND convert_from(r.source_intent,'UTF8')::jsonb->>'reset_epoch' IS DISTINCT FROM $1) OR a.envelope IS DISTINCT FROM r.source_intent OR a.envelope_digest IS DISTINCT FROM r.binding").bind(epoch.to_string()).fetch_one(&harness.pool).await?;
        assert_eq!(bad, 0);
        // Drop the in-memory initialization marker and replay the actual owner.
        owner.spell_states.lock().await.source_map_initialized = None;
        owner
            .spell_states
            .lock()
            .await
            .next_map_initialization_pass_us = 0;
        assert!(owner.ensure_source_map_initialized().await);
        assert_eq!(
            sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_native_map_item_receipts")
                .fetch_one(&harness.pool)
                .await?,
            count
        );
        let stale = sqlx::query("INSERT INTO game_native_map_item_receipts SELECT (jsonb_populate_record(NULL::game_native_map_item_receipts,to_jsonb(r)||jsonb_build_object('reset_epoch',$1::text::numeric))).* FROM game_native_map_item_receipts r LIMIT 1")
            .bind((epoch+1).to_string()).execute(&harness.pool).await.err().ok_or("stale epoch must refuse")?;
        let sqlx::Error::Database(stale) = stale else {
            return Err("expected SQL rejection".into());
        };
        assert!(
            stale
                .message()
                .contains("source receipt outside active reset epoch")
        );
        let database: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&harness.pool)
            .await?;
        eprintln!("WORLD-RESET-SOURCE-PG: {database}, {count} epoch-bound placements");
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

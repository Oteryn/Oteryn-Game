use super::house_podium::placed_podiums;
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
    RuntimeScopeAssignmentWriter,
};
use crate::foundation::WorldId;
use crate::item_transfer_postgres_cases::{
    CHANNEL, Harness, TestResult, WORLD, debug, id, runtime, scope,
};
use oteryn_gameplay_content::content::qualify_native_entry_room;
use serde_json::{Value, json};

const PUBLISH: &str = "SELECT game_world_reset_publish(world_id,reset_epoch,server_digest,map_revision,server_digest,client_digest,frame_digest,$2,calendar_revision)
    FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=0";
const ACTIVATE: &str = "SELECT game_world_reset_activate(world_id,reset_epoch,retirement_binding,target_digest,target_map_revision,
    target_server_digest,target_client_digest,target_frame_digest,encode($2,'hex')::uuid,$3::text::numeric)
    FROM game_world_reset_jobs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=0";
const HOUSE_REFUSAL: &str = "World reset House qualification unavailable";

#[test]
fn world_reset_qualified_house_publication_and_activation_preserve_custody() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "reset_house_custody", async |harness| {
        let tiles = seed(harness).await?;
        let before = custody(harness).await?;
        assert!(
            !before["locations"]
                .as_array()
                .ok_or("locations")?
                .is_empty()
        );
        assert_eq!(
            before["locations"].as_array().map(Vec::len),
            before["provenance"].as_array().map(Vec::len)
        );
        assert!(publish(harness, &tiles).await?);
        assert!(!publish(harness, &tiles).await?, "exact publication replay");
        assert_eq!(
            activate(harness).await?,
            None,
            "the House proof does not bypass drain"
        );
        drain(harness).await?;
        assert_eq!(activate(harness).await?.as_deref(), Some("NEW_EPOCH"));
        assert_eq!(activate(harness).await?.as_deref(), Some("HISTORICAL"));
        assert_eq!(custody(harness).await?, before);
        let active: String = sqlx::query_scalar("SELECT reset_epoch::text FROM game_world_reset_active WHERE world_id=encode($1,'hex')::uuid")
            .bind(id(WORLD).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(active, "1");
        eprintln!(
            "World reset PostgreSQL verified publication, activation, replay and House custody"
        );
        Ok(())
    })
}

#[test]
fn world_reset_refuses_malformed_or_unqualified_house_metadata() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "reset_house_refusal", async |harness| {
        let tiles = seed(harness).await?;
        let first = tiles.get(0).ok_or("fixture House tile")?;
        let stale_position = json!([{
            "house_key": first["house_key"], "spatial_position": "ff"
        }]);
        let mut malformed = tiles.clone();
        malformed[0]["spatial_position"] = json!(true);
        for invalid in [
            json!(null),
            json!([]),
            malformed.clone(),
            stale_position.clone(),
            json!([first, first]),
        ] {
            assert_house_refusal(publish(harness, &invalid).await);
        }
        // The same fixture must also admit exact evidence before testing drift
        // in the stored job at the independent activation boundary.
        assert!(publish(harness, &tiles).await?);
        drain(harness).await?;
        for invalid in [malformed, stale_position] {
            sqlx::query("UPDATE game_world_reset_jobs SET target_house_tiles=$2 WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=0")
                .bind(id(WORLD).as_slice()).bind(invalid).execute(&harness.pool).await?;
            assert_house_refusal(activate(harness).await);
        }
        let state: (String, String) = sqlx::query_as(
            "SELECT a.reset_epoch::text,j.phase FROM game_world_reset_active a JOIN game_world_reset_jobs j USING(world_id,reset_epoch)
             WHERE a.world_id=encode($1,'hex')::uuid",
        ).bind(id(WORLD).as_slice()).fetch_one(&harness.pool).await?;
        assert_eq!(state, ("0".into(), "RETIRING".into()));
        eprintln!("World reset PostgreSQL verified both House qualification refusals");
        Ok(())
    })
}

fn run(
    admin: String,
    name: &'static str,
    exercise: impl AsyncFnOnce(&Harness) -> TestResult + Send + 'static,
) -> TestResult {
    std::thread::Builder::new()
        .name(name.into())
        .stack_size(64 << 20)
        .spawn(move || {
            let result: TestResult = (|| {
                runtime()?.block_on(async {
                    let harness = Harness::create(admin, name).await?;
                    let result = exercise(&harness).await;
                    harness.cleanup().await?;
                    result
                })
            })();
            result.map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "world reset House PostgreSQL thread panicked")??;
    Ok(())
}

async fn seed(harness: &Harness) -> TestResult<Value> {
    placed_podiums(harness).await?;
    let world = id(WORLD);
    let channel = id(CHANNEL);
    let room = qualify_native_entry_room(WorldId::decode(&world).map_err(debug)?)?;
    let mut tx = harness.pool.begin().await?;
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation)
                 SELECT session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,op FROM unnest(ARRAY[2,4]::smallint[]) op")
        .bind(world.as_slice()).bind(channel.as_slice()).execute(&mut *tx).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(world.as_slice()).bind(channel.as_slice())
        .bind(room.compiled().server_digest().as_slice()).bind(room.compiled().client_digest().as_slice())
        .bind(room.frame_binding().digest().as_slice()).execute(&mut *tx).await?;
    let initialized: bool = sqlx::query_scalar("SELECT game_world_reset_initialize(world_id,server_artifact_digest,$3,
                 server_artifact_digest,client_artifact_digest,frame_binding_digest,'TIBIA_SERVER_SAVE_BERLIN_06',1)
                 FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND activation_sequence=1")
        .bind(world.as_slice()).bind(channel.as_slice()).bind(room.movement_cells().scope().map_revision.as_str())
        .fetch_one(&mut *tx).await?;
    assert!(initialized);
    tx.commit().await?;
    Ok(sqlx::query_scalar("SELECT jsonb_agg(jsonb_build_object('house_key',house_key,'spatial_position',encode(spatial_position,'hex')) ORDER BY spatial_position)
                           FROM game_house_tiles WHERE world_id=encode($1,'hex')::uuid")
        .bind(world.as_slice()).fetch_one(&harness.pool).await?)
}

async fn drain(harness: &Harness) -> TestResult {
    // The fixture models terminal sessions. Assignment replacement uses the
    // real writer to close the Channel at a fresh generation.
    sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state=3 WHERE world_id=encode($1,'hex')::uuid AND session_state<>3")
        .bind(id(WORLD).as_slice()).execute(&harness.pool).await?;
    let channel = scope()?;
    let writer = RuntimeScopeAssignmentWriter::open(harness.root.clone(), "item-transfer-writer")
        .await
        .map_err(debug)?;
    let predecessor = harness
        .root
        .read_runtime_scope_predecessor(channel)
        .await
        .map_err(debug)?
        .ok_or("predecessor")?;
    let result = writer
        .submit(&AssignmentRequest {
            operation_key: OperationKey::from_bytes([9; 32]),
            actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
            command: AssignmentCommand::Replace {
                scope: channel,
                predecessor,
                target: harness.node.fact(),
            },
        })
        .await
        .map_err(debug)?;
    assert!(matches!(result, AssignmentOutcome::Committed(_)));
    Ok(())
}

async fn publish(harness: &Harness, tiles: &Value) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(PUBLISH)
        .bind(id(WORLD).as_slice())
        .bind(tiles)
        .fetch_one(&harness.pool)
        .await
}

async fn activate(harness: &Harness) -> Result<Option<String>, sqlx::Error> {
    let fact = harness.node.fact();
    sqlx::query_scalar(ACTIVATE)
        .bind(id(WORLD).as_slice())
        .bind(fact.node_id().as_bytes().as_slice())
        .bind(fact.registration_revision().to_string())
        .fetch_one(&harness.pool)
        .await
}

fn assert_house_refusal<T: std::fmt::Debug>(result: Result<T, sqlx::Error>) {
    let error = result.expect_err("invalid House metadata");
    let database = error.as_database_error();
    assert_eq!(
        database.and_then(|error| error.code()).as_deref(),
        Some("23514")
    );
    assert_eq!(database.map(|error| error.message()), Some(HOUSE_REFUSAL));
}

async fn custody(harness: &Harness) -> TestResult<Value> {
    Ok(sqlx::query_scalar(
        "WITH w AS (SELECT encode($1,'hex')::uuid AS world_id)
         SELECT jsonb_build_object(
           'ownership',(SELECT jsonb_agg(to_jsonb(o) ORDER BY house_key) FROM game_house_ownership o JOIN w USING(world_id)),
           'properties',(SELECT jsonb_agg(to_jsonb(p) ORDER BY house_key) FROM game_house_properties p JOIN w USING(world_id)),
           'locations',(SELECT jsonb_agg(to_jsonb(l) ORDER BY item_instance_id) FROM game_item_house_interior_locations l JOIN w USING(world_id)),
           'provenance',(SELECT jsonb_agg(to_jsonb(r) ORDER BY item_instance_id) FROM game_item_house_reclaim_provenance r JOIN w USING(world_id)))",
    ).bind(id(WORLD).as_slice()).fetch_one(&harness.pool).await?)
}

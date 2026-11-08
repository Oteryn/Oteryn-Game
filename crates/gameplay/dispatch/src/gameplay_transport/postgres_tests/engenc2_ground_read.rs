//! Isolated SQL read/locking qualification; this does not replace full callback admission tests.
#[path = "../../../../durability/src/durability/quest_world_callback/ground_read.rs"]
mod ground_read;

use oteryn_gameplay_quests::quest::world_callbacks::{
    Position,
    ground::{GroundStackItem, GroundStackRequirement},
};
use sqlx::Connection;

#[test]
fn engenc2_ground_postgres_exact_stack_and_owner_lock() -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run())
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let Ok(url) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let mut connection = sqlx::PgConnection::connect(&url).await?;
    let mut contender = sqlx::PgConnection::connect(&url).await?;
    let mut tx = connection.begin().await?;
    sqlx::raw_sql("CREATE TEMP TABLE game_item_instances (item_instance_id uuid,world_id uuid,lifecycle smallint,definition_family text,definition_production_key text,definition_revision_ref text,quantity bigint);
CREATE TEMP TABLE game_item_ground_locations (item_instance_id uuid,world_id uuid,channel_id uuid,scope_kind smallint,instance_id uuid,runtime_scope_ownership_generation numeric,spatial_position bytea,map_revision text,content_revision text,native_room_placement_context bytea,stack_ordinal bigint);
INSERT INTO game_item_instances VALUES ('11111111-1111-1111-1111-111111111111','01010101-0101-0101-0101-010101010101',1,'Item','oteryn:item.tibia.i3031','r1',2),('22222222-2222-2222-2222-222222222222','01010101-0101-0101-0101-010101010101',1,'Item','oteryn:item.tibia.i3035','r1',1);
INSERT INTO game_item_ground_locations SELECT item_instance_id,world_id,'02020202-0202-0202-0202-020202020202',1,NULL,3,decode('0000006400000065fff9','hex'),'map-r1','content-r1',decode('03','hex'),CASE WHEN quantity=2 THEN 10 ELSE 20 END FROM game_item_instances;")
        .execute(&mut *tx).await?;
    let context = ground_read::Context {
        world: &[1; 16],
        channel: &[2; 16],
        generation: 3,
        map: "map-r1",
        content: "content-r1",
        placement: &[3],
    };
    let position = Position {
        x: 100,
        y: 101,
        floor: -7,
    };
    let required = GroundStackRequirement {
        stacks: vec![
            GroundStackItem {
                item: "oteryn:item.tibia.i3031".into(),
                revision: "r1".into(),
                quantity: 2,
            },
            GroundStackItem {
                item: "oteryn:item.tibia.i3035".into(),
                revision: "r1".into(),
                quantity: 1,
            },
        ],
    };
    assert!(
        ground_read::matches(&mut tx, &context, position, &required).await?,
        "exact current Ground stack must qualify"
    );
    let mut key = vec![1_u8; 16];
    key.extend_from_slice(&[2; 16]);
    let mut other = contender.begin().await?;
    let acquired: bool = sqlx::query_scalar(
        "SELECT pg_try_advisory_xact_lock(hashtextextended(encode($1,'hex'),33))",
    )
    .bind(&key)
    .fetch_one(&mut *other)
    .await?;
    assert!(
        !acquired,
        "read must retain existing Ground owner lock until commit"
    );
    sqlx::query("SET LOCAL lock_timeout='100ms'")
        .execute(&mut *other)
        .await?;
    let blocked =
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex'),33))")
            .bind(&key)
            .execute(&mut *other)
            .await;
    assert!(
        blocked
            .as_ref()
            .err()
            .and_then(sqlx::Error::as_database_error)
            .and_then(|e| e.code())
            .is_some_and(|code| code == "55P03"),
        "concurrent Ground writer must wait on the custody owner"
    );
    other.rollback().await?;
    for mutation in [
        "UPDATE game_item_instances SET quantity=3 WHERE quantity=2",
        "UPDATE game_item_instances SET definition_revision_ref='r2'",
        "UPDATE game_item_instances SET lifecycle=2",
        "UPDATE game_item_instances SET definition_family='Creature'",
        "UPDATE game_item_ground_locations SET stack_ordinal=NULL",
        "UPDATE game_item_ground_locations SET stack_ordinal=30-stack_ordinal",
        "UPDATE game_item_ground_locations SET runtime_scope_ownership_generation=4",
        "UPDATE game_item_ground_locations SET channel_id='03030303-0303-0303-0303-030303030303'",
        "UPDATE game_item_ground_locations SET map_revision='map-r2'",
        "UPDATE game_item_ground_locations SET content_revision='content-r2'",
        "UPDATE game_item_ground_locations SET native_room_placement_context=decode('04','hex')",
        "UPDATE game_item_ground_locations SET spatial_position=decode('0000006400000066fff9','hex')",
    ] {
        sqlx::query("SAVEPOINT mutation").execute(&mut *tx).await?;
        sqlx::query(mutation).execute(&mut *tx).await?;
        assert!(
            !ground_read::matches(&mut tx, &context, position, &required).await?,
            "accepted {mutation}"
        );
        sqlx::query("ROLLBACK TO mutation")
            .execute(&mut *tx)
            .await?;
    }
    sqlx::query("INSERT INTO game_item_instances SELECT '33333333-3333-3333-3333-333333333333',world_id,lifecycle,definition_family,'oteryn:item.tibia.i3031',definition_revision_ref,1 FROM game_item_instances LIMIT 1")
        .execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_ground_locations SELECT '33333333-3333-3333-3333-333333333333',world_id,channel_id,scope_kind,instance_id,runtime_scope_ownership_generation,spatial_position,map_revision,content_revision,native_room_placement_context,30 FROM game_item_ground_locations LIMIT 1")
        .execute(&mut *tx).await?;
    assert!(
        !ground_read::matches(&mut tx, &context, position, &required).await?,
        "an additional Ground instance invalidates the complete stack"
    );
    tx.rollback().await?;
    let mut other = contender.begin().await?;
    let acquired: bool = sqlx::query_scalar(
        "SELECT pg_try_advisory_xact_lock(hashtextextended(encode($1,'hex'),33))",
    )
    .bind(&key)
    .fetch_one(&mut *other)
    .await?;
    assert!(acquired, "rollback must release Ground owner lock");
    other.rollback().await?;
    eprintln!(
        "ENGENC2_GROUND_PG_EXECUTED elapsed_s={:.6}",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}

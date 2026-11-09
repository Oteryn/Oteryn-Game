//! A busy Item row must refuse a Ground prerequisite without aborting its quest transaction.
use super::engenc2_ground_support as ground_read;
use oteryn_gameplay_quests::quest::world_callbacks::{
    Position,
    ground::{GroundStackItem, GroundStackRequirement},
};
use sqlx::Connection;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn engenc2_ground_busy_item_refuses_without_poisoning_quest_transaction() -> Result {
    let Ok(url) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run(&url))
}

async fn run(url: &str) -> Result {
    let began = std::time::Instant::now();
    let mut writer = sqlx::PgConnection::connect(url).await?;
    let mut reader = sqlx::PgConnection::connect(url).await?;
    // Private, short-lived shared fixture: TEMP tables cannot be row-locked by another session.
    // The identifier contains only a fixed prefix and process/time digits, never user input.
    let schema = format!(
        "engenc2_ground_busy_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos()
    );
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}")))
        .execute(&mut writer)
        .await?;
    let result = exercise(&mut writer, &mut reader, &schema).await;
    sqlx::query("SET search_path TO public")
        .execute(&mut reader)
        .await?;
    sqlx::query("SET search_path TO public")
        .execute(&mut writer)
        .await?;
    let cleanup = sqlx::query(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&mut writer)
        .await;
    result?;
    cleanup?;
    eprintln!(
        "ENGENC2_GROUND_BUSY_PG_EXECUTED elapsed_s={:.6}",
        began.elapsed().as_secs_f64()
    );
    Ok(())
}

async fn exercise(
    writer: &mut sqlx::PgConnection,
    reader: &mut sqlx::PgConnection,
    schema: &str,
) -> Result {
    let search = format!("SET search_path TO {schema},public");
    sqlx::query(sqlx::AssertSqlSafe(search.as_str()))
        .execute(&mut *writer)
        .await?;
    sqlx::query(sqlx::AssertSqlSafe(search.as_str()))
        .execute(&mut *reader)
        .await?;
    sqlx::raw_sql("CREATE TABLE game_item_instances (item_instance_id uuid,world_id uuid,lifecycle smallint,definition_family text,definition_production_key text,definition_revision_ref text,quantity bigint);
CREATE TABLE game_item_ground_locations (item_instance_id uuid,world_id uuid,channel_id uuid,scope_kind smallint,instance_id uuid,runtime_scope_ownership_generation numeric,spatial_position bytea,map_revision text,content_revision text,native_room_placement_context bytea,stack_ordinal bigint);
INSERT INTO game_item_instances VALUES ('11111111-1111-1111-1111-111111111111','04040404-0404-0404-0404-040404040404',1,'Item','oteryn:item.tibia.i3031','r1',2);
INSERT INTO game_item_ground_locations SELECT item_instance_id,world_id,'05050505-0505-0505-0505-050505050505',1,NULL,3,decode('0000006400000065fff9','hex'),'map-r1','content-r1',decode('03','hex'),10 FROM game_item_instances;").execute(&mut *writer).await?;
    let mut write = writer.begin().await?;
    sqlx::query("SELECT item_instance_id FROM game_item_instances FOR UPDATE")
        .execute(&mut *write)
        .await?;
    let mut read = reader.begin().await?;
    sqlx::query("SET LOCAL lock_timeout='200ms'")
        .execute(&mut *read)
        .await?;
    let context = ground_read::Context {
        world: &[4; 16],
        channel: &[5; 16],
        generation: 3,
        map: "map-r1",
        content: "content-r1",
        placement: &[3],
    };
    let required = GroundStackRequirement {
        stacks: vec![GroundStackItem {
            item: "oteryn:item.tibia.i3031".into(),
            revision: "r1".into(),
            quantity: 2,
        }],
    };
    let position = Position {
        x: 100,
        y: 101,
        floor: -7,
    };
    let result = ground_read::matches(&mut read, &context, position, &required).await;
    if !matches!(result, Ok(false)) {
        return Err(format!("busy stack must refuse without a database error: {result:?}").into());
    }
    let intact: i32 = sqlx::query_scalar("SELECT 42")
        .fetch_one(&mut *read)
        .await?;
    if intact != 42 {
        return Err("caller must retain a usable outer quest transaction".into());
    }
    // A mutation holding the Item row cannot get the Ground owner lock until the
    // refused quest transaction rolls back; its wait also proves this PG case ran.
    let mut key = vec![4_u8; 16];
    key.extend_from_slice(&[5; 16]);
    sqlx::query("SET LOCAL lock_timeout='100ms'")
        .execute(&mut *write)
        .await?;
    let blocked =
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex'),33))")
            .bind(&key)
            .execute(&mut *write)
            .await;
    if !blocked
        .as_ref()
        .err()
        .and_then(sqlx::Error::as_database_error)
        .and_then(|e| e.code())
        .is_some_and(|c| c == "55P03")
    {
        return Err("Ground owner lock escaped the outer quest transaction".into());
    }
    write.rollback().await?;
    read.rollback().await?;
    let mut read = reader.begin().await?;
    if !ground_read::matches(&mut read, &context, position, &required).await? {
        return Err("the same stack must qualify after contention ends".into());
    }
    read.rollback().await?;
    Ok(())
}

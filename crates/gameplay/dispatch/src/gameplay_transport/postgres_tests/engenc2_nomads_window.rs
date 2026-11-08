//! Execute the production Nomads s4 Ground recipe against real PostgreSQL custody reads.
//! These isolated readset tables do not claim to qualify the complete USE/quest receipt path.
#[path = "../../../../durability/src/durability/quest_world_callback/ground_read.rs"]
mod ground_read;
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Position, WorldRequirement};
use sqlx::Connection;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

#[test]
fn engenc2_nomads_window_current_crates_and_amphora_postgres() -> Result {
    let Ok(url) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(run(&url))
}

fn spatial(p: Position) -> Vec<u8> {
    let mut bytes = i32::from(p.x).to_be_bytes().to_vec();
    bytes.extend_from_slice(&i32::from(p.y).to_be_bytes());
    bytes.extend_from_slice(&i16::from(p.floor).to_be_bytes());
    bytes
}

async fn qualifies(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    callback: &Callback,
) -> Result<bool> {
    let context = ground_read::Context {
        world: &[6; 16],
        channel: &[7; 16],
        generation: 3,
        map: "map-r1",
        content: "content-r1",
        placement: &[3],
    };
    for requirement in &callback.world_requirements {
        let WorldRequirement::Ground { position, ground } = requirement else {
            return Err("Nomads recipe lost mutable Ground evidence".into());
        };
        if !ground_read::matches(tx, &context, *position, ground).await? {
            return Ok(false);
        }
    }
    Ok(true)
}

async fn run(url: &str) -> Result {
    let began = std::time::Instant::now();
    let callback = world_callbacks::load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/engenc2/nomads-land-window")
        .ok_or("production window callback")?;
    let mut owner = sqlx::PgConnection::connect(url).await?;
    let mut contender = sqlx::PgConnection::connect(url).await?;
    let mut tx = owner.begin().await?;
    sqlx::raw_sql("CREATE TEMP TABLE game_item_instances (item_instance_id uuid,world_id uuid,lifecycle smallint,definition_family text,definition_production_key text,definition_revision_ref text,quantity bigint);
CREATE TEMP TABLE game_item_ground_locations (item_instance_id uuid,world_id uuid,channel_id uuid,scope_kind smallint,instance_id uuid,runtime_scope_ownership_generation numeric,spatial_position bytea,map_revision text,content_revision text,native_room_placement_context bytea,stack_ordinal bigint);")
        .execute(&mut *tx).await?;
    let mut instance = 0_u8;
    for requirement in &callback.world_requirements {
        let WorldRequirement::Ground { position, ground } = requirement else {
            return Err("Ground recipe".into());
        };
        for item in &ground.stacks {
            instance += 1;
            let id = sqlx::types::Uuid::from_bytes([instance; 16]);
            sqlx::query("INSERT INTO game_item_instances VALUES ($1,encode($2,'hex')::uuid,1,'Item',$3,$4,$5)")
                .bind(id).bind([6_u8;16].as_slice()).bind(&item.item).bind(&item.revision).bind(i64::from(item.quantity)).execute(&mut *tx).await?;
            sqlx::query("INSERT INTO game_item_ground_locations VALUES ($1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,NULL,3,$4,'map-r1','content-r1',decode('03','hex'),$5)")
                .bind(id).bind([6_u8;16].as_slice()).bind([7_u8;16].as_slice()).bind(spatial(*position)).bind(i64::from(instance)*10).execute(&mut *tx).await?;
        }
    }
    assert_eq!(instance, 4);
    assert!(
        qualifies(&mut tx, &callback).await?,
        "the exact production recipe must qualify"
    );
    for mutation in [
        "UPDATE game_item_ground_locations SET spatial_position=decode('000081be00008056fff7','hex') WHERE item_instance_id='01010101-0101-0101-0101-010101010101'",
        "UPDATE game_item_ground_locations SET stack_ordinal=100-stack_ordinal WHERE item_instance_id IN ('02020202-0202-0202-0202-020202020202','03030303-0303-0303-0303-030303030303')",
        "DELETE FROM game_item_ground_locations WHERE item_instance_id='03030303-0303-0303-0303-030303030303'",
        "UPDATE game_item_instances SET quantity=2 WHERE item_instance_id='03030303-0303-0303-0303-030303030303'",
        "UPDATE game_item_ground_locations SET content_revision='stale'",
    ] {
        sqlx::query("SAVEPOINT arrangement")
            .execute(&mut *tx)
            .await?;
        sqlx::query(mutation).execute(&mut *tx).await?;
        assert!(
            !qualifies(&mut tx, &callback).await?,
            "a changed live arrangement must refuse: {mutation}"
        );
        sqlx::query("ROLLBACK TO arrangement")
            .execute(&mut *tx)
            .await?;
    }
    let mut other = contender.begin().await?;
    sqlx::query("SET LOCAL lock_timeout='100ms'")
        .execute(&mut *other)
        .await?;
    let mut key = vec![6_u8; 16];
    key.extend_from_slice(&[7; 16]);
    let blocked =
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex'),33))")
            .bind(key)
            .execute(&mut *other)
            .await;
    assert!(
        blocked
            .as_ref()
            .err()
            .and_then(sqlx::Error::as_database_error)
            .and_then(|e| e.code())
            .is_some_and(|c| c == "55P03")
    );
    other.rollback().await?;
    tx.rollback().await?;
    eprintln!(
        "ENGENC2_NOMADS_WINDOW_PG_EXECUTED elapsed_s={:.6} pattern_inputs={instance}",
        began.elapsed().as_secs_f64()
    );
    Ok(())
}

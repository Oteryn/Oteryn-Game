use super::*;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::InventoryItem;

pub(super) fn run(tag: &str, case: impl AsyncFnOnce(&Harness) -> TestResult) -> TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    fixture::runtime()?.block_on(async {
        // The shared harness removes/recreates its recovery directory by tag.
        // Each parallel case needs its own tag as well as its own database.
        let harness = Harness::create(admin, tag, true).await?;
        let result = case(&harness).await;
        harness.cleanup().await?;
        result
    })
}

pub(super) async fn seed_items(
    pool: &sqlx::PgPool,
    count: i64,
    quantity: i32,
) -> TestResult<Vec<String>> {
    let mut tx = pool.begin().await?;
    tx.execute("SET LOCAL session_replication_role = replica")
        .await?;
    let items = sqlx::query_scalar(
        "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,\
         definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) \
         SELECT game_character_uuid_v7(),encode($1,'hex')::uuid,'Item',\
         'oteryn:content.item.vase','r1',$2,1,game_character_uuid_v7() \
         FROM generate_series(1,$3) RETURNING item_instance_id::text",
    ).bind(id(fixture::WORLD).as_slice()).bind(quantity).bind(count)
        .fetch_all(&mut *tx).await?;
    tx.commit().await?;
    Ok(items)
}

pub(super) async fn seed_character(pool: &sqlx::PgPool, character: u8, world: u8) -> TestResult {
    let mut tx = pool.begin().await?;
    tx.execute("SET LOCAL session_replication_role = replica")
        .await?;
    sqlx::query(
        "INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,\
         character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) \
         SELECT encode($1,'hex')::uuid,account_id,encode($2,'hex')::uuid,1,1,\
         profile_revision,ruleset_revision,content_revision,starter_template_revision,$3 \
         FROM game_character_roots WHERE character_id=encode($4,'hex')::uuid",
    ).bind(id(character).as_slice()).bind(id(world).as_slice())
        .bind(format!("Depot{}", char::from(b'A' + character % 26)))
        .bind(id(fixture::CHARACTER).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn insert(
    pool: &sqlx::PgPool,
    item: &str,
    town: i32,
    box_number: i16,
) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
    insert_on(&mut *pool.acquire().await?, item, town, box_number).await
}

pub(super) async fn insert_on(
    connection: &mut PgConnection,
    item: &str,
    town: i32,
    box_number: i16,
) -> Result<sqlx::postgres::PgQueryResult, sqlx::Error> {
    sqlx::query(
        "INSERT INTO game_item_character_depot_locations \
         (item_instance_id,world_id,character_id,town_id,box_number) \
         VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5)",
    )
    .bind(item)
    .bind(id(fixture::WORLD).as_slice())
    .bind(id(fixture::CHARACTER).as_slice())
    .bind(town)
    .bind(box_number)
    .execute(connection)
    .await
}

pub(super) fn sqlstate<T>(result: Result<T, sqlx::Error>) -> TestResult<String> {
    match result {
        Ok(_) => Err("expected a rejected write".into()),
        Err(error) => error
            .as_database_error()
            .and_then(|error| error.code())
            .map(|code| code.into_owned())
            .ok_or_else(|| error.into()),
    }
}

pub(super) fn item(uuid: &str, quantity: u32) -> TestResult<InventoryItem> {
    let hex = uuid.replace('-', "");
    let mut item_instance_id = [0; 16];
    for (index, byte) in item_instance_id.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)?;
    }
    Ok(InventoryItem {
        item_instance_id,
        definition: TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:content.item.vase".into(),
            revision_ref: "r1".into(),
        },
        quantity,
    })
}

pub(super) async fn wait_for_lock(pool: &sqlx::PgPool, pid: i32) -> TestResult {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        loop {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1 AND wait_event_type='Lock')")
                .bind(pid).fetch_one(pool).await?;
            if waiting {
                return Ok::<_, sqlx::Error>(());
            }
            tokio::task::yield_now().await;
        }
    }).await??;
    Ok(())
}

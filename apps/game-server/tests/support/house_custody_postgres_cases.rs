// HOUSE-CUSTODY-1 (migration 0025): the HouseInterior location table, its
// HousingReclaimProvenance and the deferred item-level exclusivity guard over
// every location table (HOUSE-CUSTODY-0 §3.1-§3.3, §3.5). Storage only: no
// writer exists, so fixtures are seeded as the table owner with triggers off
// and each case then writes with every trigger and constraint on.

use sqlx::{Connection, Executor, PgConnection};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const HOUSE: &str = "oteryn:content.house.admiral_s_avenue_1";
const OTHER_HOUSE: &str = "oteryn:content.house.admiral_s_avenue_2";

struct Database {
    admin_url: String,
    name: String,
    url: String,
}

impl Database {
    async fn create(admin_url: String) -> TestResult<Self> {
        if !admin_url.starts_with("postgresql://oteryn_test_admin:")
            || !admin_url.ends_with("@127.0.0.1:5432/postgres")
        {
            return Err("unsafe PostgreSQL test admin URL".into());
        }
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let name = format!("house_custody_{suffix}");
        let mut admin = PgConnection::connect(&admin_url).await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE DATABASE {name}"
            ))))
            .await?;
        admin.close().await?;
        let prefix = admin_url
            .strip_suffix("/postgres")
            .ok_or("invalid admin URL")?;
        let url = format!("{prefix}/{name}");
        let mut connection = PgConnection::connect(&url).await?;
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&mut connection)
            .await?;
        assert_eq!(version, "170006", "canonical target is PostgreSQL 17.6");
        sqlx::migrate!("./migrations").run(&mut connection).await?;
        connection.close().await?;
        Ok(Self {
            admin_url,
            name,
            url,
        })
    }

    async fn cleanup(self) -> TestResult {
        let mut admin = PgConnection::connect(&self.admin_url).await?;
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP DATABASE {} WITH (FORCE)",
                self.name
            ))))
            .await?;
        admin.close().await?;
        Ok(())
    }
}

fn run(case: impl AsyncFnOnce(&mut PgConnection, &str) -> TestResult) -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured");
        return Ok(());
    };
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let database = Database::create(admin).await?;
            let outcome = async {
                let mut connection = PgConnection::connect(&database.url).await?;
                case(&mut connection, &database.url).await
            }
            .await;
            database.cleanup().await?;
            outcome
        })
}

/// The SQLSTATE of a failed statement or commit.
fn sqlstate<T>(result: Result<T, sqlx::Error>) -> TestResult<String> {
    match result {
        Ok(_) => Err("expected the database to reject the write".into()),
        Err(error) => Ok(error
            .as_database_error()
            .and_then(|database| database.code())
            .map(|code| code.into_owned())
            .ok_or_else(|| format!("not a database error: {error}"))?),
    }
}

fn message<T>(result: Result<T, sqlx::Error>) -> TestResult<String> {
    match result {
        Ok(_) => Err("expected the database to reject the write".into()),
        Err(error) => Ok(error.to_string()),
    }
}

/// A World, an Account, one Character and `items` live ItemInstances with no
/// location, seeded with triggers off. Returns (world, character, items).
async fn seed(
    connection: &mut PgConnection,
    items: usize,
) -> TestResult<(String, String, Vec<String>)> {
    let mut tx = connection.begin().await?;
    tx.execute("SET LOCAL session_replication_role = replica")
        .await?;
    let world: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut *tx)
        .await?;
    let character = seed_character(&mut tx, &world).await?;
    let mut seeded = Vec::with_capacity(items);
    for _ in 0..items {
        let item: String = sqlx::query_scalar(
            "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
               definition_production_key, definition_revision_ref, quantity, lifecycle, \
               minted_transaction_id) \
             VALUES (game_character_uuid_v7(), $1::uuid, 'Item', 'oteryn:content.item.vase', \
               'r1', 1, 1, game_character_uuid_v7()) \
             RETURNING item_instance_id::text",
        )
        .bind(&world)
        .fetch_one(&mut *tx)
        .await?;
        seeded.push(item);
    }
    tx.commit().await?;
    Ok((world, character, seeded))
}

async fn seed_character(tx: &mut PgConnection, world: &str) -> TestResult<String> {
    let account: String = sqlx::query_scalar(
        "INSERT INTO game_character_account_guards(account_id) \
         VALUES (game_character_uuid_v7()) RETURNING account_id::text",
    )
    .fetch_one(&mut *tx)
    .await?;
    Ok(sqlx::query_scalar(
        "INSERT INTO game_character_roots(character_id, account_id, world_id, lifecycle, \
           character_revision, profile_revision, ruleset_revision, content_revision, \
           starter_template_revision, name) \
         VALUES (game_character_uuid_v7(), $1::uuid, $2::uuid, 1, 1, 'p1', 'r1', 'c1', 's1', \
           'Hero' || translate(substr(md5(random()::text), 1, 12), '0123456789', 'ghijklmnop')) \
         RETURNING character_id::text",
    )
    .bind(account)
    .bind(world)
    .fetch_one(&mut *tx)
    .await?)
}

async fn insert_location(
    tx: &mut PgConnection,
    world: &str,
    item: &str,
    house: &str,
    ordinal: i64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO game_item_house_interior_locations(item_instance_id, world_id, house_key, \
           spatial_position, stack_ordinal, placed_transaction_id) \
         VALUES ($1::uuid, $2::uuid, $3, '\\x0100020007'::bytea, $4, game_character_uuid_v7())",
    )
    .bind(item)
    .bind(world)
    .bind(house)
    .bind(ordinal)
    .execute(&mut *tx)
    .await
    .map(|_| ())
}

/// Names the placement transaction of the item's location row, if any.
async fn insert_provenance(
    tx: &mut PgConnection,
    world: &str,
    item: &str,
    house: &str,
    character: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO game_item_house_reclaim_provenance(item_instance_id, world_id, house_key, \
           reclaim_subject_character_id, placement_transaction_id, provenance_revision) \
         VALUES ($1::uuid, $2::uuid, $3, $4::uuid, \
           COALESCE((SELECT placed_transaction_id FROM game_item_house_interior_locations \
                      WHERE item_instance_id = $1::uuid), game_character_uuid_v7()), 1)",
    )
    .bind(item)
    .bind(world)
    .bind(house)
    .bind(character)
    .execute(&mut *tx)
    .await
    .map(|_| ())
}

/// One committed placement: location and provenance in one transaction.
async fn place(
    connection: &mut PgConnection,
    world: &str,
    item: &str,
    character: &str,
    ordinal: i64,
) -> Result<(), sqlx::Error> {
    let mut tx = connection.begin().await?;
    insert_location(&mut tx, world, item, HOUSE, ordinal).await?;
    insert_provenance(&mut tx, world, item, HOUSE, character).await?;
    tx.commit().await
}

async fn location_rows(connection: &mut PgConnection, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT (SELECT count(*) FROM game_item_ground_locations WHERE item_instance_id = $1::uuid) \
              + (SELECT count(*) FROM game_item_container_slots WHERE item_instance_id = $1::uuid) \
              + (SELECT count(*) FROM game_item_container_entries WHERE item_instance_id = $1::uuid) \
              + (SELECT count(*) FROM game_item_corpse_container_entries \
                  WHERE item_instance_id = $1::uuid) \
              + (SELECT count(*) FROM game_item_house_interior_locations \
                  WHERE item_instance_id = $1::uuid)",
    )
    .bind(item)
    .fetch_one(connection)
    .await?)
}

#[test]
fn house_item_has_exactly_one_location_while_live_and_none_when_retired() -> TestResult {
    run(async |connection, url| {
        let (world, character, items) = seed(connection, 5).await?;

        // Positive: a live item placed in a house has exactly one location.
        place(connection, &world, &items[0], &character, 1).await?;
        assert_eq!(location_rows(connection, &items[0]).await?, 1);

        // The guard locks the item row, so a concurrent placement of the same
        // item into another location table waits and then sees this one.
        {
            let mut tx = connection.begin().await?;
            insert_location(&mut tx, &world, &items[4], HOUSE, 5).await?;
            insert_provenance(&mut tx, &world, &items[4], HOUSE, &character).await?;
            tx.execute("SET CONSTRAINTS ALL IMMEDIATE").await?;
            let mut other = PgConnection::connect(url).await?;
            let locked = sqlx::query(
                "SELECT 1 FROM game_item_instances WHERE item_instance_id = $1::uuid \
                   FOR NO KEY UPDATE NOWAIT",
            )
            .bind(&items[4])
            .execute(&mut other)
            .await;
            assert_eq!(sqlstate(locked)?, "55P03");
            other.close().await?;
            tx.rollback().await?;
        }

        // A live item already on Ground cannot also enter a house.
        {
            let mut tx = connection.begin().await?;
            tx.execute("SET LOCAL session_replication_role = replica")
                .await?;
            sqlx::query(
                "INSERT INTO game_item_ground_locations(item_instance_id, world_id, channel_id, \
                   runtime_scope_ownership_generation, spatial_position, corpse_ref, \
                   map_revision, content_revision, native_room_placement_context) \
                 VALUES ($1::uuid, $2::uuid, game_character_uuid_v7(), 1, '\\x01', '\\x01', \
                   'm1', 'c1', '\\x01')",
            )
            .bind(&items[1])
            .bind(&world)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
        }
        let rejected = place(connection, &world, &items[1], &character, 2).await;
        assert!(message(rejected)?.contains("exactly one location"));
        assert_eq!(location_rows(connection, &items[1]).await?, 1);

        // A retired item has no location, so it cannot enter a house.
        {
            let mut tx = connection.begin().await?;
            tx.execute("SET LOCAL session_replication_role = replica")
                .await?;
            sqlx::query(
                "UPDATE game_item_instances SET lifecycle = 2, quantity = 0, \
                   last_transaction_id = game_character_uuid_v7() \
                 WHERE item_instance_id = $1::uuid",
            )
            .bind(&items[2])
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
        }
        let rejected = place(connection, &world, &items[2], &character, 3).await;
        assert!(message(rejected)?.contains("exactly one location"));
        assert_eq!(location_rows(connection, &items[2]).await?, 0);
        // The same guard accepts the retired item with no location.
        sqlx::query("SELECT game_item_location_exclusive($1::uuid)")
            .bind(&items[2])
            .execute(&mut *connection)
            .await?;

        // A live item that leaves its house without a destination has none.
        let mut tx = connection.begin().await?;
        sqlx::query(
            "DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id = $1::uuid",
        )
        .bind(&items[0])
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "DELETE FROM game_item_house_interior_locations WHERE item_instance_id = $1::uuid",
        )
        .bind(&items[0])
        .execute(&mut *tx)
        .await?;
        assert!(message(tx.commit().await)?.contains("exactly one location"));
        assert_eq!(location_rows(connection, &items[0]).await?, 1);

        // A house item has no contents: a child entry under it fails the guard.
        place(connection, &world, &items[3], &character, 4).await?;
        let mut tx = connection.begin().await?;
        tx.execute("SET LOCAL session_replication_role = replica")
            .await?;
        sqlx::query(
            "INSERT INTO game_item_corpse_container_entries(item_instance_id, world_id, \
               parent_item_instance_id, placement_ordinal, placed_transaction_id) \
             VALUES ($1::uuid, $2::uuid, $3::uuid, 1, game_character_uuid_v7())",
        )
        .bind(&items[4])
        .bind(&world)
        .bind(&items[3])
        .execute(&mut *tx)
        .await?;
        let contents = sqlx::query("SELECT game_item_location_exclusive($1::uuid)")
            .bind(&items[3])
            .execute(&mut *tx)
            .await;
        assert!(message(contents)?.contains("cannot have contents"));
        tx.rollback().await?;

        // The guard covers all six current custody locations and the item row.
        // Migration 0035 adds equipment custody to the original house contract.
        let guarded: Vec<String> = sqlx::query_scalar(
            "SELECT tgrelid::regclass::text FROM pg_trigger \
              WHERE tgfoid = 'game_item_location_exclusivity_guard'::regproc \
              ORDER BY 1",
        )
        .fetch_all(&mut *connection)
        .await?;
        assert_eq!(
            guarded,
            [
                "game_character_equipment_slots",
                "game_item_container_entries",
                "game_item_container_slots",
                "game_item_corpse_container_entries",
                "game_item_ground_locations",
                "game_item_house_interior_locations",
                "game_item_instances",
            ]
        );
        Ok(())
    })
}

#[test]
fn house_location_and_provenance_are_one_to_one() -> TestResult {
    run(async |connection, _url| {
        let (world, character, items) = seed(connection, 4).await?;

        // A HouseInterior row without its provenance fails at commit, not before.
        let mut tx = connection.begin().await?;
        insert_location(&mut tx, &world, &items[0], HOUSE, 1).await?;
        assert!(message(tx.commit().await)?.contains("HousingReclaimProvenance"));

        // A provenance without its row fails at commit (deferred FK).
        let mut tx = connection.begin().await?;
        insert_provenance(&mut tx, &world, &items[0], HOUSE, &character).await?;
        assert_eq!(sqlstate(tx.commit().await)?, "23503");

        // A provenance naming another house than the row fails too.
        let mut tx = connection.begin().await?;
        insert_location(&mut tx, &world, &items[0], HOUSE, 1).await?;
        insert_provenance(&mut tx, &world, &items[0], OTHER_HOUSE, &character).await?;
        assert!(tx.commit().await.is_err());

        // Positive, then a second provenance for the same item fails.
        place(connection, &world, &items[0], &character, 1).await?;
        let second = insert_provenance(connection, &world, &items[0], HOUSE, &character).await;
        assert_eq!(sqlstate(second)?, "23505");

        // Deleting the provenance of an item still in the house fails at commit.
        let mut tx = connection.begin().await?;
        sqlx::query(
            "DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id = $1::uuid",
        )
        .bind(&items[0])
        .execute(&mut *tx)
        .await?;
        assert!(message(tx.commit().await)?.contains("HousingReclaimProvenance"));

        // Nor can it be deleted and reinserted while the item stays, with the
        // same subject or another one: only the update guard changes it.
        let other = {
            let mut tx = connection.begin().await?;
            let other = seed_character(&mut tx, &world).await?;
            tx.commit().await?;
            other
        };
        for subject in [&character, &other] {
            let mut tx = connection.begin().await?;
            sqlx::query(
                "DELETE FROM game_item_house_reclaim_provenance WHERE item_instance_id = $1::uuid",
            )
            .bind(&items[0])
            .execute(&mut *tx)
            .await?;
            insert_provenance(&mut tx, &world, &items[0], HOUSE, subject).await?;
            assert!(message(tx.commit().await)?.contains("only when its item leaves the house"));
        }

        // The provenance names its row's placement transaction: another one
        // fails at commit.
        let mut tx = connection.begin().await?;
        insert_location(&mut tx, &world, &items[3], HOUSE, 3).await?;
        insert_provenance(&mut tx, &world, &items[3], HOUSE, &character).await?;
        sqlx::query(
            "UPDATE game_item_house_reclaim_provenance \
                SET provenance_revision = 2, placement_transaction_id = game_character_uuid_v7() \
              WHERE item_instance_id = $1::uuid",
        )
        .bind(&items[3])
        .execute(&mut *tx)
        .await?;
        // Both the row-side trigger and the provenance FK reject it.
        assert!(tx.commit().await.is_err());

        // A same-house move replaces the row and keeps the provenance, which
        // takes the new placement transaction with one revision more.
        let mut tx = connection.begin().await?;
        sqlx::query(
            "DELETE FROM game_item_house_interior_locations WHERE item_instance_id = $1::uuid",
        )
        .bind(&items[0])
        .execute(&mut *tx)
        .await?;
        insert_location(&mut tx, &world, &items[0], HOUSE, 7).await?;
        sqlx::query(
            "UPDATE game_item_house_reclaim_provenance p \
                SET provenance_revision = 2, placement_transaction_id = h.placed_transaction_id \
               FROM game_item_house_interior_locations h \
              WHERE p.item_instance_id = $1::uuid AND h.item_instance_id = p.item_instance_id",
        )
        .bind(&items[0])
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        // Moving the row without re-pointing the provenance fails at commit.
        let mut tx = connection.begin().await?;
        sqlx::query(
            "DELETE FROM game_item_house_interior_locations WHERE item_instance_id = $1::uuid",
        )
        .bind(&items[0])
        .execute(&mut *tx)
        .await?;
        insert_location(&mut tx, &world, &items[0], HOUSE, 8).await?;
        assert!(tx.commit().await.is_err());
        // Nor can a replacing row reuse the old placement transaction, whether
        // the provenance is left untouched or cycled through a temporary
        // placement and back: a placement transaction is used once, ever.
        let current: String = sqlx::query_scalar(
            "SELECT placed_transaction_id::text FROM game_item_house_interior_locations \
              WHERE item_instance_id = $1::uuid",
        )
        .bind(&items[0])
        .fetch_one(&mut *connection)
        .await?;
        for cycle in [false, true] {
            let mut tx = connection.begin().await?;
            sqlx::query(
                "DELETE FROM game_item_house_interior_locations WHERE item_instance_id = $1::uuid",
            )
            .bind(&items[0])
            .execute(&mut *tx)
            .await?;
            if cycle {
                for placement in ["game_character_uuid_v7()", "$2::uuid"] {
                    sqlx::query(sqlx::AssertSqlSafe(format!(
                        "UPDATE game_item_house_reclaim_provenance \
                            SET provenance_revision = provenance_revision + 1, \
                                placement_transaction_id = {placement} \
                          WHERE item_instance_id = $1::uuid AND $2::uuid IS NOT NULL"
                    )))
                    .bind(&items[0])
                    .bind(&current)
                    .execute(&mut *tx)
                    .await?;
                }
            }
            let reused = sqlx::query(
                "INSERT INTO game_item_house_interior_locations(item_instance_id, world_id, \
                   house_key, spatial_position, stack_ordinal, placed_transaction_id) \
                 VALUES ($1::uuid, $2::uuid, $3, '\\x0100020007'::bytea, 9, $4::uuid)",
            )
            .bind(&items[0])
            .bind(&world)
            .bind(HOUSE)
            .bind(&current)
            .execute(&mut *tx)
            .await;
            assert!(message(reused)?.contains("already used"));
            tx.rollback().await?;
        }
        // A revision bump that does not name a new placement is rejected.
        let bumped = sqlx::query(
            "UPDATE game_item_house_reclaim_provenance SET provenance_revision = 3 \
              WHERE item_instance_id = $1::uuid",
        )
        .bind(&items[0])
        .execute(&mut *connection)
        .await;
        assert_eq!(sqlstate(bumped)?, "23514");
        for change in [
            "provenance_revision = 4",
            "provenance_revision = 3, reclaim_subject_character_id = $2::uuid",
        ] {
            let other = {
                let mut tx = connection.begin().await?;
                let other = seed_character(&mut tx, &world).await?;
                tx.commit().await?;
                other
            };
            let rejected = sqlx::query(sqlx::AssertSqlSafe(format!(
                "UPDATE game_item_house_reclaim_provenance SET {change} \
                  WHERE item_instance_id = $1::uuid AND $2::uuid IS NOT NULL"
            )))
            .bind(&items[0])
            .bind(other)
            .execute(&mut *connection)
            .await;
            assert_eq!(sqlstate(rejected)?, "23514");
        }
        let moved = sqlx::query(
            "UPDATE game_item_house_interior_locations SET stack_ordinal = 9 \
              WHERE item_instance_id = $1::uuid",
        )
        .bind(&items[0])
        .execute(&mut *connection)
        .await;
        assert_eq!(sqlstate(moved)?, "23514");

        // Two items with the same (world, house, position, ordinal) fail.
        let duplicate = place(connection, &world, &items[1], &character, 7).await;
        assert_eq!(sqlstate(duplicate)?, "23505");
        place(connection, &world, &items[1], &character, 2).await?;
        // The same ordinal in another house is a different slot.
        let mut tx = connection.begin().await?;
        insert_location(&mut tx, &world, &items[2], OTHER_HOUSE, 1).await?;
        insert_provenance(&mut tx, &world, &items[2], OTHER_HOUSE, &character).await?;
        tx.commit().await?;

        // The reclaim subject is a Character of the item's own World.
        let foreign = {
            let mut tx = connection.begin().await?;
            tx.execute("SET LOCAL session_replication_role = replica")
                .await?;
            let other_world: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
                .fetch_one(&mut *tx)
                .await?;
            let foreign = seed_character(&mut tx, &other_world).await?;
            tx.commit().await?;
            foreign
        };
        let mut tx = connection.begin().await?;
        insert_location(&mut tx, &world, &items[3], HOUSE, 4).await?;
        let cross = insert_provenance(&mut tx, &world, &items[3], HOUSE, &foreign).await;
        assert!(message(cross)?.contains("same World"));
        tx.rollback().await?;

        // The HouseId is the revision-free House content key.
        let unkeyed = place_with_key(connection, &world, &items[3], &character, "house-7").await;
        assert_eq!(sqlstate(unkeyed)?, "23514");
        Ok(())
    })
}

async fn place_with_key(
    connection: &mut PgConnection,
    world: &str,
    item: &str,
    character: &str,
    house: &str,
) -> Result<(), sqlx::Error> {
    let mut tx = connection.begin().await?;
    insert_location(&mut tx, world, item, house, 1).await?;
    insert_provenance(&mut tx, world, item, house, character).await?;
    tx.commit().await
}

#[test]
fn runtime_role_cannot_read_or_write_the_house_tables() -> TestResult {
    run(async |connection, _url| {
        let (world, character, items) = seed(connection, 1).await?;
        place(connection, &world, &items[0], &character, 1).await?;
        for table in [
            "game_item_house_interior_locations",
            "game_item_house_reclaim_provenance",
            "game_item_house_placement_transactions",
        ] {
            let any: bool = sqlx::query_scalar(
                "SELECT has_table_privilege('oteryn_game_runtime', $1, \
                   'SELECT, INSERT, UPDATE, DELETE, TRUNCATE, REFERENCES, TRIGGER') \
                     OR has_any_column_privilege('oteryn_game_runtime', $1, \
                   'SELECT, INSERT, UPDATE, REFERENCES')",
            )
            .bind(table)
            .fetch_one(&mut *connection)
            .await?;
            assert!(!any, "{table} must grant nothing to oteryn_game_runtime");
            for statement in [
                format!("SELECT count(*) FROM {table}"),
                format!("DELETE FROM {table}"),
            ] {
                let mut tx = connection.begin().await?;
                tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
                let denied = tx
                    .execute(sqlx::query(sqlx::AssertSqlSafe(statement)))
                    .await;
                assert_eq!(sqlstate(denied)?, "42501");
                tx.rollback().await?;
            }
        }
        let mut tx = connection.begin().await?;
        tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
        let denied = insert_location(&mut tx, &world, &items[0], OTHER_HOUSE, 1).await;
        assert_eq!(sqlstate(denied)?, "42501");
        tx.rollback().await?;

        // Control role reads them; the guard functions are not callable by
        // PUBLIC.
        let control: bool = sqlx::query_scalar(
            "SELECT has_table_privilege('oteryn_game_control', \
                      'game_item_house_interior_locations', 'SELECT') \
                AND NOT has_table_privilege('oteryn_game_control', \
                      'game_item_house_interior_locations', 'INSERT') \
                AND NOT has_function_privilege('oteryn_game_runtime', \
                      'game_item_location_exclusive(uuid)', 'EXECUTE')",
        )
        .fetch_one(&mut *connection)
        .await?;
        assert!(control);
        Ok(())
    })
}

#[test]
fn character_named_by_a_provenance_cannot_be_deleted() -> TestResult {
    run(async |connection, _url| {
        let (world, character, items) = seed(connection, 1).await?;
        place(connection, &world, &items[0], &character, 1).await?;
        let unreferenced = {
            let mut tx = connection.begin().await?;
            let other = seed_character(&mut tx, &world).await?;
            tx.commit().await?;
            other
        };
        // The root's own guard rejects every DELETE and the name reservation
        // references it; clear both inside a rolled-back transaction so only
        // the provenance foreign key decides.
        for (subject, rejected) in [(&unreferenced, false), (&character, true)] {
            let mut tx = connection.begin().await?;
            tx.execute("SET LOCAL session_replication_role = replica")
                .await?;
            sqlx::query(
                "DELETE FROM game_character_name_reservations WHERE character_id = $1::uuid",
            )
            .bind(subject)
            .execute(&mut *tx)
            .await?;
            tx.execute("SET LOCAL session_replication_role = origin")
                .await?;
            tx.execute(
                "ALTER TABLE game_character_roots \
                 DISABLE TRIGGER game_character_root_revision_guard",
            )
            .await?;
            let deleted =
                sqlx::query("DELETE FROM game_character_roots WHERE character_id = $1::uuid")
                    .bind(subject)
                    .execute(&mut *tx)
                    .await;
            if rejected {
                let error = message(deleted)?;
                assert!(
                    error.contains("game_item_house_reclaim_provenance"),
                    "provenance FK must reject the delete: {error}"
                );
            } else {
                assert_eq!(deleted?.rows_affected(), 1);
            }
            tx.rollback().await?;
        }
        let restrict: String = sqlx::query_scalar(
            "SELECT confdeltype::text FROM pg_constraint \
              WHERE conrelid = 'game_item_house_reclaim_provenance'::regclass \
                AND confrelid = 'game_character_roots'::regclass",
        )
        .fetch_one(&mut *connection)
        .await?;
        assert_eq!(restrict, "r", "ON DELETE RESTRICT");
        Ok(())
    })
}

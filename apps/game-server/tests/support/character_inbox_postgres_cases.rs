// INBOX-1a (migration 0076): the CharacterInbox location, its counter, its
// delivery records and the unreserved delivery function (HOUSE-RT-INBOX
// §2.4; MARKET-0 §5, §7, §8; SOCIAL-MAP-PACKETS-1 §1.12). No caller exists
// yet, so a test-only SECURITY DEFINER caller with a test cause kind row,
// both created by test code, stands in for HOUSE-1b. Fixtures are seeded as
// the table owner with triggers off; every case then writes with every
// trigger and constraint on.

use oteryn_game_server::durability::character_inbox::{
    CharacterInboxCounter, CharacterInboxDeliveryError, read_character_inbox,
    read_character_inbox_counter, read_character_inbox_deliveries,
};
use sqlx::{Connection, Executor, PgConnection};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const KIND: &str = "TEST_DELIVERY";

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
        let name = format!("character_inbox_{suffix}");
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
        install_test_caller(&mut connection).await?;
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

/// The stand-in for a caller's own migration: its cause kind row and its
/// SECURITY DEFINER function owned by the migration owner, executable by the
/// runtime role.
async fn install_test_caller(connection: &mut PgConnection) -> TestResult {
    connection
        .execute(
            "INSERT INTO game_character_inbox_cause_kinds(cause_kind) VALUES ('TEST_DELIVERY'); \
             CREATE FUNCTION test_inbox_deliver(p_item UUID, p_character UUID, p_world UUID, \
                 p_cause_kind TEXT, p_cause_ref TEXT) \
               RETURNS TABLE (character_id UUID, ordinal NUMERIC) \
               LANGUAGE sql SECURITY DEFINER SET search_path = public, pg_temp AS \
               $$ SELECT * FROM game_character_inbox_deliver(p_item, p_character, p_world, \
                    p_cause_kind, p_cause_ref) $$; \
             REVOKE ALL ON FUNCTION test_inbox_deliver(UUID, UUID, UUID, TEXT, TEXT) FROM PUBLIC; \
             GRANT EXECUTE ON FUNCTION test_inbox_deliver(UUID, UUID, UUID, TEXT, TEXT) \
               TO oteryn_game_runtime;",
        )
        .await?;
    Ok(())
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

fn refusal<T>(result: Result<T, sqlx::Error>) -> TestResult<CharacterInboxDeliveryError> {
    match result {
        Ok(_) => Err("expected the delivery to be refused".into()),
        Err(error) => Ok(error.into()),
    }
}

fn bytes(uuid: &str) -> TestResult<[u8; 16]> {
    let hex: String = uuid.chars().filter(|character| *character != '-').collect();
    let mut out = [0_u8; 16];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(hex.get(index * 2..index * 2 + 2).ok_or("short uuid")?, 16)?;
    }
    Ok(out)
}

async fn new_world(connection: &mut PgConnection) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(connection)
        .await?)
}

async fn seed_character(connection: &mut PgConnection, world: &str) -> TestResult<String> {
    let mut tx = connection.begin().await?;
    tx.execute("SET LOCAL session_replication_role = replica")
        .await?;
    let account: String = sqlx::query_scalar(
        "INSERT INTO game_character_account_guards(account_id) \
         VALUES (game_character_uuid_v7()) RETURNING account_id::text",
    )
    .fetch_one(&mut *tx)
    .await?;
    let character = sqlx::query_scalar(
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
    .await?;
    tx.commit().await?;
    Ok(character)
}

/// `count` live ItemInstances of the World with no location.
async fn seed_items(
    connection: &mut PgConnection,
    world: &str,
    count: i64,
) -> TestResult<Vec<String>> {
    let mut tx = connection.begin().await?;
    tx.execute("SET LOCAL session_replication_role = replica")
        .await?;
    let items = sqlx::query_scalar(
        "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
           definition_production_key, definition_revision_ref, quantity, lifecycle, \
           minted_transaction_id) \
         SELECT game_character_uuid_v7(), $1::uuid, 'Item', 'oteryn:content.item.vase', \
           'r1', 1, 1, game_character_uuid_v7() \
           FROM generate_series(1, $2) \
         RETURNING item_instance_id::text",
    )
    .bind(world)
    .bind(count)
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(items)
}

/// Seeds the item on Ground in a fresh channel, triggers off.
async fn seed_ground(connection: &mut PgConnection, world: &str, item: &str) -> TestResult {
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
    .bind(item)
    .bind(world)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}

/// One delivery through the test caller, without committing.
async fn deliver_in(
    tx: &mut PgConnection,
    item: &str,
    character: &str,
    world: &str,
    cause_kind: &str,
    cause_ref: &str,
) -> Result<(String, String), sqlx::Error> {
    sqlx::query_as(
        "SELECT character_id::text, ordinal::text \
           FROM test_inbox_deliver($1::uuid, $2::uuid, $3::uuid, $4, $5)",
    )
    .bind(item)
    .bind(character)
    .bind(world)
    .bind(cause_kind)
    .bind(cause_ref)
    .fetch_one(&mut *tx)
    .await
}

/// One committed delivery; the returned (character, ordinal).
async fn deliver(
    connection: &mut PgConnection,
    item: &str,
    character: &str,
    world: &str,
    cause_ref: &str,
) -> Result<(String, String), sqlx::Error> {
    let mut tx = connection.begin().await?;
    let placed = deliver_in(&mut tx, item, character, world, KIND, cause_ref).await?;
    tx.commit().await?;
    Ok(placed)
}

async fn counter(
    connection: &mut PgConnection,
    character: &str,
) -> TestResult<Option<CharacterInboxCounter>> {
    Ok(read_character_inbox_counter(connection, &bytes(character)?).await?)
}

/// Waits until the backend `pid` waits on a lock.
async fn wait_for_lock(observer: &mut PgConnection, pid: i32) -> TestResult {
    for _ in 0..1000 {
        let waiting: bool = sqlx::query_scalar(
            "SELECT coalesce(bool_or(wait_event_type = 'Lock'), false) \
               FROM pg_stat_activity WHERE pid = $1",
        )
        .bind(pid)
        .fetch_one(&mut *observer)
        .await?;
        if waiting {
            return Ok(());
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    Err("the backend never waited on a lock".into())
}

#[test]
fn delivery_reaches_offline_and_other_channel_characters_and_counts() -> TestResult {
    run(async |connection, _url| {
        let world = new_world(connection).await?;
        let offline = seed_character(connection, &world).await?;
        let elsewhere = seed_character(connection, &world).await?;
        let items = seed_items(connection, &world, 4).await?;

        // `elsewhere` has a live session on some channel; `offline` has none.
        {
            let mut tx = connection.begin().await?;
            tx.execute("SET LOCAL session_replication_role = replica")
                .await?;
            sqlx::query(
                "INSERT INTO game_durability_reconnect_sessions(game_session_id, account_id, \
                   character_id, world_id, runtime_scope_kind, runtime_scope_world_id, \
                   runtime_scope_channel_id, control_loss_epoch, original_grace_deadline, \
                   predecessor_generation, character_lease_generation, \
                   scope_ownership_generation, current_generation) \
                 VALUES (game_character_uuid_v7(), \
                   (SELECT account_id FROM game_character_roots WHERE character_id = $1::uuid), \
                   $1::uuid, $2::uuid, 1, $2::uuid, game_character_uuid_v7(), 1, 0, 1, 1, 1, 1)",
            )
            .bind(&elsewhere)
            .bind(&world)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
        }
        assert_eq!(counter(connection, &offline).await?, None);

        // No session fence: both deliveries commit, and each returns its location.
        let first = deliver(connection, &items[0], &offline, &world, "r-0").await?;
        assert_eq!(first, (offline.clone(), "1".to_owned()));
        let other = deliver(connection, &items[1], &elsewhere, &world, "r-1").await?;
        assert_eq!(other, (elsewhere.clone(), "1".to_owned()));
        let body: String = sqlx::query_scalar(
            "SELECT prosrc FROM pg_proc WHERE proname = 'game_character_inbox_deliver'",
        )
        .fetch_one(&mut *connection)
        .await?;
        assert!(
            !body.contains("session"),
            "the delivery takes no session fence"
        );

        // The counter rises by one per delivery; ordinals rise and are never reused.
        assert_eq!(
            counter(connection, &offline).await?,
            Some(CharacterInboxCounter {
                committed: 1,
                next_ordinal: 2
            })
        );
        // A refused delivery (rolled back) consumes nothing.
        let refused = deliver(connection, &items[0], &offline, &world, "r-0").await;
        assert!(matches!(
            refusal(refused)?,
            CharacterInboxDeliveryError::AlreadyDelivered
        ));
        assert_eq!(
            deliver(connection, &items[2], &offline, &world, "r-2").await?,
            (offline.clone(), "2".to_owned())
        );
        // Two deliveries in one transaction take consecutive ordinals.
        let mut tx = connection.begin().await?;
        let (_, ordinal) = deliver_in(&mut tx, &items[3], &offline, &world, KIND, "r-3").await?;
        assert_eq!(ordinal, "3");
        tx.commit().await?;
        assert_eq!(
            counter(connection, &offline).await?,
            Some(CharacterInboxCounter {
                committed: 3,
                next_ordinal: 4
            })
        );
        let inbox = read_character_inbox(connection, &bytes(&offline)?).await?;
        let ordinals: Vec<u64> = inbox.iter().map(|entry| entry.ordinal).collect();
        assert_eq!(ordinals, [1, 2, 3]);
        assert_eq!(inbox[1].item_instance_id, bytes(&items[2])?);
        assert!(
            inbox
                .iter()
                .all(|entry| entry.world_id == bytes(&world).unwrap_or_default())
        );
        let records = read_character_inbox_deliveries(connection, &bytes(&items[3])?).await?;
        assert_eq!(records.len(), 1);
        assert_eq!(
            (
                records[0].cause_kind.as_str(),
                records[0].cause_ref.as_str()
            ),
            (KIND, "r-3")
        );
        assert_eq!(
            (records[0].character_id, records[0].ordinal),
            (bytes(&offline)?, 3)
        );

        // The next ordinal never falls, even for the table owner.
        let fall = sqlx::query(
            "UPDATE game_character_inbox_counters SET next_ordinal = 2 \
              WHERE character_id = $1::uuid",
        )
        .bind(&offline)
        .execute(&mut *connection)
        .await;
        assert!(message(fall)?.contains("never reuses an ordinal"));
        let delete =
            sqlx::query("DELETE FROM game_character_inbox_counters WHERE character_id = $1::uuid")
                .bind(&offline)
                .execute(&mut *connection)
                .await;
        assert_eq!(sqlstate(delete)?, "23514");
        Ok(())
    })
}

#[test]
fn one_hundred_thousand_and_one_deliveries_are_all_accepted() -> TestResult {
    run(async |connection, _url| {
        let world = new_world(connection).await?;
        let character = seed_character(connection, &world).await?;
        let items = seed_items(connection, &world, 100_001).await?;

        // MARKET0-RL-06 is MARKET-1's refusal: INBOX-1a never refuses for
        // capacity. Deliveries commit in batches, as callers' transactions do.
        let mut delivered = 0_i64;
        for batch in items.chunks(250) {
            let mut tx = connection.begin().await?;
            delivered += sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM unnest($3::text[]) i, \
                   LATERAL test_inbox_deliver(i::uuid, $1::uuid, $2::uuid, \
                     'TEST_DELIVERY', 'bulk') d",
            )
            .bind(&character)
            .bind(&world)
            .bind(batch)
            .fetch_one(&mut *tx)
            .await?;
            tx.commit().await?;
        }
        assert_eq!(delivered, 100_001);
        assert_eq!(
            counter(connection, &character).await?,
            Some(CharacterInboxCounter {
                committed: 100_001,
                next_ordinal: 100_002
            })
        );
        let (rows, distinct, low, high): (i64, i64, String, String) = sqlx::query_as(
            "SELECT count(*), count(DISTINCT ordinal), min(ordinal)::text, max(ordinal)::text \
               FROM game_item_character_inbox_locations WHERE character_id = $1::uuid",
        )
        .bind(&character)
        .fetch_one(&mut *connection)
        .await?;
        assert_eq!(
            (rows, distinct, low.as_str(), high.as_str()),
            (100_001, 100_001, "1", "100001")
        );
        Ok(())
    })
}

#[test]
fn delivery_refusals_are_typed() -> TestResult {
    run(async |connection, _url| {
        let world = new_world(connection).await?;
        let other_world = new_world(connection).await?;
        let character = seed_character(connection, &world).await?;
        let stranger = seed_character(connection, &other_world).await?;
        let items = seed_items(connection, &world, 6).await?;
        let foreign = seed_items(connection, &other_world, 1).await?;

        // An item with contents (OTI01).
        {
            let mut tx = connection.begin().await?;
            tx.execute("SET LOCAL session_replication_role = replica")
                .await?;
            sqlx::query(
                "INSERT INTO game_item_corpse_container_entries(item_instance_id, world_id, \
                   parent_item_instance_id, placement_ordinal, placed_transaction_id) \
                 VALUES ($1::uuid, $2::uuid, $3::uuid, 1, game_character_uuid_v7())",
            )
            .bind(&items[1])
            .bind(&world)
            .bind(&items[0])
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
        }
        let contents = deliver(connection, &items[0], &character, &world, "c").await;
        assert!(matches!(
            refusal(contents)?,
            CharacterInboxDeliveryError::ItemHasContents
        ));

        // A Character of another World, an item of another World, a wrong or
        // missing World and a Character without a root (OTI02).
        for (item, recipient, given) in [
            (&items[2], &stranger, &world),
            (&items[2], &stranger, &other_world),
            (&foreign[0], &character, &world),
            (&items[2], &character, &other_world),
        ] {
            let refused = deliver(connection, item, recipient, given, "w").await;
            assert!(matches!(
                refusal(refused)?,
                CharacterInboxDeliveryError::WorldMismatch
            ));
        }
        let mut tx = connection.begin().await?;
        let no_world =
            sqlx::query("SELECT * FROM test_inbox_deliver($1::uuid, $2::uuid, NULL, $3, 'w')")
                .bind(&items[2])
                .bind(&character)
                .bind(KIND)
                .execute(&mut *tx)
                .await;
        assert_eq!(sqlstate(no_world)?, "OTI02");
        tx.rollback().await?;
        let rootless = deliver(connection, &items[2], &world, &world, "w").await;
        assert!(matches!(
            refusal(rootless)?,
            CharacterInboxDeliveryError::WorldMismatch
        ));

        // An item still in another location at commit (OTI03): the refusal
        // comes from the deferred exclusivity check, at commit.
        seed_ground(connection, &world, &items[3]).await?;
        let mut tx = connection.begin().await?;
        deliver_in(&mut tx, &items[3], &character, &world, KIND, "g").await?;
        assert!(matches!(
            refusal(tx.commit().await)?,
            CharacterInboxDeliveryError::ItemInAnotherLocation
        ));
        assert!(
            read_character_inbox(connection, &bytes(&character)?)
                .await?
                .is_empty()
        );

        // A cause kind with no registry row (OTI04), not a raw FK violation.
        for kind in ["UNKNOWN_KIND", ""] {
            let mut tx = connection.begin().await?;
            let unknown = deliver_in(&mut tx, &items[4], &character, &world, kind, "u").await;
            assert!(matches!(
                refusal(unknown)?,
                CharacterInboxDeliveryError::UnknownCauseKind
            ));
            tx.rollback().await?;
        }

        // The same (item, cause) twice (OTI05).
        deliver(connection, &items[4], &character, &world, "once").await?;
        let again = deliver(connection, &items[4], &character, &world, "once").await;
        assert!(matches!(
            refusal(again)?,
            CharacterInboxDeliveryError::AlreadyDelivered
        ));
        // An item already in the Inbox under another cause is in another location.
        let twice = deliver(connection, &items[4], &character, &world, "twice").await;
        assert!(matches!(
            refusal(twice)?,
            CharacterInboxDeliveryError::ItemInAnotherLocation
        ));

        // A kind row a delivery record references cannot be deleted or renamed.
        let delete =
            sqlx::query("DELETE FROM game_character_inbox_cause_kinds WHERE cause_kind = $1")
                .bind(KIND)
                .execute(&mut *connection)
                .await;
        assert_eq!(sqlstate(delete)?, "23503");
        let rename = sqlx::query(
            "UPDATE game_character_inbox_cause_kinds SET cause_kind = 'RENAMED' WHERE cause_kind = $1",
        )
        .bind(KIND)
        .execute(&mut *connection)
        .await;
        assert_eq!(sqlstate(rename)?, "23503");

        // Only the delivery writes the Inbox: a bare row, even by the table
        // owner, fails its placement proof at commit.
        let mut tx = connection.begin().await?;
        sqlx::query(
            "INSERT INTO game_character_inbox_counters(character_id) VALUES ($1::uuid) \
               ON CONFLICT DO NOTHING",
        )
        .bind(&character)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            "INSERT INTO game_item_character_inbox_locations(item_instance_id, world_id, \
               character_id, ordinal) VALUES ($1::uuid, $2::uuid, $3::uuid, 99)",
        )
        .bind(&items[5])
        .bind(&world)
        .bind(&character)
        .execute(&mut *tx)
        .await?;
        assert!(message(tx.commit().await)?.contains("written together by the delivery"));
        Ok(())
    })
}

#[test]
fn runtime_role_cannot_deliver_or_write_the_inbox() -> TestResult {
    run(async |connection, _url| {
        let world = new_world(connection).await?;
        let character = seed_character(connection, &world).await?;
        let items = seed_items(connection, &world, 2).await?;
        deliver(connection, &items[0], &character, &world, "seed").await?;

        let denied = [
            format!(
                "SELECT * FROM game_character_inbox_deliver('{}'::uuid, '{character}'::uuid, \
                   '{world}'::uuid, 'TEST_DELIVERY', 'r')",
                items[1]
            ),
            format!(
                "INSERT INTO game_item_character_inbox_locations(item_instance_id, world_id, \
                   character_id, ordinal) VALUES ('{}'::uuid, '{world}'::uuid, \
                   '{character}'::uuid, 7)",
                items[1]
            ),
            format!(
                "DELETE FROM game_item_character_inbox_locations WHERE item_instance_id = '{}'::uuid",
                items[0]
            ),
            format!(
                "UPDATE game_character_inbox_counters SET next_ordinal = next_ordinal + 1 \
                  WHERE character_id = '{character}'::uuid"
            ),
            "INSERT INTO game_character_inbox_counters(character_id) \
               VALUES (game_character_uuid_v7())"
                .to_owned(),
            "INSERT INTO game_character_inbox_deliveries(item_instance_id, cause_kind, \
               cause_ref, world_id, character_id, ordinal) \
               VALUES (game_character_uuid_v7(), 'TEST_DELIVERY', 'r', \
                 game_character_uuid_v7(), game_character_uuid_v7(), 1)"
                .to_owned(),
            "SELECT * FROM game_character_inbox_cause_kinds".to_owned(),
            "INSERT INTO game_character_inbox_cause_kinds VALUES ('RUNTIME_KIND')".to_owned(),
        ];
        for statement in denied {
            let mut tx = connection.begin().await?;
            tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
            let result = tx
                .execute(sqlx::query(sqlx::AssertSqlSafe(statement.clone())))
                .await;
            assert_eq!(sqlstate(result)?, "42501", "{statement}");
            tx.rollback().await?;
        }
        for (table, privilege) in [
            ("game_character_inbox_cause_kinds", "SELECT"),
            ("game_character_inbox_counters", "SELECT"),
            ("game_character_inbox_deliveries", "SELECT"),
        ] {
            let granted: bool =
                sqlx::query_scalar("SELECT has_table_privilege('oteryn_game_runtime', $1, $2)")
                    .bind(table)
                    .bind(privilege)
                    .fetch_one(&mut *connection)
                    .await?;
            assert!(!granted, "{table}");
        }

        // Positive control: the runtime role reaches the delivery only through
        // a caller's SECURITY DEFINER function.
        let mut tx = connection.begin().await?;
        tx.execute("SET LOCAL ROLE oteryn_game_runtime").await?;
        let (_, ordinal) = deliver_in(&mut tx, &items[1], &character, &world, KIND, "r").await?;
        tx.commit().await?;
        assert_eq!(ordinal, "2");
        Ok(())
    })
}

#[test]
fn inbox_rows_are_immutable_and_no_other_writer_touches_them() -> TestResult {
    run(async |connection, _url| {
        let world = new_world(connection).await?;
        let character = seed_character(connection, &world).await?;
        let other = seed_character(connection, &world).await?;
        let items = seed_items(connection, &world, 1).await?;
        deliver(connection, &items[0], &character, &world, "r").await?;
        let item = &items[0];

        // Neither the Inbox row nor its delivery record changes or goes away,
        // even for the table owner.
        for statement in [
            format!(
                "UPDATE game_item_character_inbox_locations SET character_id = '{other}'::uuid \
                  WHERE item_instance_id = '{item}'::uuid"
            ),
            format!(
                "UPDATE game_item_character_inbox_locations SET ordinal = 5 \
                  WHERE item_instance_id = '{item}'::uuid"
            ),
            format!(
                "DELETE FROM game_item_character_inbox_locations WHERE item_instance_id = '{item}'::uuid"
            ),
            format!(
                "UPDATE game_character_inbox_deliveries SET cause_ref = 'x' \
                  WHERE item_instance_id = '{item}'::uuid"
            ),
            format!(
                "DELETE FROM game_character_inbox_deliveries WHERE item_instance_id = '{item}'::uuid"
            ),
            "TRUNCATE game_item_character_inbox_locations CASCADE".to_owned(),
            "TRUNCATE game_character_inbox_deliveries".to_owned(),
            "TRUNCATE game_character_inbox_counters CASCADE".to_owned(),
            "TRUNCATE game_character_inbox_cause_kinds CASCADE".to_owned(),
        ] {
            let result = connection
                .execute(sqlx::query(sqlx::AssertSqlSafe(statement.clone())))
                .await;
            assert_eq!(sqlstate(result)?, "23514", "{statement}");
        }

        // The WorldReset retirement of an Inbox item fails the exclusivity
        // guard: a retired item has no location, and the Inbox row stays.
        let mut tx = connection.begin().await?;
        sqlx::query(
            "UPDATE game_item_instances SET lifecycle = 2, quantity = 0, \
               last_transaction_id = game_character_uuid_v7() \
             WHERE item_instance_id = $1::uuid",
        )
        .bind(item)
        .execute(&mut *tx)
        .await?;
        let retired = sqlx::query("SELECT game_item_location_exclusive($1::uuid)")
            .bind(item)
            .execute(&mut *tx)
            .await;
        assert!(message(retired)?.contains("exactly one location"));
        tx.rollback().await?;

        // Death, WorldReset and channel change: no function outside the Inbox's
        // own, the item location guard and the TRANSFER consistency count names
        // the Inbox location table, so no existing writer touches it.
        let readers: Vec<String> = sqlx::query_scalar(
            "SELECT proname::text FROM pg_proc \
              WHERE prosrc LIKE '%game_item_character_inbox_locations%' \
                AND pronamespace = current_schema()::regnamespace \
              ORDER BY 1",
        )
        .fetch_all(&mut *connection)
        .await?;
        assert_eq!(
            readers,
            [
                "game_character_inbox_deliver",
                "game_character_inbox_placement_proven",
                "game_item_location_exclusive",
                "game_item_transfer_consistency_guard",
            ]
        );
        let kept = read_character_inbox(connection, &bytes(&character)?).await?;
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].item_instance_id, bytes(item)?);
        Ok(())
    })
}

#[test]
fn deliveries_serialize_on_the_counter_and_never_deadlock_with_a_root_lock() -> TestResult {
    run(async |connection, url| {
        let world = new_world(connection).await?;
        let character = seed_character(connection, &world).await?;
        let items = seed_items(connection, &world, 5).await?;
        deliver(connection, &items[0], &character, &world, "first").await?;

        // Two concurrent deliveries to one Character: the second waits on the
        // counter row and takes the next ordinal after the first commits.
        let mut first = PgConnection::connect(url).await?;
        let mut tx = first.begin().await?;
        let (_, held) = deliver_in(&mut tx, &items[1], &character, &world, KIND, "a").await?;
        assert_eq!(held, "2");
        let mut second = PgConnection::connect(url).await?;
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut second)
            .await?;
        let (item, recipient, place) = (items[2].clone(), character.clone(), world.clone());
        let waiter = tokio::spawn(async move {
            let placed = deliver(&mut second, &item, &recipient, &place, "b").await;
            placed.map_err(|error| error.to_string())
        });
        wait_for_lock(connection, pid).await?;
        let blocked: bool = sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM pg_locks l \
               JOIN pg_class c ON c.oid = l.relation \
              WHERE l.pid = $1 AND NOT l.granted) \
             OR EXISTS (SELECT 1 FROM pg_locks WHERE pid = $1 AND NOT granted \
                         AND locktype = 'transactionid')",
        )
        .bind(pid)
        .fetch_one(&mut *connection)
        .await?;
        assert!(blocked);
        tx.commit().await?;
        let (_, after) = waiter.await??;
        assert_eq!(after, "3");

        // A transaction holding the Character root FOR UPDATE and then taking
        // the counter, against a delivery: the delivery waits at the root,
        // before the counter, so both finish (MARKET-0 §7 order).
        let mut tx = first.begin().await?;
        tx.execute("SET LOCAL lock_timeout = '10s'").await?;
        sqlx::query("SELECT 1 FROM game_character_roots WHERE character_id = $1::uuid FOR UPDATE")
            .bind(&character)
            .execute(&mut *tx)
            .await?;
        let mut third = PgConnection::connect(url).await?;
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut third)
            .await?;
        let (item, recipient, place) = (items[3].clone(), character.clone(), world.clone());
        let waiter = tokio::spawn(async move {
            let placed = deliver(&mut third, &item, &recipient, &place, "c").await;
            placed.map_err(|error| error.to_string())
        });
        wait_for_lock(connection, pid).await?;
        sqlx::query(
            "SELECT 1 FROM game_character_inbox_counters WHERE character_id = $1::uuid FOR UPDATE",
        )
        .bind(&character)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        let (_, after) = waiter.await??;
        assert_eq!(after, "4");

        // The other way round: a delivery holding the counter, and a root
        // FOR UPDATE that waits for it; the delivery needs nothing more.
        let mut tx = first.begin().await?;
        deliver_in(&mut tx, &items[4], &character, &world, KIND, "d").await?;
        let mut fourth = PgConnection::connect(url).await?;
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut fourth)
            .await?;
        let recipient = character.clone();
        let locker = tokio::spawn(async move {
            let mut tx = fourth.begin().await.map_err(|error| error.to_string())?;
            sqlx::query(
                "SELECT 1 FROM game_character_roots WHERE character_id = $1::uuid FOR UPDATE",
            )
            .bind(&recipient)
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
            sqlx::query(
                "SELECT 1 FROM game_character_inbox_counters WHERE character_id = $1::uuid \
                   FOR UPDATE",
            )
            .bind(&recipient)
            .execute(&mut *tx)
            .await
            .map_err(|error| error.to_string())?;
            tx.commit().await.map_err(|error| error.to_string())
        });
        wait_for_lock(connection, pid).await?;
        tx.commit().await?;
        locker.await??;
        assert_eq!(
            counter(connection, &character).await?,
            Some(CharacterInboxCounter {
                committed: 5,
                next_ordinal: 6
            })
        );
        first.close().await?;
        Ok(())
    })
}

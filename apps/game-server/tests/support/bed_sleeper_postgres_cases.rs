//! BED-1b storage-only qualification on the registered PostgreSQL target.
//! Admin-seeded sleepers are immutable test DATA, not a qualified logout,
//! reservation, ACL, activation or session-fence path. No guards are disabled.

use crate::bestiary_postgres_harness::{CHARACTER, Harness, TestResult, WORLD, id, runtime};
use sqlx::{Connection, PgConnection, PgPool};

const HOUSE: &str = "oteryn:content.house.bed_storage";
const OTHER: u8 = 61;
const FOREIGN: u8 = 62;

async fn fixture(tag: &str) -> TestResult<Harness> {
    // Required, including non-Unicode failure: never report a skipped PG case as green.
    let admin = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
    let harness = Harness::create(admin, tag, true).await?;
    let installed: bool =
        sqlx::query_scalar("SELECT success FROM _sqlx_migrations WHERE version=139")
            .fetch_one(&harness.pool)
            .await?;
    assert!(installed, "BED schema must come from the embedded migrator");
    for (character, world, name) in [(OTHER, WORLD, "Bed Other"), (FOREIGN, 63, "Bed Foreign")] {
        sqlx::query(
            "INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,\
             character_revision,profile_revision,ruleset_revision,content_revision,\
             starter_template_revision,name) \
             SELECT encode($1,'hex')::uuid,account_id,encode($2,'hex')::uuid,1,1,\
             profile_revision,ruleset_revision,content_revision,starter_template_revision,$3 \
             FROM game_character_roots WHERE character_id=encode($4,'hex')::uuid",
        )
        .bind(id(character).as_slice())
        .bind(id(world).as_slice())
        .bind(name)
        .bind(id(CHARACTER).as_slice())
        .execute(&harness.pool)
        .await?;
    }
    sqlx::query(
        "INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,\
         ownership_revision,acquisition_receipt_ref) \
         VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,1,game_character_uuid_v7())",
    )
    .bind(id(WORLD).as_slice())
    .bind(HOUSE)
    .bind(id(CHARACTER).as_slice())
    .execute(&harness.pool)
    .await?;
    Ok(harness)
}

async fn insert(connection: &mut PgConnection, character: u8, x: i32) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO game_house_bed_sleepers(character_id,world_id,house_key,bed_x,bed_y,bed_z,\
         lease_generation,occupied_variant,slept_at) \
         VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,-2,3,\
         18446744073709551615,'female',1000)",
    )
    .bind(id(character).as_slice())
    .bind(id(WORLD).as_slice())
    .bind(HOUSE)
    .bind(x)
    .execute(connection)
    .await?;
    Ok(())
}

async fn revision(pool: &PgPool) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT revision::text FROM game_house_bed_revisions \
         WHERE world_id=encode($1,'hex')::uuid AND house_key=$2",
    )
    .bind(id(WORLD).as_slice())
    .bind(HOUSE)
    .fetch_one(pool)
    .await?)
}

fn sqlstate<T>(result: Result<T, sqlx::Error>, expected: &str) {
    assert!(
        result.is_err(),
        "statement unexpectedly succeeded; expected SQLSTATE {expected}"
    );
    if let Err(error) = result {
        assert_eq!(
            error
                .as_database_error()
                .and_then(|error| error.code())
                .as_deref(),
            Some(expected),
            "unexpected database error: {error}"
        );
    }
}

async fn transfer(
    connection: &mut PgConnection,
    owner: u8,
    receipt: u8,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE game_house_ownership SET owner_character_id=encode($1,'hex')::uuid,\
         ownership_revision=ownership_revision+1,acquisition_receipt_ref=encode($4,'hex')::uuid \
         WHERE world_id=encode($2,'hex')::uuid AND house_key=$3",
    )
    .bind(id(owner).as_slice())
    .bind(id(WORLD).as_slice())
    .bind(HOUSE)
    .bind(id(receipt).as_slice())
    .execute(connection)
    .await?;
    Ok(())
}

#[test]
fn bed_sleeper_storage_identity_occupancy_and_rollback() -> TestResult {
    runtime()?.block_on(async {
        let h = fixture("bed_storage_identity").await?;
        let mut connection = PgConnection::connect(&h.database.url).await?;
        assert_eq!(revision(&h.pool).await?, "1");
        insert(&mut connection, CHARACTER, i32::MIN).await?;
        assert_eq!(revision(&h.pool).await?, "2");
        sqlstate(insert(&mut connection, OTHER, i32::MIN).await, "23505");
        sqlstate(insert(&mut connection, CHARACTER, 1).await, "23505");
        sqlstate(insert(&mut connection, FOREIGN, 1).await, "23514");
        assert_eq!(revision(&h.pool).await?, "2");

        let mut tx = connection.begin().await?;
        insert(&mut tx, OTHER, i32::MAX).await?;
        tx.rollback().await?;
        assert_eq!(revision(&h.pool).await?, "2");
        assert_eq!(h.count("game_house_bed_sleepers").await?, 1);
        let (lease, variant, x): (String, String, i32) = sqlx::query_as(
            "SELECT lease_generation::text,occupied_variant,bed_x \
             FROM game_house_bed_sleepers WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(CHARACTER).as_slice())
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(
            (lease.as_str(), variant.as_str(), x),
            ("18446744073709551615", "female", i32::MIN)
        );
        connection.close().await?;
        h.cleanup().await
    })
}

#[test]
fn bed_sleeper_storage_rejects_invalid_insert_fields() -> TestResult {
    runtime()?.block_on(async {
        let h = fixture("bed_storage_insert_fields").await?;
        let mut connection = PgConnection::connect(&h.database.url).await?;
        for (lease, variant, slept_at, freed_at, cause, code) in [
            ("0", "female", "1000", "NULL", "NULL", "23514"),
            ("18446744073709551616", "female", "1000", "NULL", "NULL", "23514"),
            ("1", "unknown", "1000", "NULL", "NULL", "23514"),
            ("1", "male", "-1", "NULL", "NULL", "23514"),
            ("1", "male", "1000", "2000", "'owner_wake'", "23514"),
            ("1", "male", "1000", "NULL", "'owner_wake'", "23514"),
            ("1", "male", "1000", "2000", "NULL", "23514"),
        ] {
            // Every interpolation is a closed test literal, never external input.
            let statement = format!(
                "INSERT INTO game_house_bed_sleepers(character_id,world_id,house_key,\
                 bed_x,bed_y,bed_z,lease_generation,occupied_variant,slept_at,freed_at,freed_cause) \
                 VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,1,2,3,\
                 {lease},'{variant}',{slept_at},{freed_at},{cause})"
            );
            sqlstate(sqlx::query(sqlx::AssertSqlSafe(statement))
                .bind(id(CHARACTER).as_slice()).bind(id(WORLD).as_slice()).bind(HOUSE)
                .execute(&mut connection).await, code);
        }
        assert_eq!(revision(&h.pool).await?, "1");
        assert_eq!(h.count("game_house_bed_sleepers").await?, 0);
        connection.close().await?;
        h.cleanup().await
    })
}

#[test]
fn bed_sleeper_storage_closed_fields_and_one_way_free() -> TestResult {
    runtime()?.block_on(async {
        let h = fixture("bed_storage_fields").await?;
        let mut connection = PgConnection::connect(&h.database.url).await?;
        insert(&mut connection, CHARACTER, 1).await?;
        for assignment in [
            "character_id=game_character_uuid_v7()",
            "world_id=game_character_uuid_v7()",
            "house_key='oteryn:content.house.other'",
            "bed_x=2",
            "bed_y=2",
            "bed_z=2",
            "lease_generation=1",
            "occupied_variant='male'",
            "slept_at=0",
            "freed_at=2000",
            "freed_cause='owner_wake'",
            "freed_at=999,freed_cause='owner_wake'",
            "freed_at=2000,freed_cause='unknown'",
        ] {
            sqlstate(
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "UPDATE game_house_bed_sleepers SET {assignment}"
                )))
                .execute(&mut connection)
                .await,
                "23514",
            );
        }
        assert_eq!(revision(&h.pool).await?, "2");
        sqlx::query("UPDATE game_house_bed_sleepers SET freed_at=2000,freed_cause='owner_wake'")
            .execute(&mut connection)
            .await?;
        assert_eq!(revision(&h.pool).await?, "3");
        // A freed sleeper still occupies its Character key, but no longer its bed.
        sqlstate(insert(&mut connection, CHARACTER, 2).await, "23505");
        insert(&mut connection, OTHER, 1).await?;
        for assignment in [
            "freed_at=NULL,freed_cause=NULL",
            "freed_at=2001,freed_cause='owner_wake'",
            "freed_at=2000,freed_cause='disposition'",
            "freed_at=2000,freed_cause='owner_wake'",
        ] {
            sqlstate(
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "UPDATE game_house_bed_sleepers SET {assignment} WHERE freed_at IS NOT NULL"
                )))
                .execute(&mut connection)
                .await,
                "23514",
            );
        }
        assert_eq!(revision(&h.pool).await?, "4");
        sqlx::query("DELETE FROM game_house_bed_sleepers WHERE freed_at IS NOT NULL")
            .execute(&mut connection)
            .await?;
        assert_eq!(revision(&h.pool).await?, "5");
        connection.close().await?;
        h.cleanup().await
    })
}

#[test]
fn bed_sleeper_storage_native_owner_change_preserves_first_cause() -> TestResult {
    runtime()?.block_on(async {
        let h = fixture("bed_storage_owner").await?;
        let mut connection = PgConnection::connect(&h.database.url).await?;
        insert(&mut connection, CHARACTER, 1).await?;
        insert(&mut connection, OTHER, 2).await?;
        let mut tx = connection.begin().await?;
        transfer(&mut tx, OTHER, 70).await?;
        tx.rollback().await?;
        let open: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_house_bed_sleepers WHERE freed_at IS NULL",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(open, 2);
        assert_eq!(revision(&h.pool).await?, "3");
        // Same-owner revision changes are not wake causes.
        transfer(&mut connection, CHARACTER, 71).await?;
        assert_eq!(revision(&h.pool).await?, "3");
        sqlx::query(
            "UPDATE game_house_bed_sleepers SET freed_at=2000,freed_cause='disposition' \
                     WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(CHARACTER).as_slice())
        .execute(&mut connection)
        .await?;
        let mut tx = connection.begin().await?;
        // Exercise the actual control role's native ownership UPDATE. It has no
        // sleeper privileges; only the definer trigger may perform the fallback.
        sqlx::query("SET LOCAL ROLE oteryn_game_control")
            .execute(&mut *tx)
            .await?;
        transfer(&mut tx, OTHER, 72).await?;
        tx.commit().await?;
        assert_eq!(revision(&h.pool).await?, "5");
        let rows: Vec<(i64, String)> = sqlx::query_as(
            "SELECT freed_at,freed_cause FROM game_house_bed_sleepers ORDER BY bed_x LIMIT 3",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], (2000, "disposition".into()));
        assert_eq!(rows[1].1, "owner_change");
        assert!(
            rows[1].0 > 1_000_000_000_000,
            "Unix milliseconds, not seconds"
        );
        transfer(&mut connection, CHARACTER, 73).await?;
        let after: Vec<(i64, String)> = sqlx::query_as(
            "SELECT freed_at,freed_cause FROM game_house_bed_sleepers ORDER BY bed_x LIMIT 3",
        )
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(after, rows);
        assert_eq!(revision(&h.pool).await?, "5");
        connection.close().await?;
        h.cleanup().await
    })
}

#[test]
fn bed_sleeper_storage_roles_have_no_direct_access() -> TestResult {
    runtime()?.block_on(async {
        let h = fixture("bed_storage_roles").await?;
        for role in ["oteryn_game_runtime", "oteryn_game_control"] {
            for table in ["game_house_bed_sleepers", "game_house_bed_revisions"] {
                let granted: bool = sqlx::query_scalar(
                    "SELECT has_table_privilege($1,$2,\
                    'SELECT,INSERT,UPDATE,DELETE,TRUNCATE,REFERENCES,TRIGGER')",
                )
                .bind(role)
                .bind(table)
                .fetch_one(&h.pool)
                .await?;
                assert!(!granted, "unexpected direct {role} privilege on {table}");
            }
            for function in [
                "game_house_bed_initialize()",
                "game_house_bed_sleeper_guard()",
                "game_house_bed_advance_revision()",
                "game_house_bed_owner_changed()",
            ] {
                let granted: bool =
                    sqlx::query_scalar("SELECT has_function_privilege($1,$2,'EXECUTE')")
                        .bind(role)
                        .bind(function)
                        .fetch_one(&h.pool)
                        .await?;
                assert!(!granted, "unexpected direct {role} execute on {function}");
            }
        }
        let functions: Vec<(bool, Option<Vec<String>>)> = sqlx::query_as(
            "SELECT p.prosecdef,p.proconfig FROM pg_proc p JOIN pg_namespace n ON n.oid=p.pronamespace \
             WHERE n.nspname=current_schema() AND p.proname IN ('game_house_bed_initialize',\
             'game_house_bed_sleeper_guard','game_house_bed_advance_revision',\
             'game_house_bed_owner_changed') ORDER BY p.proname LIMIT 5",
        ).fetch_all(&h.pool).await?;
        assert_eq!(functions.len(), 4);
        for (definer, config) in functions {
            assert!(definer);
            assert_eq!(config, Some(vec!["search_path=pg_catalog, public, pg_temp".into()]));
        }
        h.cleanup().await
    })
}

#[test]
fn bed_sleeper_storage_overflow_rolls_back_every_transition() -> TestResult {
    runtime()?.block_on(async {
        let h = fixture("bed_storage_overflow").await?;
        let mut connection = PgConnection::connect(&h.database.url).await?;
        insert(&mut connection, CHARACTER, 1).await?;
        sqlx::query("UPDATE game_house_bed_revisions SET revision=18446744073709551615")
            .execute(&mut connection)
            .await?;
        sqlstate(insert(&mut connection, OTHER, 2).await, "23514");
        for statement in [
            "UPDATE game_house_bed_sleepers SET freed_at=2000,freed_cause='bed_removed'",
            "DELETE FROM game_house_bed_sleepers",
        ] {
            sqlstate(
                sqlx::query(statement).execute(&mut connection).await,
                "23514",
            );
        }
        sqlstate(transfer(&mut connection, OTHER, 74).await, "23514");
        assert_eq!(revision(&h.pool).await?, "18446744073709551615");
        let (count, open): (i64, bool) = sqlx::query_as(
            "SELECT count(*),bool_and(freed_at IS NULL) FROM game_house_bed_sleepers",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!((count, open), (1, true));
        let owner: Vec<u8> = sqlx::query_scalar(
            "SELECT uuid_send(owner_character_id) FROM \
            game_house_ownership WHERE world_id=encode($1,'hex')::uuid AND house_key=$2",
        )
        .bind(id(WORLD).as_slice())
        .bind(HOUSE)
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(owner, id(CHARACTER));
        connection.close().await?;
        h.cleanup().await
    })
}

#[test]
fn bed_sleeper_storage_revision_lock_and_competing_insert() -> TestResult {
    runtime()?.block_on(async {
        let h = fixture("bed_storage_contention").await?;
        let mut first = PgConnection::connect(&h.database.url).await?;
        let mut second = PgConnection::connect(&h.database.url).await?;
        let mut tx = first.begin().await?;
        sqlx::query("SELECT revision FROM game_house_bed_revisions FOR UPDATE")
            .execute(&mut *tx)
            .await?;
        insert(&mut tx, CHARACTER, 1).await?;
        sqlstate(
            sqlx::query("SELECT revision FROM game_house_bed_revisions FOR UPDATE NOWAIT")
                .execute(&mut second)
                .await,
            "55P03",
        );
        // The uncommitted unique key also blocks an improperly ordered contender.
        sqlx::query("SET lock_timeout='100ms'")
            .execute(&mut second)
            .await?;
        sqlstate(insert(&mut second, OTHER, 1).await, "55P03");
        tx.commit().await?;
        sqlstate(insert(&mut second, OTHER, 1).await, "23505");
        assert_eq!(revision(&h.pool).await?, "2");
        assert_eq!(h.count("game_house_bed_sleepers").await?, 1);
        first.close().await?;
        second.close().await?;
        h.cleanup().await
    })
}

//! Real private-schema tests. No marker, access grant or gameplay sleep is simulated.
use super::*;
use crate::durability::bed_sleeper::{
    PRIVATE_SLEEPER_SCHEMA_SQL, SleeperReadError, read_sleeper, read_sleeper_for_update,
};
use sqlx::{Connection, PgConnection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn id(seed: u8) -> [u8; 16] {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

fn character(seed: u8) -> Result<CharacterId, Box<dyn std::error::Error>> {
    CharacterId::from_bytes(id(seed)).map_err(|error| format!("{error:?}").into())
}

fn debug(error: impl std::fmt::Debug) -> Box<dyn std::error::Error> {
    format!("{error:?}").into()
}

fn run_case(tag: &str, body: impl AsyncFnOnce(&mut PgConnection) -> TestResult) -> TestResult {
    let Ok(url) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("PRE-ROUTING/NONCANONICAL: no OTERYN_TEST_POSTGRES_ADMIN_URL");
        return Ok(());
    };
    if !url.starts_with("postgresql://oteryn_test_admin:")
        || !url.ends_with("@127.0.0.1:5432/postgres")
        || !tag.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
    {
        return Err("unsafe private PostgreSQL route/tag".into());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut admin = PgConnection::connect(&url).await?;
            let version: String = sqlx::query_scalar("SHOW server_version_num")
                .fetch_one(&mut admin)
                .await?;
            if version != "170006" {
                return Err("canonical target is PostgreSQL 17.6".into());
            }
            eprintln!("CANONICAL PostgreSQL 17.6 private sleeper case: {tag}");
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos();
            let name = format!("bed_{tag}_{}_{nonce}", std::process::id());
            sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
                .execute(&mut admin)
                .await?;
            let result = async {
                let prefix = url.strip_suffix("/postgres").ok_or("invalid route")?;
                let mut connection = PgConnection::connect(&format!("{prefix}/{name}")).await?;
                sqlx::migrate!("./migrations").run(&mut connection).await?;
                sqlx::raw_sql(PRIVATE_SLEEPER_SCHEMA_SQL)
                    .execute(&mut connection)
                    .await?;
                // Native rows supplied only as migration-owner fixtures. This
                // bypass does not qualify a creation/acquisition/session grant.
                let mut tx = connection.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *tx)
                    .await?;
                sqlx::query("INSERT INTO game_character_account_guards VALUES(encode($1,'hex')::uuid)")
                    .bind(id(50).as_slice()).execute(&mut *tx).await?;
                for seed in 1..=4 {
                    sqlx::query("INSERT INTO game_character_roots(character_id,account_id,world_id,lifecycle,character_revision,profile_revision,ruleset_revision,content_revision,starter_template_revision,name) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,1,'r1','r1','r1','r1',$4)")
                        .bind(id(seed).as_slice()).bind(id(50).as_slice()).bind(id(if seed==4 {9}else{8}).as_slice()).bind(["Sleeper One","Sleeper Two","Sleeper Three","Sleeper Four"][usize::from(seed-1)]).execute(&mut *tx).await?;
                }
                for (key,world,owner) in [("oteryn:content.house.a",8,1),("oteryn:content.house.b",8,2),("oteryn:content.house.c",9,4)] {
                    sqlx::query("INSERT INTO game_house_ownership VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
                        .bind(id(world).as_slice()).bind(key).bind(id(owner).as_slice()).bind(id(owner+20).as_slice()).execute(&mut *tx).await?;
                }
                tx.commit().await?;
                let result = body(&mut connection).await;
                connection.close().await?;
                result
            }.await;
            sqlx::query(sqlx::AssertSqlSafe(format!("DROP DATABASE {name} WITH(FORCE)")))
                .execute(&mut admin).await?;
            result
        })
}

async fn insert(
    connection: &mut PgConnection,
    who: u8,
    house: &str,
    world: u8,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO game_house_bed_sleepers(world_id,house_key,bed_x,bed_y,bed_z,character_id,lease_generation,occupied_variant,slept_at) VALUES(encode($1,'hex')::uuid,$2,10,20,7,encode($3,'hex')::uuid,18446744073709551615,'female',100)")
        .bind(id(world).as_slice()).bind(house).bind(id(who).as_slice()).execute(connection).await?;
    Ok(())
}

async fn revision(connection: &mut PgConnection, house: &str) -> Result<String, sqlx::Error> {
    sqlx::query_scalar("SELECT revision::text FROM game_house_bed_revisions WHERE house_key=$1")
        .bind(house)
        .fetch_one(connection)
        .await
}

fn code<T: std::fmt::Debug>(result: Result<T, sqlx::Error>, expected: &str) -> TestResult {
    match result {
        Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some(expected) => Ok(()),
        other => Err(format!("expected SQLSTATE {expected}, got {other:?}").into()),
    }
}

#[test]
fn freed_causes_are_closed_and_access_revocation_is_not_wake() {
    for (key, cause) in [
        ("owner_wake", FreedCause::OwnerWake),
        ("owner_change", FreedCause::OwnerChange),
        ("disposition", FreedCause::Disposition),
        ("bed_removed", FreedCause::BedRemoved),
    ] {
        assert_eq!(FreedCause::from_stored(key), Some(cause));
    }
    for key in ["", "access_revoked", "logout", "OWNER_WAKE"] {
        assert_eq!(FreedCause::from_stored(key), None);
    }
}

#[test]
fn row_lifecycle_revisions_reuse_and_rollback_are_atomic() -> TestResult {
    run_case("lifecycle", async |c| {
        assert_eq!(read_sleeper(c, character(1)?).await.map_err(debug)?, None);
        insert(c, 1, "oteryn:content.house.a", 8).await?;
        let row = read_sleeper(c, character(1)?)
            .await
            .map_err(debug)?
            .ok_or("missing")?;
        assert_eq!(row.bed.house.house_key(), "oteryn:content.house.a");
        assert_eq!(row.bed.house.world_id().as_bytes(), &id(8));
        assert_eq!(row.bed.head, LogicalCell { x: 10, y: 20, z: 7 });
        assert_eq!(row.lease_generation.get(), u64::MAX);
        assert_eq!(row.variant, OccupiedVariant::Female);
        assert_eq!(row.occupancy_revision.get(), 1);
        assert_eq!(row.slept_at, 100);
        assert_eq!(row.freed, None);
        code(insert(c, 2, "oteryn:content.house.a", 8).await, "23505")?;
        assert_eq!(revision(c, "oteryn:content.house.a").await?, "1");
        let mut tx = c.begin().await?;
        sqlx::query("UPDATE game_house_bed_sleepers SET freed_at=110,freed_cause='owner_wake'")
            .execute(&mut *tx)
            .await?;
        tx.rollback().await?;
        assert_eq!(revision(c, "oteryn:content.house.a").await?, "1");
        assert!(
            read_sleeper(c, character(1)?)
                .await
                .map_err(debug)?
                .ok_or("missing")?
                .freed
                .is_none()
        );
        sqlx::query("UPDATE game_house_bed_sleepers SET freed_at=110,freed_cause='owner_wake' WHERE freed_at IS NULL").execute(&mut *c).await?;
        let freed = read_sleeper(c, character(1)?)
            .await
            .map_err(debug)?
            .ok_or("missing")?;
        assert_eq!(
            freed.freed,
            Some(FreedSleeper {
                at: 110,
                cause: FreedCause::OwnerWake
            })
        );
        assert_eq!(freed.occupancy_revision.get(), 2);
        assert_eq!(sqlx::query("UPDATE game_house_bed_sleepers SET freed_at=120,freed_cause='owner_wake' WHERE freed_at IS NULL").execute(&mut *c).await?.rows_affected(),0);
        insert(c, 2, "oteryn:content.house.a", 8).await?;
        code(insert(c, 1, "oteryn:content.house.b", 8).await, "23505")?;
        assert_eq!(revision(c, "oteryn:content.house.a").await?, "3");
        sqlx::query(
            "DELETE FROM game_house_bed_sleepers WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(1).as_slice())
        .execute(&mut *c)
        .await?;
        assert_eq!(revision(c, "oteryn:content.house.a").await?, "4");
        insert(c, 1, "oteryn:content.house.b", 8).await?;
        assert_eq!(revision(c, "oteryn:content.house.b").await?, "1");
        Ok(())
    })
}

#[test]
fn invalid_transitions_bindings_and_revision_overflow_fail_closed() -> TestResult {
    run_case("guards", async |c| {
        code(insert(c, 4, "oteryn:content.house.a", 8).await, "23514")?;
        code(
            insert(c, 1, "oteryn:content.house.missing", 8).await,
            "23503",
        )?;
        insert(c, 1, "oteryn:content.house.a", 8).await?;
        // Fresh valid Character/house/bed binding: only the lease bound varies.
        for lease in ["0", "18446744073709551616"] {
            code(sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO game_house_bed_sleepers(world_id,house_key,bed_x,bed_y,bed_z,character_id,lease_generation,occupied_variant,slept_at) VALUES(encode($1,'hex')::uuid,'oteryn:content.house.b',10,20,7,encode($2,'hex')::uuid,{lease},'female',100)")))
                .bind(id(8).as_slice()).bind(id(3).as_slice()).execute(&mut *c).await,"23514")?;
        }
        for assignment in [
            "lease_generation=2",
            "house_key='oteryn:content.house.b'",
            "bed_x=11",
            "occupied_variant='male'",
            "slept_at=101",
            "freed_at=99,freed_cause='owner_wake'",
            "freed_at=110",
            "freed_cause='owner_wake'",
            "freed_at=110,freed_cause='access_revoked'",
        ] {
            code(
                sqlx::query(sqlx::AssertSqlSafe(format!(
                    "UPDATE game_house_bed_sleepers SET {assignment}"
                )))
                .execute(&mut *c)
                .await,
                "23514",
            )?;
        }
        for cause in ["owner_change", "disposition", "bed_removed"] {
            sqlx::query("UPDATE game_house_bed_sleepers SET freed_at=110,freed_cause=$1")
                .bind(cause)
                .execute(&mut *c)
                .await?;
            assert_eq!(
                read_sleeper(c, character(1)?)
                    .await
                    .map_err(debug)?
                    .ok_or("missing")?
                    .freed
                    .map(|f| f.cause),
                FreedCause::from_stored(cause)
            );
            code(
                sqlx::query("UPDATE game_house_bed_sleepers SET freed_at=120")
                    .execute(&mut *c)
                    .await,
                "23514",
            )?;
            sqlx::query("DELETE FROM game_house_bed_sleepers")
                .execute(&mut *c)
                .await?;
            insert(c, 1, "oteryn:content.house.a", 8).await?;
        }
        sqlx::query("UPDATE game_house_bed_revisions SET revision=18446744073709551615")
            .execute(&mut *c)
            .await?;
        code(
            sqlx::query("DELETE FROM game_house_bed_sleepers")
                .execute(&mut *c)
                .await,
            "23514",
        )?;
        assert!(
            read_sleeper(c, character(1)?)
                .await
                .map_err(debug)?
                .is_some()
        );
        Ok(())
    })
}

#[test]
fn locked_read_pins_row_until_transaction_ends() -> TestResult {
    run_case("locking", async |c| {
        insert(c, 1, "oteryn:content.house.a", 8).await?;
        let db: String = sqlx::query_scalar("SELECT current_database()")
            .fetch_one(&mut *c)
            .await?;
        let url = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
        let prefix = url.strip_suffix("/postgres").ok_or("route")?;
        let mut other = PgConnection::connect(&format!("{prefix}/{db}")).await?;
        sqlx::query("SET lock_timeout='50ms'")
            .execute(&mut other)
            .await?;
        let mut tx = c.begin().await?;
        assert!(
            read_sleeper_for_update(&mut tx, character(1)?)
                .await
                .map_err(debug)?
                .is_some()
        );
        code(
            sqlx::query("UPDATE game_house_bed_sleepers SET freed_at=110,freed_cause='owner_wake'")
                .execute(&mut other)
                .await,
            "55P03",
        )?;
        tx.rollback().await?;
        sqlx::query("UPDATE game_house_bed_sleepers SET freed_at=110,freed_cause='owner_wake'")
            .execute(&mut other)
            .await?;
        assert_eq!(
            read_sleeper(c, character(1)?)
                .await
                .map_err(debug)?
                .ok_or("missing")?
                .occupancy_revision
                .get(),
            2
        );
        other.close().await?;
        Ok(())
    })
}

#[test]
fn corruption_and_missing_revision_are_not_decoded_as_no_sleeper() -> TestResult {
    run_case("corruption", async |c| {
        insert(c, 1, "oteryn:content.house.a", 8).await?;
        sqlx::query("DELETE FROM game_house_bed_revisions")
            .execute(&mut *c)
            .await?;
        assert!(matches!(
            read_sleeper(c, character(1)?).await,
            Err(SleeperReadError::InvalidStoredState)
        ));
        sqlx::query("INSERT INTO game_house_bed_revisions VALUES(encode($1,'hex')::uuid,'oteryn:content.house.a',1)").bind(id(8).as_slice()).execute(&mut *c).await?;
        // Admin corruption bypasses trigger+CHECK only in this isolated DB.
        let mut tx = c.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "ALTER TABLE game_house_bed_sleepers DROP CONSTRAINT game_house_bed_sleepers_check",
        )
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE game_house_bed_sleepers SET freed_at=110")
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        assert!(matches!(
            read_sleeper(c, character(1)?).await,
            Err(SleeperReadError::InvalidStoredState)
        ));
        Ok(())
    })
}

#[test]
fn native_build_receipt_millisecond_clock_does_not_block_owner_change() -> TestResult {
    run_case("clock_units", async |c| {
        // Actual character_build.rs committed_at representation: Unix ms.
        // BED0 slept_at must carry that exact marker value, not seconds.
        sqlx::query("INSERT INTO game_house_bed_sleepers(world_id,house_key,bed_x,bed_y,bed_z,character_id,lease_generation,occupied_variant,slept_at) VALUES(encode($1,'hex')::uuid,'oteryn:content.house.a',10,20,7,encode($2,'hex')::uuid,1,'female',floor(extract(epoch FROM statement_timestamp())*1000)::bigint)")
            .bind(id(8).as_slice()).bind(id(1).as_slice()).execute(&mut *c).await?;
        change_owner(c, 2, 80).await?;
        let row = read_sleeper(c, character(1)?)
            .await
            .map_err(debug)?
            .ok_or("missing")?;
        assert!(row.freed.ok_or("not freed")?.at >= row.slept_at);
        assert_eq!(row.freed.map(|f| f.cause), Some(FreedCause::OwnerChange));
        assert_eq!(row.occupancy_revision.get(), 2);
        Ok(())
    })
}

// Migration-owner storage transitions only, not a qualified ownership acquisition.
async fn change_owner(c: &mut PgConnection, owner: u8, receipt: u8) -> TestResult {
    sqlx::query("UPDATE game_house_ownership SET owner_character_id=encode($1,'hex')::uuid,ownership_revision=ownership_revision+1,acquisition_receipt_ref=encode($2,'hex')::uuid WHERE world_id=encode($3,'hex')::uuid AND house_key='oteryn:content.house.a'")
        .bind(id(owner).as_slice()).bind(id(receipt).as_slice()).bind(id(8).as_slice()).execute(c).await?;
    Ok(())
}

#[test]
fn native_owner_change_frees_open_rows_atomically_and_preserves_specific_first_causes() -> TestResult
{
    run_case("owner_change", async |c| {
        insert(c, 1, "oteryn:content.house.a", 8).await?;
        sqlx::query("INSERT INTO game_house_bed_sleepers SELECT world_id,house_key,bed_x+1,bed_y,bed_z,encode($1,'hex')::uuid,lease_generation,occupied_variant,slept_at,NULL,NULL FROM game_house_bed_sleepers")
            .bind(id(2).as_slice()).execute(&mut *c).await?;
        sqlx::query("UPDATE game_house_bed_sleepers SET freed_at=110,freed_cause='owner_wake' WHERE character_id=encode($1,'hex')::uuid")
            .bind(id(2).as_slice()).execute(&mut *c).await?;
        assert_eq!(revision(c, "oteryn:content.house.a").await?, "3");

        // Actual native ownership guard remains enabled: a non-successor fails.
        code(sqlx::query("UPDATE game_house_ownership SET owner_character_id=encode($1,'hex')::uuid WHERE house_key='oteryn:content.house.a'")
            .bind(id(3).as_slice()).execute(&mut *c).await,"23514")?;
        assert_eq!(revision(c, "oteryn:content.house.a").await?, "3");
        change_owner(c, 1, 70).await?; // Same owner, valid native revision successor.
        assert!(
            read_sleeper(c, character(1)?)
                .await
                .map_err(debug)?
                .ok_or("missing")?
                .freed
                .is_none()
        );
        assert_eq!(revision(c, "oteryn:content.house.a").await?, "3");

        let mut tx = c.begin().await?;
        change_owner(&mut tx, 3, 71).await?;
        let in_tx = read_sleeper_for_update(&mut tx, character(1)?)
            .await
            .map_err(debug)?
            .ok_or("missing")?;
        assert_eq!(in_tx.freed.map(|f| f.cause), Some(FreedCause::OwnerChange));
        assert_eq!(in_tx.occupancy_revision.get(), 4);
        tx.rollback().await?;
        let owner_rolled_back:bool=sqlx::query_scalar("SELECT owner_character_id=encode($1,'hex')::uuid AND ownership_revision=2 FROM game_house_ownership WHERE house_key='oteryn:content.house.a'")
            .bind(id(1).as_slice()).fetch_one(&mut *c).await?;
        assert!(owner_rolled_back);
        assert!(
            read_sleeper(c, character(1)?)
                .await
                .map_err(debug)?
                .ok_or("missing")?
                .freed
                .is_none()
        );
        assert_eq!(revision(c, "oteryn:content.house.a").await?, "3");

        let before: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&mut *c)
                .await?;
        change_owner(c, 3, 72).await?;
        let after: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&mut *c)
                .await?;
        let first = read_sleeper(c, character(1)?)
            .await
            .map_err(debug)?
            .ok_or("missing")?;
        let free = first.freed.ok_or("not freed")?;
        assert_eq!(free.cause, FreedCause::OwnerChange);
        assert!((before..=after).contains(&free.at));
        assert_eq!(first.occupancy_revision.get(), 4);
        let earlier = read_sleeper(c, character(2)?)
            .await
            .map_err(debug)?
            .ok_or("missing")?;
        assert_eq!(
            earlier.freed,
            Some(FreedSleeper {
                at: 110,
                cause: FreedCause::OwnerWake
            })
        );
        change_owner(c, 2, 73).await?;
        assert_eq!(
            read_sleeper(c, character(1)?).await.map_err(debug)?,
            Some(first)
        );
        assert_eq!(
            read_sleeper(c, character(2)?).await.map_err(debug)?,
            Some(earlier)
        );

        // A later fixture cycle verifies the mandatory disposition-first order.
        // It is not an actual logout/settlement or an entitlement fixture.
        sqlx::query(
            "DELETE FROM game_house_bed_sleepers WHERE character_id=encode($1,'hex')::uuid",
        )
        .bind(id(1).as_slice())
        .execute(&mut *c)
        .await?;
        insert(c, 1, "oteryn:content.house.a", 8).await?;
        let mut disposition = c.begin().await?;
        sqlx::query("UPDATE game_house_bed_sleepers SET freed_at=floor(extract(epoch FROM clock_timestamp())*1000)::bigint,freed_cause='disposition' WHERE freed_at IS NULL")
            .execute(&mut *disposition).await?;
        change_owner(&mut disposition, 3, 74).await?;
        disposition.commit().await?;
        assert_eq!(
            read_sleeper(c, character(1)?)
                .await
                .map_err(debug)?
                .ok_or("missing")?
                .freed
                .map(|f| f.cause),
            Some(FreedCause::Disposition)
        );
        assert_eq!(
            read_sleeper(c, character(2)?)
                .await
                .map_err(debug)?
                .ok_or("missing")?
                .freed,
            Some(FreedSleeper {
                at: 110,
                cause: FreedCause::OwnerWake
            })
        );
        assert_eq!(revision(c, "oteryn:content.house.a").await?, "7");
        Ok(())
    })
}

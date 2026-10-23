//! Private tier schema/read qualification only; no forge operation or authority is simulated.
//! Mounted by character_authority_postgres (PG-COVERAGE-1), on the actual native migration chain.
use crate::domain::forge::ItemTier;
use crate::durability::forge::{PRIVATE_TIER_SCHEMA_SQL, TierReadError, read_item_tier};
use sqlx::{Connection, PgConnection};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn id(seed: u8) -> [u8; 16] {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

fn debug(error: impl std::fmt::Debug) -> Box<dyn std::error::Error> {
    format!("{error:?}").into()
}

fn run_case(tag: &str, body: impl AsyncFnOnce(&mut PgConnection) -> TestResult) -> TestResult {
    let Ok(admin_url) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        eprintln!("PRE-ROUTING/NONCANONICAL: no OTERYN_TEST_POSTGRES_ADMIN_URL");
        return Ok(());
    };
    if !admin_url.starts_with("postgresql://oteryn_test_admin:")
        || !admin_url.ends_with("@127.0.0.1:5432/postgres")
        || !tag
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
    {
        return Err("unsafe private PostgreSQL test route/tag".into());
    }
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut admin = PgConnection::connect(&admin_url).await?;
            let version: String = sqlx::query_scalar("SHOW server_version_num")
                .fetch_one(&mut admin)
                .await?;
            if version != oteryn_gameplay_durability::CANONICAL_POSTGRES_SERVER_VERSION_NUM {
                return Err("canonical target is PostgreSQL 18.6".into());
            }
            eprintln!("CANONICAL PostgreSQL 18.6 private tier case: {tag}");
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos();
            let name = format!("forge_tier_{tag}_{}_{nonce}", std::process::id());
            sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
                .execute(&mut admin)
                .await?;
            let result = async {
                let prefix = admin_url.strip_suffix("/postgres").ok_or("invalid route")?;
                let mut connection = PgConnection::connect(&format!("{prefix}/{name}")).await?;
                // The actual native migration chain supplies game_item_instances;
                // the only additional schema is the exact owned private tier DDL.
                sqlx::migrate!("./migrations").run(&mut connection).await?;
                sqlx::raw_sql(PRIVATE_TIER_SCHEMA_SQL)
                    .execute(&mut connection)
                    .await?;
                // Migration-owner fixture data, not a qualified item MINT or writer.
                let mut tx = connection.begin().await?;
                sqlx::query("SET LOCAL session_replication_role = replica")
                    .execute(&mut *tx)
                    .await?;
                for seed in [1, 2] {
                    sqlx::query(
                        "INSERT INTO game_item_instances \
                         (item_instance_id,world_id,definition_family,definition_production_key,\
                          definition_revision_ref,quantity,lifecycle,minted_transaction_id) \
                         VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,\
                         'Item','oteryn:item.test','definition-1',1,1,encode($3,'hex')::uuid)",
                    )
                    .bind(id(seed).as_slice())
                    .bind(id(3).as_slice())
                    .bind(id(seed + 10).as_slice())
                    .execute(&mut *tx)
                    .await?;
                }
                tx.commit().await?;
                let result = body(&mut connection).await;
                connection.close().await?;
                result
            }
            .await;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP DATABASE {name} WITH (FORCE)"
            )))
            .execute(&mut admin)
            .await?;
            result
        })
}

fn code(result: Result<sqlx::postgres::PgQueryResult, sqlx::Error>, expected: &str) -> TestResult {
    match result {
        Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some(expected) => Ok(()),
        other => Err(format!("expected SQLSTATE {expected}, got {other:?}").into()),
    }
}

#[test]
fn tier_value_bounds() {
    assert_eq!(ItemTier::new(0), Some(ItemTier::ZERO));
    assert_eq!(ItemTier::new(10).map(ItemTier::get), Some(10));
    assert_eq!(ItemTier::new(11), None);
    assert_eq!(ItemTier::new(u8::MAX), None);
}

#[test]
fn tier_reader_distinguishes_missing_row_unknown_item_and_another_instance() -> TestResult {
    run_case("read", async |connection| {
        let empty = read_item_tier(connection, id(1)).await.map_err(debug)?;
        assert_eq!(empty.item_instance_id, id(1));
        assert_eq!(empty.tier, ItemTier::ZERO);
        assert!(matches!(
            read_item_tier(connection, id(9)).await,
            Err(TierReadError::UnknownItem)
        ));
        assert!(matches!(
            read_item_tier(connection, [0; 16]).await,
            Err(TierReadError::InvalidItemIdentity)
        ));
        sqlx::query("INSERT INTO game_item_tiers VALUES (encode($1,'hex')::uuid,10)")
            .bind(id(1).as_slice())
            .execute(&mut *connection)
            .await?;
        let stored = read_item_tier(connection, id(1)).await.map_err(debug)?;
        assert_eq!(stored.item_instance_id, id(1));
        assert_eq!(stored.tier.get(), 10);
        assert_eq!(
            read_item_tier(connection, id(2)).await.map_err(debug)?.tier,
            ItemTier::ZERO
        );
        sqlx::query("DELETE FROM game_item_tiers")
            .execute(&mut *connection)
            .await?;
        assert_eq!(
            read_item_tier(connection, id(1)).await.map_err(debug)?.tier,
            ItemTier::ZERO
        );
        Ok(())
    })
}

#[test]
fn tier_schema_enforces_bounds_foreign_key_uniqueness_and_fixed_identity() -> TestResult {
    run_case("constraints", async |connection| {
        for tier in [i16::MIN, -1, 0, 11, i16::MAX] {
            code(
                sqlx::query("INSERT INTO game_item_tiers VALUES (encode($1,'hex')::uuid,$2)")
                    .bind(id(1).as_slice())
                    .bind(tier)
                    .execute(&mut *connection)
                    .await,
                "23514",
            )?;
        }
        code(
            sqlx::query("INSERT INTO game_item_tiers VALUES (encode($1,'hex')::uuid,NULL)")
                .bind(id(1).as_slice())
                .execute(&mut *connection)
                .await,
            "23502",
        )?;
        code(
            sqlx::query("INSERT INTO game_item_tiers VALUES (encode($1,'hex')::uuid,1)")
                .bind(id(9).as_slice())
                .execute(&mut *connection)
                .await,
            "23503",
        )?;
        sqlx::query("INSERT INTO game_item_tiers VALUES (encode($1,'hex')::uuid,1)")
            .bind(id(1).as_slice())
            .execute(&mut *connection)
            .await?;
        code(
            sqlx::query("INSERT INTO game_item_tiers VALUES (encode($1,'hex')::uuid,2)")
                .bind(id(1).as_slice())
                .execute(&mut *connection)
                .await,
            "23505",
        )?;
        // Item2 exists and has no tier row: only identity reassignment is invalid.
        code(
            sqlx::query("UPDATE game_item_tiers SET item_instance_id=encode($1,'hex')::uuid")
                .bind(id(2).as_slice())
                .execute(&mut *connection)
                .await,
            "23514",
        )?;
        sqlx::query("UPDATE game_item_tiers SET tier=10")
            .execute(&mut *connection)
            .await?;
        assert_eq!(
            read_item_tier(connection, id(1))
                .await
                .map_err(debug)?
                .tier
                .get(),
            10
        );
        assert_eq!(
            read_item_tier(connection, id(2)).await.map_err(debug)?.tier,
            ItemTier::ZERO
        );
        Ok(())
    })
}

#[test]
fn corrupt_present_tier_is_never_silently_defaulted_to_zero() -> TestResult {
    run_case("corrupt", async |connection| {
        // Deliberate migration-owner corruption in the isolated DB only. The
        // reader must still distinguish an invalid present row from no row.
        sqlx::query("ALTER TABLE game_item_tiers DROP CONSTRAINT game_item_tier_range")
            .execute(&mut *connection)
            .await?;
        sqlx::query("INSERT INTO game_item_tiers VALUES (encode($1,'hex')::uuid,0)")
            .bind(id(1).as_slice())
            .execute(&mut *connection)
            .await?;
        for tier in [0_i16, -1, 11] {
            sqlx::query("UPDATE game_item_tiers SET tier=$1")
                .bind(tier)
                .execute(&mut *connection)
                .await?;
            assert!(matches!(
                read_item_tier(connection, id(1)).await,
                Err(TierReadError::InvalidStoredState)
            ));
        }
        sqlx::query("ALTER TABLE game_item_tiers ALTER COLUMN tier DROP NOT NULL")
            .execute(&mut *connection)
            .await?;
        sqlx::query("UPDATE game_item_tiers SET tier=NULL")
            .execute(&mut *connection)
            .await?;
        assert!(matches!(
            read_item_tier(connection, id(1)).await,
            Err(TierReadError::InvalidStoredState)
        ));
        Ok(())
    })
}

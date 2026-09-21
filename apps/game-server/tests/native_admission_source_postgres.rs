use sqlx::{Connection, Executor, Row};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

fn configured() -> Result<bool, Box<dyn std::error::Error>> {
    Ok(env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_some())
}

#[test]
fn migration_declares_nonrollback_registration_floors_and_two_slots() {
    let migration = include_str!("../migrations/0003_native_admission_source.sql");
    assert!(migration.contains("registration_id SMALLINT PRIMARY KEY CHECK (registration_id = 1)"));
    assert!(migration.contains("slot_id SMALLINT NOT NULL CHECK (slot_id IN (1, 2))"));
    assert!(migration.contains("native source registration cannot roll back or be recreated"));
    assert!(migration.contains("native source floor cannot roll back or be rewritten"));
    assert!(migration.contains("native source canonical history is immutable"));
}

#[test]
fn postgres_preserves_bootstrap_floors_replay_and_fixed_slot_custody()
-> Result<(), Box<dyn std::error::Error>> {
    if !configured()? {
        eprintln!("PRE-ROUTING / NONCANONICAL: OTERYN_TEST_POSTGRES_ADMIN_URL is not configured");
        return Ok(());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let admin_url = env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
        if !admin_url.starts_with("postgresql://oteryn_test_admin:")
            || !admin_url.ends_with("@127.0.0.1:5432/postgres")
        { return Err("unsafe PostgreSQL test admin URL".into()); }
        let suffix = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let database_name = format!("native_source_s2_{suffix}");
        let mut admin = sqlx::PgConnection::connect(&admin_url).await?;
        admin.execute(sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {database_name}")))).await?;
        admin.close().await?;
        let prefix = admin_url.strip_suffix("/postgres").ok_or("invalid admin URL")?;
        let url = format!("{prefix}/{database_name}");
        let mut connection = sqlx::PgConnection::connect(&url).await?;
        sqlx::migrate!("./migrations").run(&mut connection).await?;

        // Missing provenance cannot initialize: all fields are mandatory and positive.
        assert!(sqlx::query("INSERT INTO game_durability_native_source_registration VALUES (1,'','owner',1,E'\\\\x01',1)").execute(&mut connection).await.is_err());
        sqlx::query("INSERT INTO game_durability_native_source_registration VALUES (1,'store:one','owner:approved',18446744073709551615,E'\\\\x01',10)").execute(&mut connection).await?;
        sqlx::query("INSERT INTO game_durability_native_source_descriptor_history VALUES (1,18446744073709551615,E'\\\\x01',10)").execute(&mut connection).await?;
        sqlx::query("INSERT INTO game_durability_native_source_publication_slots (registration_id,slot_id) VALUES (1,1),(1,2)").execute(&mut connection).await?;
        assert!(sqlx::query("UPDATE game_durability_native_source_registration SET descriptor_revision=1 WHERE registration_id=1").execute(&mut connection).await.is_err());
        assert!(sqlx::query("DELETE FROM game_durability_native_source_registration WHERE registration_id=1").execute(&mut connection).await.is_err());

        sqlx::query("INSERT INTO game_durability_native_source_floors VALUES (1,'platform','ReadAccountSecurityV1','account:1',18446744073709551615,'decision:max',100,E'\\\\x00')").execute(&mut connection).await?;
        sqlx::query("INSERT INTO game_durability_native_source_observation_history VALUES (1,'platform','ReadAccountSecurityV1','account:1',18446744073709551615,'decision:max',100,E'\\\\x00')").execute(&mut connection).await?;
        assert!(sqlx::query("UPDATE game_durability_native_source_floors SET source_revision=1 WHERE registration_id=1").execute(&mut connection).await.is_err());
        assert!(sqlx::query("DELETE FROM game_durability_native_source_observation_history").execute(&mut connection).await.is_err());

        sqlx::query("UPDATE game_durability_native_source_publication_slots SET operation_binding=E'\\\\x01',checkpointed_at=100 WHERE slot_id=1").execute(&mut connection).await?;
        sqlx::query("UPDATE game_durability_native_source_publication_slots SET operation_binding=E'\\\\x02',checkpointed_at=101 WHERE slot_id=2").execute(&mut connection).await?;
        let free: i64 = sqlx::query("SELECT count(*) AS count FROM game_durability_native_source_publication_slots WHERE operation_binding IS NULL").fetch_one(&mut connection).await?.try_get("count")?;
        assert_eq!(free, 0);
        assert!(sqlx::query("UPDATE game_durability_native_source_publication_slots SET operation_binding=E'\\\\x03' WHERE slot_id=1").execute(&mut connection).await.is_err());
        sqlx::query("UPDATE game_durability_native_source_publication_slots SET operation_binding=NULL,checkpointed_at=NULL WHERE slot_id=1").execute(&mut connection).await?;
        connection.close().await?;
        let mut admin = sqlx::PgConnection::connect(&admin_url).await?;
        admin.execute(sqlx::query(sqlx::AssertSqlSafe(format!("DROP DATABASE {database_name}")))).await?;
        admin.close().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    })
}

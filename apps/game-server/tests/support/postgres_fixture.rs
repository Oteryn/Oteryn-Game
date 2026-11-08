//! Disposable PG fixture databases. No application rows or runtime owners are shared.
//! Nextest starts a process per case, so the migrated, connection-disabled template
//! lives in the same local test cluster instead of a process-local OnceLock.
use sha2::{Digest, Sha256};
use sqlx::{Connection, Executor};

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

/// Create a fresh database, retaining each caller's normal migrator checksum checks.
/// Only an empty, fully migrated schema is reused; seeding happens in the clone.
pub async fn create_database(admin_url: &str, name: &str) -> TestResult<String> {
    let started = std::time::Instant::now();
    if !admin_url.starts_with("postgresql://oteryn_test_admin:")
        || !admin_url.rsplit_once('@').is_some_and(|(_, host)| {
            host.strip_prefix("127.0.0.1:")
                .and_then(|rest| rest.strip_suffix("/postgres"))
                .is_some_and(|port| port.parse::<u16>().is_ok())
        })
        || name.is_empty()
        || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
        || !name.as_bytes()[0].is_ascii_alphabetic()
    {
        return Err("unsafe PostgreSQL fixture database identity".into());
    }
    let prefix = admin_url
        .strip_suffix("/postgres")
        .ok_or("admin database")?;
    let url = format!("{prefix}/{name}");
    let mut admin = sqlx::PgConnection::connect(admin_url).await?;
    let result: TestResult<()> = async {
        let version: String = sqlx::query_scalar("SHOW server_version_num")
            .fetch_one(&mut admin)
            .await?;
        if version != crate::CANONICAL_POSTGRES_SERVER_VERSION_NUM {
            return Err("canonical target is PostgreSQL 18.6".into());
        }
        if std::env::var_os("OTERYN_TEST_FIXTURE_CACHE").as_deref()
            == Some(std::ffi::OsStr::new("0"))
        {
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "CREATE DATABASE {name}"
                ))))
                .await?;
            return Ok(());
        }
        let mut digest = Sha256::new();
        digest.update(b"oteryn.pg_fixture_template.v2\0");
        digest.update(version.as_bytes());
        for migration in crate::GAME_MIGRATOR.iter() {
            digest.update(migration.version.to_be_bytes());
            digest.update(migration.checksum.as_ref());
        }
        let digest = digest.finalize();
        let suffix: String = digest[..16].iter().map(|b| format!("{b:02x}")).collect();
        let template = format!("oteryn_pg_fixture_v2_{suffix}");
        // A session lock spans CREATE DATABASE and migration transactions. A crashed
        // builder releases it automatically; datallowconn=true marks an incomplete build.
        sqlx::query("SELECT pg_advisory_lock(hashtextextended($1, 0))")
            .bind(&template)
            .execute(&mut admin)
            .await?;
        let ready: Option<bool> =
            sqlx::query_scalar("SELECT NOT datallowconn FROM pg_database WHERE datname=$1")
                .bind(&template)
                .fetch_optional(&mut admin)
                .await?;
        if ready != Some(true) {
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "DROP DATABASE IF EXISTS {template} WITH (FORCE)"
                ))))
                .await?;
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "CREATE DATABASE {template}"
                ))))
                .await?;
            let mut connection =
                sqlx::PgConnection::connect(&format!("{prefix}/{template}")).await?;
            crate::GAME_MIGRATOR.run(&mut connection).await?;
            // The private template excludes per-database decision secrets. It has
            // no application owner or receipts. Only its construction bypasses
            // delete guards; the caller's clone always has normal triggers.
            let mut seeds = connection.begin().await?;
            seeds.execute("SET LOCAL session_replication_role = replica").await?;
            seeds.execute("DELETE FROM game_atelier_decision_root; DELETE FROM game_prey_decision_root; DELETE FROM game_proficiency_shaping_decision_root; DELETE FROM game_forge_decision_root").await?;
            seeds.commit().await?;
            connection.close().await?;
            admin
                .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                    "ALTER DATABASE {template} ALLOW_CONNECTIONS false"
                ))))
                .await?;
        }
        sqlx::query("SELECT pg_advisory_unlock(hashtextextended($1, 0))")
            .bind(&template)
            .execute(&mut admin)
            .await?;
        // PG18's default WAL_LOG strategy avoids the cluster-wide checkpoints of
        // FILE_COPY. The source has no open sessions and is never seeded or mutated.
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "CREATE DATABASE {name} TEMPLATE {template}"
            ))))
            .await?;
        // Database ACLs are not copied by TEMPLATE. Reapply the database-level
        // isolation from 0006_node_boot_control_plane.sql:31-38 before seeding.
        admin
            .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
                "REVOKE CONNECT ON DATABASE {name} FROM PUBLIC"
            ))))
            .await?;

        let mut fresh = sqlx::PgConnection::connect(&url).await?;
        // Initial secrets match migrations 0313:97-98, 0337:65, 0575:110-112,
        // 0723:8. Never reseed an active owner or share entropy between clones.
        fresh.execute("INSERT INTO game_atelier_decision_root VALUES(true,sha256(uuid_send(gen_random_uuid())||uuid_send(gen_random_uuid())||uuid_send(gen_random_uuid()))); INSERT INTO game_prey_decision_root VALUES(true,sha256(uuid_send(gen_random_uuid())||uuid_send(gen_random_uuid()))); INSERT INTO game_proficiency_shaping_decision_root VALUES(true,sha256(uuid_send(gen_random_uuid())||uuid_send(gen_random_uuid()))); INSERT INTO game_forge_decision_root VALUES(true,sha256(uuid_send(gen_random_uuid())||uuid_send(gen_random_uuid())))").await?;
        fresh.close().await?;
        if std::env::var_os("OTERYN_TEST_TIMINGS").is_some() {
            eprintln!(
                "PGSPEED phase=database_template cold={}",
                ready != Some(true)
            );
        }
        Ok(())
    }
    .await;
    let closed = admin.close().await;
    result?;
    closed?;
    if std::env::var_os("OTERYN_TEST_TIMINGS").is_some() {
        eprintln!(
            "PGSPEED phase=database_fixture elapsed={:?}",
            started.elapsed()
        );
    }
    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn postgres_fixture_template_keeps_rows_isolated_and_migration_checks_enforced()
    -> TestResult<()> {
        let Ok(admin_url) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
            return Ok(());
        };
        tokio::runtime::Builder::new_current_thread()
            .enable_all().build()?.block_on(async {
                let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_nanos();
                let first = format!("pgspeed_a_{stamp}");
                let second = format!("pgspeed_b_{stamp}");
                let left_admin = admin_url.clone();
                let left_name = first.clone();
                let right_admin = admin_url.clone();
                let right_name = second.clone();
                let (a, b) = tokio::task::LocalSet::new().run_until(async move {
                    let left = tokio::task::spawn_local(async move { create_database(&left_admin, &left_name).await });
                    let right = tokio::task::spawn_local(async move { create_database(&right_admin, &right_name).await });
                    Ok::<_, Box<dyn std::error::Error>>((left.await??, right.await??))
                }).await?;
                let mut a = sqlx::PgConnection::connect(&a).await?;
                let mut b = sqlx::PgConnection::connect(&b).await?;
                crate::GAME_MIGRATOR.run(&mut a).await?;
                crate::GAME_MIGRATOR.run(&mut b).await?;
                // CREATE DATABASE TEMPLATE does not copy database-level ACLs.
                // Preserve migration 0006_node_boot_control_plane.sql:31-38.
                for database in [&first, &second] {
                    let public_connect: bool = sqlx::query_scalar("SELECT COALESCE(bool_or(a.grantee=0 AND a.privilege_type='CONNECT'),false) FROM pg_database d CROSS JOIN LATERAL aclexplode(COALESCE(d.datacl,acldefault('d',d.datdba))) a WHERE d.datname=$1")
                        .bind(database).fetch_one(&mut a).await?;
                    assert!(!public_connect, "a clone must preserve the migration's PUBLIC CONNECT refusal");
                }
                let secrets = "SELECT secret FROM (SELECT 'atelier' AS name,secret FROM game_atelier_decision_root UNION ALL SELECT 'prey',secret FROM game_prey_decision_root UNION ALL SELECT 'shaping',secret FROM game_proficiency_shaping_decision_root UNION ALL SELECT 'forge',secret FROM game_forge_decision_root) s ORDER BY name";
                let left: Vec<Vec<u8>> = sqlx::query_scalar(secrets).fetch_all(&mut a).await?;
                let right: Vec<Vec<u8>> = sqlx::query_scalar(secrets).fetch_all(&mut b).await?;
                assert_eq!(left.len(), 4);
                assert_eq!(right.len(), 4);
                assert!(left.iter().zip(&right).all(|(a,b)| a != b), "clones must have independent database decision secrets");
                a.execute("CREATE TABLE pgspeed_isolation(value bigint); INSERT INTO pgspeed_isolation VALUES(7)").await?;
                let absent: bool = sqlx::query_scalar("SELECT to_regclass('pgspeed_isolation') IS NULL").fetch_one(&mut b).await?;
                assert!(absent, "one fixture's DDL and rows cannot enter another clone");
                let first_version: i64 = sqlx::query_scalar("SELECT min(version) FROM _sqlx_migrations").fetch_one(&mut a).await?;
                sqlx::query("UPDATE _sqlx_migrations SET checksum=decode('00','hex') WHERE version=$1").bind(first_version).execute(&mut a).await?;
                assert!(matches!(crate::GAME_MIGRATOR.run(&mut a).await, Err(sqlx::migrate::MigrateError::VersionMismatch(v)) if v == first_version));
                crate::GAME_MIGRATOR.run(&mut b).await?;
                a.close().await?;
                b.close().await?;
                let mut admin = sqlx::PgConnection::connect(&admin_url).await?;
                for name in [first, second] {
                    admin.execute(sqlx::query(sqlx::AssertSqlSafe(format!("DROP DATABASE {name} WITH (FORCE)")))).await?;
                }
                admin.close().await?;
                Ok(())
            })
    }

    #[test]
    fn postgres_fixture_rejects_foreign_admin_and_sql_identifiers_before_connecting()
    -> TestResult<()> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                assert!(
                    create_database(
                        "postgresql://not_the_fixture_admin@127.0.0.1/postgres",
                        "pgspeed"
                    )
                    .await
                    .is_err()
                );
                assert!(
                    create_database(
                        "postgresql://oteryn_test_admin:unused@127.0.0.1:1/postgres",
                        "bad; DROP DATABASE postgres"
                    )
                    .await
                    .is_err()
                );
                Ok(())
            })
    }
}

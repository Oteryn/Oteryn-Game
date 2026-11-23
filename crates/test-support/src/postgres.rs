//! Isolated PostgreSQL databases for tests that need the full game schema.

use sha2::{Digest, Sha256};
use sqlx::migrate::Migrator;
use sqlx::postgres::{PgConnectOptions, PgConnection};
use sqlx::{AssertSqlSafe, Connection, Executor};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

static DATABASE_SEQUENCE: AtomicU64 = AtomicU64::new(0);

/// Keep process and sequence before the diagnostic tag, within PostgreSQL's identifier limit.
#[must_use]
pub fn database_name(tag: &str) -> String {
    let sequence = DATABASE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mut name = format!("oteryn_case_{:x}_{sequence:x}_", std::process::id());
    name.extend(tag.bytes().take(63 - name.len()).map(|byte| {
        if byte.is_ascii_alphanumeric() {
            char::from(byte.to_ascii_lowercase())
        } else {
            '_'
        }
    }));
    name
}

/// Clone the shared migration-only template. Return its name and whether it was reused.
/// The caller owns the case database and its cleanup.
pub async fn create_migrated_database(
    admin: &mut PgConnection,
    admin_url: &str,
    name: &str,
    migrator: &Migrator,
) -> Result<(String, bool), sqlx::Error> {
    let (template, reused) = migrated_template(admin, admin_url, migrator).await?;
    clone_migrated_database(admin, admin_url, name, &template).await?;
    Ok((template, reused))
}

async fn migrated_template(
    admin: &mut PgConnection,
    admin_url: &str,
    migrator: &Migrator,
) -> Result<(String, bool), sqlx::Error> {
    let mut hash = Sha256::new();
    for migration in migrator.iter() {
        hash.update(migration.version.to_be_bytes());
        hash.update(migration.checksum.as_ref());
    }
    let digest = hash.finalize();
    // Use the existing revision-sequencer template namespace so all harnesses share it.
    let digest_prefix = u64::from_be_bytes(std::array::from_fn(|index| digest[index]));
    let template = format!("oteryn_tpl_{digest_prefix:016x}");

    // Keep the original key so older test binaries also coordinate with this builder.
    sqlx::query("SELECT pg_advisory_lock(hashtextextended($1, 0))")
        .bind(&template)
        .execute(&mut *admin)
        .await?;
    let result = async {
        let is_template: Option<bool> =
            sqlx::query_scalar("SELECT datistemplate FROM pg_database WHERE datname = $1")
                .bind(&template)
                .fetch_optional(&mut *admin)
                .await?;
        if is_template.is_none() {
            let building = format!("{template}_building");
            let options = admin_url.parse::<PgConnectOptions>()?;
            // An interrupted migration can only leave this unpublished database.
            execute(
                admin,
                format!("DROP DATABASE IF EXISTS {building} WITH (FORCE)"),
            )
            .await?;
            execute(admin, format!("CREATE DATABASE {building}")).await?;
            let mut connection = PgConnection::connect_with(&options.database(&building)).await?;
            migrator.run(&mut connection).await?;
            connection.close().await?;
            // Prevent new source connections and wait for any exiting backend before renaming.
            execute(
                admin,
                format!("ALTER DATABASE {building} ALLOW_CONNECTIONS false"),
            )
            .await?;
            sqlx::query(
                "SELECT pg_terminate_backend(pid, 5000) FROM pg_stat_activity WHERE datname = $1",
            )
            .bind(&building)
            .execute(&mut *admin)
            .await?;
            execute(
                admin,
                format!("ALTER DATABASE {building} RENAME TO {template}"),
            )
            .await?;
        }
        // An interrupted process can also leave a fully migrated database just after rename.
        if is_template != Some(true) {
            execute(admin, format!("ALTER DATABASE {template} IS_TEMPLATE true")).await?;
        }
        Ok::<_, sqlx::Error>(is_template.is_some())
    }
    .await;
    let unlocked = sqlx::query("SELECT pg_advisory_unlock(hashtextextended($1, 0))")
        .bind(&template)
        .execute(admin)
        .await;
    let reused = result?;
    unlocked?;
    Ok((template, reused))
}

/// Clone a fully migrated fixture, including a case-owned template used for a batch.
pub async fn clone_migrated_database(
    admin: &mut PgConnection,
    admin_url: &str,
    name: &str,
    template: &str,
) -> Result<(), sqlx::Error> {
    let options = admin_url.parse::<PgConnectOptions>()?;
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        match execute(admin, format!("CREATE DATABASE {name} TEMPLATE {template}")).await {
            Ok(()) => break,
            Err(error)
                if error
                    .as_database_error()
                    .and_then(|error| error.code())
                    .as_deref()
                    == Some("55006")
                    && Instant::now() < deadline =>
            {
                // A case-owned template can still have an exiting connection. The caller
                // may have paused Tokio's clock.
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(error) => return Err(error),
        }
    }

    let setup = async {
        // PostgreSQL copies schema ACLs, but migration 0006's database ACL needs reapplying.
        execute(admin, format!("REVOKE CONNECT ON DATABASE {name} FROM PUBLIC")).await?;
        let mut connection = PgConnection::connect_with(&options.database(name)).await?;
        // Migration 2833 starts the service-feed resync clock at migration time.
        sqlx::query(
            "UPDATE game_character_service_feed \
             SET resync_started_at_ms = floor(extract(epoch FROM transaction_timestamp())*1000)::bigint \
             WHERE singleton AND phase = 1",
        )
        .execute(&mut connection)
        .await?;
        connection.close().await
    }
    .await;
    if let Err(error) = setup {
        execute(admin, format!("DROP DATABASE {name} WITH (FORCE)")).await?;
        return Err(error);
    }
    Ok(())
}

async fn execute(admin: &mut PgConnection, statement: String) -> Result<(), sqlx::Error> {
    admin.execute(sqlx::query(AssertSqlSafe(statement))).await?;
    Ok(())
}

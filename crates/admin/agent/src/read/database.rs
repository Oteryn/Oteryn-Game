use super::*;
use sqlx::{PgConnection, Row};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

pub(crate) static MIGRATOR: sqlx::migrate::Migrator =
    sqlx::migrate!("../../../apps/game-server/migrations");

pub(super) async fn status(conn: &mut PgConnection) -> Result<DatabaseStatus, E> {
    let tables = sqlx::query_scalar::<_, Json<TableSize>>(
        "SELECT jsonb_build_object('schema', n.nspname, 'table', c.relname, 'bytes', sizes.bytes::text)
         FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid=c.relnamespace
         CROSS JOIN LATERAL (SELECT pg_catalog.pg_total_relation_size(c.oid) AS bytes) sizes
         WHERE c.relkind IN ('r','m') AND n.nspname NOT IN ('pg_catalog','information_schema')
           AND n.nspname NOT LIKE 'pg_toast%' AND n.nspname NOT LIKE 'pg_temp_%'
           AND sizes.bytes IS NOT NULL
         ORDER BY sizes.bytes DESC, n.nspname, c.relname LIMIT 50",
    )
    .fetch_all(&mut *conn).await.map_err(db_error)?;
    let connections = sqlx::query_scalar::<_, Json<RoleConnections>>(
        "SELECT jsonb_build_object('role', usename, 'connections', count(*))
         FROM pg_catalog.pg_stat_activity WHERE datname=current_database() AND usename IS NOT NULL
         GROUP BY usename ORDER BY count(*) DESC, usename LIMIT 100",
    )
    .fetch_all(&mut *conn)
    .await
    .map_err(db_error)?;
    let row = sqlx::query(
        "SELECT (SELECT count(*) FROM pg_catalog.pg_locks
                 WHERE NOT granted AND pid IN
                   (SELECT pid FROM pg_catalog.pg_stat_activity WHERE datname=current_database())) AS waiting,
                (SELECT EXTRACT(EPOCH FROM statement_timestamp()-min(xact_start))::bigint
                 FROM pg_catalog.pg_stat_activity
                 WHERE datname=current_database() AND pid<>pg_backend_pid()) AS oldest,
                COALESCE(blks_hit::double precision / NULLIF(blks_hit+blks_read,0), 0.0) AS hit,
                CASE WHEN pg_is_in_recovery() AND pg_last_xact_replay_timestamp() IS NOT NULL
                     THEN greatest(0, EXTRACT(EPOCH FROM statement_timestamp()-pg_last_xact_replay_timestamp()))::double precision
                END AS lag
         FROM pg_catalog.pg_stat_database WHERE datname=current_database()",
    ).fetch_one(conn).await.map_err(db_error)?;
    let waiting: i64 = row.try_get("waiting").map_err(db_error)?;
    let oldest: Option<i64> = row.try_get("oldest").map_err(db_error)?;
    Ok(DatabaseStatus {
        tables: json_rows(tables),
        connections: json_rows(connections),
        locks_waiting: u32::try_from(waiting).map_err(|_| E::AdmInternal)?,
        replication_lag_seconds: row.try_get("lag").map_err(db_error)?,
        bloat_estimate_bytes: None,
        oldest_transaction_seconds: oldest.map(|age| age.clamp(0, i64::from(u32::MAX)) as u32),
        cache_hit_ratio: row.try_get("hit").map_err(db_error)?,
    })
}

pub(super) async fn migrations(conn: &mut PgConnection, params: Page) -> Result<MigrationPage, E> {
    let cursor: Option<i64> = params
        .cursor
        .as_deref()
        .map(str::parse)
        .transpose()
        .map_err(|_| E::AdmInvalidParams)?;
    let mut versions = Vec::new();
    let mut descriptions = Vec::new();
    let mut checksums = Vec::new();
    for migration in MIGRATOR
        .iter()
        .filter(|migration| !migration.migration_type.is_down_migration())
    {
        versions.push(migration.version);
        descriptions.push(migration.description.as_ref());
        checksums.push(migration.checksum.as_ref());
    }
    let rows = sqlx::query_scalar::<_, Json<Migration>>(
        "WITH pinned AS (
            SELECT * FROM unnest($1::bigint[], $2::text[], $3::bytea[]) AS p(version,description,checksum)
         )
         SELECT jsonb_build_object('version', version::text,
                  'description', COALESCE(p.description,a.description),
                  'applied', COALESCE(a.success,false),
                  'checksum_drift', a.version IS NOT NULL AND a.checksum IS DISTINCT FROM p.checksum)
         FROM pinned p FULL JOIN admin_read.migrations a USING(version)
         WHERE ($4::bigint IS NULL OR version>$4)
         ORDER BY version LIMIT $5",
    )
    .bind(versions).bind(descriptions).bind(checksums).bind(cursor)
    .bind(i64::from(params.limit)+1)
    .fetch_all(conn).await.map_err(db_error)?;
    let (migrations, next_cursor) = page(json_rows(rows), params.limit, |row| row.version.clone());
    Ok(MigrationPage {
        migrations,
        next_cursor,
    })
}

fn timestamp(text: &str) -> Result<OffsetDateTime, E> {
    OffsetDateTime::parse(text, &Rfc3339).map_err(|_| E::AdmInvalidParams)
}

pub(super) async fn ledger(
    conn: &mut PgConnection,
    target: &Target,
    worlds: &[String],
    params: LedgerQuery,
) -> Result<LedgerPage, E> {
    let from = timestamp(&params.from)?;
    let to = timestamp(&params.to)?;
    if from >= to {
        return Err(E::AdmInvalidParams);
    }
    if to - from > time::Duration::days(31) {
        return Err(E::AdmLimitExceeded);
    }
    let (cursor_time, cursor_id) = match params.cursor.as_deref() {
        Some(cursor) => {
            let (at, id) = cursor.split_once('/').ok_or(E::AdmInvalidParams)?;
            timestamp(at)?;
            (Some(at), Some(uuid(id)?))
        }
        None => (None, None),
    };
    let rows = sqlx::query_scalar::<_, Json<LedgerEntry>>(
        "SELECT jsonb_build_object('ledger_id', ledger_id, 'request_id', request_id,
                   'admin_operation_id', admin_operation_id, 'operation', operation,
                   'admin_id', admin_id, 'risk_class', risk_class, 'target', target,
                   'status', status, 'error_code', error_code,
                   'started_at', started_at, 'finished_at', finished_at)
         FROM admin_read.ledger
         WHERE started_at >= $1::text::timestamptz AND started_at < $2::text::timestamptz
           AND ($3::text IS NULL OR admin_id=$3)
           AND ($4::text IS NULL OR operation=$4)
           AND ($5::text IS NULL OR target->>'world_id'=$5)
           AND ($6::text IS NULL OR target->>'character_id'=$6)
           AND ($7::text IS NULL OR target->>'character_id'=$7)
           AND ($8::text IS NULL OR target->>'node_id'=$8)
           AND ($9::text IS NULL OR target->>'account_ref'=$9)
           AND target->>'world_id'=ANY($10)
           AND ($11::text IS NULL OR (started_at,ledger_id)<($11::text::timestamptz,$12))
         ORDER BY started_at DESC, ledger_id DESC LIMIT $13",
    )
    .bind(&params.from)
    .bind(&params.to)
    .bind(&params.admin_id)
    .bind(&params.operation)
    .bind(&params.world_id)
    .bind(&params.character_id)
    .bind(&target.character_id)
    .bind(&target.node_id)
    .bind(&target.account_ref)
    .bind(worlds)
    .bind(cursor_time)
    .bind(cursor_id)
    .bind(i64::from(params.limit) + 1)
    .fetch_all(conn)
    .await
    .map_err(db_error)?;
    let (entries, next_cursor) = page(json_rows(rows), params.limit, |row| {
        format!("{}/{}", row.started_at, row.ledger_id)
    });
    Ok(LedgerPage {
        entries,
        next_cursor,
    })
}

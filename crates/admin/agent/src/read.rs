//! Phase-one queries. The channel owns replay protection and audit writes.
pub(crate) mod database;
mod players;
mod topology;

use crate::{config::Config, policy::Policy};
use oteryn_admin_contracts::{ErrorCode as E, *};
use serde::de::DeserializeOwned;
use sqlx::{PgPool, Postgres, SqlSafeStr, Transaction, types::Json};
use tokio::time::Instant;
use uuid::Uuid;

pub(crate) const IMPLEMENTED: &[Operation] = &[
    Operation::AdminLedgerQuery,
    Operation::AdminPolicyGet,
    Operation::DbMigrationsList,
    Operation::DbStatus,
    Operation::NodeGet,
    Operation::NodeList,
    Operation::OpsOverviewGet,
    Operation::PlayerCharacterGet,
    Operation::PlayerInventoryGet,
    Operation::PlayerSearch,
    Operation::SanctionList,
    Operation::WorldGet,
    Operation::WorldList,
];

pub(crate) async fn preconditions(
    op: Operation,
    target: &Target,
    pool: Option<&PgPool>,
    deadline: Instant,
) -> Result<Option<Transaction<'static, Postgres>>, E> {
    if matches!(
        op,
        Operation::PlayerCharacterGet | Operation::PlayerInventoryGet
    ) {
        let mut tx = begin(pool.ok_or(E::AdmTargetUnavailable)?, deadline).await?;
        players::check_offline_target(&mut tx, target).await?;
        return Ok(Some(tx));
    }
    Ok(None)
}

pub(crate) async fn check_retained_sanctions(
    request: &Request,
    config: &Config,
    policy: &Policy,
    pool: Option<&PgPool>,
    deadline: Instant,
    response: &Response,
) -> Result<(), E> {
    if let Some(OperationResult::SanctionList(result)) = &response.result {
        if result.sanctions.is_empty() {
            return Ok(());
        }
        let worlds = scoped_worlds(Operation::SanctionList, &request.target, config, policy)?;
        let worlds = world_ids(&worlds);
        let mut tx = begin(pool.ok_or(E::AdmTargetUnavailable)?, deadline).await?;
        players::check_sanction_visibility(&mut tx, &worlds, &result.sanctions, &request.target)
            .await?;
        tx.commit().await.map_err(db_error)?;
    }
    Ok(())
}

fn scoped_worlds(
    op: Operation,
    target: &Target,
    config: &Config,
    policy: &Policy,
) -> Result<Vec<String>, E> {
    if op.is_global_read() {
        return Ok(vec![]);
    }
    let rule = policy.operations.get(op.name()).ok_or(E::AdmForbidden)?;
    let mut worlds: Vec<_> = config
        .worlds
        .iter()
        .filter(|id| rule.worlds.contains(*id))
        .filter(|id| target.world_id.as_ref().is_none_or(|world| world == *id))
        .cloned()
        .collect();
    worlds.sort_unstable();
    worlds.dedup();
    Ok(worlds)
}

fn world_ids(worlds: &[String]) -> Vec<Uuid> {
    worlds
        .iter()
        .filter_map(|id| Uuid::parse_str(id).ok())
        .collect()
}

pub(crate) async fn execute(
    op: Operation,
    request: &Request,
    config: &Config,
    policy: &Policy,
    pool: Option<&PgPool>,
    snapshot: Option<Transaction<'static, Postgres>>,
    deadline: Instant,
) -> Result<OperationResult, E> {
    use OperationResult as R;
    check_deadline(deadline)?;
    if op == Operation::AdminPolicyGet {
        return Ok(R::AdminPolicyGet(policy.view()));
    }
    let params = &request.params;
    let target = &request.target;
    let worlds = scoped_worlds(op, target, config, policy)?;
    let world_ids = world_ids(&worlds);
    let nodes: Vec<_> = config
        .nodes
        .iter()
        .filter(|id| target.node_id.as_ref().is_none_or(|node| node == *id))
        .filter_map(|id| Uuid::parse_str(id).ok())
        .collect();

    if op == Operation::OpsOverviewGet {
        return Ok(R::OpsOverviewGet(
            topology::overview(pool, &worlds, &world_ids, &nodes, deadline).await?,
        ));
    }
    let mut tx = match (snapshot, pool) {
        (Some(tx), _) => Some(tx),
        (None, Some(pool)) => Some(begin(pool, deadline).await?),
        (None, None) => None,
    };
    let conn = tx.as_deref_mut();
    let first_page = || Page {
        limit: 1,
        cursor: None,
    };
    let result = match op {
        Operation::WorldList => {
            R::WorldList(topology::world_page(conn, &worlds, parse(params)?).await?)
        }
        Operation::WorldGet => {
            let page = topology::world_page(conn, &worlds, first_page()).await?;
            R::WorldGet(page.worlds.into_iter().next().ok_or(E::AdmTargetNotFound)?)
        }
        Operation::NodeList => R::NodeList(match conn {
            Some(conn) => topology::node_page(conn, &nodes, &world_ids, parse(params)?).await?,
            None => NodePage {
                nodes: vec![],
                next_cursor: None,
            },
        }),
        Operation::NodeGet => {
            if nodes.is_empty() {
                return Err(E::AdmTargetNotFound);
            }
            let page = topology::node_page(
                conn.ok_or(E::AdmTargetUnavailable)?,
                &nodes,
                &world_ids,
                first_page(),
            )
            .await?;
            R::NodeGet(page.nodes.into_iter().next().ok_or(E::AdmTargetNotFound)?)
        }
        Operation::DbStatus => {
            R::DbStatus(database::status(conn.ok_or(E::AdmTargetUnavailable)?).await?)
        }
        Operation::DbMigrationsList => R::DbMigrationsList(
            database::migrations(conn.ok_or(E::AdmTargetUnavailable)?, parse(params)?).await?,
        ),
        Operation::PlayerSearch => R::PlayerSearch(
            players::search(
                conn.ok_or(E::AdmTargetUnavailable)?,
                &world_ids,
                target,
                parse(params)?,
            )
            .await?,
        ),
        Operation::PlayerCharacterGet => R::PlayerCharacterGet(
            players::character(conn.ok_or(E::AdmTargetUnavailable)?, target).await?,
        ),
        Operation::PlayerInventoryGet => R::PlayerInventoryGet(
            players::inventory(conn.ok_or(E::AdmTargetUnavailable)?, target, parse(params)?)
                .await?,
        ),
        Operation::SanctionList => R::SanctionList(
            players::sanctions(
                conn.ok_or(E::AdmTargetUnavailable)?,
                target,
                &world_ids,
                parse(params)?,
            )
            .await?,
        ),
        Operation::AdminLedgerQuery => R::AdminLedgerQuery(
            database::ledger(
                conn.ok_or(E::AdmTargetUnavailable)?,
                target,
                &worlds,
                parse(params)?,
            )
            .await?,
        ),
        _ => return Err(E::AdmUnknownOperation),
    };
    if let Some(tx) = tx {
        tx.commit().await.map_err(db_error)?;
    }
    Ok(result)
}

pub(crate) fn check_deadline(deadline: Instant) -> Result<(), E> {
    if Instant::now() >= deadline {
        return Err(E::AdmExpired);
    }
    Ok(())
}

async fn begin(pool: &PgPool, deadline: Instant) -> Result<Transaction<'static, Postgres>, E> {
    let connection = pool.acquire().await.map_err(db_error)?;
    check_deadline(deadline)?;
    let mut tx = Transaction::begin(
        connection,
        Some("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY".into_sql_str()),
    )
    .await
    .map_err(db_error)?;
    check_deadline(deadline)?;
    let remaining = deadline.saturating_duration_since(Instant::now());
    let transaction_timeout = format!("{}ms", remaining.as_millis() + 1);
    sqlx::query(
        "SELECT pg_catalog.set_config('transaction_timeout', $1, true),
                pg_catalog.set_config('statement_timeout', '5s', true),
                pg_catalog.set_config('search_path', 'pg_catalog, public', true),
                pg_catalog.set_config('TimeZone', 'UTC', true)",
    )
    .bind(transaction_timeout)
    .execute(&mut *tx)
    .await
    .map_err(db_error)?;
    check_deadline(deadline)?;
    Ok(tx)
}

fn db_error(error: sqlx::Error) -> E {
    match error {
        // PostgreSQL's transaction_timeout can win the race with the local timer.
        sqlx::Error::Database(error) if error.code().as_deref() == Some("25P04") => E::AdmExpired,
        sqlx::Error::ColumnDecode { .. } | sqlx::Error::Decode(_) => E::AdmInternal,
        _ => E::AdmTargetUnavailable,
    }
}

fn parse<T: DeserializeOwned>(params: &serde_json::Value) -> Result<T, E> {
    serde_json::from_value(params.clone()).map_err(|_| E::AdmInvalidParams)
}

fn required(id: &Option<String>) -> Result<&str, E> {
    id.as_deref().ok_or(E::AdmInvalidParams)
}

fn uuid(id: &str) -> Result<Uuid, E> {
    Uuid::parse_str(id).map_err(|_| E::AdmInvalidParams)
}

fn optional_uuid(id: Option<&str>) -> Result<Option<Uuid>, E> {
    id.map(uuid).transpose()
}

/// SQL (or the config iterator) returns one extra item to prove a next page.
fn page<T>(
    mut rows: Vec<T>,
    limit: u16,
    cursor: impl FnOnce(&T) -> String,
) -> (Vec<T>, Option<String>) {
    let limit = usize::from(limit);
    let next = if rows.len() > limit {
        rows.truncate(limit);
        rows.last().map(cursor)
    } else {
        None
    };
    (rows, next)
}

fn json_rows<T>(rows: Vec<Json<T>>) -> Vec<T> {
    rows.into_iter().map(|Json(row)| row).collect()
}

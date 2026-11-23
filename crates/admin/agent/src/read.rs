//! Phase-one queries. The channel owns replay protection and audit writes.
pub(crate) mod database;
mod players;
mod topology;

use crate::{config::Config, policy::Policy};
use oteryn_admin_contracts::{ErrorCode as E, *};
use serde::de::DeserializeOwned;
use sqlx::{PgPool, Postgres, Transaction, types::Json};
use uuid::Uuid;

pub(crate) fn implemented(op: Operation) -> bool {
    !matches!(op, Operation::AdminAgentList | Operation::AnalyticsCatalog)
}

pub(crate) async fn execute(
    op: Operation,
    request: &Request,
    config: &Config,
    policy: &Policy,
    pool: Option<&PgPool>,
) -> Result<OperationResult, E> {
    use OperationResult as R;
    if op == Operation::AdminPolicyGet {
        return Ok(R::AdminPolicyGet(policy.view()));
    }
    if !implemented(op) {
        return Err(E::AdmUnknownOperation);
    }
    let params = &request.params;
    let target = &request.target;
    let rule = policy.operations.get(op.name()).ok_or(E::AdmForbidden)?;
    let mut worlds: Vec<_> = config
        .worlds
        .iter()
        .filter(|id| rule.worlds.is_empty() || rule.worlds.contains(*id))
        .filter(|id| target.world_id.as_ref().is_none_or(|world| world == *id))
        .cloned()
        .collect();
    worlds.sort_unstable();
    worlds.dedup();

    if target.world_id.is_some() && worlds.is_empty() {
        return Err(E::AdmTargetNotFound);
    }
    let world_ids: Vec<_> = worlds
        .iter()
        .filter_map(|id| Uuid::parse_str(id).ok())
        .collect();
    let nodes: Vec<_> = config
        .nodes
        .iter()
        .filter_map(|id| Uuid::parse_str(id).ok())
        .collect();
    let global = target.world_id.is_none() && rule.worlds.is_empty();

    if op == Operation::OpsOverviewGet {
        return Ok(R::OpsOverviewGet(
            topology::overview(pool, &worlds, &world_ids, &nodes, global).await?,
        ));
    }
    let mut tx = match pool {
        Some(pool) => Some(begin(pool).await?),
        None => None,
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
            Some(conn) => {
                topology::node_page(conn, &nodes, &world_ids, global, parse(params)?).await?
            }
            None => NodePage {
                nodes: vec![],
                next_cursor: None,
            },
        }),
        Operation::NodeGet => {
            let id = required(&target.node_id)?;
            if !config.nodes.iter().any(|node| node == id) {
                return Err(E::AdmTargetNotFound);
            }
            let node = Uuid::parse_str(id).map_err(|_| E::AdmTargetUnavailable)?;
            let page = topology::node_page(
                conn.ok_or(E::AdmTargetUnavailable)?,
                &[node],
                &world_ids,
                global,
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
                global,
                parse(params)?,
            )
            .await?,
        ),
        Operation::AdminLedgerQuery => R::AdminLedgerQuery(
            database::ledger(
                conn.ok_or(E::AdmTargetUnavailable)?,
                target,
                &worlds,
                global,
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

async fn begin(pool: &PgPool) -> Result<Transaction<'static, Postgres>, E> {
    let mut tx = pool
        .begin_with("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .await
        .map_err(db_error)?;
    sqlx::raw_sql(
        "SET LOCAL statement_timeout = '5s';
        SET LOCAL search_path = pg_catalog, public;
        SET LOCAL TIME ZONE 'UTC'",
    )
    .execute(&mut *tx)
    .await
    .map_err(db_error)?;
    Ok(tx)
}

fn db_error(error: sqlx::Error) -> E {
    match error {
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

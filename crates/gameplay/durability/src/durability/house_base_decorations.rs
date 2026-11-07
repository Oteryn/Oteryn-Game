//! W18: static House placement state, separate from movable Item instance custody.
use super::consume_transform::HouseActionContext;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use sqlx::Row;

#[derive(Debug, Clone)]
pub struct HouseBaseDecoration {
    pub placement_key: u64,
    pub definition_key: String,
    pub revision: u64,
}

pub struct HouseBaseDecorationSnapshot {
    pub materialized: bool,
    pub entries: Vec<HouseBaseDecoration>,
}

#[derive(serde::Serialize)]
pub struct HouseBaseDecorationSource {
    pub placement_key: u64,
    pub before_key: String,
    pub position: [i32; 3],
}

pub struct HouseBaseDecorationSet {
    pub bundle: [u8; 32],
    pub command: u64,
    pub placement: u64,
    pub expected_revision: u64,
    pub before_key: String,
    pub after_key: String,
    pub intent: Vec<u8>,
    /// Server-resolved source House cell (the adjoining cell for a boundary window).
    pub house_cell: [i32; 3],
}

macro_rules! fence_args {
    () => { "encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4::text::numeric,$5::text::numeric,encode($6,'hex')::uuid,$7::text::numeric,encode($8,'hex')::uuid,$9" };
}
const LOAD: &str = concat!(
    "SELECT placement_key::text,definition_key,revision::text,materialized FROM game_house_base_decoration_load_fenced(",
    fence_args!(),
    ",$10)"
);
const REPLAY: &str = concat!(
    "SELECT game_house_base_decoration_replay_fenced(",
    fence_args!(),
    ",$10,$11::text::numeric,$12)"
);
const SET: &str = concat!(
    "SELECT game_house_base_decoration_set_fenced(",
    fence_args!(),
    ",$10,$11::text::numeric,$12::text::numeric,$13::text::numeric,$14,$15,$16,$17)"
);
const MATERIALIZE: &str = concat!(
    "SELECT game_house_base_decoration_materialize_fenced(",
    fence_args!(),
    ",$10,$11)"
);

fn query(
    sql: &'static str,
    node: &NodeIncarnationProof,
    context: &HouseActionContext,
) -> sqlx::query::Query<'static, sqlx::Postgres, sqlx::postgres::PgArguments> {
    let fence = context.fence();
    sqlx::query(sql)
        .bind(fence.game_session_id.as_bytes().to_vec())
        .bind(fence.character_id.as_bytes().to_vec())
        .bind(fence.connection_generation.get().to_string())
        .bind(fence.character_lease_generation.to_string())
        .bind(fence.scope_ownership_generation.get().to_string())
        .bind(node.fact().node_id().as_bytes().to_vec())
        .bind(node.fact().registration_revision().to_string())
        .bind(context.house().world_id().as_bytes().to_vec())
        .bind(context.house().house_key().to_owned())
}
fn disposition(value: &str) -> UseDisposition {
    match value {
        "committed" => UseDisposition::Committed,
        "stale" => UseDisposition::StaleState,
        "exhausted" => UseDisposition::Exhausted,
        _ => UseDisposition::NotPossible,
    }
}

impl DurabilityRoot {
    pub async fn read_house_base_decorations(
        &self,
        node: &NodeIncarnationProof,
        context: &HouseActionContext,
        bundle: [u8; 32],
    ) -> Result<HouseBaseDecorationSnapshot, DurabilityError> {
        let node = node.clone();
        let context = context.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !prove_current_incarnation(&mut tx, &node).await? {
                        return Err(DurabilityError::Unavailable);
                    }
                    let rows = query(LOAD, &node, &context)
                        .bind(bundle.to_vec())
                        .fetch_all(&mut *tx)
                        .await?;
                    if rows.len() > 2000 {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let materialized = rows
                        .first()
                        .ok_or(DurabilityError::Unavailable)?
                        .try_get("materialized")?;
                    let mut result = Vec::with_capacity(rows.len());
                    for row in rows {
                        let Some(placement) = row.try_get::<Option<String>, _>("placement_key")?
                        else {
                            continue;
                        };
                        result.push(HouseBaseDecoration {
                            placement_key: placement
                                .parse()
                                .map_err(|_| DurabilityError::InvalidStoredState)?,
                            definition_key: row.try_get("definition_key")?,
                            revision: row
                                .try_get::<String, _>("revision")?
                                .parse()
                                .map_err(|_| DurabilityError::InvalidStoredState)?,
                        });
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(HouseBaseDecorationSnapshot {
                        materialized,
                        entries: result,
                    })
                })
            })
            .await
    }

    pub async fn materialize_house_base_decorations(
        &self,
        node: &NodeIncarnationProof,
        context: &HouseActionContext,
        bundle: [u8; 32],
        sources: Vec<HouseBaseDecorationSource>,
    ) -> Result<bool, DurabilityError> {
        if sources.len() > 2000 {
            return Err(DurabilityError::InvalidStoredState);
        }
        let sources =
            serde_json::to_value(sources).map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        let context = context.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !prove_current_incarnation(&mut tx, &node).await? {
                        return Err(DurabilityError::Unavailable);
                    }
                    let row = query(MATERIALIZE, &node, &context)
                        .bind(bundle.to_vec())
                        .bind(sources)
                        .fetch_one(&mut *tx)
                        .await?;
                    let initialized = row.try_get(0)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(initialized)
                })
            })
            .await
    }

    pub async fn replay_house_base_decoration(
        &self,
        node: &NodeIncarnationProof,
        context: &HouseActionContext,
        bundle: [u8; 32],
        command: u64,
        intent: &[u8],
    ) -> Result<Option<UseDisposition>, DurabilityError> {
        let node = node.clone();
        let context = context.clone();
        let intent = intent.to_vec();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !prove_current_incarnation(&mut tx, &node).await? {
                        return Err(DurabilityError::Unavailable);
                    }
                    let row = query(REPLAY, &node, &context)
                        .bind(bundle.to_vec())
                        .bind(command.to_string())
                        .bind(intent)
                        .fetch_one(&mut *tx)
                        .await?;
                    let value: Option<String> = row.try_get(0)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(value.as_deref().map(disposition))
                })
            })
            .await
    }

    pub async fn set_house_base_decoration(
        &self,
        node: &NodeIncarnationProof,
        context: &HouseActionContext,
        request: HouseBaseDecorationSet,
    ) -> Result<UseDisposition, DurabilityError> {
        let node = node.clone();
        let context = context.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !prove_current_incarnation(&mut tx, &node).await? {
                        return Err(DurabilityError::Unavailable);
                    }
                    let row = query(SET, &node, &context)
                        .bind(request.bundle.to_vec())
                        .bind(request.command.to_string())
                        .bind(request.placement.to_string())
                        .bind(request.expected_revision.to_string())
                        .bind(request.before_key)
                        .bind(request.after_key)
                        .bind(request.intent)
                        .bind(
                            request
                                .house_cell
                                .into_iter()
                                .flat_map(i32::to_be_bytes)
                                .collect::<Vec<_>>(),
                        )
                        .fetch_one(&mut *tx)
                        .await?;
                    let value: String = row.try_get(0)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(disposition(&value))
                })
            })
            .await
    }
}

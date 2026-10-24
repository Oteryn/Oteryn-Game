//! Private native HOUSE-1b bridge. Receipts are data, never current authority.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::house_scope_handoff::HouseId;
use super::item_transfer::{CurrentCharacterItemFence, character_item_fence_is_current};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{CommandRef, RuntimeScopeRefV1};
use oteryn_gameplay_housing::house_move_out::{HouseMoveOutCommand, valid_deadline};
use oteryn_protocol_oteryn::house_move_out::{
    HouseMoveOutReply, decode_house_move_out_reply, encode_house_move_out_reply,
};
use serde_json::Value;
use sqlx::{Postgres, Transaction};

mod audit;
mod jobs;

#[derive(Debug)]
pub enum HouseDispositionError {
    Rejected,
    Conflict,
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for HouseDispositionError {
    fn from(e: DurabilityError) -> Self {
        Self::Unavailable(e)
    }
}
impl From<sqlx::Error> for HouseDispositionError {
    fn from(e: sqlx::Error) -> Self {
        Self::Unavailable(e.into())
    }
}
type Result<T> = std::result::Result<T, HouseDispositionError>;

impl DurabilityRoot {
    /// One original CommandRef and full fence bind the immutable original notice/cancel outcome.
    /// Exact replay precedes current ownership/clock reads; a successor command uses current fences.
    pub async fn apply_house_move_out_command(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        house: HouseId,
        action: HouseMoveOutCommand,
    ) -> Result<HouseMoveOutReply> {
        self.house_move_out_request(authority, node, fence, house, (command, Some(action)))
            .await
    }

    /// Single-property owner projection. No player identity comes from the request payload.
    pub async fn read_house_move_out_owner(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        house: HouseId,
        command: CommandRef,
    ) -> Result<HouseMoveOutReply> {
        // Query CommandRef is only used for the current fence predicate; no command is consumed.
        self.house_move_out_request(authority, node, fence, house, (command, None))
            .await
    }

    async fn house_move_out_request(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        house: HouseId,
        request: (CommandRef, Option<HouseMoveOutCommand>),
    ) -> Result<HouseMoveOutReply> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseDispositionError::Rejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let result = execute(&mut tx, &node, fence, &house, request).await;
                    match result {
                        Ok(reply) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(reply))
                        }
                        Err(HouseDispositionError::Unavailable(e)) => Err(e),
                        Err(e) => Ok(Err(e)),
                    }
                })
            })
            .await?
    }
}

async fn execute(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    house: &HouseId,
    request: (CommandRef, Option<HouseMoveOutCommand>),
) -> Result<HouseMoveOutReply> {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fence.runtime_scope
    else {
        return Err(HouseDispositionError::Rejected);
    };
    let (command, action) = request;
    if house.world_id() != world_id || command.game_session_id() != fence.game_session_id {
        return Err(HouseDispositionError::Rejected);
    }
    let (kind, due) = match action {
        Some(HouseMoveOutCommand::Notice { due_at_unix_ms }) => (1, due_at_unix_ms),
        Some(HouseMoveOutCommand::Cancel) => (2, 0),
        None => (0, 0),
    };
    let mut p = scope_payload(fence.runtime_scope, fence.scope_ownership_generation.get())?;
    p["house"] = house.house_key().into();
    p["session"] = hex(fence.game_session_id.as_bytes()).into();
    p["character"] = hex(fence.character_id.as_bytes()).into();
    p["command"] = command.command_id().get().to_string().into();
    p["connection"] = fence.connection_generation.get().to_string().into();
    p["lease"] = fence.character_lease_generation.to_string().into();
    p["action"] = kind.into();
    p["due"] = due.into();
    let mode = if action.is_some() { "command" } else { "query" };
    let value = port(tx, node, mode, &p).await?;
    if value.get("conflict").is_some() {
        return Err(HouseDispositionError::Conflict);
    }
    if value.is_null() {
        return Err(HouseDispositionError::Rejected);
    }
    // Keep the shared Character item fence as the final current-authority predicate. The
    // native port already holds these rows in the same order; a historical replay skips it.
    if (action.is_none() || value.get("new").and_then(Value::as_bool) == Some(true))
        && !character_item_fence_is_current(
            tx,
            node,
            &fence,
            command,
            fence.character_id,
            *world_id.as_bytes(),
            *channel_id.as_bytes(),
        )
        .await?
    {
        return Err(HouseDispositionError::Rejected);
    }
    if value.get("state").and_then(Value::as_u64) == Some(0) {
        return Ok(HouseMoveOutReply {
            operation: None,
            state: 0,
            due_at_unix_ms: 0,
            census_count: 0,
            delivered: 0,
        });
    }
    if let Some(HouseMoveOutCommand::Notice { due_at_unix_ms }) = action {
        let now = value
            .get("now")
            .and_then(Value::as_i64)
            .ok_or(HouseDispositionError::Rejected)?;
        if !valid_deadline(now, due_at_unix_ms) {
            return Err(HouseDispositionError::Rejected);
        }
    }
    let result = reply(&value)?;
    if value.get("new").and_then(Value::as_bool) == Some(true) {
        let operation = value
            .get("operation")
            .and_then(Value::as_str)
            .ok_or(HouseDispositionError::Rejected)?;
        audit::write(tx, node, &p, operation).await?;
    }
    Ok(result)
}

fn hex(id: &[u8; 16]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}
fn scope_payload(scope: RuntimeScopeRefV1, generation: u64) -> Result<Value> {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = scope
    else {
        return Err(HouseDispositionError::Rejected);
    };
    Ok(
        serde_json::json!({"world":hex(world_id.as_bytes()),"channel":hex(channel_id.as_bytes()),"generation":generation.to_string()}),
    )
}
async fn port(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    mode: &str,
    p: &Value,
) -> std::result::Result<Value, DurabilityError> {
    let result: Option<Value> = sqlx::query_scalar(
        "SELECT game_house_move_out_port($1,$2,encode($3,'hex')::uuid,$4::text::numeric,$5)",
    )
    .bind(mode)
    .bind(p)
    .bind(node.fact().node_id().as_bytes().as_slice())
    .bind(node.fact().registration_revision().to_string())
    .bind(node.sql_possession_secret())
    .fetch_one(&mut **tx)
    .await?;
    Ok(result.unwrap_or(Value::Null))
}

fn reply(v: &Value) -> Result<HouseMoveOutReply> {
    let bad = || HouseDispositionError::Unavailable(DurabilityError::InvalidStoredState);
    let operation = v
        .get("operation")
        .and_then(Value::as_str)
        .map(super::character_progression::uuid_text)
        .transpose()?
        .ok_or_else(bad)?;
    let r = HouseMoveOutReply {
        operation: Some(operation),
        state: v
            .get("state")
            .and_then(Value::as_u64)
            .and_then(|n| u8::try_from(n).ok())
            .ok_or_else(bad)?,
        due_at_unix_ms: v.get("due").and_then(Value::as_i64).ok_or_else(bad)?,
        census_count: v
            .get("count")
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(bad)?,
        delivered: v
            .get("delivered")
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(bad)?,
    };
    let bytes = encode_house_move_out_reply(&r).map_err(|_| bad())?;
    decode_house_move_out_reply(&bytes).map_err(|_| bad())
}

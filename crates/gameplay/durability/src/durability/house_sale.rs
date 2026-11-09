//! Ownership negotiations use current actor fences; receipts are never authority.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::{CurrentCharacterItemFence, character_item_fence_is_current};
use super::premium_activation_latch::{PremiumGateOwner, PremiumGateTx};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::premium::PremiumStatus;
use crate::foundation::{CommandRef, RuntimeScopeRefV1};
use oteryn_gameplay_character_recovery_fence::CharacterRecoveryFenceV1;
use oteryn_protocol_oteryn::house_sale::HouseSaleRequest;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{Acquire, Postgres, Transaction};
pub(in crate::durability) mod audit;
#[derive(Debug)]
pub enum HouseSaleError {
    Rejected,
    Conflict,
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for HouseSaleError {
    fn from(e: DurabilityError) -> Self {
        Self::Unavailable(e)
    }
}
impl From<sqlx::Error> for HouseSaleError {
    fn from(e: sqlx::Error) -> Self {
        Self::Unavailable(e.into())
    }
}
type Result<T> = std::result::Result<T, HouseSaleError>;
#[derive(Debug, Clone, Deserialize)]
pub struct HouseSaleTerms {
    pub house: String,
    pub revision: u64,
    pub owner: String,
    pub recipient: String,
    pub transfer_state: u8,
    #[serde(rename = "due")]
    pub due_at_unix_ms: i64,
    pub price: u64,
    pub rent: u64,
    pub allowed: u8,
    pub move_out_state: u8,
}
#[derive(Debug, Clone, Deserialize)]
pub struct HouseSaleSnapshot {
    #[serde(rename = "now")]
    pub now_unix_ms: i64,
    pub rows: Vec<HouseSaleTerms>,
    pub has_more: bool,
}
#[derive(Clone)]
struct Work {
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    p: Value,
    mode: &'static str,
}
impl DurabilityRoot {
    pub async fn house_sale_request<G: PremiumGateOwner>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        gate: Option<&G>,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        request: HouseSaleRequest,
    ) -> Result<HouseSaleSnapshot> {
        if !request.valid() || command.game_session_id() != fence.game_session_id {
            return Err(HouseSaleError::Rejected);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseSaleError::Rejected)?;
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Err(HouseSaleError::Rejected);
        };
        let p = json!({"world":hex(world_id.as_bytes()),"channel":hex(channel_id.as_bytes()),
            "generation":fence.scope_ownership_generation.get().to_string(),"session":hex(fence.game_session_id.as_bytes()),
            "character":hex(fence.character_id.as_bytes()),"command":command.command_id().get().to_string(),
            "connection":fence.connection_generation.get().to_string(),"lease":fence.character_lease_generation.to_string(),
            "action":request.action,"house":request.house,"revision":request.expected_revision.to_string(),
            "due":request.due_at_unix_ms,"recipient":request.recipient,"price":request.price,"after":request.after_house});
        let mut work = Work {
            fence,
            command,
            p,
            mode: "inspect",
        };
        let inspected = run(self, None, &recovery, node, work.clone()).await?;
        if let Some(prior) = inspected.get("prior") {
            return snapshot(prior.clone());
        }
        if request.action == 0 {
            return snapshot(inspected);
        }
        work.mode = "command";
        let outcome = if matches!(request.action, 3 | 5) {
            let account = inspected
                .get("target")
                .and_then(Value::as_str)
                .ok_or(HouseSaleError::Rejected)?;
            let account = super::character_progression::uuid_text(account)?;
            let gate = gate.ok_or(HouseSaleError::Rejected)?;
            let root = self.clone();
            let node = node.clone();
            gate.with_premium_gate(account, move |status, tx| {
                Box::pin(async move {
                    work.p["premium_account"] = hex(&account).into();
                    work.p["premium_allowed"] =
                        (!matches!(status, PremiumStatus::NotCurrent)).into();
                    run(&root, tx, &recovery, &node, work).await
                })
            })
            .await??
        } else {
            run(self, None, &recovery, node, work).await?
        };
        snapshot(
            outcome
                .get("prior")
                .or_else(|| outcome.get("outcome"))
                .cloned()
                .ok_or(HouseSaleError::Rejected)?,
        )
    }
}
fn snapshot(value: Value) -> Result<HouseSaleSnapshot> {
    let result: HouseSaleSnapshot =
        serde_json::from_value(value).map_err(|_| DurabilityError::InvalidStoredState)?;
    if result.now_unix_ms <= 0
        || result.rows.len() > 20
        || result.rows.windows(2).any(|w| w[0].house >= w[1].house)
        || result.rows.iter().any(|r| {
            r.revision == 0
                || r.price
                    .checked_add(r.rent)
                    .is_none_or(|v| v > 999_999_999_999)
                || r.transfer_state > 2
                || r.move_out_state > 4
                || r.allowed & !0x7e != 0
                || r.due_at_unix_ms < 0
        })
    {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    Ok(result)
}
async fn run(
    root: &DurabilityRoot,
    gate_tx: Option<&mut PremiumGateTx<'_>>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    work: Work,
) -> Result<Value> {
    if let Some(tx) = gate_tx {
        let mut savepoint = tx.begin().await?;
        let result = execute(&mut savepoint, recovery, node, work).await?;
        savepoint.commit().await?;
        return Ok(result);
    }
    let recovery = recovery.clone();
    let node = node.clone();
    root.try_issue_semantic_pass()?
        .run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                let result = execute(&mut tx, &recovery, &node, work).await;
                if result.is_ok() {
                    commit_semantic_transaction(tx, deadline).await?;
                }
                Ok(result)
            })
        })
        .await?
}
async fn execute(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    work: Work,
) -> Result<Value> {
    assert_recovery_fence(tx, recovery).await?;
    lock_admission_relations(tx).await?;
    let value = port(tx, node, work.mode, &work.p).await?;
    if value.is_null() {
        return Err(HouseSaleError::Rejected);
    }
    if value.get("conflict").is_some() {
        return Err(HouseSaleError::Conflict);
    }
    if value.get("prior").is_none() {
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = work.fence.runtime_scope
        else {
            return Err(HouseSaleError::Rejected);
        };
        if !character_item_fence_is_current(
            tx,
            node,
            &work.fence,
            work.command,
            work.fence.character_id,
            *world_id.as_bytes(),
            *channel_id.as_bytes(),
        )
        .await?
        {
            return Err(HouseSaleError::Rejected);
        }
    }
    if value.get("new").and_then(Value::as_bool) == Some(true) {
        if let Some(operation) = value.get("disposition").and_then(Value::as_str) {
            super::house_disposition::write_disposition_audit(
                tx,
                node,
                work.fence.runtime_scope,
                work.fence.scope_ownership_generation.get(),
                operation,
            )
            .await?;
        }
        if let Some(event) = value.get("event").and_then(Value::as_str) {
            audit::write(tx, node, &work.p, event).await?;
        }
    }
    Ok(value)
}
async fn port(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    mode: &str,
    p: &Value,
) -> std::result::Result<Value, DurabilityError> {
    let result: Option<Value> = sqlx::query_scalar(
        "SELECT game_house_sale_port($1,$2,encode($3,'hex')::uuid,$4::text::numeric,$5)",
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
fn hex(id: &[u8; 16]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

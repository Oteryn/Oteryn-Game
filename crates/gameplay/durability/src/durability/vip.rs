//! Native VIP transaction owner. Recovery precedes immutable replay; fresh work additionally
//! proves the current session/lease/scope/node. All mutations run through installed definers.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{assert_session_fence, uuid_text};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::domain::premium::PremiumStatus;
use oteryn_protocol_oteryn::vip::{Operation, Request};
use serde_json::{Value, json};
use sqlx::{Acquire, Postgres, Row, Transaction};

type Result<T> = std::result::Result<T, DurabilityError>;

pub enum VipPrepared {
    Replayed(Value),
    Fresh(Box<VipWrite>),
}
/// An owned proposal, never an authority token: commit independently rechecks every fence.
pub struct VipWrite {
    pub account: [u8; 16],
    root: DurabilityRoot,
    recovery: CharacterRecoveryFenceV1,
    node: NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    command: u64,
    request: Value,
    operation: Operation,
}

fn canonical(request: &Request) -> Value {
    let text = match request.operation {
        Operation::Edit => {
            crate::domain::vip::metadata(&request.text, request.icon, &request.groups)
        }
        Operation::GroupAdd | Operation::GroupRename => {
            crate::domain::vip::group_name(&request.text)
        }
        _ => Some(request.text.clone()),
    };
    // An invalid shape still reaches the receipted REJECTED path and consumes its rate token.
    json!({"op":request.operation as u8,"number":request.number,"text":text.as_deref().unwrap_or(&request.text),
        "icon":request.icon,"notify":request.notify,"groups":request.groups,"valid":text.is_some() && request.number <= i64::MAX as u64})
}
fn rejected() -> Value {
    json!({"code":10,"revision":0,"number":0,"wait":0})
}
async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    command: u64,
    request: &Value,
) -> Result<Option<Value>> {
    let row = sqlx::query("SELECT v.character_id=encode($1,'hex')::uuid AND v.binding=sha256(convert_to($4::jsonb::text,'UTF8')) AS matches,v.result FROM game_vip_command_receipts v JOIN game_character_roots r ON r.account_id=v.account_id AND r.world_id=v.world_id WHERE r.character_id=encode($1,'hex')::uuid AND v.world_id=encode($5,'hex')::uuid AND v.game_session_id=encode($2,'hex')::uuid AND v.command_id=$3::text::numeric")
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.game_session_id.as_bytes().as_slice()).bind(command.to_string()).bind(request).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    row.map(|row| {
        Ok(if row.try_get::<bool, _>("matches")? {
            row.try_get("result")?
        } else {
            rejected()
        })
    })
    .transpose()
}
impl DurabilityRoot {
    /// Resolve current authority or replay a retained result without consulting Premium/live state.
    pub async fn prepare_vip(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: u64,
        request: Request,
    ) -> Result<VipPrepared> {
        if command == 0 {
            return Ok(VipPrepared::Replayed(rejected()));
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let root = self.clone();
        let node = node.clone();
        let operation = request.operation;
        let request = canonical(&request);
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    if operation != Operation::Query
                        && let Some(result) = replay(&mut tx, &fence, command, &request).await?
                    {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(VipPrepared::Replayed(result));
                    }
                    lock_admission_relations(&mut tx).await?;
                    let Ok(account) = assert_session_fence(&mut tx, &fence, &node).await? else {
                        return Ok(VipPrepared::Replayed(rejected()));
                    };
                    let account = uuid_text(&account)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(VipPrepared::Fresh(Box::new(VipWrite {
                        account,
                        root,
                        recovery,
                        node,
                        fence,
                        command,
                        request,
                        operation,
                    })))
                })
            })
            .await
    }
}
impl VipWrite {
    /// Commit in the Premium owner's physical transaction before delivery, or a fresh native
    /// semantic transaction for a delivered status. A savepoint rolls back every partial write.
    pub async fn commit(
        self,
        status: PremiumStatus<'_>,
        tx: Option<&mut Transaction<'_, Postgres>>,
    ) -> Result<Value> {
        let premium = !matches!(status, PremiumStatus::NotCurrent);
        if let Some(tx) = tx {
            if !matches!(status, PremiumStatus::NotActivated(_)) {
                return Err(DurabilityError::Unavailable);
            }
            let mut savepoint = tx.begin().await?;
            let result = self.execute(&mut savepoint, premium).await;
            match result {
                Ok(result) => {
                    savepoint.commit().await?;
                    Ok(result)
                }
                Err(error) => {
                    savepoint.rollback().await?;
                    Err(error)
                }
            }
        } else {
            if matches!(status, PremiumStatus::NotActivated(_)) {
                return Err(DurabilityError::Unavailable);
            }
            self.root
                .try_issue_semantic_pass()?
                .run(move |holder, deadline| {
                    Box::pin(async move {
                        let mut tx = begin_semantic_transaction(holder, deadline).await?;
                        let result = self.execute(&mut tx, premium).await?;
                        commit_semantic_transaction(tx, deadline).await?;
                        Ok(result)
                    })
                })
                .await
        }
    }
    async fn execute(&self, tx: &mut Transaction<'_, Postgres>, premium: bool) -> Result<Value> {
        assert_recovery_fence(tx, &self.recovery).await?;
        if self.operation != Operation::Query
            && let Some(result) = replay(tx, &self.fence, self.command, &self.request).await?
        {
            return Ok(result);
        }
        lock_admission_relations(tx).await?;
        if self.operation != Operation::Query
            && let Some(result) = replay(tx, &self.fence, self.command, &self.request).await?
        {
            return Ok(result);
        }
        if assert_session_fence(tx, &self.fence, &self.node)
            .await?
            .is_err()
        {
            return Ok(rejected());
        }
        let record = &self.recovery;
        let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let recovery = json!({"scope":record.authority_scope_id,"generation":record.recovery_generation.to_string(),"event":uuid(&record.recovery_event_id),
            "predecessor":record.predecessor_generation.to_string(),"digest":hex(&record.predecessor_digest),"issued":record.issued_at.to_string(),"issuer":record.issuer_identity});
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = self.fence.runtime_scope
        else {
            return Ok(rejected());
        };
        let fact = self.node.fact();
        let fence = json!({"actor":uuid(self.fence.character_id.as_bytes()),"session":uuid(self.fence.game_session_id.as_bytes()),"world":uuid(world_id.as_bytes()),"channel":uuid(channel_id.as_bytes()),
            "connection":self.fence.connection_generation.get().to_string(),"lease":self.fence.character_lease_generation.to_string(),"scope_generation":self.fence.scope_ownership_generation.get().to_string(),
            "node":uuid(fact.node_id().as_bytes()),"node_revision":fact.registration_revision().to_string()});
        let function = match self.operation {
            Operation::Add => "add",
            Operation::Remove => "remove",
            Operation::Edit => "edit",
            Operation::GroupAdd => "group_add",
            Operation::GroupRename => "group_rename",
            Operation::GroupRemove => "group_remove",
            Operation::Accept => "accept",
            Operation::Reject => "reject",
            Operation::Revoke => "revoke",
            Operation::Withdraw => "withdraw",
            Operation::Status => "status",
            Operation::Query => "query",
        };
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT game_vip_{function}($1,$2,$3::text::numeric,$4,$5,$6)"
        )))
        .bind(recovery)
        .bind(fence)
        .bind(self.command.to_string())
        .bind(&self.request)
        .bind(if premium { 100_i32 } else { 20 })
        .bind(if premium { 5_i32 } else { 0 })
        .fetch_one(&mut **tx)
        .await
        .map_err(Into::into)
    }
}
fn uuid(bytes: &[u8; 16]) -> String {
    let h: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &h[..8],
        &h[8..12],
        &h[12..16],
        &h[16..20],
        &h[20..]
    )
}

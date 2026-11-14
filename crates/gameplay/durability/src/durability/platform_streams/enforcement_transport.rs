//! Fenced cursor and result-delivery state; command decisions remain in the Part-A owner.
use super::super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::{DurabilityError, DurabilityRoot};
use super::enforcement_wire::EnforcementResult;
use sqlx::Row;

pub const OVERDUE_AFTER_MS: i64 = 300_000;

#[derive(Debug, Clone)]
pub struct Publication {
    pub operation: String,
    pub revision: u16,
    pub bytes: Vec<u8>,
}
#[derive(Debug, Clone, Copy)]
pub enum Delivery {
    Ack,
    Hold,
    Missing,
    Republish,
}
#[derive(Debug, Clone)]
pub enum Action {
    Cursor {
        reconcile: bool,
    },
    Advance {
        reconcile: bool,
        expected: Option<String>,
        next: Option<String>,
    },
    Stop {
        reconcile: bool,
    },
    Next,
    Mark {
        operation: String,
        revision: u16,
        delivery: Delivery,
    },
    Lookup {
        operation: String,
    },
    Overdue {
        now_ms: i64,
    },
}
#[derive(Debug, Clone)]
pub enum Reply {
    Cursor(Option<String>, bool),
    Done,
    Publication(Option<Publication>),
    Bytes(Option<Vec<u8>>),
    Alarms(usize),
}
impl DurabilityRoot {
    pub async fn enforcement_transport(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        action: Action,
    ) -> Result<Reply, DurabilityError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            let enabled:bool=sqlx::query_scalar("SELECT enabled AND supported_policy_revision IS NOT NULL FROM game_platform_enforcement_configuration WHERE singleton").fetch_one(&mut *tx).await?;
            if !enabled {return Err(DurabilityError::Unavailable);}
            let reply=match action {
                Action::Cursor{reconcile}=>{
                    let row=sqlx::query("SELECT stream_cursor,reconcile_cursor,stream_stopped,reconcile_stopped FROM game_platform_enforcement_cursor WHERE singleton").fetch_one(&mut *tx).await?;
                    Reply::Cursor(row.try_get(if reconcile {"reconcile_cursor"}else{"stream_cursor"})?,row.try_get(if reconcile {"reconcile_stopped"}else{"stream_stopped"})?)
                },
                Action::Advance{reconcile,expected,next}=>{
                    if let Some(next)=&next {
                        let next=next.strip_prefix("v1:").and_then(|n|n.parse::<i64>().ok()).filter(|n|*n>0).ok_or(DurabilityError::InvalidStoredState)?;
                        if expected.as_deref().and_then(|s|s.strip_prefix("v1:")).and_then(|n|n.parse::<i64>().ok()).is_some_and(|old|next<=old) {return Err(DurabilityError::InvalidStoredState);}
                    }
                    let updated=sqlx::query("UPDATE game_platform_enforcement_cursor SET stream_cursor=CASE WHEN NOT $1 THEN COALESCE($3,stream_cursor) ELSE stream_cursor END,reconcile_cursor=CASE WHEN $1 THEN $3 ELSE reconcile_cursor END WHERE singleton AND CASE WHEN $1 THEN reconcile_cursor ELSE stream_cursor END IS NOT DISTINCT FROM $2")
                        .bind(reconcile).bind(expected).bind(next).execute(&mut *tx).await?;
                    if updated.rows_affected()!=1 {return Err(DurabilityError::Unavailable);} Reply::Done
                },
                Action::Stop{reconcile}=>{
                    sqlx::query("UPDATE game_platform_enforcement_cursor SET stream_stopped=stream_stopped OR NOT $1,reconcile_stopped=reconcile_stopped OR $1 WHERE singleton").bind(reconcile).execute(&mut *tx).await?;Reply::Done
                },
                Action::Next=>{
                    let row=sqlx::query("SELECT o.operation_id::text,o.receipt_revision,o.result_bytes FROM game_platform_enforcement_result_outbox o JOIN game_platform_enforcement_receipts r USING(operation_id) WHERE NOT acknowledged AND NOT held AND NOT reconcile_required AND NOT r.publication_stopped ORDER BY delivery_position LIMIT 1").fetch_optional(&mut *tx).await?;
                    Reply::Publication(row.map(|row|Ok::<_,sqlx::Error>(Publication{operation:row.try_get("operation_id")?,revision:u16::try_from(row.try_get::<i32,_>("receipt_revision")?).map_err(|_|sqlx::Error::Protocol("receipt revision".into()))?,bytes:row.try_get("result_bytes")?})).transpose()?)
                },
                Action::Mark{operation,revision,delivery}=>{
                    if matches!(delivery,Delivery::Republish) {
                        sqlx::query("INSERT INTO game_platform_enforcement_result_outbox(operation_id,command_digest,receipt_revision,result_bytes) SELECT operation_id,command_digest,receipt_revision,result_bytes FROM game_platform_enforcement_receipts WHERE operation_id=$1::uuid AND receipt_revision=$2 ON CONFLICT(operation_id,receipt_revision) DO UPDATE SET acknowledged=false WHERE NOT game_platform_enforcement_result_outbox.held AND NOT game_platform_enforcement_result_outbox.reconcile_required").bind(&operation).bind(i32::from(revision)).execute(&mut *tx).await?;
                    } else {
                        if matches!(delivery,Delivery::Missing) {
                            sqlx::query("UPDATE game_platform_enforcement_receipts SET publication_stopped=true WHERE operation_id=$1::uuid").bind(&operation).execute(&mut *tx).await?;
                        }
                        let updated=sqlx::query("UPDATE game_platform_enforcement_result_outbox SET acknowledged=$3,held=held OR $4,reconcile_required=reconcile_required OR $5 WHERE operation_id=$1::uuid AND receipt_revision=$2")
                            .bind(operation).bind(i32::from(revision)).bind(matches!(delivery,Delivery::Ack)).bind(matches!(delivery,Delivery::Hold)).bind(matches!(delivery,Delivery::Missing)).execute(&mut *tx).await?;
                        if updated.rows_affected()!=1 {return Err(DurabilityError::InvalidStoredState);}
                    }
                    Reply::Done
                },
                Action::Lookup{operation}=>Reply::Bytes(sqlx::query_scalar("SELECT result_bytes FROM game_platform_enforcement_receipts WHERE operation_id=$1::uuid").bind(operation).fetch_optional(&mut *tx).await?),
                Action::Overdue{now_ms}=>{
                    if now_ms<=0 {return Err(DurabilityError::Unavailable);}
                    let rows=sqlx::query("SELECT o.operation_id::text,o.receipt_revision,o.result_bytes FROM game_platform_enforcement_result_outbox o JOIN game_platform_enforcement_receipts r USING(operation_id) WHERE NOT acknowledged AND NOT r.overdue_alarmed ORDER BY delivery_position LIMIT 500 FOR UPDATE OF r").fetch_all(&mut *tx).await?;
                    let mut alarms=0;
                    for row in rows {
                        let bytes:Vec<u8>=row.try_get("result_bytes")?;
                        let result=EnforcementResult::parse(&bytes).ok_or(DurabilityError::InvalidStoredState)?;
                        if now_ms.saturating_sub(result.decided_at_ms)>=OVERDUE_AFTER_MS {
                            let updated=sqlx::query("UPDATE game_platform_enforcement_receipts SET overdue_alarmed=true WHERE operation_id=$1::uuid AND NOT overdue_alarmed").bind(row.try_get::<String,_>("operation_id")?).execute(&mut *tx).await?;
                            alarms+=usize::try_from(updated.rows_affected()).map_err(|_|DurabilityError::InvalidStoredState)?;
                        }
                    }
                    Reply::Alarms(alarms)
                },
            };
            commit_semantic_transaction(tx,deadline).await?;Ok(reply)
        })).await
    }
}

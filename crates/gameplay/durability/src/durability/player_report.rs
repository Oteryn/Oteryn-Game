//! PARITY-REPORT-1 native owner of the append-only player report queue. Recovery precedes
//! the immutable receipt replay; fresh work additionally proves the current session, lease,
//! scope and node, and the installed definer rechecks every fence in the same transaction.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::assert_session_fence;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use oteryn_protocol_oteryn::player_report::{
    PlayerReportCode, PlayerReportReply, PlayerReportRequest, validate_player_report,
};
use serde_json::{Value, json};
use sqlx::{Postgres, Transaction};

type Result<T> = std::result::Result<T, DurabilityError>;

/// The reporter's server-observed position; never a client value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerReportPosition {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

/// The receipt binding covers this client request alone, so a lost answer replays exactly.
fn canonical(request: &PlayerReportRequest, position: PlayerReportPosition) -> Value {
    json!({
        "client": {"kind":request.kind,"category":request.category,"reason":request.reason,
            "action":request.action,"target":request.target,"comment":request.comment,
            "map_x":request.map_x,"map_y":request.map_y,"map_floor":request.map_floor},
        "position": {"x":position.x,"y":position.y,"floor":position.floor},
        // An invalid shape still reaches the receipted INVALID path and spends its rate token.
        "valid": validate_player_report(request).is_ok(),
    })
}

fn reply(result: &Value) -> Result<PlayerReportReply> {
    let field = |name| {
        result
            .get(name)
            .and_then(Value::as_u64)
            .ok_or(DurabilityError::Unavailable)
    };
    Ok(PlayerReportReply {
        code: u8::try_from(field("code")?)
            .ok()
            .and_then(PlayerReportCode::from_u8)
            .ok_or(DurabilityError::Unavailable)?,
        report: field("report")?,
        wait_seconds: u32::try_from(field("wait")?).map_err(|_| DurabilityError::Unavailable)?,
    })
}

async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    command: u64,
    request: &Value,
) -> Result<Option<Value>> {
    let row: Option<(bool, Value)> = sqlx::query_as("SELECT character_id=encode($1,'hex')::uuid AND binding=sha256(convert_to(($4::jsonb->'client')::text,'UTF8')),result FROM game_player_report_receipts WHERE game_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric")
        .bind(fence.character_id.as_bytes().as_slice())
        .bind(fence.game_session_id.as_bytes().as_slice())
        .bind(command.to_string())
        .bind(request)
        .fetch_optional(&mut **tx)
        .await?;
    Ok(row.map(|(matches, result)| {
        if matches {
            result
        } else {
            json!({"code":PlayerReportCode::Invalid as u8,"report":0,"wait":0})
        }
    }))
}

impl DurabilityRoot {
    /// Append one report in one fenced transaction with its CommandRef receipt. `None` is a
    /// stale session/lease/scope/node fence: nothing is written and the command is refused.
    pub async fn submit_player_report(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        command: u64,
        request: &PlayerReportRequest,
        position: PlayerReportPosition,
    ) -> Result<Option<PlayerReportReply>> {
        if command == 0 {
            return Ok(None);
        }
        let record = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let crate::foundation::RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = fence.runtime_scope
        else {
            return Ok(None);
        };
        let node = node.clone();
        let request = canonical(request, position);
        let hex = |bytes: &[u8]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let recovery = json!({"scope":record.authority_scope_id,"generation":record.recovery_generation.to_string(),"event":uuid(&record.recovery_event_id),
            "predecessor":record.predecessor_generation.to_string(),"digest":hex(&record.predecessor_digest),"issued":record.issued_at.to_string(),"issuer":record.issuer_identity});
        let fact = node.fact();
        let fence_json = json!({"actor":uuid(fence.character_id.as_bytes()),"session":uuid(fence.game_session_id.as_bytes()),"world":uuid(world_id.as_bytes()),"channel":uuid(channel_id.as_bytes()),
            "connection":fence.connection_generation.get().to_string(),"lease":fence.character_lease_generation.to_string(),"scope_generation":fence.scope_ownership_generation.get().to_string(),
            "node":uuid(fact.node_id().as_bytes()),"node_revision":fact.registration_revision().to_string()});
        let result = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &record).await?;
                    if let Some(result) = replay(&mut tx, &fence, command, &request).await? {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Some(result));
                    }
                    lock_admission_relations(&mut tx).await?;
                    if assert_session_fence(&mut tx, &fence, &node).await?.is_err() {
                        return Ok(None);
                    }
                    let result: Value = sqlx::query_scalar(
                        "SELECT game_player_report_submit($1,$2,$3::text::numeric,$4)",
                    )
                    .bind(recovery)
                    .bind(fence_json)
                    .bind(command.to_string())
                    .bind(&request)
                    .fetch_one(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some(result))
                })
            })
            .await?;
        result.as_ref().map(reply).transpose()
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

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::player_report::{KIND_BUG, KIND_RULE_VIOLATION};

    fn request(kind: u8) -> PlayerReportRequest {
        PlayerReportRequest {
            kind,
            category: 0,
            reason: 0,
            action: 0,
            target: String::new(),
            comment: "x".into(),
            map_x: 0,
            map_y: 0,
            map_floor: 0,
        }
    }

    #[test]
    fn player_report_binding_excludes_server_position_and_marks_invalid_shapes() {
        let here = PlayerReportPosition {
            x: 1,
            y: 2,
            floor: 7,
        };
        let moved = PlayerReportPosition { x: 9, ..here };
        let (a, b) = (
            canonical(&request(KIND_BUG), here),
            canonical(&request(KIND_BUG), moved),
        );
        assert_eq!(a["client"], b["client"]);
        assert_ne!(a["position"], b["position"]);
        assert_eq!(a["valid"], json!(true));
        // A rule violation without a name and an unknown kind reach SQL as INVALID.
        assert_eq!(
            canonical(&request(KIND_RULE_VIOLATION), here)["valid"],
            json!(false)
        );
        assert_eq!(canonical(&request(9), here)["valid"], json!(false));
    }

    #[test]
    fn player_report_result_maps_only_the_typed_reply() {
        let accepted = reply(&json!({"code":1,"report":7,"wait":0})).ok();
        assert_eq!(
            accepted,
            Some(PlayerReportReply {
                code: PlayerReportCode::Accepted,
                report: 7,
                wait_seconds: 0
            })
        );
        assert!(reply(&json!({"code":6,"report":0,"wait":0})).is_err());
        assert!(reply(&json!({"code":1,"report":7})).is_err());
    }
}

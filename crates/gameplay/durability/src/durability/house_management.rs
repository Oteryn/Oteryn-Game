//! HOUSE-WIRE-1: one bounded, session-fenced transaction for remote management.
mod acl;
mod query;
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{assert_item_only_fence, numeric_u64};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::house_scope_handoff::HouseId;
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::CommandRef;
use oteryn_gameplay_housing::house_management::{HouseManagementAction, valid_move_out_date};
pub use query::HouseManagementSummary;
#[derive(Debug, Clone)]
pub struct QualifiedHouse {
    pub key: String,
    pub kind: &'static str,
}
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};
type Tx<'a> = Transaction<'a, Postgres>;

#[derive(Debug)]
pub enum HouseManagementError {
    Rejected,
    Conflict,
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for HouseManagementError {
    fn from(e: DurabilityError) -> Self {
        Self::Unavailable(e)
    }
}
impl From<sqlx::Error> for HouseManagementError {
    fn from(e: sqlx::Error) -> Self {
        Self::Unavailable(e.into())
    }
}
type Result<T> = std::result::Result<T, HouseManagementError>;
#[derive(Debug, Clone)]
pub struct HouseManagementRequest {
    pub house: HouseId,
    pub command: CommandRef,
    pub action: HouseManagementAction,
    /// The server's active pin, not a body field.
    pub content_digest: [u8; 32],
}
impl DurabilityRoot {
    pub async fn commit_house_management(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: HouseManagementRequest,
    ) -> Result<u64> {
        if request.command.game_session_id() != fence.game_session_id
            || request.house.world_id() != fence.runtime_scope.world_id()
            || request.content_digest == [0; 32]
        {
            return Err(HouseManagementError::Rejected);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseManagementError::Rejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    // A historical receipt is observable only under this admitted current session.
                    // Replacing a transport can never let the old generation perform a fresh write.
                    if assert_item_only_fence(&mut tx, &fence, &node)
                        .await?
                        .is_err()
                    {
                        return Ok(Err(HouseManagementError::Rejected));
                    }
                    let outcome = apply(&mut tx, &fence, &request).await;
                    if outcome.is_ok() {
                        commit_semantic_transaction(tx, deadline).await?;
                    }
                    Ok(outcome)
                })
            })
            .await?
    }
}
fn binding(
    fence: &CurrentCharacterItemFence,
    request: &HouseManagementRequest,
) -> Result<[u8; 32]> {
    let mut hash = Sha256::new();
    hash.update(b"oteryn:house-management:v1");
    hash.update(fence.character_id.as_bytes());
    hash.update(request.house.world_id().as_bytes());
    hash.update(request.house.house_key().as_bytes());
    match &request.action {
        HouseManagementAction::ReplaceAcl {
            list,
            expected_revision,
            members,
        } => {
            hash.update([1]);
            hash.update(list.key().ok_or(HouseManagementError::Rejected)?.as_bytes());
            hash.update(expected_revision.to_be_bytes());
            for member in members {
                hash.update(member);
            }
        }
        HouseManagementAction::MoveOut { due_at_unix_ms } => {
            hash.update([2]);
            hash.update(due_at_unix_ms.to_be_bytes());
        }
        HouseManagementAction::CancelMoveOut => hash.update([3]),
    }
    Ok(hash.finalize().into())
}
async fn apply(
    tx: &mut Tx<'_>,
    fence: &CurrentCharacterItemFence,
    request: &HouseManagementRequest,
) -> Result<u64> {
    let session = fence.game_session_id.as_bytes().as_slice();
    let command = request.command.command_id().get().to_string();
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex')||$2,168))")
        .bind(session)
        .bind(&command)
        .execute(&mut **tx)
        .await?;
    let hash = binding(fence, request)?;
    if let Some(row) = sqlx::query("SELECT request_binding,result_revision::text FROM game_house_management_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(session).bind(&command).fetch_optional(&mut **tx).await? {
        return if row.try_get::<Vec<u8>,_>("request_binding")? == hash {
            Ok(numeric_u64(&row,"result_revision")?)
        } else { Err(HouseManagementError::Conflict) };
    }
    let world = request.house.world_id();
    let house = request.house.house_key();
    // Property before ACL/notice, consistent with auction/settlement and ACL revision trigger.
    let property = sqlx::query("SELECT state,content_digest,ownership_generation::text,acl_revision::text FROM game_house_property_lock(encode($1,'hex')::uuid,$2)")
        .bind(world.as_bytes().as_slice()).bind(house).fetch_optional(&mut **tx).await?
        .ok_or(HouseManagementError::Rejected)?;
    if ![3_i16, 4].contains(&property.try_get("state")?)
        || property.try_get::<Vec<u8>, _>("content_digest")? != request.content_digest
    {
        return Err(HouseManagementError::Rejected);
    }
    let ownership = numeric_u64(&property, "ownership_generation")?;
    let owner: Option<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(owner_character_id) FROM game_house_ownership WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND ownership_revision=$3::text::numeric FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(house).bind(ownership.to_string()).fetch_optional(&mut **tx).await?;
    let owner = owner.ok_or(HouseManagementError::Rejected)?;
    let revision = match &request.action {
        HouseManagementAction::ReplaceAcl {
            list,
            expected_revision,
            members,
        } => {
            acl::replace(
                tx,
                fence,
                request,
                ownership,
                owner == fence.character_id.as_bytes(),
                numeric_u64(&property, "acl_revision")?,
                *list,
                *expected_revision,
                members,
            )
            .await?
        }
        HouseManagementAction::MoveOut { .. } | HouseManagementAction::CancelMoveOut => {
            if owner != fence.character_id.as_bytes() {
                return Err(HouseManagementError::Rejected);
            }
            notice(tx, request, ownership).await?
        }
    };
    let (kind, list_key) = match &request.action {
        HouseManagementAction::ReplaceAcl { list, .. } => (1_i16, list.key()),
        HouseManagementAction::MoveOut { .. } => (2, None),
        HouseManagementAction::CancelMoveOut => (3, None),
    };
    sqlx::query("INSERT INTO game_house_management_receipts(game_session_id,command_id,character_id,world_id,house_key,request_binding,result_revision,action_kind,list_key) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7::text::numeric,$8,$9)")
        .bind(session).bind(&command).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(house).bind(hash.as_slice()).bind(revision.to_string()).bind(kind).bind(list_key).execute(&mut **tx).await?;
    if kind == 1 {
        sqlx::query(
            "SELECT game_house_management_acl_advance(encode($1,'hex')::uuid,$2::text::numeric)",
        )
        .bind(session)
        .bind(&command)
        .execute(&mut **tx)
        .await?;
    }
    Ok(revision)
}
async fn notice(tx: &mut Tx<'_>, request: &HouseManagementRequest, ownership: u64) -> Result<u64> {
    let world = request.house.world_id();
    let house = request.house.house_key();
    let due = match request.action {
        HouseManagementAction::MoveOut { due_at_unix_ms } => {
            let now: i64 = sqlx::query_scalar(
                "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
            )
            .fetch_one(&mut **tx)
            .await?;
            if !u64::try_from(now)
                .ok()
                .is_some_and(|now| valid_move_out_date(now, due_at_unix_ms))
            {
                return Err(HouseManagementError::Rejected);
            }
            Some(i64::try_from(due_at_unix_ms).map_err(|_| HouseManagementError::Rejected)?)
        }
        HouseManagementAction::CancelMoveOut => None,
        _ => return Err(HouseManagementError::Rejected),
    };
    let previous = sqlx::query("SELECT revision::text,ownership_revision::text,due_at_unix_ms FROM game_house_move_out_notices WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(house).fetch_optional(&mut **tx).await?;
    if matches!(request.action, HouseManagementAction::CancelMoveOut) {
        let pending = if let Some(row) = previous.as_ref() {
            numeric_u64(row, "ownership_revision")? == ownership
                && row.try_get::<Option<i64>, _>("due_at_unix_ms")?.is_some()
        } else {
            false
        };
        if !pending {
            return Err(HouseManagementError::Rejected);
        }
    }
    let revision = previous
        .as_ref()
        .map(|row| numeric_u64(row, "revision"))
        .transpose()?
        .unwrap_or(0)
        .checked_add(1)
        .ok_or(HouseManagementError::Conflict)?;
    // FK is deferred until the corresponding immutable management receipt is inserted below.
    sqlx::query("INSERT INTO game_house_move_out_notices(world_id,house_key,ownership_revision,revision,due_at_unix_ms,last_session_id,last_command_id) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,$4::text::numeric,$5,encode($6,'hex')::uuid,$7::text::numeric) ON CONFLICT(world_id,house_key) DO UPDATE SET ownership_revision=EXCLUDED.ownership_revision,revision=EXCLUDED.revision,due_at_unix_ms=EXCLUDED.due_at_unix_ms,last_session_id=EXCLUDED.last_session_id,last_command_id=EXCLUDED.last_command_id")
        .bind(world.as_bytes().as_slice()).bind(house).bind(ownership.to_string()).bind(revision.to_string()).bind(due).bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).execute(&mut **tx).await?;
    Ok(revision)
}

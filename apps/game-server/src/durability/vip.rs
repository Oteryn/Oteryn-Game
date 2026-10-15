//! VIP0 §§4–6: private, receipted consent-ending storage preparation.
//!
//! Uses native recovery/session/scope/node checks unchanged. This unregistered module is
//! qualified with an isolated schema extension; production migration, SECURITY DEFINER
//! functions/grants, relay and handles remain CP work. No runtime role is granted direct DML.
//! No CharacterRevision, item, consent grant or presence state is written.
#![allow(
    dead_code,
    reason = "VIP production module/definer registration awaits the CP slot"
)]

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence, uuid_text};
use super::character_progression::assert_session_fence;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::domain::vip::{
    ConsentEnd, ConsentState, VipRateBucket, can_end_consent, charge_vip_bucket,
};
use crate::domain::{AccountId, CharacterId, WorldId};
use crate::foundation::{CommandRef, RuntimeScopeRefV1};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};
use std::collections::BTreeSet;

type Result<T> = std::result::Result<T, VipError>;

#[derive(Debug)]
pub(crate) enum VipError {
    AuthorityRejected,
    ConflictingCommand,
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for VipError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}
impl From<sqlx::Error> for VipError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}
fn corrupt() -> VipError {
    VipError::Unavailable(DurabilityError::InvalidStoredState)
}

/// Internal Account+World key, never a wire identity or authority capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VipListKey {
    pub(crate) account_id: AccountId,
    pub(crate) world_id: WorldId,
}

/// Future per-session handle resolution supplies this exact retained invitation binding.
/// It is not a new public ContactInviteId, nor evidence of consent/current authority.
#[derive(Debug, Clone, Copy)]
pub(crate) struct VipInvitationBinding {
    pub(crate) watcher_account: AccountId,
    pub(crate) inviter: CharacterId,
    pub(crate) invitation_id: u64,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum VipEndOperation {
    Withdraw { entry_no: u64 },
    Reject(VipInvitationBinding),
    Revoke(VipInvitationBinding),
}
impl VipEndOperation {
    fn kind(self) -> ConsentEnd {
        match self {
            Self::Withdraw { .. } => ConsentEnd::Withdraw,
            Self::Reject(_) => ConsentEnd::Reject,
            Self::Revoke(_) => ConsentEnd::Revoke,
        }
    }
    fn code(self) -> i16 {
        match self {
            Self::Withdraw { .. } => 1,
            Self::Reject(_) => 2,
            Self::Revoke(_) => 3,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct VipEndRequest {
    pub(crate) list: VipListKey,
    pub(crate) command: CommandRef,
    pub(crate) operation: VipEndOperation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VipResult {
    Ok,
    NotFound,
    Exhausted { seconds: u8 },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VipReceipt {
    pub(crate) result: VipResult,
    /// The acting Account's receipt/rate-list revision, also retained for refusals.
    pub(crate) revision: u64,
    pub(crate) replayed: bool,
}

fn digest(actor: CharacterId, request: VipEndRequest) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"VIP0-END-CONSENT/v1");
    hash.update(actor.as_bytes());
    hash.update(request.list.account_id.as_bytes());
    hash.update(request.list.world_id.as_bytes());
    hash.update(request.operation.code().to_be_bytes());
    match request.operation {
        VipEndOperation::Withdraw { entry_no } => hash.update(entry_no.to_be_bytes()),
        VipEndOperation::Reject(key) | VipEndOperation::Revoke(key) => {
            hash.update(key.watcher_account.as_bytes());
            hash.update(key.inviter.as_bytes());
            hash.update(key.invitation_id.to_be_bytes());
        }
    }
    hash.finalize().into()
}

impl DurabilityRoot {
    /// A real semantic transaction when composed into the native owner. The root capability
    /// supplies recovery provenance; immutable requests never manufacture a current fence.
    pub(crate) async fn end_vip_consent(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: VipEndRequest,
    ) -> Result<VipReceipt> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| VipError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    match end_in_transaction(&mut tx, &recovery, &node, fence, request).await {
                        Ok(receipt) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(receipt))
                        }
                        Err(VipError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

async fn receipt(
    tx: &mut Transaction<'_, Postgres>,
    actor: CharacterId,
    request: VipEndRequest,
) -> Result<Option<VipReceipt>> {
    let row = sqlx::query("SELECT character_id::text,kind,binding,result,seconds,revision::text FROM game_vip_command_receipts WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND game_session_id=encode($3,'hex')::uuid AND command_id=$4::text::numeric")
        .bind(request.list.account_id.as_bytes().as_slice()).bind(request.list.world_id.as_bytes().as_slice())
        .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    if uuid_text(row.try_get("character_id")?)? != *actor.as_bytes()
        || row.try_get::<i16, _>("kind")? != request.operation.code()
        || row.try_get::<Vec<u8>, _>("binding")? != digest(actor, request)
    {
        return Err(VipError::ConflictingCommand);
    }
    let seconds: Option<i16> = row.try_get("seconds")?;
    let result = match (row.try_get::<i16, _>("result")?, seconds) {
        (1, None) => VipResult::Ok,
        (2, None) => VipResult::NotFound,
        (3, Some(value)) if (1..=3).contains(&value) => VipResult::Exhausted {
            seconds: u8::try_from(value).map_err(|_| corrupt())?,
        },
        _ => return Err(corrupt()),
    };
    let revision = row
        .try_get::<String, _>("revision")?
        .parse::<u64>()
        .map_err(|_| corrupt())?;
    if revision == 0 {
        return Err(corrupt());
    }
    Ok(Some(VipReceipt {
        result,
        revision,
        replayed: true,
    }))
}

#[derive(Clone, Copy)]
struct Relationship {
    list: VipListKey,
    inviter: CharacterId,
    target: CharacterId,
    invitation: Option<u64>,
}

async fn withdraw_target(
    tx: &mut Transaction<'_, Postgres>,
    list: VipListKey,
    entry_no: u64,
) -> Result<Option<CharacterId>> {
    let target: Option<String> = sqlx::query_scalar("SELECT character_id::text FROM game_vip_entries WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND entry_no=$3::text::numeric")
        .bind(list.account_id.as_bytes().as_slice()).bind(list.world_id.as_bytes().as_slice()).bind(entry_no.to_string()).fetch_optional(&mut **tx).await?;
    target
        .map(|value| CharacterId::from_bytes(uuid_text(&value)?).map_err(|_| corrupt()))
        .transpose()
}

/// Recovery first, historical receipt next, then current native authority. Root locks precede
/// all list locks; both roots and Account+World lists are ordered, including reciprocal revokes.
/// No caller-controlled consent/presence state is accepted.
async fn end_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    request: VipEndRequest,
) -> Result<VipReceipt> {
    assert_recovery_fence(tx, recovery).await?;
    if let Some(stored) = receipt(tx, fence.character_id, request).await? {
        return Ok(stored);
    }
    lock_admission_relations(tx).await?;
    if request.command.game_session_id() != fence.game_session_id {
        return Err(VipError::AuthorityRejected);
    }
    let account = assert_session_fence(tx, &fence, node)
        .await?
        .map_err(|_| VipError::AuthorityRejected)?;
    let RuntimeScopeRefV1::Channel { world_id, .. } = fence.runtime_scope else {
        return Err(VipError::AuthorityRejected);
    };
    if uuid_text(&account)? != *request.list.account_id.as_bytes()
        || world_id.as_bytes() != request.list.world_id.as_bytes()
    {
        return Err(VipError::AuthorityRejected);
    }
    let relation = match request.operation {
        VipEndOperation::Withdraw { entry_no } => withdraw_target(tx, request.list, entry_no)
            .await?
            .map(|target| Relationship {
                list: request.list,
                inviter: fence.character_id,
                target,
                invitation: None,
            }),
        VipEndOperation::Reject(key) | VipEndOperation::Revoke(key) => Some(Relationship {
            list: VipListKey {
                account_id: key.watcher_account,
                world_id: request.list.world_id,
            },
            inviter: key.inviter,
            target: fence.character_id,
            invitation: Some(key.invitation_id),
        }),
    };
    let mut roots = BTreeSet::from([*fence.character_id.as_bytes()]);
    if let Some(key) = relation {
        roots.insert(*key.inviter.as_bytes());
        roots.insert(*key.target.as_bytes());
    }
    let mut valid_relation = relation.is_some();
    for character in roots {
        let row = sqlx::query("SELECT account_id::text,world_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1 FOR UPDATE")
            .bind(character.as_slice()).fetch_optional(&mut **tx).await?;
        let Some(row) = row else {
            if character == *fence.character_id.as_bytes() {
                return Err(VipError::AuthorityRejected);
            }
            valid_relation = false;
            continue;
        };
        let owner = uuid_text(row.try_get("account_id")?)?;
        let world = uuid_text(row.try_get("world_id")?)?;
        if character == *fence.character_id.as_bytes()
            && (owner != *request.list.account_id.as_bytes()
                || world != *request.list.world_id.as_bytes())
        {
            return Err(VipError::AuthorityRejected);
        }
        if let Some(key) = relation
            && (world != *key.list.world_id.as_bytes()
                || (character == *key.inviter.as_bytes()
                    && owner != *key.list.account_id.as_bytes()))
        {
            valid_relation = false;
        }
    }
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let mut keys = vec![request.list];
    if let Some(key) = relation
        && key.list != request.list
    {
        keys.push(key.list);
    }
    keys.sort_by_key(|key| (*key.account_id.as_bytes(), *key.world_id.as_bytes()));
    let mut acting = None;
    let mut watcher_revision = None;
    for key in keys {
        if key == request.list {
            sqlx::query("INSERT INTO game_vip_lists(account_id,world_id,revision,next_entry_no,rate_tokens,rate_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,20,$3) ON CONFLICT(account_id,world_id) DO NOTHING")
                .bind(key.account_id.as_bytes().as_slice()).bind(key.world_id.as_bytes().as_slice()).bind(now).execute(&mut **tx).await?;
        }
        let row = sqlx::query("SELECT revision::text,rate_tokens,rate_at FROM game_vip_lists WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
            .bind(key.account_id.as_bytes().as_slice()).bind(key.world_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        let Some(row) = row else {
            valid_relation = false;
            continue;
        };
        let revision = row
            .try_get::<String, _>("revision")?
            .parse::<u64>()
            .map_err(|_| corrupt())?;
        if revision == 0 {
            return Err(corrupt());
        }
        if key == request.list {
            acting = Some((
                revision,
                VipRateBucket {
                    tokens: u8::try_from(row.try_get::<i16, _>("rate_tokens")?)
                        .map_err(|_| corrupt())?,
                    at_us: row.try_get("rate_at")?,
                },
            ));
        }
        if relation.is_some_and(|relation| relation.list == key) {
            watcher_revision = Some(revision);
        }
    }
    if let Some(stored) = receipt(tx, fence.character_id, request).await? {
        return Ok(stored);
    }
    if let VipEndOperation::Withdraw { entry_no } = request.operation {
        // The first read finds roots to lock, never authority. Entry numbers are never reused;
        // a concurrently appearing mapping is retried rather than touching an unlocked target.
        let after = withdraw_target(tx, request.list, entry_no).await?;
        if after != relation.map(|key| key.target) {
            if after.is_none() {
                valid_relation = false;
            } else {
                return Err(VipError::Unavailable(DurabilityError::Unavailable));
            }
        }
    }
    // Read time after every blocking list lock; an earlier timestamp must not make a
    // concurrent writer's freshly persisted bucket look like a future/corrupt clock.
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let (revision, bucket) = acting.ok_or_else(corrupt)?;
    let charge = charge_vip_bucket(bucket, now).ok_or_else(corrupt)?;
    let result = if let Some(seconds) = charge.exhausted_seconds {
        VipResult::Exhausted { seconds }
    } else if valid_relation {
        erase_consent(
            tx,
            relation.ok_or_else(corrupt)?,
            request.operation.kind(),
            now,
        )
        .await?
    } else {
        VipResult::NotFound
    };
    let revision = revision.checked_add(1).ok_or_else(corrupt)?;
    if result == VipResult::Ok {
        let key = relation.ok_or_else(corrupt)?;
        if key.list != request.list {
            let next = watcher_revision
                .ok_or_else(corrupt)?
                .checked_add(1)
                .ok_or_else(corrupt)?;
            sqlx::query("UPDATE game_vip_lists SET revision=$3::text::numeric WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
                .bind(key.list.account_id.as_bytes().as_slice()).bind(key.list.world_id.as_bytes().as_slice()).bind(next.to_string()).execute(&mut **tx).await?;
        }
        // Every existing consent was produced together with this target-side view revision.
        // Missing producer state is corrupt, never a reason to invent PARTY settings defaults.
        let changed = sqlx::query("UPDATE game_character_social_settings SET vip_consent_revision=vip_consent_revision+1 WHERE character_id=encode($1,'hex')::uuid")
            .bind(key.target.as_bytes().as_slice()).execute(&mut **tx).await?.rows_affected();
        if changed != 1 {
            return Err(corrupt());
        }
    }
    sqlx::query("UPDATE game_vip_lists SET revision=$3::text::numeric,rate_tokens=$4,rate_at=$5 WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(request.list.account_id.as_bytes().as_slice()).bind(request.list.world_id.as_bytes().as_slice()).bind(revision.to_string())
        .bind(i16::from(charge.bucket.tokens)).bind(charge.bucket.at_us).execute(&mut **tx).await?;
    let (code, seconds) = match result {
        VipResult::Ok => (1i16, None),
        VipResult::NotFound => (2, None),
        VipResult::Exhausted { seconds } => (3, Some(i16::from(seconds))),
    };
    sqlx::query("INSERT INTO game_vip_command_receipts(account_id,world_id,game_session_id,command_id,character_id,kind,binding,result,seconds,revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,$6,$7,$8,$9,$10::text::numeric)")
        .bind(request.list.account_id.as_bytes().as_slice()).bind(request.list.world_id.as_bytes().as_slice())
        .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .bind(fence.character_id.as_bytes().as_slice()).bind(request.operation.code()).bind(digest(fence.character_id,request).as_slice())
        .bind(code).bind(seconds).bind(revision.to_string()).execute(&mut **tx).await?;
    Ok(VipReceipt {
        result,
        revision,
        replayed: false,
    })
}

async fn erase_consent(
    tx: &mut Transaction<'_, Postgres>,
    key: Relationship,
    action: ConsentEnd,
    now: i64,
) -> Result<VipResult> {
    let row = sqlx::query("SELECT invitation_id::text,consent,floor(extract(epoch FROM invite_expires_at)*1000000)::bigint AS expires FROM game_vip_consents WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND inviter_character_id=encode($3,'hex')::uuid AND character_id=encode($4,'hex')::uuid FOR UPDATE")
        .bind(key.list.account_id.as_bytes().as_slice()).bind(key.list.world_id.as_bytes().as_slice()).bind(key.inviter.as_bytes().as_slice()).bind(key.target.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(VipResult::NotFound);
    };
    let invitation = row
        .try_get::<String, _>("invitation_id")?
        .parse::<u64>()
        .map_err(|_| corrupt())?;
    if key
        .invitation
        .is_some_and(|expected| expected != invitation)
    {
        return Ok(VipResult::NotFound);
    }
    let state = match row.try_get::<i16, _>("consent")? {
        1 => ConsentState::Pending {
            expires_at_us: row.try_get("expires")?,
        },
        2 => ConsentState::Accepted,
        _ => return Err(corrupt()),
    };
    if !can_end_consent(action, state, now) {
        return Ok(VipResult::NotFound);
    }
    let changed = sqlx::query("DELETE FROM game_vip_consents WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND inviter_character_id=encode($3,'hex')::uuid AND character_id=encode($4,'hex')::uuid AND invitation_id=$5::text::numeric")
        .bind(key.list.account_id.as_bytes().as_slice()).bind(key.list.world_id.as_bytes().as_slice()).bind(key.inviter.as_bytes().as_slice()).bind(key.target.as_bytes().as_slice()).bind(invitation.to_string()).execute(&mut **tx).await?.rows_affected();
    if changed != 1 {
        return Err(corrupt());
    }
    Ok(VipResult::Ok)
}

#[cfg(test)]
#[path = "vip_tests.rs"]
mod tests;

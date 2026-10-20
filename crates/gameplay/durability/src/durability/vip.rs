//! VIP0 §§4–6: private, receipted consent lifecycle storage preparation.
//!
//! Uses native recovery/session/scope/node checks unchanged. This unregistered module is
//! qualified with an isolated schema extension; production migration, SECURITY DEFINER
//! functions/grants, relay and handles remain CP work. No runtime role is granted direct DML.
//! Accept changes an existing pending consent only under the target's actual native fence.
//! No CharacterRevision, item or presence state is written.
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
    AcceptDecision, ConsentEnd, ConsentState, VipRateBucket, accept_consent, can_end_consent,
    charge_vip_bucket,
};
use crate::domain::{AccountId, CharacterId, WorldId};
use crate::foundation::{CommandRef, RuntimeScopeRefV1};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};
use std::collections::BTreeSet;

type Result<T> = std::result::Result<T, VipError>;

#[derive(Debug)]
pub enum VipError {
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
pub struct VipListKey {
    pub account_id: AccountId,
    pub world_id: WorldId,
}

/// Future per-session handle resolution supplies this exact retained invitation binding.
/// It is not a new public ContactInviteId, nor evidence of consent/current authority.
#[derive(Debug, Clone, Copy)]
pub struct VipInvitationBinding {
    pub watcher_account: AccountId,
    pub inviter: CharacterId,
    pub invitation_id: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum VipEndOperation {
    Withdraw {
        entry_no: u64,
    },
    Reject(VipInvitationBinding),
    Revoke(VipInvitationBinding),
    /// Private lifecycle extension; numeric code is fixture-local, not a wire registry ID.
    Accept(VipInvitationBinding),
    /// Remove the Account-shared entry and every attached relationship/membership.
    /// The numeric arm remains a private proposed SQL receipt kind, never a wire ID.
    Remove {
        entry_no: u64,
    },
}
impl VipEndOperation {
    fn kind(self) -> Option<ConsentEnd> {
        match self {
            Self::Withdraw { .. } => Some(ConsentEnd::Withdraw),
            Self::Reject(_) => Some(ConsentEnd::Reject),
            Self::Revoke(_) => Some(ConsentEnd::Revoke),
            Self::Accept(_) | Self::Remove { .. } => None,
        }
    }
    fn code(self) -> i16 {
        match self {
            Self::Withdraw { .. } => 1,
            Self::Reject(_) => 2,
            Self::Revoke(_) => 3,
            Self::Accept(_) => 4,
            Self::Remove { .. } => 6,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VipEndRequest {
    pub list: VipListKey,
    pub command: CommandRef,
    pub operation: VipEndOperation,
}

#[derive(Debug, Clone, Copy)]
pub struct VipAcceptRequest {
    /// The acting target's Account+World, independently compared with its current session.
    pub list: VipListKey,
    pub command: CommandRef,
    pub invitation: VipInvitationBinding,
}

#[derive(Debug, Clone, Copy)]
pub struct VipRemoveRequest {
    pub list: VipListKey,
    pub command: CommandRef,
    pub entry_no: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VipResult {
    Ok,
    NotFound,
    ListFull,
    Exhausted { seconds: u8 },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VipReceipt {
    pub result: VipResult,
    /// The acting Account's receipt/rate-list revision, also retained for refusals.
    pub revision: u64,
    pub replayed: bool,
}

fn digest(actor: CharacterId, request: VipEndRequest) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"VIP0-END-CONSENT/v1");
    hash.update(actor.as_bytes());
    hash.update(request.list.account_id.as_bytes());
    hash.update(request.list.world_id.as_bytes());
    hash.update(request.operation.code().to_be_bytes());
    match request.operation {
        VipEndOperation::Withdraw { entry_no } | VipEndOperation::Remove { entry_no } => {
            hash.update(entry_no.to_be_bytes());
        }
        VipEndOperation::Reject(key)
        | VipEndOperation::Revoke(key)
        | VipEndOperation::Accept(key) => {
            hash.update(key.watcher_account.as_bytes());
            hash.update(key.inviter.as_bytes());
            hash.update(key.invitation_id.to_be_bytes());
        }
    }
    hash.finalize().into()
}

impl DurabilityRoot {
    /// Remove one Account+World entry under the same native authority/receipt transaction
    /// as consent mutations. Removal requires no Premium benefit or target settings producer.
    pub async fn remove_vip_entry(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: VipRemoveRequest,
    ) -> Result<VipReceipt> {
        self.end_vip_consent(
            authority,
            node,
            fence,
            VipEndRequest {
                list: request.list,
                command: request.command,
                operation: VipEndOperation::Remove {
                    entry_no: request.entry_no,
                },
            },
        )
        .await
    }

    /// Accept exactly one retained invitation. Its target is always the independently
    /// current acting Character, never a Character supplied by a resolved handle.
    pub async fn accept_vip_invitation(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: VipAcceptRequest,
    ) -> Result<VipReceipt> {
        self.end_vip_consent(
            authority,
            node,
            fence,
            VipEndRequest {
                list: request.list,
                command: request.command,
                operation: VipEndOperation::Accept(request.invitation),
            },
        )
        .await
    }

    /// A real semantic transaction when composed into the native owner. The root capability
    /// supplies recovery provenance; immutable requests never manufacture a current fence.
    /// The original method name is retained for existing ending callers; Accept shares
    /// this one transaction spine through the dedicated typed entry point above.
    pub async fn end_vip_consent(
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

// One receipt binding validator for Add and the existing consent operations.
async fn bound_receipt(
    tx: &mut Transaction<'_, Postgres>,
    actor: CharacterId,
    list: VipListKey,
    command: CommandRef,
    kind: i16,
    binding: [u8; 32],
) -> Result<Option<sqlx::postgres::PgRow>> {
    let row = sqlx::query("SELECT character_id::text,kind,binding,result,seconds,revision::text,entry_no::text FROM game_vip_command_receipts WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND game_session_id=encode($3,'hex')::uuid AND command_id=$4::text::numeric")
        .bind(list.account_id.as_bytes().as_slice()).bind(list.world_id.as_bytes().as_slice())
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
        .fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    if uuid_text(row.try_get("character_id")?)? != *actor.as_bytes()
        || row.try_get::<i16, _>("kind")? != kind
        || row.try_get::<Vec<u8>, _>("binding")? != binding
    {
        return Err(VipError::ConflictingCommand);
    }
    Ok(Some(row))
}

async fn receipt(
    tx: &mut Transaction<'_, Postgres>,
    actor: CharacterId,
    request: VipEndRequest,
) -> Result<Option<VipReceipt>> {
    let Some(row) = bound_receipt(
        tx,
        actor,
        request.list,
        request.command,
        request.operation.code(),
        digest(actor, request),
    )
    .await?
    else {
        return Ok(None);
    };
    let seconds: Option<i16> = row.try_get("seconds")?;
    let result = match (row.try_get::<i16, _>("result")?, seconds) {
        (1, None) => VipResult::Ok,
        (2, None) => VipResult::NotFound,
        (4, None) => VipResult::ListFull,
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
pub async fn end_in_transaction(
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
        VipEndOperation::Withdraw { entry_no } | VipEndOperation::Remove { entry_no } => {
            withdraw_target(tx, request.list, entry_no)
                .await?
                .map(|target| Relationship {
                    list: request.list,
                    inviter: fence.character_id,
                    target,
                    invitation: None,
                })
        }
        VipEndOperation::Reject(key)
        | VipEndOperation::Revoke(key)
        | VipEndOperation::Accept(key) => Some(Relationship {
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
    if let VipEndOperation::Withdraw { entry_no } | VipEndOperation::Remove { entry_no } =
        request.operation
    {
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
    // The optional view target is a SQL-derived mutation effect, never caller authority.
    let (result, changed_view) = if let Some(seconds) = charge.exhausted_seconds {
        (VipResult::Exhausted { seconds }, None)
    } else if valid_relation {
        let key = relation.ok_or_else(corrupt)?;
        if let VipEndOperation::Remove { entry_no } = request.operation {
            (VipResult::Ok, remove_shared_entry(tx, key, entry_no).await?)
        } else {
            let result = change_consent(tx, key, request.operation, now).await?;
            (result, (result == VipResult::Ok).then_some(key.target))
        }
    } else {
        (VipResult::NotFound, None)
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
        if let Some(target) = changed_view {
            // Every existing consent was produced together with this target-side view.
            // PARTY0046 constrains its source_policy; never manufacture a missing baseline.
            let changed = sqlx::query("UPDATE game_character_social_settings SET vip_consent_revision=vip_consent_revision+1 WHERE character_id=encode($1,'hex')::uuid")
                .bind(target.as_bytes().as_slice()).execute(&mut **tx).await?.rows_affected();
            if changed != 1 {
                return Err(corrupt());
            }
        }
    }
    sqlx::query("UPDATE game_vip_lists SET revision=$3::text::numeric,rate_tokens=$4,rate_at=$5 WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(request.list.account_id.as_bytes().as_slice()).bind(request.list.world_id.as_bytes().as_slice()).bind(revision.to_string())
        .bind(i16::from(charge.bucket.tokens)).bind(charge.bucket.at_us).execute(&mut **tx).await?;
    let (code, seconds) = match result {
        VipResult::Ok => (1i16, None),
        VipResult::NotFound => (2, None),
        VipResult::ListFull => (4, None),
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

/// Called only after native actor/target roots and the owning list are locked and the
/// entry locator is revalidated. Cascades use the existing private schema, not a row loop.
async fn remove_shared_entry(
    tx: &mut Transaction<'_, Postgres>,
    key: Relationship,
    entry_no: u64,
) -> Result<Option<CharacterId>> {
    let had_consent: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_vip_consents WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid)")
        .bind(key.list.account_id.as_bytes().as_slice()).bind(key.list.world_id.as_bytes().as_slice())
        .bind(key.target.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let changed = sqlx::query("DELETE FROM game_vip_entries WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid AND entry_no=$4::text::numeric")
        .bind(key.list.account_id.as_bytes().as_slice()).bind(key.list.world_id.as_bytes().as_slice())
        .bind(key.target.as_bytes().as_slice()).bind(entry_no.to_string()).execute(&mut **tx).await?.rows_affected();
    if changed != 1 {
        return Err(corrupt());
    }
    // VIP0 §5.1 expressly covers Remove with a consent row, including retained expired
    // Pending. An unconfirmed entry with no consent changes no view and needs no settings.
    Ok(had_consent.then_some(key.target))
}

async fn change_consent(
    tx: &mut Transaction<'_, Postgres>,
    key: Relationship,
    operation: VipEndOperation,
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
    if matches!(operation, VipEndOperation::Accept(_)) {
        // Every invitation producer and consent mutation locks this same target root
        // before any list, so this count serializes accepts across ALL watcher lists.
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_vip_consents WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND consent=2")
            .bind(key.target.as_bytes().as_slice()).bind(key.list.world_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        match accept_consent(state, now, u64::try_from(count).map_err(|_| corrupt())?) {
            AcceptDecision::NotFound => return Ok(VipResult::NotFound),
            AcceptDecision::ListFull => return Ok(VipResult::ListFull),
            AcceptDecision::Accept => {}
        }
        let changed = sqlx::query("UPDATE game_vip_consents SET consent=2 WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND inviter_character_id=encode($3,'hex')::uuid AND character_id=encode($4,'hex')::uuid AND invitation_id=$5::text::numeric AND consent=1")
            .bind(key.list.account_id.as_bytes().as_slice()).bind(key.list.world_id.as_bytes().as_slice()).bind(key.inviter.as_bytes().as_slice()).bind(key.target.as_bytes().as_slice()).bind(invitation.to_string()).execute(&mut **tx).await?.rows_affected();
        if changed != 1 {
            return Err(corrupt());
        }
        return Ok(VipResult::Ok);
    }
    if !can_end_consent(operation.kind().ok_or_else(corrupt)?, state, now) {
        return Ok(VipResult::NotFound);
    }
    let changed = sqlx::query("DELETE FROM game_vip_consents WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND inviter_character_id=encode($3,'hex')::uuid AND character_id=encode($4,'hex')::uuid AND invitation_id=$5::text::numeric")
        .bind(key.list.account_id.as_bytes().as_slice()).bind(key.list.world_id.as_bytes().as_slice()).bind(key.inviter.as_bytes().as_slice()).bind(key.target.as_bytes().as_slice()).bind(invitation.to_string()).execute(&mut **tx).await?.rows_affected();
    if changed != 1 {
        return Err(corrupt());
    }
    Ok(VipResult::Ok)
}

use crate::domain::character_name::CharacterName;
use crate::domain::premium::PremiumStatus;
use crate::durability::premium_activation_latch::PremiumGateOwner;
use sqlx::Acquire;

/// Bounded parsed name DATA plus the digest of the exact original name. Invalid grammar is
/// retained only as a digest and is a receipted REJECTED command, not rate-free probing.
#[derive(Clone, Debug)]
pub struct VipAddRequest {
    pub list: VipListKey,
    pub command: CommandRef,
    name: Option<CharacterName>,
    name_digest: [u8; 32],
}
impl VipAddRequest {
    pub fn new(list: VipListKey, command: CommandRef, name: &str) -> Self {
        Self {
            list,
            command,
            name: CharacterName::parse(name).ok(),
            name_digest: Sha256::digest(name.as_bytes()).into(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VipAddResult {
    Ok,
    UnknownName,
    AlreadyListed,
    ListFull,
    Rejected,
    Exhausted { seconds: u8 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VipAddReceipt {
    pub result: VipAddResult,
    pub entry_no: Option<u64>,
    pub revision: u64,
    pub replayed: bool,
}
// Local private SQL proposal arm; not a protocol, capability, state domain or event ID.
const ADD_KIND: i16 = 5;
fn add_digest(actor: CharacterId, request: &VipAddRequest) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"VIP0-ADD/v1");
    hash.update(actor.as_bytes());
    hash.update(request.list.account_id.as_bytes());
    hash.update(request.list.world_id.as_bytes());
    hash.update(ADD_KIND.to_be_bytes());
    // Binds exact supplied bytes, including malformed input, without retaining unbounded text.
    // Never substitutes a later resolved target or a current name into a historical request.
    hash.update(request.name_digest);
    hash.finalize().into()
}
fn stored_u64(row: &sqlx::postgres::PgRow, name: &str) -> Result<u64> {
    row.try_get::<String, _>(name)?
        .parse()
        .map_err(|_| corrupt())
}
async fn add_receipt(
    tx: &mut Transaction<'_, Postgres>,
    actor: CharacterId,
    request: &VipAddRequest,
) -> Result<Option<VipAddReceipt>> {
    let Some(row) = bound_receipt(
        tx,
        actor,
        request.list,
        request.command,
        ADD_KIND,
        add_digest(actor, request),
    )
    .await?
    else {
        return Ok(None);
    };
    let seconds: Option<i16> = row.try_get("seconds")?;
    let result = match (row.try_get::<i16, _>("result")?, seconds) {
        (1, None) => VipAddResult::Ok,
        (4, None) => VipAddResult::ListFull,
        (5, None) => VipAddResult::UnknownName,
        (6, None) => VipAddResult::AlreadyListed,
        (7, None) => VipAddResult::Rejected,
        (3, Some(n)) if (1..=3).contains(&n) => VipAddResult::Exhausted {
            seconds: u8::try_from(n).map_err(|_| corrupt())?,
        },
        _ => return Err(corrupt()),
    };
    let entry_no: Option<String> = row.try_get("entry_no")?;
    let entry_no = entry_no
        .map(|value| value.parse::<u64>().map_err(|_| corrupt()))
        .transpose()?;
    if entry_no == Some(0)
        || matches!(result, VipAddResult::Ok | VipAddResult::AlreadyListed) != entry_no.is_some()
    {
        return Err(corrupt());
    }
    let revision = stored_u64(&row, "revision")?;
    if revision == 0 {
        return Err(corrupt());
    }
    Ok(Some(VipAddReceipt {
        result,
        entry_no,
        revision,
        replayed: true,
    }))
}

impl DurabilityRoot {
    /// Native Add, preparing private/uninstalled VIP storage. Premium is read by the actual
    /// closed gate internally, never supplied as a bool/status or rebuilt from stored evidence.
    /// No relay/listener/wire or production first-admission baseline is activated here.
    pub async fn add_vip(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        premium: &impl PremiumGateOwner,
        request: VipAddRequest,
    ) -> Result<VipAddReceipt> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| VipError::AuthorityRejected)?;
        // Historical replay is independent of current Premium, session, caps and rate. This
        // read holds no VIP list/root lock and cannot execute a fresh command.
        let replay_recovery = recovery.clone();
        let replay_request = request.clone();
        let replay = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let outcome: Result<Option<VipAddReceipt>> = async {
                        assert_recovery_fence(&mut tx, &replay_recovery).await?;
                        add_receipt(&mut tx, fence.character_id, &replay_request).await
                    }
                    .await;
                    tx.rollback().await?;
                    match outcome {
                        Err(VipError::Unavailable(error)) => Err(error),
                        other => Ok(other),
                    }
                })
            })
            .await??;
        if let Some(receipt) = replay {
            return Ok(receipt);
        }
        let root = self.clone();
        let node = node.clone();
        premium
            .with_premium_gate(
                *request.list.account_id.as_bytes(),
                move |status, supplied| {
                    Box::pin(async move {
                        match (status, supplied) {
                            (PremiumStatus::NotActivated(_held), Some(tx)) => {
                                // The gate already took its shared latch lock as statement one and
                                // commits callback output even on Err: all VIP writes need a savepoint.
                                let mut savepoint = tx.begin().await?;
                                let result = add_in_transaction(
                                    &mut savepoint,
                                    &recovery,
                                    &node,
                                    fence,
                                    &request,
                                    100,
                                )
                                .await;
                                match result {
                                    Ok(value) => {
                                        savepoint.commit().await?;
                                        Ok(value)
                                    }
                                    Err(error) => {
                                        savepoint.rollback().await?;
                                        Err(error)
                                    }
                                }
                            }
                            (PremiumStatus::Current, None) => {
                                root.add_delivered(recovery, node, fence, request, 100)
                                    .await
                            }
                            (PremiumStatus::NotCurrent, None) => {
                                root.add_delivered(recovery, node, fence, request, 20).await
                            }
                            _ => Err(VipError::AuthorityRejected),
                        }
                    })
                },
            )
            .await?
    }
    async fn add_delivered(
        &self,
        recovery: CharacterRecoveryFenceV1,
        node: NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: VipAddRequest,
        cap: u64,
    ) -> Result<VipAddReceipt> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    match add_in_transaction(&mut tx, &recovery, &node, fence, &request, cap).await
                    {
                        Ok(value) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(VipError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}

async fn add_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    request: &VipAddRequest,
    cap: u64,
) -> Result<VipAddReceipt> {
    assert_recovery_fence(tx, recovery).await?;
    if let Some(receipt) = add_receipt(tx, fence.character_id, request).await? {
        return Ok(receipt);
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
    // This first name lookup chooses locks, not identity authority. In particular it requires
    // no online session. The reservation PK bounds candidate lookup; it is not current-name
    // authority: both joined live-root facts and the later locked-root recheck are required.
    // Unknown/other-World names have the identical later refusal.
    let name_key = request.name.as_ref().map(CharacterName::comparison_key);
    let candidate: Option<String> = if let Some(key) = &name_key {
        sqlx::query_scalar("SELECT r.character_id::text FROM game_character_name_reservations n JOIN game_character_roots r ON r.character_id=n.character_id WHERE n.name_key=$2 AND r.name_key=n.name_key AND r.world_id=encode($1,'hex')::uuid AND r.lifecycle=1")
            .bind(request.list.world_id.as_bytes().as_slice()).bind(key)
            .fetch_optional(&mut **tx).await?
    } else {
        None
    };
    let candidate = candidate.map(|value| uuid_text(&value)).transpose()?;
    let mut roots = BTreeSet::from([*fence.character_id.as_bytes()]);
    if let Some(target) = candidate {
        roots.insert(target);
    }
    let mut target = None;
    for character in roots {
        let row = sqlx::query("SELECT account_id::text,world_id::text,name,name_key,lifecycle FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(character.as_slice()).fetch_optional(&mut **tx).await?;
        let Some(row) = row else {
            if character == *fence.character_id.as_bytes() {
                return Err(VipError::AuthorityRejected);
            }
            continue;
        };
        let owner = uuid_text(row.try_get("account_id")?)?;
        let world = uuid_text(row.try_get("world_id")?)?;
        let live = row.try_get::<i16, _>("lifecycle")? == 1;
        if character == *fence.character_id.as_bytes()
            && (!live
                || owner != *request.list.account_id.as_bytes()
                || world != *request.list.world_id.as_bytes())
        {
            return Err(VipError::AuthorityRejected);
        }
        if Some(character) == candidate
            && live
            && world == *request.list.world_id.as_bytes()
            && name_key.as_ref() == Some(&row.try_get::<String, _>("name_key")?)
        {
            let current =
                CharacterName::parse(&row.try_get::<String, _>("name")?).map_err(|_| corrupt())?;
            if Some(current.comparison_key()) != name_key {
                return Err(corrupt());
            }
            target = Some(character);
        }
    }
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    sqlx::query("INSERT INTO game_vip_lists(account_id,world_id,revision,next_entry_no,rate_tokens,rate_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,20,$3) ON CONFLICT DO NOTHING")
        .bind(request.list.account_id.as_bytes().as_slice()).bind(request.list.world_id.as_bytes().as_slice()).bind(now).execute(&mut **tx).await?;
    let list = sqlx::query("SELECT revision::text,next_entry_no::text,rate_tokens,rate_at FROM game_vip_lists WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(request.list.account_id.as_bytes().as_slice()).bind(request.list.world_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if let Some(receipt) = add_receipt(tx, fence.character_id, request).await? {
        return Ok(receipt);
    }
    // Lock waits cannot make an earlier sampled time reject a valid freshly written bucket.
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let revision = stored_u64(&list, "revision")?
        .checked_add(1)
        .ok_or_else(corrupt)?;
    let charge = charge_vip_bucket(
        VipRateBucket {
            tokens: u8::try_from(list.try_get::<i16, _>("rate_tokens")?).map_err(|_| corrupt())?,
            at_us: list.try_get("rate_at")?,
        },
        now,
    )
    .ok_or_else(corrupt)?;
    let (result, entry_no) = if let Some(seconds) = charge.exhausted_seconds {
        (VipAddResult::Exhausted { seconds }, None)
    } else if name_key.is_none() || target == Some(*fence.character_id.as_bytes()) {
        (VipAddResult::Rejected, None)
    } else if let Some(target) = target {
        add_target(
            tx,
            fence.character_id,
            request.list,
            target,
            &list,
            cap,
            now,
        )
        .await?
    } else {
        (VipAddResult::UnknownName, None)
    };
    sqlx::query("UPDATE game_vip_lists SET revision=$3::text::numeric,rate_tokens=$4,rate_at=$5 WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(request.list.account_id.as_bytes().as_slice()).bind(request.list.world_id.as_bytes().as_slice()).bind(revision.to_string()).bind(i16::from(charge.bucket.tokens)).bind(charge.bucket.at_us).execute(&mut **tx).await?;
    let (code, seconds) = match result {
        VipAddResult::Ok => (1_i16, None),
        VipAddResult::UnknownName => (5, None),
        VipAddResult::AlreadyListed => (6, None),
        VipAddResult::ListFull => (4, None),
        VipAddResult::Rejected => (7, None),
        VipAddResult::Exhausted { seconds } => (3, Some(i16::from(seconds))),
    };
    sqlx::query("INSERT INTO game_vip_command_receipts(account_id,world_id,game_session_id,command_id,character_id,kind,binding,result,seconds,revision,entry_no) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,$6,$7,$8,$9,$10::text::numeric,$11::text::numeric)")
        .bind(request.list.account_id.as_bytes().as_slice()).bind(request.list.world_id.as_bytes().as_slice())
        .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .bind(fence.character_id.as_bytes().as_slice()).bind(ADD_KIND).bind(add_digest(fence.character_id, request).as_slice())
        .bind(code).bind(seconds).bind(revision.to_string()).bind(entry_no.map(|value| value.to_string())).execute(&mut **tx).await?;
    Ok(VipAddReceipt {
        result,
        entry_no,
        revision,
        replayed: false,
    })
}

async fn add_target(
    tx: &mut Transaction<'_, Postgres>,
    actor: CharacterId,
    list: VipListKey,
    target: [u8; 16],
    locked_list: &sqlx::postgres::PgRow,
    cap: u64,
    now: i64,
) -> Result<(VipAddResult, Option<u64>)> {
    // PARTY's other required settings have no defaults. VIP cannot invent their provenance
    // for an offline target. Its genuine admission initializer remains a CP dependency.
    let settings = sqlx::query("SELECT vip_consent_revision::text,vip_status,party_invites,channel_visibility,source_policy FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid FOR SHARE")
        .bind(target.as_slice()).fetch_optional(&mut **tx).await?.ok_or_else(corrupt)?;
    if settings.try_get::<String, _>("source_policy")? != "PARTYPVP0-PARTIES-AND-PVP-V1"
        || ![1, 2].contains(&settings.try_get::<i16, _>("party_invites")?)
        || ![1, 2].contains(&settings.try_get::<i16, _>("channel_visibility")?)
        || ![1, 2].contains(&settings.try_get::<i16, _>("vip_status")?)
    {
        return Err(corrupt());
    }
    let old: Option<String> = sqlx::query_scalar("SELECT entry_no::text FROM game_vip_entries WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid")
        .bind(list.account_id.as_bytes().as_slice()).bind(list.world_id.as_bytes().as_slice()).bind(target.as_slice()).fetch_optional(&mut **tx).await?;
    let (entry_no, fresh) = match old {
        Some(value) => (value.parse::<u64>().map_err(|_| corrupt())?, false),
        None => {
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_vip_entries WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
                .bind(list.account_id.as_bytes().as_slice()).bind(list.world_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            if count < 0 {
                return Err(corrupt());
            }
            if count as u64 >= cap {
                return Ok((VipAddResult::ListFull, None));
            }
            (stored_u64(locked_list, "next_entry_no")?, true)
        }
    };
    if entry_no == 0 {
        return Err(corrupt());
    }
    let existing = sqlx::query("SELECT consent,invite_expires_at>to_timestamp($5::bigint::double precision/1000000) AS unexpired FROM game_vip_consents WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND inviter_character_id=encode($3,'hex')::uuid AND character_id=encode($4,'hex')::uuid")
        .bind(list.account_id.as_bytes().as_slice()).bind(list.world_id.as_bytes().as_slice()).bind(actor.as_bytes().as_slice()).bind(target.as_slice()).bind(now).fetch_optional(&mut **tx).await?;
    if let Some(row) = existing {
        match (
            row.try_get::<i16, _>("consent")?,
            row.try_get::<bool, _>("unexpired")?,
        ) {
            (2, _) | (1, true) => return Ok((VipAddResult::AlreadyListed, Some(entry_no))),
            (1, false) => {
                sqlx::query("DELETE FROM game_vip_consents WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND inviter_character_id=encode($3,'hex')::uuid AND character_id=encode($4,'hex')::uuid")
                    .bind(list.account_id.as_bytes().as_slice()).bind(list.world_id.as_bytes().as_slice()).bind(actor.as_bytes().as_slice()).bind(target.as_slice()).execute(&mut **tx).await?;
            }
            _ => return Err(corrupt()),
        }
    }
    if fresh {
        let next = entry_no.checked_add(1).ok_or_else(corrupt)?;
        sqlx::query("INSERT INTO game_vip_entries(account_id,world_id,character_id,entry_no) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric)")
            .bind(list.account_id.as_bytes().as_slice()).bind(list.world_id.as_bytes().as_slice()).bind(target.as_slice()).bind(entry_no.to_string()).execute(&mut **tx).await?;
        sqlx::query("UPDATE game_vip_lists SET next_entry_no=$3::text::numeric WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
            .bind(list.account_id.as_bytes().as_slice()).bind(list.world_id.as_bytes().as_slice()).bind(next.to_string()).execute(&mut **tx).await?;
    }
    let blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_social_blocks WHERE character_id=encode($1,'hex')::uuid AND blocked_character_id=encode($2,'hex')::uuid)")
        .bind(target.as_slice()).bind(actor.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let pending: i64 = sqlx::query_scalar("SELECT count(*) FROM game_vip_consents WHERE character_id=encode($1,'hex')::uuid AND consent=1 AND invite_expires_at>to_timestamp($2::bigint::double precision/1000000)")
        .bind(target.as_slice()).bind(now).fetch_one(&mut **tx).await?;
    if pending < 0 {
        return Err(corrupt());
    }
    if !blocked && pending < 20 {
        sqlx::query("INSERT INTO game_vip_consents(account_id,world_id,inviter_character_id,character_id,consent,invite_expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,to_timestamp($5::bigint::double precision/1000000)+interval '7days')")
            .bind(list.account_id.as_bytes().as_slice()).bind(list.world_id.as_bytes().as_slice()).bind(actor.as_bytes().as_slice()).bind(target.as_slice()).bind(now).execute(&mut **tx).await?;
        let revision = stored_u64(&settings, "vip_consent_revision")?
            .checked_add(1)
            .ok_or_else(corrupt)?;
        sqlx::query("UPDATE game_character_social_settings SET vip_consent_revision=$2::text::numeric WHERE character_id=encode($1,'hex')::uuid")
            .bind(target.as_slice()).bind(revision.to_string()).execute(&mut **tx).await?;
    }
    Ok((VipAddResult::Ok, Some(entry_no)))
}

/// List metadata operations. These values are private SQL proposals, not wire tags.
#[derive(Clone, Debug)]
pub enum VipListOperation {
    GroupAdd {
        name: String,
    },
    GroupRename {
        group_no: u8,
        name: String,
    },
    GroupRemove {
        group_no: u8,
    },
    Edit {
        entry_no: u64,
        description: String,
        icon: u8,
        notify: bool,
        groups: Vec<u8>,
    },
    Status {
        hidden: bool,
    },
}
impl VipListOperation {
    fn code(&self) -> i16 {
        match self {
            Self::GroupAdd { .. } => 7,
            Self::GroupRename { .. } => 8,
            Self::GroupRemove { .. } => 9,
            Self::Edit { .. } => 10,
            Self::Status { .. } => 11,
        }
    }
    fn needs_premium(&self) -> bool {
        matches!(self, Self::GroupAdd { .. } | Self::GroupRename { .. })
    }
}
#[derive(Clone, Debug)]
pub struct VipListRequest {
    pub list: VipListKey,
    pub command: CommandRef,
    pub operation: VipListOperation,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VipListResult {
    Ok,
    NotFound,
    Rejected,
    GroupLimit,
    NameTaken,
    Exhausted { seconds: u8 },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VipListReceipt {
    pub result: VipListResult,
    pub group_no: Option<u8>,
    pub revision: u64,
    pub replayed: bool,
}
fn list_digest(actor: CharacterId, request: &VipListRequest) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"VIP0-LIST/v1");
    hash.update(actor.as_bytes());
    hash.update(request.list.account_id.as_bytes());
    hash.update(request.list.world_id.as_bytes());
    hash.update(request.operation.code().to_be_bytes());
    // Explicit lengths bind exact request bytes, including invalid input, without ambiguity.
    let text = |hash: &mut Sha256, value: &str| {
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value.as_bytes());
    };
    match &request.operation {
        VipListOperation::GroupAdd { name } => text(&mut hash, name),
        VipListOperation::GroupRename { group_no, name } => {
            hash.update([*group_no]);
            text(&mut hash, name);
        }
        VipListOperation::GroupRemove { group_no } => hash.update([*group_no]),
        VipListOperation::Edit {
            entry_no,
            description,
            icon,
            notify,
            groups,
        } => {
            hash.update(entry_no.to_be_bytes());
            text(&mut hash, description);
            hash.update([*icon, u8::from(*notify)]);
            hash.update((groups.len() as u64).to_be_bytes());
            hash.update(groups);
        }
        VipListOperation::Status { hidden } => hash.update([u8::from(*hidden)]),
    }
    hash.finalize().into()
}
async fn list_receipt(
    tx: &mut Transaction<'_, Postgres>,
    actor: CharacterId,
    request: &VipListRequest,
) -> Result<Option<VipListReceipt>> {
    let Some(row) = bound_receipt(
        tx,
        actor,
        request.list,
        request.command,
        request.operation.code(),
        list_digest(actor, request),
    )
    .await?
    else {
        return Ok(None);
    };
    let result = match (
        row.try_get::<i16, _>("result")?,
        row.try_get::<Option<i16>, _>("seconds")?,
    ) {
        (1, None) => VipListResult::Ok,
        (2, None) => VipListResult::NotFound,
        (7, None) => VipListResult::Rejected,
        (8, None) => VipListResult::GroupLimit,
        (9, None) => VipListResult::NameTaken,
        (3, Some(seconds)) if (1..=3).contains(&seconds) => VipListResult::Exhausted {
            seconds: u8::try_from(seconds).map_err(|_| corrupt())?,
        },
        _ => return Err(corrupt()),
    };
    let group: Option<i16> = sqlx::query_scalar("SELECT group_no FROM game_vip_command_receipts WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND game_session_id=encode($3,'hex')::uuid AND command_id=$4::text::numeric")
        .bind(request.list.account_id.as_bytes().as_slice()).bind(request.list.world_id.as_bytes().as_slice())
        .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_one(&mut **tx).await?;
    let group_no = group
        .map(|n| u8::try_from(n).map_err(|_| corrupt()))
        .transpose()?;
    if group_no.is_some_and(|n| !(4..=8).contains(&n))
        || group_no.is_some()
            != (matches!(request.operation, VipListOperation::GroupAdd { .. })
                && result == VipListResult::Ok)
    {
        return Err(corrupt());
    }
    let revision = stored_u64(&row, "revision")?;
    if revision == 0 {
        return Err(corrupt());
    }
    Ok(Some(VipListReceipt {
        result,
        group_no,
        revision,
        replayed: true,
    }))
}
impl DurabilityRoot {
    /// Native metadata writer. Only growth/rename consult Premium. Removal, Edit and Status
    /// remain usable after expiry and while over cap. Relay publication follows commit at CP.
    pub async fn modify_vip_list(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        premium: &impl PremiumGateOwner,
        request: VipListRequest,
    ) -> Result<VipListReceipt> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| VipError::AuthorityRejected)?;
        let replay_recovery = recovery.clone();
        let replay_request = request.clone();
        let replay = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let outcome: Result<Option<VipListReceipt>> = async {
                        assert_recovery_fence(&mut tx, &replay_recovery).await?;
                        list_receipt(&mut tx, fence.character_id, &replay_request).await
                    }
                    .await;
                    tx.rollback().await?;
                    match outcome {
                        Err(VipError::Unavailable(error)) => Err(error),
                        other => Ok(other),
                    }
                })
            })
            .await??;
        if let Some(replay) = replay {
            return Ok(replay);
        }
        let node = node.clone();
        if !request.operation.needs_premium() {
            return self
                .modify_vip_delivered(recovery, node, fence, request, 0)
                .await;
        }
        let root = self.clone();
        premium
            .with_premium_gate(
                *request.list.account_id.as_bytes(),
                move |status, supplied| {
                    Box::pin(async move {
                        match (status, supplied) {
                            (PremiumStatus::NotActivated(_held), Some(tx)) => {
                                // The gate commits even an Err callback: a savepoint encloses all VIP writes.
                                let mut savepoint = tx.begin().await?;
                                let result = modify_list_in_transaction(
                                    &mut savepoint,
                                    &recovery,
                                    &node,
                                    fence,
                                    &request,
                                    5,
                                )
                                .await;
                                match result {
                                    Ok(value) => {
                                        savepoint.commit().await?;
                                        Ok(value)
                                    }
                                    Err(error) => {
                                        savepoint.rollback().await?;
                                        Err(error)
                                    }
                                }
                            }
                            (PremiumStatus::Current, None) => {
                                root.modify_vip_delivered(recovery, node, fence, request, 5)
                                    .await
                            }
                            (PremiumStatus::NotCurrent, None) => {
                                root.modify_vip_delivered(recovery, node, fence, request, 0)
                                    .await
                            }
                            _ => Err(VipError::AuthorityRejected),
                        }
                    })
                },
            )
            .await?
    }
    async fn modify_vip_delivered(
        &self,
        recovery: CharacterRecoveryFenceV1,
        node: NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: VipListRequest,
        group_cap: u8,
    ) -> Result<VipListReceipt> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    match modify_list_in_transaction(
                        &mut tx, &recovery, &node, fence, &request, group_cap,
                    )
                    .await
                    {
                        Ok(value) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(VipError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }
}
pub async fn modify_list_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    request: &VipListRequest,
    group_cap: u8,
) -> Result<VipListReceipt> {
    assert_recovery_fence(tx, recovery).await?;
    if let Some(replay) = list_receipt(tx, fence.character_id, request).await? {
        return Ok(replay);
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
    let actor = sqlx::query("SELECT account_id::text,world_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1 FOR UPDATE")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.ok_or(VipError::AuthorityRejected)?;
    if uuid_text(actor.try_get("account_id")?)? != *request.list.account_id.as_bytes()
        || uuid_text(actor.try_get("world_id")?)? != *request.list.world_id.as_bytes()
    {
        return Err(VipError::AuthorityRejected);
    }
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    sqlx::query("INSERT INTO game_vip_lists(account_id,world_id,revision,next_entry_no,rate_tokens,rate_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,20,$3) ON CONFLICT DO NOTHING")
        .bind(request.list.account_id.as_bytes().as_slice()).bind(request.list.world_id.as_bytes().as_slice()).bind(now).execute(&mut **tx).await?;
    let list = sqlx::query("SELECT revision::text,rate_tokens,rate_at FROM game_vip_lists WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(request.list.account_id.as_bytes().as_slice()).bind(request.list.world_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    if let Some(replay) = list_receipt(tx, fence.character_id, request).await? {
        return Ok(replay);
    }
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let revision = stored_u64(&list, "revision")?
        .checked_add(1)
        .ok_or_else(corrupt)?;
    let charge = charge_vip_bucket(
        VipRateBucket {
            tokens: u8::try_from(list.try_get::<i16, _>("rate_tokens")?).map_err(|_| corrupt())?,
            at_us: list.try_get("rate_at")?,
        },
        now,
    )
    .ok_or_else(corrupt)?;
    let (result, group_no) = if let Some(seconds) = charge.exhausted_seconds {
        (VipListResult::Exhausted { seconds }, None)
    } else {
        mutate_list(tx, fence.character_id, request, group_cap).await?
    };
    sqlx::query("UPDATE game_vip_lists SET revision=$3::text::numeric,rate_tokens=$4,rate_at=$5 WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(request.list.account_id.as_bytes().as_slice()).bind(request.list.world_id.as_bytes().as_slice()).bind(revision.to_string()).bind(i16::from(charge.bucket.tokens)).bind(charge.bucket.at_us).execute(&mut **tx).await?;
    let (code, seconds) = match result {
        VipListResult::Ok => (1_i16, None),
        VipListResult::NotFound => (2, None),
        VipListResult::Rejected => (7, None),
        VipListResult::GroupLimit => (8, None),
        VipListResult::NameTaken => (9, None),
        VipListResult::Exhausted { seconds } => (3, Some(i16::from(seconds))),
    };
    sqlx::query("INSERT INTO game_vip_command_receipts(account_id,world_id,game_session_id,command_id,character_id,kind,binding,result,seconds,revision,group_no) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::numeric,encode($5,'hex')::uuid,$6,$7,$8,$9,$10::text::numeric,$11)")
        .bind(request.list.account_id.as_bytes().as_slice()).bind(request.list.world_id.as_bytes().as_slice()).bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .bind(fence.character_id.as_bytes().as_slice()).bind(request.operation.code()).bind(list_digest(fence.character_id, request).as_slice()).bind(code).bind(seconds).bind(revision.to_string()).bind(group_no.map(i16::from)).execute(&mut **tx).await?;
    Ok(VipListReceipt {
        result,
        group_no,
        revision,
        replayed: false,
    })
}
async fn mutate_list(
    tx: &mut Transaction<'_, Postgres>,
    actor: CharacterId,
    request: &VipListRequest,
    group_cap: u8,
) -> Result<(VipListResult, Option<u8>)> {
    use crate::domain::vip::{vip_edit_description, vip_group_name};
    let account = request.list.account_id.as_bytes().as_slice();
    let world = request.list.world_id.as_bytes().as_slice();
    let outcome = match &request.operation {
        VipListOperation::GroupAdd { name } | VipListOperation::GroupRename { name, .. } => {
            let rename = match request.operation {
                VipListOperation::GroupRename { group_no, .. } => Some(group_no),
                _ => None,
            };
            if rename.is_some_and(|n| !(4..=8).contains(&n)) {
                return Ok((VipListResult::Rejected, None));
            }
            let Some((name, _)) = vip_group_name(name) else {
                return Ok((VipListResult::Rejected, None));
            };
            let rows = sqlx::query("SELECT group_no,name,lower(name)=lower($3) AS same_name FROM game_vip_groups WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid ORDER BY group_no")
                .bind(account).bind(world).bind(&name).fetch_all(&mut **tx).await?;
            let mut groups = BTreeSet::new();
            let mut taken = false;
            for row in &rows {
                let number =
                    u8::try_from(row.try_get::<i16, _>("group_no")?).map_err(|_| corrupt())?;
                let stored = row.try_get::<String, _>("name")?;
                vip_group_name(&stored).ok_or_else(corrupt)?;
                if !(4..=8).contains(&number) || !groups.insert(number) {
                    return Err(corrupt());
                }
                taken |= Some(number) != rename && row.try_get::<bool, _>("same_name")?;
            }
            if rename.is_some_and(|n| !groups.contains(&n)) {
                return Ok((VipListResult::NotFound, None));
            }
            if rows.len() >= usize::from(group_cap) {
                return Ok((VipListResult::GroupLimit, None));
            }
            if taken {
                return Ok((VipListResult::NameTaken, None));
            }
            if let Some(number) = rename {
                sqlx::query("UPDATE game_vip_groups SET name=$4 WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND group_no=$3")
                    .bind(account).bind(world).bind(i16::from(number)).bind(name).execute(&mut **tx).await?;
                (VipListResult::Ok, None)
            } else {
                let number = (4..=8).find(|n| !groups.contains(n)).ok_or_else(corrupt)?;
                sqlx::query("INSERT INTO game_vip_groups(account_id,world_id,group_no,name) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4)")
                    .bind(account).bind(world).bind(i16::from(number)).bind(name).execute(&mut **tx).await?;
                (VipListResult::Ok, Some(number))
            }
        }
        VipListOperation::GroupRemove { group_no } => {
            if !(4..=8).contains(group_no) {
                return Ok((VipListResult::Rejected, None));
            }
            let changed = sqlx::query("DELETE FROM game_vip_groups WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND group_no=$3")
                .bind(account).bind(world).bind(i16::from(*group_no)).execute(&mut **tx).await?.rows_affected();
            if changed == 0 {
                return Ok((VipListResult::NotFound, None));
            }
            // Fixed memberships (1–3) have no custom-group FK. Delete custom memberships
            // explicitly within this transaction; the production slot may additionally cascade.
            sqlx::query("DELETE FROM game_vip_group_members WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND group_no=$3")
                .bind(account).bind(world).bind(i16::from(*group_no)).execute(&mut **tx).await?;
            (VipListResult::Ok, None)
        }
        VipListOperation::Edit {
            entry_no,
            description,
            icon,
            notify,
            groups,
        } => {
            let Some(description) = vip_edit_description(description, *icon, groups) else {
                return Ok((VipListResult::Rejected, None));
            };
            let target: Option<String> = sqlx::query_scalar("SELECT character_id::text FROM game_vip_entries WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND entry_no=$3::text::numeric")
                .bind(account).bind(world).bind(entry_no.to_string()).fetch_optional(&mut **tx).await?;
            let Some(target) = target else {
                return Ok((VipListResult::NotFound, None));
            };
            let target = uuid_text(&target)?;
            for group in groups.iter().filter(|n| **n >= 4) {
                let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_vip_groups WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND group_no=$3)")
                    .bind(account).bind(world).bind(i16::from(*group)).fetch_one(&mut **tx).await?;
                if !exists {
                    return Ok((VipListResult::NotFound, None));
                }
            }
            sqlx::query("UPDATE game_vip_entries SET description=$4,icon=$5,notify=$6 WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid")
                .bind(account).bind(world).bind(target.as_slice()).bind(description).bind(i16::from(*icon)).bind(*notify).execute(&mut **tx).await?;
            sqlx::query("DELETE FROM game_vip_group_members WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid")
                .bind(account).bind(world).bind(target.as_slice()).execute(&mut **tx).await?;
            for group in groups {
                sqlx::query("INSERT INTO game_vip_group_members(account_id,world_id,character_id,group_no) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4)")
                    .bind(account).bind(world).bind(target.as_slice()).bind(i16::from(*group)).execute(&mut **tx).await?;
            }
            (VipListResult::Ok, None)
        }
        VipListOperation::Status { hidden } => {
            // The real PARTY admission producer owns all required baseline fields. VIP never
            // reconstructs that baseline; missing provenance rolls back the rate and receipt.
            let changed = sqlx::query("UPDATE game_character_social_settings SET vip_status=$2 WHERE character_id=encode($1,'hex')::uuid AND source_policy='PARTYPVP0-PARTIES-AND-PVP-V1'")
                .bind(actor.as_bytes().as_slice()).bind(if *hidden { 2_i16 } else { 1_i16 }).execute(&mut **tx).await?.rows_affected();
            if changed != 1 {
                return Err(corrupt());
            }
            (VipListResult::Ok, None)
        }
    };
    Ok(outcome)
}

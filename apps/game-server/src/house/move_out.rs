//! HOUSE-1b commands and bounded disposition over native ownership, handoff and custody.
//!
//! The real Durability transaction envelope invokes these canonical House operations.
//! Migration 0121 installs their retained producer and native custody proof through the
//! existing MIGRATOR, identically for production and native tests. D1384 owner commands
//! retain an exact absolute due time and replay the original CommandRef-bound outcome.
//! Registered HouseOperation events, production caller permissions, acquisition/property/
//! slot reconciliation, exit/quiesce, house content revision and BED integration remain
//! separate integration gates. DRAINED deliberately keeps the content fence and owner.

use crate::durability::house_scope_handoff::HouseId;
use crate::durability::item_transfer::{
    CurrentCharacterItemFence, character_item_fence_is_current,
};
use crate::durability::runtime_scope_assignment::{
    NodeIncarnationProof, prove_current_incarnation,
};
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::{CommandRef, RuntimeScopeRefV1};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

/// Accepted maximum per disposition step (HOUSE-OWN-0 §7).
pub const DISPOSITION_STEP_ITEMS: usize = 100;
/// Accepted house item bound, still PARITY_PENDING in the owning decision.
pub const HOUSE_ITEMS_MAX: usize = 2_000;
const DAY_MS: i64 = 86_400_000;

/// A player's requested action. The native owner resolves every durable identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseMoveOutCommand {
    Notice { due_at_unix_ms: i64 },
    Cancel,
}

/// The retained command result. It is not a current owner or custody capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseMoveOutCommandReceipt {
    pub operation: [u8; 16],
    pub due_at_unix_ms: i64,
    pub cancelled: bool,
    pub replayed: bool,
}

/// An authorized owner snapshot. A later command independently resolves ownership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseMoveOutOwnerView {
    pub house: HouseId,
    pub ownership_revision: u64,
    pub notice: Option<DispositionView>,
}

fn check_absolute_due_at(db_now_ms: i64, due_at_ms: i64) -> Result<()> {
    let span = due_at_ms
        .checked_sub(db_now_ms)
        .ok_or(HouseMoveOutError::InvalidInput)?;
    if !(DAY_MS..=30 * DAY_MS).contains(&span) {
        return Err(HouseMoveOutError::InvalidInput);
    }
    Ok(())
}

#[cfg(test)]
mod absolute_due_tests {
    use super::{DAY_MS, HouseMoveOutError, check_absolute_due_at};

    #[test]
    fn absolute_due_bounds_are_inclusive_checked_and_not_rounded() {
        let now = 1_800_000_000_123_i64;
        assert!(check_absolute_due_at(now, now + DAY_MS).is_ok());
        assert!(check_absolute_due_at(now, now + 30 * DAY_MS).is_ok());
        assert!(check_absolute_due_at(now, now + DAY_MS + 1).is_ok());
        assert!(matches!(
            check_absolute_due_at(now, now + DAY_MS - 1),
            Err(HouseMoveOutError::InvalidInput)
        ));
        assert!(matches!(
            check_absolute_due_at(now, now + 30 * DAY_MS + 1),
            Err(HouseMoveOutError::InvalidInput)
        ));
        assert!(matches!(
            check_absolute_due_at(i64::MIN, i64::MAX),
            Err(HouseMoveOutError::InvalidInput)
        ));
        assert!(matches!(
            check_absolute_due_at(i64::MAX, i64::MIN),
            Err(HouseMoveOutError::InvalidInput)
        ));
    }
}

/// A native current-owner notice, not a direct player-to-player property sale.
#[derive(Debug, Clone)]
pub struct MoveOutNotice {
    pub operation: [u8; 16],
    pub house: HouseId,
    pub ownership_revision: u64,
    pub notice_days: u8,
}

/// Cancellation of one retained owner notice before its DB-clock due time.
#[derive(Debug, Clone)]
pub struct MoveOutCancellation {
    pub operation: [u8; 16],
    pub house: HouseId,
    pub ownership_revision: u64,
}

/// Every mutation resolves its own current authority; none accepts a quiescent/current flag.
#[derive(Debug, Clone)]
pub enum HouseMoveOutPass {
    Notice {
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        request: MoveOutNotice,
    },
    Cancel {
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        request: MoveOutCancellation,
    },
    Freeze {
        operation: [u8; 16],
        house: HouseId,
        execution_scope: RuntimeScopeRefV1,
        scope_generation: u64,
    },
    Step {
        operation: [u8; 16],
        house: HouseId,
        execution_scope: RuntimeScopeRefV1,
        scope_generation: u64,
        step: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispositionView {
    pub operation: [u8; 16],
    pub due_at_unix_ms: i64,
    pub frozen_count: Option<u64>,
    pub frozen_digest: Option<[u8; 32]>,
    pub next_step: u64,
    /// Items are safely in Inbox; this does NOT mean ownership/slot/fence release.
    pub drained: bool,
    pub cancelled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReturnedHouseItem {
    pub item: [u8; 16],
    pub reclaim_character: [u8; 16],
    pub ordinal: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseMoveOutOutcome {
    Notice(DispositionView),
    Frozen(DispositionView),
    Cancelled(DispositionView),
    Step {
        view: DispositionView,
        items: Vec<ReturnedHouseItem>,
        replayed: bool,
    },
}

#[derive(Debug)]
pub enum HouseMoveOutError {
    InvalidInput,
    ConflictingOperation,
    MissingOperation,
    AuthorityRejected,
    OwnershipChanged,
    NotDue,
    CancellationClosed,
    Cancelled,
    NotFrozen,
    UnexpectedStep,
    /// Current canonical house sessions or unreleased entry reservations exist.
    Occupied,
    InvalidCustody,
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for HouseMoveOutError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}
impl From<sqlx::Error> for HouseMoveOutError {
    fn from(error: sqlx::Error) -> Self {
        Self::Unavailable(error.into())
    }
}
impl std::fmt::Display for HouseMoveOutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for HouseMoveOutError {}
pub(crate) type Result<T> = std::result::Result<T, HouseMoveOutError>;

pub(crate) fn valid_id(id: &[u8; 16]) -> bool {
    id[6] >> 4 == 7 && id[8] & 0xc0 == 0x80
}
pub(crate) fn number(row: &sqlx::postgres::PgRow, column: &str) -> Result<u64> {
    row.try_get::<String, _>(column)?
        .parse()
        .map_err(|_| HouseMoveOutError::InvalidCustody)
}
pub(crate) fn bytes(row: &sqlx::postgres::PgRow, column: &str) -> Result<[u8; 16]> {
    row.try_get::<Vec<u8>, _>(column)?
        .try_into()
        .map_err(|_| HouseMoveOutError::InvalidCustody)
}
pub(crate) fn view(row: &sqlx::postgres::PgRow) -> Result<DispositionView> {
    Ok(DispositionView {
        operation: bytes(row, "operation")?,
        due_at_unix_ms: row.try_get("due_at_ms")?,
        frozen_count: row
            .try_get::<Option<i64>, _>("frozen_count")?
            .map(|n| u64::try_from(n).map_err(|_| HouseMoveOutError::InvalidCustody))
            .transpose()?,
        frozen_digest: row
            .try_get::<Option<Vec<u8>>, _>("frozen_digest")?
            .map(|b| b.try_into().map_err(|_| HouseMoveOutError::InvalidCustody))
            .transpose()?,
        next_step: number(row, "next_step")?,
        drained: row.try_get::<i16, _>("state")? == 3,
        cancelled: row.try_get::<i16, _>("state")? == 4,
    })
}
pub(crate) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    operation: [u8; 16],
    house: &HouseId,
) -> Result<sqlx::postgres::PgRow> {
    if !valid_id(&operation) {
        return Err(HouseMoveOutError::InvalidInput);
    }
    let row = sqlx::query("SELECT uuid_send(operation_id) AS operation,uuid_send(world_id) AS world,house_key,uuid_send(owner_character_id) AS owner,ownership_revision::text,command_binding,due_at_ms,state,frozen_count,frozen_digest,next_step::text FROM game_house_dispositions WHERE operation_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(operation.as_slice()).fetch_optional(&mut **tx).await?.ok_or(HouseMoveOutError::MissingOperation)?;
    if bytes(&row, "world")? != *house.world_id().as_bytes()
        || row.try_get::<String, _>("house_key")? != house.house_key()
    {
        return Err(HouseMoveOutError::ConflictingOperation);
    }
    Ok(row)
}

pub(crate) async fn current_owner(
    tx: &mut Transaction<'_, Postgres>,
    row: &sqlx::postgres::PgRow,
    house: &HouseId,
) -> Result<()> {
    let owner = sqlx::query("SELECT uuid_send(owner_character_id) AS owner,ownership_revision::text FROM game_house_ownership WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 FOR UPDATE")
        .bind(house.world_id().as_bytes().as_slice()).bind(house.house_key()).fetch_optional(&mut **tx).await?.ok_or(HouseMoveOutError::OwnershipChanged)?;
    if bytes(&owner, "owner")? != bytes(row, "owner")?
        || number(&owner, "ownership_revision")? != number(row, "ownership_revision")?
    {
        return Err(HouseMoveOutError::OwnershipChanged);
    }
    Ok(())
}
pub(crate) async fn current_world_job(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    house: &HouseId,
    execution_scope: RuntimeScopeRefV1,
    generation: u64,
) -> Result<()> {
    if generation == 0 || !prove_current_incarnation(tx, node).await? {
        return Err(HouseMoveOutError::AuthorityRejected);
    }
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = execution_scope
    else {
        return Err(HouseMoveOutError::AuthorityRejected);
    };
    if world_id != house.world_id() {
        return Err(HouseMoveOutError::AuthorityRejected);
    }
    let fact = node.fact();
    let held: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_runtime_scope_assignments WHERE scope_key=$1 AND scope_kind=1 AND world_id=encode($2,'hex')::uuid AND channel_id=encode($3,'hex')::uuid AND ownership_generation=$4::text::numeric(20,0) AND state=1 AND holder_node_id=encode($5,'hex')::uuid AND holder_registration_revision=$6::text::numeric(20,0) FOR SHARE)")
        .bind(DurabilityRoot::house_disposition_channel_scope_key(world_id,channel_id).as_slice()).bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(generation.to_string()).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).fetch_one(&mut **tx).await?;
    if !held {
        return Err(HouseMoveOutError::AuthorityRejected);
    }
    Ok(())
}

fn command_binding(
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    house: &HouseId,
    ownership_revision: u64,
    tag: u8,
) -> Result<Vec<u8>> {
    if command.game_session_id() != fence.game_session_id {
        return Err(HouseMoveOutError::AuthorityRejected);
    }
    let (world, channel) = crate::durability::item_transfer::scope_of(fence)
        .map_err(|_| HouseMoveOutError::AuthorityRejected)?;
    // Private durable binding, not a new wire codec: fixed-width identities and length-prefixed
    // House key; includes the command's original fences. Never reused as current authority.
    let mut binding = Vec::new();
    binding.extend_from_slice(command.game_session_id().as_bytes());
    binding.extend_from_slice(&command.command_id().get().to_be_bytes());
    binding.extend_from_slice(fence.character_id.as_bytes());
    binding.extend_from_slice(house.world_id().as_bytes());
    binding.extend_from_slice(&(house.house_key().len() as u64).to_be_bytes());
    binding.extend_from_slice(house.house_key().as_bytes());
    binding.extend_from_slice(&ownership_revision.to_be_bytes());
    binding.push(tag);
    binding.extend_from_slice(world.as_bytes());
    binding.extend_from_slice(channel.as_bytes());
    binding.extend_from_slice(&fence.connection_generation.get().to_be_bytes());
    binding.extend_from_slice(&fence.character_lease_generation.to_be_bytes());
    binding.extend_from_slice(&fence.scope_ownership_generation.get().to_be_bytes());
    Ok(binding)
}

fn absolute_command_binding(
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    house: &HouseId,
    ownership_revision: u64,
    operation: [u8; 16],
    request: HouseMoveOutCommand,
) -> Result<Vec<u8>> {
    // Separate tags keep the original private day-based qualification path distinct.
    // The exact supplied fence and absolute date are bound, without scope normalization.
    let tag = match request {
        HouseMoveOutCommand::Notice { .. } => 31,
        HouseMoveOutCommand::Cancel => 32,
    };
    let mut binding = command_binding(fence, command, house, ownership_revision, tag)?;
    binding.extend_from_slice(&operation);
    if let HouseMoveOutCommand::Notice { due_at_unix_ms } = request {
        binding.extend_from_slice(&due_at_unix_ms.to_be_bytes());
    }
    Ok(binding)
}

pub(crate) async fn apply_command(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    house: HouseId,
    request: HouseMoveOutCommand,
) -> Result<HouseMoveOutCommandReceipt> {
    if command.game_session_id() != fence.game_session_id {
        return Err(HouseMoveOutError::AuthorityRejected);
    }
    // Occurrence before Character/property locks. No durable row is written for a refusal.
    // This lock uses the original CommandRef, never a newly minted operation identifier.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:house-command:'||encode($1,'hex')||':'||$2,0))")
        .bind(command.game_session_id().as_bytes().as_slice())
        .bind(command.command_id().get().to_string())
        .execute(&mut **tx).await?;
    if let Some(previous) = sqlx::query("SELECT uuid_send(operation_id) AS operation,ownership_revision::text,command_binding,cancel_binding,due_at_ms,game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0) AS notice_command FROM game_house_dispositions WHERE (game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0)) OR (cancel_session_id=encode($1,'hex')::uuid AND cancel_command_id=$2::text::numeric(20,0))")
        .bind(command.game_session_id().as_bytes().as_slice())
        .bind(command.command_id().get().to_string())
        .fetch_optional(&mut **tx).await? {
        let operation = bytes(&previous, "operation")?;
        let was_notice: bool = previous.try_get("notice_command")?;
        if was_notice != matches!(request, HouseMoveOutCommand::Notice { .. }) {
            return Err(HouseMoveOutError::ConflictingOperation);
        }
        let expected = absolute_command_binding(
            &fence, command, &house, number(&previous, "ownership_revision")?, operation, request,
        )?;
        let retained: Option<Vec<u8>> = if was_notice {
            Some(previous.try_get("command_binding")?)
        } else {
            previous.try_get("cancel_binding")?
        };
        if retained.as_deref() != Some(expected.as_slice()) {
            return Err(HouseMoveOutError::ConflictingOperation);
        }
        // These fields are immutable. Reconstruct the original command outcome even if
        // custody progressed, the owner changed or a newer notice now exists. No clock,
        // current owner lookup, UUID allocation or fresh mutation occurs on this branch.
        return Ok(HouseMoveOutCommandReceipt {
            operation,
            due_at_unix_ms: previous.try_get("due_at_ms")?,
            cancelled: !was_notice,
            replayed: true,
        });
    }

    let (world, channel) = crate::durability::item_transfer::scope_of(&fence)
        .map_err(|_| HouseMoveOutError::AuthorityRejected)?;
    if world != house.world_id()
        || !character_item_fence_is_current(
            tx,
            node,
            &fence,
            command,
            fence.character_id,
            *world.as_bytes(),
            *channel.as_bytes(),
        )
        .await?
    {
        return Err(HouseMoveOutError::AuthorityRejected);
    }
    let owner = sqlx::query("SELECT uuid_send(owner_character_id) AS owner,ownership_revision::text FROM game_house_ownership WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(house.house_key())
        .fetch_optional(&mut **tx).await?.ok_or(HouseMoveOutError::OwnershipChanged)?;
    if owner.try_get::<Option<Vec<u8>>, _>("owner")?.as_deref()
        != Some(fence.character_id.as_bytes().as_slice())
    {
        return Err(HouseMoveOutError::OwnershipChanged);
    }
    let ownership_revision = number(&owner, "ownership_revision")?;
    let pending = sqlx::query("SELECT uuid_send(operation_id) AS operation,uuid_send(owner_character_id) AS owner,ownership_revision::text,due_at_ms,state FROM game_house_dispositions WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND state<>4 FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(house.house_key())
        .fetch_optional(&mut **tx).await?;

    match request {
        HouseMoveOutCommand::Notice { due_at_unix_ms } => {
            if pending.is_some() {
                return Err(HouseMoveOutError::ConflictingOperation);
            }
            let now: i64 = sqlx::query_scalar(
                "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
            )
            .fetch_one(&mut **tx)
            .await?;
            check_absolute_due_at(now, due_at_unix_ms)?;
            // The server allocates this identity once, after admission and before the
            // single retained INSERT. Rollback discards the allocation with the command.
            let operation: Vec<u8> =
                sqlx::query_scalar("SELECT uuid_send(game_character_uuid_v7())")
                    .fetch_one(&mut **tx)
                    .await?;
            let operation: [u8; 16] = operation
                .try_into()
                .map_err(|_| HouseMoveOutError::InvalidCustody)?;
            let binding = absolute_command_binding(
                &fence,
                command,
                &house,
                ownership_revision,
                operation,
                request,
            )?;
            sqlx::query("INSERT INTO game_house_dispositions(operation_id,world_id,house_key,owner_character_id,ownership_revision,game_session_id,command_id,command_binding,requested_at_ms,due_at_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,$5::text::numeric(20,0),encode($6,'hex')::uuid,$7::text::numeric(20,0),$8,$9,$10)")
                .bind(operation.as_slice()).bind(world.as_bytes().as_slice()).bind(house.house_key())
                .bind(fence.character_id.as_bytes().as_slice()).bind(ownership_revision.to_string())
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
                .bind(binding).bind(now).bind(due_at_unix_ms).execute(&mut **tx).await?;
            Ok(HouseMoveOutCommandReceipt {
                operation,
                due_at_unix_ms,
                cancelled: false,
                replayed: false,
            })
        }
        HouseMoveOutCommand::Cancel => {
            let pending = pending.ok_or(HouseMoveOutError::MissingOperation)?;
            if bytes(&pending, "owner")? != *fence.character_id.as_bytes()
                || number(&pending, "ownership_revision")? != ownership_revision
            {
                return Err(HouseMoveOutError::OwnershipChanged);
            }
            if pending.try_get::<i16, _>("state")? != 1 {
                return Err(HouseMoveOutError::CancellationClosed);
            }
            let operation = bytes(&pending, "operation")?;
            let binding = absolute_command_binding(
                &fence,
                command,
                &house,
                ownership_revision,
                operation,
                request,
            )?;
            let now: i64 = sqlx::query_scalar(
                "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
            )
            .fetch_one(&mut **tx)
            .await?;
            let due_at_unix_ms: i64 = pending.try_get("due_at_ms")?;
            if now >= due_at_unix_ms {
                return Err(HouseMoveOutError::CancellationClosed);
            }
            let changed = sqlx::query("UPDATE game_house_dispositions SET state=4,cancel_session_id=encode($2,'hex')::uuid,cancel_command_id=$3::text::numeric(20,0),cancel_binding=$4,cancelled_at_ms=$5 WHERE operation_id=encode($1,'hex')::uuid AND state=1")
                .bind(operation.as_slice()).bind(command.game_session_id().as_bytes().as_slice())
                .bind(command.command_id().get().to_string()).bind(binding).bind(now)
                .execute(&mut **tx).await?.rows_affected();
            if changed != 1 {
                return Err(HouseMoveOutError::CancellationClosed);
            }
            Ok(HouseMoveOutCommandReceipt {
                operation,
                due_at_unix_ms,
                cancelled: true,
                replayed: false,
            })
        }
    }
}

/// Called only after the native envelope checks the complete current Character fence.
/// Locks protect this read; no notice, receipt, revision or lifecycle state is written.
pub(crate) async fn read_owner_projection(
    tx: &mut Transaction<'_, Postgres>,
    actor: crate::domain::CharacterId,
    house: HouseId,
) -> Result<HouseMoveOutOwnerView> {
    let owner = sqlx::query("SELECT uuid_send(owner_character_id) AS owner,ownership_revision::text FROM game_house_ownership WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 FOR SHARE")
        .bind(house.world_id().as_bytes().as_slice()).bind(house.house_key())
        .fetch_optional(&mut **tx).await?.ok_or(HouseMoveOutError::OwnershipChanged)?;
    if owner.try_get::<Option<Vec<u8>>, _>("owner")?.as_deref() != Some(actor.as_bytes().as_slice())
    {
        return Err(HouseMoveOutError::OwnershipChanged);
    }
    let ownership_revision = number(&owner, "ownership_revision")?;
    let notice = sqlx::query("SELECT uuid_send(operation_id) AS operation,uuid_send(owner_character_id) AS owner,ownership_revision::text,due_at_ms,state,frozen_count,frozen_digest,next_step::text FROM game_house_dispositions WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND state<>4 FOR SHARE")
        .bind(house.world_id().as_bytes().as_slice()).bind(house.house_key())
        .fetch_optional(&mut **tx).await?;
    let notice = notice
        .map(|row| {
            if bytes(&row, "owner")? != *actor.as_bytes()
                || number(&row, "ownership_revision")? != ownership_revision
            {
                return Err(HouseMoveOutError::OwnershipChanged);
            }
            view(&row)
        })
        .transpose()?;
    Ok(HouseMoveOutOwnerView {
        house,
        ownership_revision,
        notice,
    })
}

pub(crate) async fn notice(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    request: MoveOutNotice,
) -> Result<HouseMoveOutOutcome> {
    if !valid_id(&request.operation)
        || !(1..=30).contains(&request.notice_days)
        || request.ownership_revision == 0
    {
        return Err(HouseMoveOutError::InvalidInput);
    }
    let (world, channel) = crate::durability::item_transfer::scope_of(&fence)
        .map_err(|_| HouseMoveOutError::AuthorityRejected)?;
    let binding = command_binding(
        &fence,
        command,
        &request.house,
        request.ownership_revision,
        request.notice_days,
    )?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:house-disposition:'||encode($1,'hex'),0))").bind(request.operation.as_slice()).execute(&mut **tx).await?;
    if let Some(row) = sqlx::query("SELECT uuid_send(operation_id) AS operation,command_binding FROM game_house_dispositions WHERE operation_id=encode($1,'hex')::uuid OR (game_session_id=encode($2,'hex')::uuid AND command_id=$3::text::numeric(20,0)) OR (cancel_session_id=encode($2,'hex')::uuid AND cancel_command_id=$3::text::numeric(20,0))").bind(request.operation.as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut **tx).await? {
        if bytes(&row,"operation")? != request.operation || row.try_get::<Vec<u8>,_>("command_binding")? != binding { return Err(HouseMoveOutError::ConflictingOperation); }
        return Ok(HouseMoveOutOutcome::Notice(view(&load(tx,request.operation,&request.house).await?)?));
    }
    if world != request.house.world_id()
        || !character_item_fence_is_current(
            tx,
            node,
            &fence,
            command,
            fence.character_id,
            *world.as_bytes(),
            *channel.as_bytes(),
        )
        .await?
    {
        return Err(HouseMoveOutError::AuthorityRejected);
    }
    let owner=sqlx::query("SELECT uuid_send(owner_character_id) AS owner,ownership_revision::text FROM game_house_ownership WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 FOR UPDATE").bind(world.as_bytes().as_slice()).bind(request.house.house_key()).fetch_optional(&mut **tx).await?.ok_or(HouseMoveOutError::OwnershipChanged)?;
    if bytes(&owner, "owner")? != *fence.character_id.as_bytes()
        || number(&owner, "ownership_revision")? != request.ownership_revision
    {
        return Err(HouseMoveOutError::OwnershipChanged);
    }
    let occupied:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_house_dispositions WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND state<>4)").bind(world.as_bytes().as_slice()).bind(request.house.house_key()).fetch_one(&mut **tx).await?;
    if occupied {
        return Err(HouseMoveOutError::ConflictingOperation);
    }
    sqlx::query("INSERT INTO game_house_dispositions(operation_id,world_id,house_key,owner_character_id,ownership_revision,game_session_id,command_id,command_binding,requested_at_ms,due_at_ms) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,$5::text::numeric(20,0),encode($6,'hex')::uuid,$7::text::numeric(20,0),$8,n,n+$9 FROM (SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS n) clock")
        .bind(request.operation.as_slice()).bind(world.as_bytes().as_slice()).bind(request.house.house_key()).bind(fence.character_id.as_bytes().as_slice()).bind(request.ownership_revision.to_string()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(binding).bind(i64::from(request.notice_days)*DAY_MS).execute(&mut **tx).await?;
    Ok(HouseMoveOutOutcome::Notice(view(
        &load(tx, request.operation, &request.house).await?,
    )?))
}

pub(crate) async fn cancel(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    request: MoveOutCancellation,
) -> Result<HouseMoveOutOutcome> {
    if !valid_id(&request.operation) || request.ownership_revision == 0 {
        return Err(HouseMoveOutError::InvalidInput);
    }
    let mut binding = command_binding(
        &fence,
        command,
        &request.house,
        request.ownership_revision,
        0,
    )?;
    binding.extend_from_slice(&request.operation);
    // Native admission relation locks serialize CommandRef and same-house successors.
    if let Some(previous)=sqlx::query("SELECT uuid_send(operation_id) AS operation,cancel_binding,game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0) AS notice_command FROM game_house_dispositions WHERE (game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0)) OR (cancel_session_id=encode($1,'hex')::uuid AND cancel_command_id=$2::text::numeric(20,0))").bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut **tx).await? {
        if previous.try_get::<bool,_>("notice_command")? || bytes(&previous,"operation")? !=request.operation || previous.try_get::<Option<Vec<u8>>,_>("cancel_binding")?.as_deref()!=Some(binding.as_slice()) {return Err(HouseMoveOutError::ConflictingOperation);}
        return Ok(HouseMoveOutOutcome::Cancelled(view(&load(tx,request.operation,&request.house).await?)?));
    }
    let (world, channel) = crate::durability::item_transfer::scope_of(&fence)
        .map_err(|_| HouseMoveOutError::AuthorityRejected)?;
    if world != request.house.world_id()
        || !character_item_fence_is_current(
            tx,
            node,
            &fence,
            command,
            fence.character_id,
            *world.as_bytes(),
            *channel.as_bytes(),
        )
        .await?
    {
        return Err(HouseMoveOutError::AuthorityRejected);
    }
    // Character root/session before property, matching NOTICE/native accepted lock order.
    let row = load(tx, request.operation, &request.house).await?;
    current_owner(tx, &row, &request.house).await?;
    if bytes(&row, "owner")? != *fence.character_id.as_bytes()
        || number(&row, "ownership_revision")? != request.ownership_revision
    {
        return Err(HouseMoveOutError::OwnershipChanged);
    }
    if row.try_get::<i16, _>("state")? != 1 {
        return Err(HouseMoveOutError::CancellationClosed);
    }
    let changed=sqlx::query("WITH clock AS MATERIALIZED(SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS n) UPDATE game_house_dispositions SET state=4,cancel_session_id=encode($2,'hex')::uuid,cancel_command_id=$3::text::numeric(20,0),cancel_binding=$4,cancelled_at_ms=clock.n FROM clock WHERE operation_id=encode($1,'hex')::uuid AND due_at_ms>clock.n").bind(request.operation.as_slice()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(binding).execute(&mut **tx).await?.rows_affected();
    if changed != 1 {
        return Err(HouseMoveOutError::CancellationClosed);
    }
    Ok(HouseMoveOutOutcome::Cancelled(view(
        &load(tx, request.operation, &request.house).await?,
    )?))
}

pub(crate) async fn freeze(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    operation: [u8; 16],
    house: &HouseId,
    execution_scope: RuntimeScopeRefV1,
    generation: u64,
) -> Result<HouseMoveOutOutcome> {
    current_world_job(tx, node, house, execution_scope, generation).await?;
    // The owner is locked before operation to agree with the source fence trigger.
    sqlx::query("SELECT 1 FROM game_house_ownership WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 FOR UPDATE").bind(house.world_id().as_bytes().as_slice()).bind(house.house_key()).execute(&mut **tx).await?;
    let row = load(tx, operation, house).await?;
    current_owner(tx, &row, house).await?;
    if row.try_get::<i16, _>("state")? == 4 {
        return Err(HouseMoveOutError::Cancelled);
    }
    if row.try_get::<i16, _>("state")? != 1 {
        return Ok(HouseMoveOutOutcome::Frozen(view(&row)?));
    }
    let due: bool=sqlx::query_scalar("SELECT due_at_ms<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_house_dispositions WHERE operation_id=encode($1,'hex')::uuid").bind(operation.as_slice()).fetch_one(&mut **tx).await?;
    if !due {
        return Err(HouseMoveOutError::NotDue);
    }
    let items=sqlx::query("SELECT uuid_send(h.item_instance_id) AS item FROM game_item_house_interior_locations h WHERE h.world_id=encode($1,'hex')::uuid AND h.house_key=$2 ORDER BY h.item_instance_id LIMIT 2001 FOR UPDATE")
        .bind(house.world_id().as_bytes().as_slice()).bind(house.house_key()).fetch_all(&mut **tx).await?;
    if items.len() > HOUSE_ITEMS_MAX {
        return Err(HouseMoveOutError::InvalidCustody);
    }
    let mut digest = Sha256::new();
    for item in &items {
        digest.update(bytes(item, "item")?);
    }
    let digest: [u8; 32] = digest.finalize().into();
    sqlx::query("INSERT INTO game_house_disposition_items(operation_id,item_instance_id,source_location,source_provenance,source_item) SELECT encode($1,'hex')::uuid,h.item_instance_id,to_jsonb(h),to_jsonb(p),to_jsonb(i) FROM game_item_house_interior_locations h JOIN game_item_house_reclaim_provenance p USING(item_instance_id) JOIN game_item_instances i USING(item_instance_id) WHERE h.world_id=encode($2,'hex')::uuid AND h.house_key=$3")
        .bind(operation.as_slice()).bind(house.world_id().as_bytes().as_slice()).bind(house.house_key()).execute(&mut **tx).await?;
    let captured: i64=sqlx::query_scalar("SELECT count(*) FROM game_house_disposition_items WHERE operation_id=encode($1,'hex')::uuid").bind(operation.as_slice()).fetch_one(&mut **tx).await?;
    if usize::try_from(captured).ok() != Some(items.len()) {
        return Err(HouseMoveOutError::InvalidCustody);
    }
    sqlx::query("UPDATE game_house_dispositions SET state=2,frozen_count=$2,frozen_digest=$3 WHERE operation_id=encode($1,'hex')::uuid").bind(operation.as_slice()).bind(captured).bind(digest.as_slice()).execute(&mut **tx).await?;
    Ok(HouseMoveOutOutcome::Frozen(view(
        &load(tx, operation, house).await?,
    )?))
}

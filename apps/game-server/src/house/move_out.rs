//! Bounded HOUSE-1b preparation over the native ownership, handoff and custody tables.
//!
//! UNINSTALLED: this source is intended as a durability child at the CP slot. The private
//! schema below and its callers are qualified only in an isolated database. Its retained SQL
//! evidence is not the registered HouseOperation event: production installation/permissions,
//! acquisition/property/slot reconciliation, exit/quiesce, house content revision and BED
//! integration remain mandatory. DRAINED deliberately keeps the content fence and owner.

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::house_scope_handoff::HouseId;
use super::item_transfer::{CurrentCharacterItemFence, character_item_fence_is_current};
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{CommandRef, RuntimeScopeRefV1};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

/// Accepted maximum per disposition step (HOUSE-OWN-0 §7).
pub const DISPOSITION_STEP_ITEMS: usize = 100;
/// Accepted house item bound, still PARITY_PENDING in the owning decision.
pub const HOUSE_ITEMS_MAX: usize = 2_000;
const DAY_MS: i64 = 86_400_000;

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

/// A private owner pass. It is executable with the native Root and fences, but is not wired
/// or production-qualified until the owning registered event/projection/release seams land.
impl DurabilityRoot {
    pub async fn run_private_house_move_out(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        operation: HouseMoveOutPass,
    ) -> Result<HouseMoveOutOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseMoveOutError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    // Before property/item locks. Together with the native admission relation fence
                    // this freezes both canonical session and PREPARED/COMMITTED entrant census.
                    sqlx::query("LOCK TABLE game_house_scope_handoffs IN SHARE MODE")
                        .execute(&mut *tx)
                        .await?;
                    let result = match operation {
                        HouseMoveOutPass::Notice {
                            fence,
                            command,
                            request,
                        } => notice(&mut tx, &node, fence, command, request).await,
                        HouseMoveOutPass::Cancel {
                            fence,
                            command,
                            request,
                        } => cancel(&mut tx, &node, fence, command, request).await,
                        HouseMoveOutPass::Freeze {
                            operation,
                            house,
                            execution_scope,
                            scope_generation,
                        } => {
                            freeze(
                                &mut tx,
                                &node,
                                operation,
                                &house,
                                execution_scope,
                                scope_generation,
                            )
                            .await
                        }
                        HouseMoveOutPass::Step {
                            operation,
                            house,
                            execution_scope,
                            scope_generation,
                            step,
                        } => {
                            super::house_transfer::step(
                                &mut tx,
                                &node,
                                operation,
                                &house,
                                execution_scope,
                                scope_generation,
                                step,
                            )
                            .await
                        }
                    };
                    match result {
                        Ok(value) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(error) => Ok(Err(error)), // transaction drops/rolls back as a whole
                    }
                })
            })
            .await?
    }
}

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
        .bind(super::runtime_scope_assignment::scope_key(world_id,channel_id).as_slice()).bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(generation.to_string()).bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).fetch_one(&mut **tx).await?;
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
    let (world, channel) =
        super::item_transfer::scope_of(fence).map_err(|_| HouseMoveOutError::AuthorityRejected)?;
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

async fn notice(
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
    let (world, channel) =
        super::item_transfer::scope_of(&fence).map_err(|_| HouseMoveOutError::AuthorityRejected)?;
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
    sqlx::query("INSERT INTO game_house_dispositions(operation_id,world_id,house_key,owner_character_id,ownership_revision,game_session_id,command_id,command_binding,notice_days,requested_at_ms,due_at_ms) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,$5::text::numeric(20,0),encode($6,'hex')::uuid,$7::text::numeric(20,0),$8,$9,n,n+$10 FROM (SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS n) clock")
        .bind(request.operation.as_slice()).bind(world.as_bytes().as_slice()).bind(request.house.house_key()).bind(fence.character_id.as_bytes().as_slice()).bind(request.ownership_revision.to_string()).bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(binding).bind(i16::from(request.notice_days)).bind(i64::from(request.notice_days)*DAY_MS).execute(&mut **tx).await?;
    Ok(HouseMoveOutOutcome::Notice(view(
        &load(tx, request.operation, &request.house).await?,
    )?))
}

async fn cancel(
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
    let (world, channel) =
        super::item_transfer::scope_of(&fence).map_err(|_| HouseMoveOutError::AuthorityRejected)?;
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

async fn freeze(
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

/// Delegated private design, deliberately not a numbered migration or production install.
pub const PRIVATE_DISPOSITION_SCHEMA: &str = r#"
CREATE TABLE game_house_dispositions (
 operation_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(operation_id)),
 world_id UUID NOT NULL, house_key TEXT NOT NULL CHECK(game_house_scope_key_valid(house_key)),
 owner_character_id UUID NOT NULL REFERENCES game_character_roots(character_id),
 ownership_revision NUMERIC(20,0) NOT NULL CHECK(ownership_revision BETWEEN 1 AND 18446744073709551615),
 game_session_id UUID NOT NULL,command_id NUMERIC(20,0) NOT NULL CHECK(command_id>0),
 command_binding BYTEA NOT NULL CHECK(octet_length(command_binding) BETWEEN 1 AND 2048),
 cause TEXT NOT NULL DEFAULT 'MOVE_OUT' CHECK(cause='MOVE_OUT'),
 notice_days SMALLINT NOT NULL CHECK(notice_days BETWEEN 1 AND 30),requested_at_ms BIGINT NOT NULL,
 due_at_ms BIGINT NOT NULL CHECK(due_at_ms=requested_at_ms+notice_days::bigint*86400000),
 state SMALLINT NOT NULL DEFAULT 1 CHECK(state IN(1,2,3,4)),
 cancel_session_id UUID,cancel_command_id NUMERIC(20,0),cancel_binding BYTEA,cancelled_at_ms BIGINT,
 frozen_count BIGINT CHECK(frozen_count BETWEEN 0 AND 2000),frozen_digest BYTEA CHECK(octet_length(frozen_digest)=32),
 next_step NUMERIC(20,0) NOT NULL DEFAULT 0 CHECK(next_step BETWEEN 0 AND 18446744073709551615),
 UNIQUE(game_session_id,command_id),UNIQUE(cancel_session_id,cancel_command_id),
 CHECK((state IN(1,4))=(frozen_count IS NULL)),CHECK((state IN(1,4))=(frozen_digest IS NULL)),
 CHECK((state=4)=(cancel_session_id IS NOT NULL)),CHECK((state=4)=(cancel_command_id IS NOT NULL)),
 CHECK((state=4)=(cancel_binding IS NOT NULL)),CHECK((state=4)=(cancelled_at_ms IS NOT NULL)),
 CHECK(cancel_command_id>0),CHECK(octet_length(cancel_binding) BETWEEN 1 AND 2048),
 CHECK(cancelled_at_ms<due_at_ms AND cancelled_at_ms>=requested_at_ms)

);
CREATE UNIQUE INDEX house_disposition_live_house ON game_house_dispositions(world_id,house_key) WHERE state<>4;
CREATE TABLE game_house_disposition_items (
 operation_id UUID NOT NULL REFERENCES game_house_dispositions(operation_id),item_instance_id UUID NOT NULL REFERENCES game_item_instances(item_instance_id),
 source_location JSONB NOT NULL,source_provenance JSONB NOT NULL,source_item JSONB NOT NULL,
 moved_step NUMERIC(20,0),moving_xact_id XID8,ordinal NUMERIC(20,0),
 PRIMARY KEY(operation_id,item_instance_id),CHECK((moved_step IS NULL)=(moving_xact_id IS NULL)),CHECK(ordinal IS NULL OR (ordinal>0 AND moved_step IS NOT NULL))
);
CREATE TABLE game_house_disposition_steps (
 operation_id UUID NOT NULL REFERENCES game_house_dispositions(operation_id),step NUMERIC(20,0) NOT NULL CHECK(step>=0),
 moved_count BIGINT NOT NULL CHECK(moved_count BETWEEN 0 AND 100),created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 PRIMARY KEY(operation_id,step)
);
INSERT INTO game_character_inbox_cause_kinds(cause_kind) VALUES('HOUSE_DISPOSITION');
CREATE FUNCTION game_house_disposition_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF TG_OP='DELETE' THEN RAISE EXCEPTION 'disposition evidence is retained' USING ERRCODE='23514'; END IF;
 IF TG_TABLE_NAME='game_house_dispositions' THEN
  IF OLD.state=1 AND NEW.state=4 THEN
   IF (to_jsonb(NEW)-ARRAY['state','cancel_session_id','cancel_command_id','cancel_binding','cancelled_at_ms']) IS DISTINCT FROM
      (to_jsonb(OLD)-ARRAY['state','cancel_session_id','cancel_command_id','cancel_binding','cancelled_at_ms'])
   THEN RAISE EXCEPTION 'cancellation changed immutable notice' USING ERRCODE='23514'; END IF;
   RETURN NEW;
  END IF;
  IF (to_jsonb(NEW)-ARRAY['state','frozen_count','frozen_digest','next_step']) IS DISTINCT FROM
     (to_jsonb(OLD)-ARRAY['state','frozen_count','frozen_digest','next_step']) OR
     NOT ((OLD.state=1 AND NEW.state=2 AND NEW.next_step=OLD.next_step) OR
          (OLD.state=2 AND NEW.state IN(2,3) AND NEW.next_step=OLD.next_step+1 AND NEW.frozen_count=OLD.frozen_count AND NEW.frozen_digest=OLD.frozen_digest))
   THEN RAISE EXCEPTION 'immutable disposition binding/cause or invalid successor' USING ERRCODE='23514'; END IF;
 ELSIF TG_TABLE_NAME='game_house_disposition_items' THEN
  IF (to_jsonb(NEW)-ARRAY['moved_step','moving_xact_id','ordinal']) IS DISTINCT FROM
     (to_jsonb(OLD)-ARRAY['moved_step','moving_xact_id','ordinal']) OR
     NOT ((OLD.moved_step IS NULL AND NEW.moved_step IS NOT NULL AND NEW.moving_xact_id=pg_current_xact_id() AND NEW.ordinal IS NULL) OR
          (OLD.moved_step=NEW.moved_step AND OLD.moving_xact_id=pg_current_xact_id() AND NEW.moving_xact_id=OLD.moving_xact_id AND OLD.ordinal IS NULL AND NEW.ordinal IS NOT NULL))
   THEN RAISE EXCEPTION 'immutable census or already returned item' USING ERRCODE='23514'; END IF;
 ELSE RAISE EXCEPTION 'step evidence is immutable' USING ERRCODE='23514'; END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER house_disposition_immutable BEFORE UPDATE OR DELETE ON game_house_dispositions FOR EACH ROW EXECUTE FUNCTION game_house_disposition_immutable();
CREATE TRIGGER house_disposition_item_immutable BEFORE UPDATE OR DELETE ON game_house_disposition_items FOR EACH ROW EXECUTE FUNCTION game_house_disposition_immutable();
CREATE TRIGGER house_disposition_step_immutable BEFORE UPDATE OR DELETE ON game_house_disposition_steps FOR EACH ROW EXECUTE FUNCTION game_house_disposition_immutable();
CREATE FUNCTION game_house_disposition_fence_check(w UUID,h TEXT,i UUID,deleting BOOLEAN) RETURNS void LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE op UUID;guild_relation REGCLASS;guild_guard REGPROCEDURE;
BEGIN
 -- Native row triggers can arrive with an item/provenance row already locked. Never
 -- wait here behind a disposition that owns this anchor and is waiting for that row.
 -- Root writers prelock the anchor; late generic writers fail closed with55P03.
 PERFORM 1 FROM game_house_ownership WHERE world_id=w AND house_key=h FOR SHARE NOWAIT;
 SELECT operation_id INTO op FROM game_house_dispositions WHERE world_id=w AND house_key=h AND state IN(2,3);
 IF FOUND AND NOT (deleting AND EXISTS(SELECT 1 FROM game_house_disposition_items c WHERE c.operation_id=op AND c.item_instance_id=i AND c.moved_step IS NOT NULL AND c.moving_xact_id=pg_current_xact_id()))
 THEN RAISE EXCEPTION 'house disposition content fence' USING ERRCODE='23514'; END IF;
 -- The typed Guild owner supplies its actual census/reservation guard. Both native item
 -- mutations and House entry already pass through this one boundary. Ordinary DRAINED
 -- history remains fenced; a Guild release cannot bypass the ordinary branch above.
 guild_relation:=to_regclass(format('%I.game_house_guild_dispositions',current_schema()));
 IF guild_relation IS NOT NULL OR EXISTS(SELECT 1 FROM pg_catalog.pg_attribute
     WHERE attrelid='game_house_ownership'::regclass AND attname='owner_guild_id' AND NOT attisdropped) THEN
  guild_guard:=to_regprocedure(format('%I.game_house_guild_disposition_fence_check(uuid,text,uuid,boolean)',current_schema()));
  IF guild_relation IS NULL OR guild_guard IS NULL OR NOT EXISTS(SELECT 1 FROM pg_catalog.pg_proc
      WHERE oid=guild_guard AND prokind='f' AND prorettype='void'::regtype) THEN
   RAISE EXCEPTION 'Guild disposition fence owner is not installed' USING ERRCODE='23514';
  END IF;
  EXECUTE format('SELECT %I.game_house_guild_disposition_fence_check($1,$2,$3,$4)',current_schema())
   USING w,h,i,deleting;
 END IF;
END $$;
CREATE FUNCTION game_house_disposition_content_fence() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF TG_OP<>'INSERT' THEN PERFORM game_house_disposition_fence_check(OLD.world_id,OLD.house_key,OLD.item_instance_id,TG_OP='DELETE'); END IF;
 IF TG_OP<>'DELETE' THEN PERFORM game_house_disposition_fence_check(NEW.world_id,NEW.house_key,NEW.item_instance_id,false); END IF;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF; RETURN NEW;
END $$;
CREATE TRIGGER a_house_disposition_content_fence BEFORE INSERT OR UPDATE OR DELETE ON game_item_house_interior_locations FOR EACH ROW EXECUTE FUNCTION game_house_disposition_content_fence();
CREATE TRIGGER a_house_disposition_provenance_fence BEFORE INSERT OR UPDATE OR DELETE ON game_item_house_reclaim_provenance FOR EACH ROW EXECUTE FUNCTION game_house_disposition_content_fence();
CREATE FUNCTION game_house_disposition_item_fence() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
DECLARE location RECORD;
BEGIN
 FOR location IN SELECT world_id,house_key FROM game_item_house_interior_locations
     WHERE item_instance_id=OLD.item_instance_id ORDER BY world_id,house_key LOOP
  -- A custody return moves its location/provenance; it never rewrites the frozen item.
  PERFORM game_house_disposition_fence_check(location.world_id,location.house_key,OLD.item_instance_id,false);
 END LOOP;
 IF TG_OP='DELETE' THEN RETURN OLD; END IF; RETURN NEW;
END $$;
CREATE TRIGGER a_house_disposition_item_fence BEFORE UPDATE OR DELETE ON game_item_instances FOR EACH ROW EXECUTE FUNCTION game_house_disposition_item_fence();
CREATE FUNCTION game_house_disposition_entry_fence() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.state IN(1,2) THEN
  PERFORM game_house_disposition_fence_check(NEW.world_id,NEW.house_key,NULL,false);
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER a_house_disposition_entry_fence BEFORE INSERT OR UPDATE ON game_house_scope_handoffs FOR EACH ROW EXECUTE FUNCTION game_house_disposition_entry_fence();
CREATE FUNCTION game_house_disposition_delivery_proven() RETURNS trigger LANGUAGE plpgsql SECURITY DEFINER AS $$
BEGIN
 IF NEW.moved_step IS NOT NULL THEN
  IF NOT EXISTS(SELECT 1 FROM game_house_disposition_items c JOIN game_house_dispositions o USING(operation_id)
   JOIN game_item_character_inbox_locations l ON l.item_instance_id=c.item_instance_id
   JOIN game_character_inbox_deliveries d ON d.item_instance_id=l.item_instance_id AND d.world_id=l.world_id AND d.character_id=l.character_id AND d.ordinal=l.ordinal
   JOIN game_item_instances i ON i.item_instance_id=c.item_instance_id
   JOIN game_house_disposition_steps s ON s.operation_id=c.operation_id AND s.step=c.moved_step
   WHERE c.operation_id=NEW.operation_id AND c.item_instance_id=NEW.item_instance_id AND c.ordinal=l.ordinal
    AND l.character_id=(c.source_provenance->>'reclaim_subject_character_id')::uuid AND l.world_id=o.world_id
    AND d.cause_kind='HOUSE_DISPOSITION' AND d.cause_ref=o.operation_id::text AND d.created_xact_id=c.moving_xact_id
    AND s.created_xact_id=c.moving_xact_id AND to_jsonb(i)=c.source_item
    AND NOT EXISTS(SELECT 1 FROM game_item_house_interior_locations h WHERE h.item_instance_id=c.item_instance_id)
    AND NOT EXISTS(SELECT 1 FROM game_item_house_reclaim_provenance p WHERE p.item_instance_id=c.item_instance_id))
  THEN RAISE EXCEPTION 'disposition item lacks native delivery/source/step proof' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER house_disposition_delivery_proven AFTER UPDATE ON game_house_disposition_items DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_disposition_delivery_proven();
-- Installed with the real common, location, provenance, item and entry guards above.
-- The typed Guild owner requires this protected ABI before freezing or applying effects;
-- an older ordinary-only fence has no such capability and must refuse Guild integration.
CREATE FUNCTION game_house_disposition_guild_fence_abi() RETURNS INTEGER LANGUAGE sql IMMUTABLE AS $$ SELECT 1 $$;
REVOKE ALL ON game_house_dispositions,game_house_disposition_items,game_house_disposition_steps FROM PUBLIC;
DO $$ DECLARE n TEXT; BEGIN
 FOREACH n IN ARRAY ARRAY['game_house_disposition_immutable','game_house_disposition_content_fence','game_house_disposition_item_fence','game_house_disposition_entry_fence','game_house_disposition_delivery_proven','game_house_disposition_guild_fence_abi'] LOOP
  EXECUTE format('ALTER FUNCTION %I() SET search_path=%I,pg_temp',n,current_schema());
  EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
 END LOOP;
 EXECUTE format('ALTER FUNCTION game_house_disposition_fence_check(UUID,TEXT,UUID,BOOLEAN) SET search_path=%I,pg_temp',current_schema());
END $$;
REVOKE ALL ON FUNCTION game_house_disposition_fence_check(UUID,TEXT,UUID,BOOLEAN) FROM PUBLIC;
-- The genuine Guild participant reads this constant on its runtime transaction. This
-- grants no trigger, mutation, table or terminal-release authority to that role.
GRANT EXECUTE ON FUNCTION game_house_disposition_guild_fence_abi() TO oteryn_game_runtime;
"#;

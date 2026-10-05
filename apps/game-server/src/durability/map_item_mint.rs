//! MAP-OVERLAY-1b map-item MINT onto Ground (ADR-0021 §4.4; DUR-03 "Map
//! items and world reset"; social/map packets §1.5, §1.11 and §2.6).
//!
//! A player pickup of one eligible base-map entry mints exactly one fresh
//! live item onto Ground at the entry's own tile, in the Channel that takes
//! it, in one DUR-03 transaction with its receipt and audit event. The move
//! into the player's inventory is a separate, ordinary B3-1 TRANSFER of that
//! Ground item ([`DurabilityRoot::freeze_item_transfer`]) in its own
//! transaction. This component owns durable admission, application,
//! idempotency and reconciliation of the MINT only. It does not decide
//! whether the entry is eligible or within reach (`crate::map::overlay::pickup`
//! does, from the base map and the live overlay) nor resolve the item facts
//! from Content: the caller supplies them from the current compatible
//! Content and the base bundle.
//!
//! The cause is the actual player CommandRef and the exact entry: World,
//! Channel, base bundle digest, `placement_key` and reset epoch. The receipt
//! is unique per entry, so an entry is taken at most once per Channel and
//! reset epoch; another Channel, or a later reset epoch, can take it again.
//! Retiring the items of a reset and the reset record are not part of this
//! component.
//!
//! Lifecycle of one logical map-item MINT, keyed by its full CommandRef:
//! 1. [`DurabilityRoot::freeze_map_item_mint`] validates the intent, checks
//!    the complete current fence and that the entry is not taken, then
//!    reserves TransactionId, EventId, ItemInstanceId, the trusted timestamp
//!    and a zero DUR03-RL-08 budget in its own transaction. A refusal writes
//!    nothing. Reservations do not take the entry: only a committed receipt
//!    does, so an abandoned reservation never blocks another player.
//! 2. [`DurabilityRoot::commit_map_item_mint`] re-checks the fence and that
//!    the entry is still free under the `character_root` row lock and the
//!    entry lock, materializes the exact event bytes and commits the item,
//!    its Ground line, the receipt and the audit event together.
//! 3. [`DurabilityRoot::reconcile_map_item_mint`] reads the receipt after an
//!    unknown outcome under the current recovery fence.
//!
//! The same CommandRef returns its first outcome (the existing item), so a
//! retry never mints twice; the caller still checks reach before the
//! TRANSFER. The fence is the B3-1 TRANSFER fence
//! ([`super::item_transfer::character_item_fence_is_current`]).
//!
//! [`DurabilityRoot::read_map_item_mint_placements`] lists the taken entries
//! of one (World, Channel, digest, reset epoch) for the overlay rebuild,
//! which re-hides every one of them.

use super::character_authority::{
    ReconciledCharacterAuthority, SERVER_BUILD_ID, assert_recovery_fence,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_mint::uuid_text;
use super::item_mint_audit::{
    self as mint_audit, AuditError, ITEM_LIFECYCLE_LIVE, OneItemGroundV1, OneItemStateV1,
    RL08_RETRY_WORK_UNITS_MAX,
};
use super::item_transfer::{
    CurrentCharacterItemFence, ItemDefinitionFacts, ItemTransferError, ItemTransferRefusal,
    character_item_fence_is_current, definition_message, push_facts, push_text, scope_of,
    stack_maximum, validate_facts,
};
use super::item_transfer_audit::OneItemCommandRefV1;
use super::map_item_mint_audit::{
    self as audit, MAP_ITEM_CORPSE_REF, MAP_ITEM_MINT_TYPED_CAUSE, MapItemMintEventIdentity,
    OneItemMapItemMaterializationV1, OneItemMapItemMintCauseV1, OneItemMapItemMintV1,
    map_item_spatial_position, map_revision_of,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::domain::CharacterId;
use crate::foundation::{ChannelId, CommandRef, WorldId};
use sha2::{Digest, Sha256};
use sqlx::Row;

type Result<T> = std::result::Result<T, MapItemMintError>;
type Pass<T> = std::result::Result<std::result::Result<T, MapItemMintError>, DurabilityError>;
const INTENT_BINDING_VERSION: u8 = 1;
const EVENT_TYPE_ID: i64 = mint_audit::EVENT_TYPE_ID as i64;
const EVENT_SCHEMA_REVISION: i64 = mint_audit::EVENT_SCHEMA_REVISION as i64;
/// The only definition family a map entry mints.
const ITEM_FAMILY: &str = "Item";

/// The native tile and top-level ordinal a `placement_key` names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MapItemPlacement {
    pub x: u16,
    pub y: u16,
    pub floor: i8,
    pub ordinal: u8,
}

impl MapItemPlacement {
    /// Decode a bundle `placement_key`; `None` unless it is exactly the key
    /// `oteryn_world_bundle::bundle::placement_key` makes for its tile.
    #[must_use]
    pub fn of_key(placement_key: u64) -> Option<Self> {
        let x = u16::try_from(placement_key >> 32).ok()?;
        let y = ((placement_key >> 16) & 0xffff) as u16;
        let floor = -i8::try_from((placement_key >> 8) & 0xff).ok()?;
        let ordinal = (placement_key & 0xff) as u8;
        (oteryn_world_bundle::bundle::placement_key(floor, x, y, ordinal) == Some(placement_key))
            .then_some(Self {
                x,
                y,
                floor,
                ordinal,
            })
    }
}

/// Complete semantic intent of one map-item MINT. The World and Channel are
/// the fenced runtime scope's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MapItemMintRequest {
    /// The actual FND-02 CommandRef of the player's pickup.
    pub command: CommandRef,
    /// The digest of the World's pinned base bundle.
    pub base_bundle_digest: [u8; 32],
    /// The entry's bundle `placement_key`.
    pub placement_key: u64,
    /// The Channel's current reset epoch.
    pub reset_epoch: u64,
    /// Facts of the entry's item definition.
    pub item: ItemDefinitionFacts,
    pub quantity: u32,
    pub content_revision: String,
    pub ruleset_revision: String,
    pub sim_revision: String,
}

/// Refusal reasons. A refusal writes nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapItemMintRefusal {
    /// The entry already has a receipt in this Channel and reset epoch.
    AlreadyTaken,
    /// A container entry waits for the nested-bags decision (RL-05 = 0).
    ItemIsContainer,
    /// Unknown stack class (D82: fail closed).
    UnknownStackClass,
    /// A proven stack maximum of 0 or above GAMEITEM01-STACK-QUANTITY-MAX.
    UnsupportedStackMaximum,
    /// The quantity exceeds the definition's stack maximum.
    QuantityAboveStackMaximum,
}

/// Terminal committed result of one CommandRef: the fresh item on Ground.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedMapItemMint {
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub item_instance_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub envelope_sha256: [u8; 32],
    pub quantity: u32,
    pub placement: MapItemPlacement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapItemMintOutcome {
    Committed(CommittedMapItemMint),
    /// The CommandRef already committed; this is its original result.
    AlreadyCommitted(CommittedMapItemMint),
}

#[derive(Debug)]
pub enum MapItemMintError {
    /// Malformed or oversize input; never truncated.
    InvalidInput,
    /// A registered DUR-03 ceiling (including RL-08) was exceeded.
    CapacityExceeded,
    /// A current recovery, session, lease, scope, node or binding fence
    /// rejected the MINT.
    AuthorityRejected,
    /// The MINT is not admissible in the current state.
    Refused(MapItemMintRefusal),
    /// The CommandRef was already used with a different intent.
    ConflictingCause,
    /// A frozen identity is bound to a different cause.
    ConflictingCandidate,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for MapItemMintError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<AuditError> for MapItemMintError {
    fn from(error: AuditError) -> Self {
        match error {
            AuditError::InvalidInput => Self::InvalidInput,
            AuditError::CapacityExceeded => Self::CapacityExceeded,
        }
    }
}

/// Shared B3-1 helpers report input errors as TRANSFER errors.
fn from_transfer_input(error: ItemTransferError) -> MapItemMintError {
    match error {
        ItemTransferError::CapacityExceeded => MapItemMintError::CapacityExceeded,
        _ => MapItemMintError::InvalidInput,
    }
}

impl std::fmt::Display for MapItemMintError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid map-item MINT input"),
            Self::CapacityExceeded => formatter.write_str("map-item MINT capacity exceeded"),
            Self::AuthorityRejected => formatter.write_str("map-item MINT authority rejected"),
            Self::Refused(reason) => write!(formatter, "map-item MINT refused: {reason:?}"),
            Self::ConflictingCause => {
                formatter.write_str("map-item command was reused with different semantics")
            }
            Self::ConflictingCandidate => {
                formatter.write_str("map-item identity is bound to a different command")
            }
            Self::Unavailable(error) => {
                write!(formatter, "map-item MINT storage is unavailable: {error:?}")
            }
        }
    }
}

impl std::error::Error for MapItemMintError {}

/// Frozen candidate of one logical map-item MINT: a process-local view of its
/// durable reservation. Deliberately not `Clone`.
#[derive(Debug)]
pub struct MapItemMintCandidate {
    request: MapItemMintRequest,
    character_id: CharacterId,
    transaction_id: [u8; 16],
    event_id: [u8; 16],
    item_instance_id: [u8; 16],
    occurred_at_unix_ms: i64,
    intent_binding: [u8; 33],
    work_units_used: u8,
}

impl MapItemMintCandidate {
    #[must_use]
    pub const fn request(&self) -> &MapItemMintRequest {
        &self.request
    }
    #[must_use]
    pub const fn transaction_id(&self) -> &[u8; 16] {
        &self.transaction_id
    }
    #[must_use]
    pub const fn event_id(&self) -> &[u8; 16] {
        &self.event_id
    }
    #[must_use]
    pub const fn item_instance_id(&self) -> &[u8; 16] {
        &self.item_instance_id
    }
    #[must_use]
    pub const fn occurred_at_unix_ms(&self) -> i64 {
        self.occurred_at_unix_ms
    }
    #[must_use]
    pub const fn work_units_used(&self) -> u8 {
        self.work_units_used
    }
}

fn from_stack_refusal(refusal: ItemTransferRefusal) -> MapItemMintRefusal {
    match refusal {
        ItemTransferRefusal::UnknownStackClass => MapItemMintRefusal::UnknownStackClass,
        _ => MapItemMintRefusal::UnsupportedStackMaximum,
    }
}

/// Pure admission of the entry and its item.
pub(crate) fn plan_map_item_mint(
    item: &ItemDefinitionFacts,
    quantity: u32,
    already_taken: bool,
) -> std::result::Result<(), MapItemMintRefusal> {
    use MapItemMintRefusal as Refusal;
    if already_taken {
        return Err(Refusal::AlreadyTaken);
    }
    if item.container_capacity.is_some() {
        return Err(Refusal::ItemIsContainer);
    }
    let (maximum, _) = stack_maximum(item.stack).map_err(from_stack_refusal)?;
    if quantity == 0 || quantity > maximum {
        return Err(Refusal::QuantityAboveStackMaximum);
    }
    Ok(())
}

/// Owned copy of the frozen candidate moved into one database pass.
struct FrozenMint {
    request: MapItemMintRequest,
    character_id: CharacterId,
    transaction_id: [u8; 16],
    event_id: [u8; 16],
    item_instance_id: [u8; 16],
    occurred_at_unix_ms: i64,
    intent_binding: [u8; 33],
}

impl FrozenMint {
    fn of(candidate: &MapItemMintCandidate) -> Self {
        Self {
            request: candidate.request.clone(),
            character_id: candidate.character_id,
            transaction_id: candidate.transaction_id,
            event_id: candidate.event_id,
            item_instance_id: candidate.item_instance_id,
            occurred_at_unix_ms: candidate.occurred_at_unix_ms,
            intent_binding: candidate.intent_binding,
        }
    }
}

/// The durable reservation row of one CommandRef.
struct Reservation {
    character_id: CharacterId,
    world_id: [u8; 16],
    channel_id: [u8; 16],
    transaction_id: [u8; 16],
    event_id: [u8; 16],
    item_instance_id: [u8; 16],
    occurred_at_unix_ms: i64,
    work_units_used: u8,
}

impl DurabilityRoot {
    /// Validate the intent and reserve the logical map-item MINT of this
    /// CommandRef, or resume its reservation with the same identities and
    /// budget. A new reservation, and the resumption of one without a
    /// receipt, require the complete current fence and an entry not taken;
    /// a refusal writes nothing. This mints nothing.
    pub async fn freeze_map_item_mint(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: MapItemMintRequest,
    ) -> Result<MapItemMintCandidate> {
        validate_request(&request)?;
        let (world_id, channel_id) =
            scope_of(&fence).map_err(|_| MapItemMintError::AuthorityRejected)?;
        let intent_binding = intent_binding(&request, fence.character_id, world_id, channel_id)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| MapItemMintError::AuthorityRejected)?;
        let node = node.clone();
        let (request, reservation) = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    lock_cause(&mut tx, request.command).await?;
                    if let Some(row) = load_reservation(&mut tx, request.command).await? {
                        let stored: Vec<u8> = row.try_get("intent_binding")?;
                        if stored != intent_binding {
                            return Ok(Err(MapItemMintError::ConflictingCause));
                        }
                        let reservation = decode_reservation(&row)?;
                        if load_receipt(&mut tx, request.command).await?.is_none()
                            && let Err(error) =
                                admit(&mut tx, &node, &fence, &request, &reservation).await?
                        {
                            return Ok(Err(error));
                        }
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok((request, reservation)));
                    }
                    let row = sqlx::query(
                        "SELECT game_character_uuid_v7()::text AS transaction_id, \
                                game_character_uuid_v7()::text AS event_id, \
                                game_character_uuid_v7()::text AS item_instance_id, \
                                floor(extract(epoch FROM statement_timestamp())*1000)::bigint \
                                  AS occurred_at",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    let reservation = Reservation {
                        character_id: fence.character_id,
                        world_id: *world_id.as_bytes(),
                        channel_id: *channel_id.as_bytes(),
                        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
                        event_id: uuid_text(row.try_get("event_id")?)?,
                        item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
                        occurred_at_unix_ms: row.try_get("occurred_at")?,
                        work_units_used: 0,
                    };
                    if let Err(error) =
                        admit(&mut tx, &node, &fence, &request, &reservation).await?
                    {
                        return Ok(Err(error));
                    }
                    insert_reservation(&mut tx, &request, &intent_binding, &reservation).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok((request, reservation)))
                })
            })
            .await??;
        Ok(MapItemMintCandidate {
            request,
            character_id: reservation.character_id,
            transaction_id: reservation.transaction_id,
            event_id: reservation.event_id,
            item_instance_id: reservation.item_instance_id,
            occurred_at_unix_ms: reservation.occurred_at_unix_ms,
            intent_binding,
            work_units_used: reservation.work_units_used,
        })
    }

    /// Durably charge one DUR03-RL-08 work unit to the reservation in its own
    /// committed transaction before the pass does any work.
    async fn charge_map_item_mint_work_unit(
        &self,
        recovery: CharacterRecoveryFenceV1,
        candidate: &mut MapItemMintCandidate,
    ) -> Result<()> {
        if candidate.work_units_used >= RL08_RETRY_WORK_UNITS_MAX {
            return Err(MapItemMintError::CapacityExceeded);
        }
        let command = candidate.request.command;
        let transaction_id = candidate.transaction_id;
        let charged = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let used = sqlx::query(
                        "UPDATE game_map_item_mint_reservations \
                            SET work_units_used = work_units_used + 1 \
                          WHERE game_session_id = encode($1,'hex')::uuid \
                            AND command_id = $2::text::numeric(20,0) \
                            AND transaction_id = encode($3,'hex')::uuid \
                            AND work_units_used < $4 RETURNING work_units_used",
                    )
                    .bind(command.game_session_id().as_bytes().as_slice())
                    .bind(command.command_id().get().to_string())
                    .bind(transaction_id.as_slice())
                    .bind(i16::from(RL08_RETRY_WORK_UNITS_MAX))
                    .fetch_optional(&mut *tx)
                    .await?;
                    let Some(used) = used else {
                        let reserved = sqlx::query(
                            "SELECT 1 FROM game_map_item_mint_reservations \
                              WHERE game_session_id = encode($1,'hex')::uuid \
                                AND command_id = $2::text::numeric(20,0) \
                                AND transaction_id = encode($3,'hex')::uuid",
                        )
                        .bind(command.game_session_id().as_bytes().as_slice())
                        .bind(command.command_id().get().to_string())
                        .bind(transaction_id.as_slice())
                        .fetch_optional(&mut *tx)
                        .await?;
                        return Ok(Err(if reserved.is_some() {
                            MapItemMintError::CapacityExceeded
                        } else {
                            MapItemMintError::ConflictingCandidate
                        }));
                    };
                    let used = u8::try_from(used.try_get::<i16, _>("work_units_used")?)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(used))
                })
            })
            .await?;
        match charged {
            Ok(used) => {
                candidate.work_units_used = used;
                Ok(())
            }
            Err(MapItemMintError::CapacityExceeded) => {
                candidate.work_units_used = RL08_RETRY_WORK_UNITS_MAX;
                Err(MapItemMintError::CapacityExceeded)
            }
            Err(error) => Err(error),
        }
    }

    /// Commit one frozen map-item MINT. A CommandRef that already committed
    /// returns its original result (the existing item) under the current
    /// recovery fence without reacquiring session authority; a changed intent
    /// conflicts. A new MINT commits only under the complete current fence
    /// and with the entry still free.
    pub async fn commit_map_item_mint(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        candidate: &mut MapItemMintCandidate,
    ) -> Result<MapItemMintOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| MapItemMintError::AuthorityRejected)?;
        self.charge_map_item_mint_work_unit(recovery.clone(), candidate)
            .await?;
        let frozen = FrozenMint::of(candidate);
        let node = node.clone();

        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let command = frozen.request.command;
                    lock_cause(&mut tx, command).await?;

                    if let Some(row) = load_receipt(&mut tx, command).await? {
                        let stored: Vec<u8> = row.try_get("intent_binding")?;
                        if stored != frozen.intent_binding {
                            return Ok(Err(MapItemMintError::ConflictingCause));
                        }
                        let committed = decode_receipt(&row)?;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(MapItemMintOutcome::AlreadyCommitted(committed)));
                    }

                    let Some(row) = load_reservation(&mut tx, command).await? else {
                        return Ok(Err(MapItemMintError::ConflictingCandidate));
                    };
                    let stored: Vec<u8> = row.try_get("intent_binding")?;
                    if stored != frozen.intent_binding {
                        return Ok(Err(MapItemMintError::ConflictingCause));
                    }
                    let reservation = decode_reservation(&row)?;
                    if reservation.transaction_id != frozen.transaction_id
                        || reservation.event_id != frozen.event_id
                        || reservation.item_instance_id != frozen.item_instance_id
                        || reservation.occurred_at_unix_ms != frozen.occurred_at_unix_ms
                        || reservation.character_id != frozen.character_id
                    {
                        return Ok(Err(MapItemMintError::ConflictingCandidate));
                    }
                    let identity_reused: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM game_map_item_mint_receipts \
                                         WHERE transaction_id = encode($1,'hex')::uuid \
                                            OR event_id = encode($2,'hex')::uuid \
                                            OR item_instance_id = encode($3,'hex')::uuid) \
                             OR EXISTS (SELECT 1 FROM game_item_audit_outbox \
                                         WHERE transaction_id = encode($1,'hex')::uuid \
                                            OR event_id = encode($2,'hex')::uuid) \
                             OR EXISTS (SELECT 1 FROM game_item_instances \
                                         WHERE item_instance_id = encode($3,'hex')::uuid \
                                            OR minted_transaction_id = encode($1,'hex')::uuid)",
                    )
                    .bind(frozen.transaction_id.as_slice())
                    .bind(frozen.event_id.as_slice())
                    .bind(frozen.item_instance_id.as_slice())
                    .fetch_one(&mut *tx)
                    .await?;
                    if identity_reused {
                        return Ok(Err(MapItemMintError::ConflictingCandidate));
                    }

                    if let Err(error) =
                        admit(&mut tx, &node, &fence, &frozen.request, &reservation).await?
                    {
                        return Ok(Err(error));
                    }
                    let ground = ground_of(&frozen.request, &reservation, &fence);
                    let message = mint_message(&frozen, &reservation, &fence, ground.clone());
                    let envelope = match audit::encode_map_item_mint_event(
                        MapItemMintEventIdentity {
                            event_id: frozen.event_id,
                            transaction_id: frozen.transaction_id,
                            occurred_at_unix_ms: frozen.occurred_at_unix_ms,
                            server_build_id: SERVER_BUILD_ID,
                        },
                        message,
                    ) {
                        Ok(envelope) => envelope,
                        Err(error) => return Ok(Err(error.into())),
                    };
                    let committed =
                        apply_mint(&mut tx, &frozen, &reservation, &ground, &envelope).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(MapItemMintOutcome::Committed(committed)))
                })
            })
            .await?
    }

    /// Resolve an unknown outcome. The cause lock waits for any in-flight
    /// attempt, so `None` proves nothing committed for this CommandRef and the
    /// same candidate may be retried. Never reacquires session authority.
    pub async fn reconcile_map_item_mint(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        candidate: &mut MapItemMintCandidate,
    ) -> Result<Option<CommittedMapItemMint>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| MapItemMintError::AuthorityRejected)?;
        self.charge_map_item_mint_work_unit(recovery.clone(), candidate)
            .await?;
        let command = candidate.request.command;
        let binding = candidate.intent_binding;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_cause(&mut tx, command).await?;
                    let Some(row) = load_receipt(&mut tx, command).await? else {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(None));
                    };
                    let stored: Vec<u8> = row.try_get("intent_binding")?;
                    if stored != binding {
                        return Ok(Err(MapItemMintError::ConflictingCause));
                    }
                    let committed = decode_receipt(&row)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(Some(committed)))
                })
            })
            .await?
    }

    /// The `placement_key` of every entry taken in this World, Channel, base
    /// bundle and reset epoch, ascending: the origins the overlay rebuild
    /// re-hides. Read-only. More than `max_entries` (the bundle's entry
    /// count) is an invalid state, reported as `CapacityExceeded`, never
    /// truncated.
    pub async fn read_map_item_mint_placements(
        &self,
        world_id: WorldId,
        channel_id: ChannelId,
        base_bundle_digest: [u8; 32],
        reset_epoch: u64,
        max_entries: usize,
    ) -> Result<Vec<u64>> {
        let limit = i64::try_from(max_entries.saturating_add(1)).unwrap_or(i64::MAX);
        let keys = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let keys: Vec<String> = sqlx::query_scalar(
                        "SELECT placement_key::text FROM game_map_item_mint_receipts \
                          WHERE world_id = encode($1,'hex')::uuid \
                            AND channel_id = encode($2,'hex')::uuid \
                            AND base_bundle_digest = $3 \
                            AND reset_epoch = $4::text::numeric(20,0) \
                          ORDER BY placement_key LIMIT $5",
                    )
                    .bind(world_id.as_bytes().as_slice())
                    .bind(channel_id.as_bytes().as_slice())
                    .bind(base_bundle_digest.as_slice())
                    .bind(reset_epoch.to_string())
                    .bind(limit)
                    .fetch_all(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(keys)
                })
            })
            .await?;
        if keys.len() > max_entries {
            return Err(MapItemMintError::CapacityExceeded);
        }
        keys.iter()
            .map(|key| {
                key.parse::<u64>()
                    .map_err(|_| DurabilityError::InvalidStoredState.into())
            })
            .collect()
    }
}

/// The complete current fence (shared with B3-1 TRANSFER, ending with the
/// `character_root` row lock), then the entry lock and whether the entry is
/// already taken, then the item admission. Writes no row.
async fn admit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    request: &MapItemMintRequest,
    reservation: &Reservation,
) -> Pass<()> {
    if !character_item_fence_is_current(
        tx,
        node,
        fence,
        request.command,
        reservation.character_id,
        reservation.world_id,
        reservation.channel_id,
    )
    .await?
    {
        return Ok(Err(MapItemMintError::AuthorityRejected));
    }
    lock_entry(tx, request, reservation).await?;
    let already_taken: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_map_item_mint_receipts \
                         WHERE world_id = encode($1,'hex')::uuid \
                           AND channel_id = encode($2,'hex')::uuid \
                           AND base_bundle_digest = $3 \
                           AND placement_key = $4::text::numeric(20,0) \
                           AND reset_epoch = $5::text::numeric(20,0))",
    )
    .bind(reservation.world_id.as_slice())
    .bind(reservation.channel_id.as_slice())
    .bind(request.base_bundle_digest.as_slice())
    .bind(request.placement_key.to_string())
    .bind(request.reset_epoch.to_string())
    .fetch_one(&mut **tx)
    .await?;
    Ok(
        plan_map_item_mint(&request.item, request.quantity, already_taken)
            .map_err(MapItemMintError::Refused),
    )
}

/// The Ground line the MINT places: the entry's own tile, in the fenced
/// Channel and scope ownership generation, framed by the base bundle.
fn ground_of(
    request: &MapItemMintRequest,
    reservation: &Reservation,
    fence: &CurrentCharacterItemFence,
) -> OneItemGroundV1 {
    // `validate_request` proved the key decodes.
    let placement = MapItemPlacement::of_key(request.placement_key).unwrap_or(MapItemPlacement {
        x: 0,
        y: 0,
        floor: 0,
        ordinal: 0,
    });
    OneItemGroundV1 {
        world_id: reservation.world_id.to_vec(),
        channel_id: reservation.channel_id.to_vec(),
        spatial_position: map_item_spatial_position(placement.x, placement.y, placement.floor),
        corpse_ref: MAP_ITEM_CORPSE_REF.to_vec(),
        map_revision: map_revision_of(&request.base_bundle_digest),
        content_revision: request.content_revision.clone(),
        native_room_placement_context: request.base_bundle_digest.to_vec(),
        runtime_scope_ownership_generation: fence.scope_ownership_generation.get(),
    }
}

/// The complete typed after-state evidence of the admitted MINT.
fn mint_message(
    frozen: &FrozenMint,
    reservation: &Reservation,
    fence: &CurrentCharacterItemFence,
    ground: OneItemGroundV1,
) -> OneItemMapItemMintV1 {
    let request = &frozen.request;
    OneItemMapItemMintV1 {
        after: Some(OneItemStateV1 {
            item_instance_id: frozen.item_instance_id.to_vec(),
            world_id: reservation.world_id.to_vec(),
            definition: Some(definition_message(&request.item.definition)),
            quantity: request.quantity,
            lifecycle: ITEM_LIFECYCLE_LIVE,
        }),
        destination: Some(ground),
        source: Some(OneItemMapItemMintCauseV1 {
            typed_cause: MAP_ITEM_MINT_TYPED_CAUSE.into(),
            content_revision: request.content_revision.clone(),
            ruleset_revision: request.ruleset_revision.clone(),
            sim_revision: request.sim_revision.clone(),
            command_ref: Some(OneItemCommandRefV1 {
                game_session_id: request.command.game_session_id().as_bytes().to_vec(),
                command_id: request.command.command_id().get(),
            }),
            materialization: Some(OneItemMapItemMaterializationV1 {
                world_id: reservation.world_id.to_vec(),
                channel_id: reservation.channel_id.to_vec(),
                base_bundle_digest: request.base_bundle_digest.to_vec(),
                placement_key: request.placement_key,
                reset_epoch: request.reset_epoch,
            }),
        }),
        before_semantically_absent: true,
        connection_generation: fence.connection_generation.get(),
    }
}

/// Apply the item, its Ground line, the audit event and the receipt; the
/// deferred guards prove they commit together.
async fn apply_mint(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenMint,
    reservation: &Reservation,
    ground: &OneItemGroundV1,
    envelope: &[u8],
) -> std::result::Result<CommittedMapItemMint, DurabilityError> {
    let request = &frozen.request;
    let placement = MapItemPlacement::of_key(request.placement_key)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let tx_id = frozen.transaction_id.as_slice();
    let item_id = frozen.item_instance_id.as_slice();
    sqlx::query(
        "INSERT INTO game_item_instances(item_instance_id, world_id, definition_family, \
           definition_production_key, definition_revision_ref, quantity, lifecycle, \
           minted_transaction_id) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, $4, $5, $6, 1, \
           encode($7,'hex')::uuid)",
    )
    .bind(item_id)
    .bind(reservation.world_id.as_slice())
    .bind(&request.item.definition.family)
    .bind(&request.item.definition.production_key)
    .bind(&request.item.definition.revision_ref)
    .bind(i64::from(request.quantity))
    .bind(tx_id)
    .execute(&mut **tx)
    .await?;
    let generation = ground.runtime_scope_ownership_generation.to_string();
    sqlx::query(
        "INSERT INTO game_item_ground_locations(item_instance_id, world_id, channel_id, \
           runtime_scope_ownership_generation, spatial_position, corpse_ref, map_revision, \
           content_revision, native_room_placement_context) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
           $4::text::numeric(20,0), $5, $6, $7, $8, $9)",
    )
    .bind(item_id)
    .bind(reservation.world_id.as_slice())
    .bind(reservation.channel_id.as_slice())
    .bind(&generation)
    .bind(ground.spatial_position.as_slice())
    .bind(ground.corpse_ref.as_slice())
    .bind(&ground.map_revision)
    .bind(&ground.content_revision)
    .bind(ground.native_room_placement_context.as_slice())
    .execute(&mut **tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_item_audit_outbox(event_id, transaction_id, transaction_ordinal, \
           transaction_count, event_type_id, schema_revision, retention_profile_id, \
           item_instance_id, occurred_at, expires_at, envelope, envelope_sha256, \
           publication_state) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, 1, $3, $4, $5, \
           encode($6,'hex')::uuid, $7, $7 + $8, $9, sha256($9), 1)",
    )
    .bind(frozen.event_id.as_slice())
    .bind(tx_id)
    .bind(EVENT_TYPE_ID)
    .bind(EVENT_SCHEMA_REVISION)
    .bind(mint_audit::RETENTION_PROFILE_ID)
    .bind(item_id)
    .bind(frozen.occurred_at_unix_ms)
    .bind(mint_audit::AUDIT_RETENTION_P90D_MS)
    .bind(envelope)
    .execute(&mut **tx)
    .await?;
    let committed_at: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let envelope_sha256: [u8; 32] = Sha256::digest(envelope).into();
    let command = request.command;
    sqlx::query(
        "INSERT INTO game_map_item_mint_receipts(game_session_id, command_id, character_id, \
           world_id, channel_id, base_bundle_digest, placement_key, reset_epoch, \
           intent_binding, transaction_id, event_id, item_instance_id, definition_family, \
           definition_production_key, definition_revision_ref, quantity, \
           runtime_scope_ownership_generation, spatial_position, map_revision, \
           content_revision, occurred_at, envelope_sha256, committed_at) \
         VALUES (encode($1,'hex')::uuid, $2::text::numeric(20,0), encode($3,'hex')::uuid, \
           encode($4,'hex')::uuid, encode($5,'hex')::uuid, $6, $7::text::numeric(20,0), \
           $8::text::numeric(20,0), $9, encode($10,'hex')::uuid, encode($11,'hex')::uuid, \
           encode($12,'hex')::uuid, $13, $14, $15, $16, $17::text::numeric(20,0), $18, $19, \
           $20, $21, $22, $23)",
    )
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .bind(frozen.character_id.as_bytes().as_slice())
    .bind(reservation.world_id.as_slice())
    .bind(reservation.channel_id.as_slice())
    .bind(request.base_bundle_digest.as_slice())
    .bind(request.placement_key.to_string())
    .bind(request.reset_epoch.to_string())
    .bind(frozen.intent_binding.as_slice())
    .bind(tx_id)
    .bind(frozen.event_id.as_slice())
    .bind(item_id)
    .bind(&request.item.definition.family)
    .bind(&request.item.definition.production_key)
    .bind(&request.item.definition.revision_ref)
    .bind(i64::from(request.quantity))
    .bind(&generation)
    .bind(ground.spatial_position.as_slice())
    .bind(&ground.map_revision)
    .bind(&ground.content_revision)
    .bind(frozen.occurred_at_unix_ms)
    .bind(envelope_sha256.as_slice())
    .bind(committed_at)
    .execute(&mut **tx)
    .await?;
    Ok(CommittedMapItemMint {
        transaction_id: frozen.transaction_id,
        event_id: frozen.event_id,
        item_instance_id: frozen.item_instance_id,
        occurred_at_unix_ms: frozen.occurred_at_unix_ms,
        envelope_sha256,
        quantity: request.quantity,
        placement,
    })
}

async fn lock_cause(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: CommandRef,
) -> std::result::Result<(), DurabilityError> {
    let mut key = Vec::with_capacity(24);
    key.extend_from_slice(command.game_session_id().as_bytes());
    key.extend_from_slice(&command.command_id().get().to_be_bytes());
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:map-item-mint:' || encode($1, 'hex'), 0))",
    )
    .bind(key.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Serializes every admission of one entry (World, Channel, digest,
/// `placement_key`, reset epoch), taken after the cause lock.
async fn lock_entry(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &MapItemMintRequest,
    reservation: &Reservation,
) -> std::result::Result<(), DurabilityError> {
    let mut key = Vec::with_capacity(96);
    key.extend_from_slice(&reservation.world_id);
    key.extend_from_slice(&reservation.channel_id);
    key.extend_from_slice(&request.base_bundle_digest);
    key.extend_from_slice(&request.placement_key.to_be_bytes());
    key.extend_from_slice(&request.reset_epoch.to_be_bytes());
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:map-item-entry:' || encode($1, 'hex'), 0))",
    )
    .bind(key.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn load_reservation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: CommandRef,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT intent_binding, character_id::text, world_id::text, channel_id::text, \
                transaction_id::text, event_id::text, item_instance_id::text, occurred_at, \
                work_units_used \
           FROM game_map_item_mint_reservations \
          WHERE game_session_id = encode($1,'hex')::uuid \
            AND command_id = $2::text::numeric(20,0)",
    )
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_reservation(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<Reservation, DurabilityError> {
    let invalid = |_| DurabilityError::InvalidStoredState;
    Ok(Reservation {
        character_id: CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        world_id: uuid_text(row.try_get("world_id")?)?,
        channel_id: uuid_text(row.try_get("channel_id")?)?,
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
        occurred_at_unix_ms: row.try_get("occurred_at")?,
        work_units_used: u8::try_from(row.try_get::<i16, _>("work_units_used")?)
            .map_err(invalid)?,
    })
}

async fn insert_reservation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &MapItemMintRequest,
    intent_binding: &[u8; 33],
    reservation: &Reservation,
) -> std::result::Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_map_item_mint_reservations(game_session_id, command_id, \
           character_id, world_id, channel_id, base_bundle_digest, placement_key, \
           reset_epoch, intent_binding, transaction_id, event_id, item_instance_id, \
           occurred_at, work_units_used, reserved_at) \
         VALUES (encode($1,'hex')::uuid, $2::text::numeric(20,0), encode($3,'hex')::uuid, \
           encode($4,'hex')::uuid, encode($5,'hex')::uuid, $6, $7::text::numeric(20,0), \
           $8::text::numeric(20,0), $9, encode($10,'hex')::uuid, encode($11,'hex')::uuid, \
           encode($12,'hex')::uuid, $13, 0, \
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint)",
    )
    .bind(request.command.game_session_id().as_bytes().as_slice())
    .bind(request.command.command_id().get().to_string())
    .bind(reservation.character_id.as_bytes().as_slice())
    .bind(reservation.world_id.as_slice())
    .bind(reservation.channel_id.as_slice())
    .bind(request.base_bundle_digest.as_slice())
    .bind(request.placement_key.to_string())
    .bind(request.reset_epoch.to_string())
    .bind(intent_binding.as_slice())
    .bind(reservation.transaction_id.as_slice())
    .bind(reservation.event_id.as_slice())
    .bind(reservation.item_instance_id.as_slice())
    .bind(reservation.occurred_at_unix_ms)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn load_receipt(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: CommandRef,
) -> std::result::Result<Option<sqlx::postgres::PgRow>, DurabilityError> {
    Ok(sqlx::query(
        "SELECT intent_binding, transaction_id::text, event_id::text, \
                item_instance_id::text, quantity, placement_key::text, occurred_at, \
                envelope_sha256 \
           FROM game_map_item_mint_receipts \
          WHERE game_session_id = encode($1,'hex')::uuid \
            AND command_id = $2::text::numeric(20,0)",
    )
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .fetch_optional(&mut **tx)
    .await?)
}

fn decode_receipt(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<CommittedMapItemMint, DurabilityError> {
    let digest: Vec<u8> = row.try_get("envelope_sha256")?;
    let placement_key: u64 = row
        .try_get::<String, _>("placement_key")?
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    Ok(CommittedMapItemMint {
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
        occurred_at_unix_ms: row.try_get("occurred_at")?,
        envelope_sha256: digest
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        placement: MapItemPlacement::of_key(placement_key)
            .ok_or(DurabilityError::InvalidStoredState)?,
    })
}

fn validate_request(request: &MapItemMintRequest) -> Result<()> {
    validate_facts(&request.item).map_err(from_transfer_input)?;
    if request.item.definition.family != ITEM_FAMILY
        || request.quantity == 0
        || MapItemPlacement::of_key(request.placement_key).is_none()
    {
        return Err(MapItemMintError::InvalidInput);
    }
    for revision in [
        &request.content_revision,
        &request.ruleset_revision,
        &request.sim_revision,
    ] {
        mint_audit::check_content_key(revision)?;
    }
    Ok(())
}

/// Version byte plus SHA-256 over the complete intent: the CommandRef, the
/// fenced Character, World and Channel, the entry (digest, placement key,
/// reset epoch), the item facts and quantity and the interpretation
/// revisions. The connection generation is not part of the intent (DUR-03
/// §31).
fn intent_binding(
    request: &MapItemMintRequest,
    character_id: CharacterId,
    world_id: WorldId,
    channel_id: ChannelId,
) -> Result<[u8; 33]> {
    let mut canonical = Vec::new();
    canonical.extend_from_slice(request.command.game_session_id().as_bytes());
    canonical.extend_from_slice(&request.command.command_id().get().to_be_bytes());
    canonical.extend_from_slice(character_id.as_bytes());
    canonical.extend_from_slice(world_id.as_bytes());
    canonical.extend_from_slice(channel_id.as_bytes());
    canonical.extend_from_slice(&request.base_bundle_digest);
    canonical.extend_from_slice(&request.placement_key.to_be_bytes());
    canonical.extend_from_slice(&request.reset_epoch.to_be_bytes());
    push_facts(&mut canonical, &request.item).map_err(from_transfer_input)?;
    canonical.extend_from_slice(&request.quantity.to_be_bytes());
    for revision in [
        &request.content_revision,
        &request.ruleset_revision,
        &request.sim_revision,
    ] {
        push_text(&mut canonical, revision.as_bytes()).map_err(from_transfer_input)?;
    }
    let mut out = [0_u8; 33];
    out[0] = INTENT_BINDING_VERSION;
    out[1..].copy_from_slice(&Sha256::digest(&canonical));
    Ok(out)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::durability::item_mint::TypedDefinitionRef;
    use crate::durability::item_transfer::ItemStackClass;

    fn id(seed: u8) -> [u8; 16] {
        [
            seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
        ]
    }

    fn facts(key: &str, stack: ItemStackClass) -> ItemDefinitionFacts {
        ItemDefinitionFacts {
            definition: TypedDefinitionRef {
                family: ITEM_FAMILY.into(),
                production_key: key.into(),
                revision_ref: "definition-r1".into(),
            },
            stack,
            container_capacity: None,
            container_slot_equip_pattern: false,
        }
    }

    const COIN: ItemStackClass = ItemStackClass::Stackable {
        proven_maximum: None,
    };

    fn key(floor: i8, x: u16, y: u16, ordinal: u8) -> u64 {
        oteryn_world_bundle::bundle::placement_key(floor, x, y, ordinal).expect("key")
    }

    fn request() -> MapItemMintRequest {
        use crate::foundation::{CommandId, GameSessionId};
        MapItemMintRequest {
            command: CommandRef::new(
                GameSessionId::decode(&id(50)).expect("session"),
                CommandId::new(7).expect("command"),
            ),
            base_bundle_digest: [0xab; 32],
            placement_key: key(-7, 32000, 31000, 2),
            reset_epoch: 0,
            item: facts("fixture:map.coin", COIN),
            quantity: 3,
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        }
    }

    #[test]
    fn map_item_mint_placement_keys_round_trip_and_refuse_every_other_key() {
        for (floor, x, y, ordinal) in [(0, 0, 0, 0), (-15, 65535, 65535, 63), (-7, 32000, 31000, 2)]
        {
            let placement = MapItemPlacement::of_key(key(floor, x, y, ordinal)).expect("decodes");
            assert_eq!(
                placement,
                MapItemPlacement {
                    x,
                    y,
                    floor,
                    ordinal
                }
            );
        }
        // An ordinal of 64 or more, a floor beyond -15 or bits above x.
        for bad in [64, 16 << 8, 1 << 48, u64::MAX] {
            assert_eq!(MapItemPlacement::of_key(bad), None);
        }
    }

    #[test]
    fn map_item_mint_every_refusal_is_typed() {
        use MapItemMintRefusal as R;
        let coin = facts("fixture:map.coin", COIN);
        let stone = facts("fixture:map.stone", ItemStackClass::NonStackable);
        assert_eq!(plan_map_item_mint(&coin, 1, false), Ok(()));
        assert_eq!(plan_map_item_mint(&coin, 100, false), Ok(()));
        assert_eq!(plan_map_item_mint(&stone, 1, false), Ok(()));
        // A taken entry is refused before any item check.
        assert_eq!(plan_map_item_mint(&coin, 1, true), Err(R::AlreadyTaken));
        assert_eq!(plan_map_item_mint(&coin, 0, true), Err(R::AlreadyTaken));
        let mut bag = facts("fixture:map.bag", ItemStackClass::NonStackable);
        bag.container_capacity = Some(8);
        assert_eq!(plan_map_item_mint(&bag, 1, false), Err(R::ItemIsContainer));
        assert_eq!(
            plan_map_item_mint(&facts("u", ItemStackClass::Unknown), 1, false),
            Err(R::UnknownStackClass)
        );
        let oversize = ItemStackClass::Stackable {
            proven_maximum: Some(101),
        };
        assert_eq!(
            plan_map_item_mint(&facts("u", oversize), 1, false),
            Err(R::UnsupportedStackMaximum)
        );
        assert_eq!(
            plan_map_item_mint(&stone, 2, false),
            Err(R::QuantityAboveStackMaximum)
        );
        assert_eq!(
            plan_map_item_mint(&coin, 101, false),
            Err(R::QuantityAboveStackMaximum)
        );
    }

    #[test]
    fn map_item_mint_request_is_validated() {
        assert!(validate_request(&request()).is_ok());
        let invalid = [
            MapItemMintRequest {
                quantity: 0,
                ..request()
            },
            MapItemMintRequest {
                placement_key: 64,
                ..request()
            },
            MapItemMintRequest {
                item: ItemDefinitionFacts {
                    definition: TypedDefinitionRef {
                        family: "RewardClaim".into(),
                        ..request().item.definition
                    },
                    ..request().item
                },
                ..request()
            },
            MapItemMintRequest {
                content_revision: String::new(),
                ..request()
            },
        ];
        for request in invalid {
            assert!(matches!(
                validate_request(&request),
                Err(MapItemMintError::InvalidInput)
            ));
        }
    }

    #[test]
    fn map_item_mint_intent_binds_every_field_of_the_cause() {
        use crate::foundation::{ChannelId, WorldId};
        let character = CharacterId::from_bytes(id(41)).expect("character");
        let world = WorldId::decode(&id(42)).expect("world");
        let channel = ChannelId::decode(&id(43)).expect("channel");
        let bind = |request: &MapItemMintRequest| {
            intent_binding(request, character, world, channel).expect("binding")
        };
        let base = bind(&request());
        assert_eq!(base[0], INTENT_BINDING_VERSION);
        let changed = [
            MapItemMintRequest {
                base_bundle_digest: [0xac; 32],
                ..request()
            },
            MapItemMintRequest {
                placement_key: key(-7, 32000, 31000, 3),
                ..request()
            },
            MapItemMintRequest {
                reset_epoch: 1,
                ..request()
            },
            MapItemMintRequest {
                quantity: 4,
                ..request()
            },
            MapItemMintRequest {
                item: facts("fixture:map.other", COIN),
                ..request()
            },
            MapItemMintRequest {
                content_revision: "content-2".into(),
                ..request()
            },
        ];
        let mut bindings = vec![base];
        bindings.extend(changed.iter().map(bind));
        let other_channel = ChannelId::decode(&id(44)).expect("channel");
        bindings
            .push(intent_binding(&request(), character, world, other_channel).expect("binding"));
        let other_character = CharacterId::from_bytes(id(45)).expect("character");
        bindings
            .push(intent_binding(&request(), other_character, world, channel).expect("binding"));
        for (index, binding) in bindings.iter().enumerate() {
            assert!(bindings[index + 1..].iter().all(|other| other != binding));
        }
    }
}

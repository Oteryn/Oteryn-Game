//! DUR-03 TRANSFER from Ground into the equipped main backpack (B3 decision
//! `B3-INVENTORY-DESTINATION-CAPACITY-STACKS-V1`, D80-D83; child B3-1).
//!
//! This component owns durable admission, application, idempotency and
//! reconciliation. It does not prove that the player command was legitimately
//! ingested (the runtime owner's FND-02 `CommandIngress` holds the pending
//! [`CommandRef`]) nor resolve the item definition facts from Content: the
//! caller supplies [`ItemDefinitionFacts`] from the current compatible
//! Content, and this component checks them against the stored items.
//!
//! Destinations (D80): the character's CharacterEquipment `container` slot
//! (an empty container whose definition declares a complete `container`-slot
//! equip pattern, into an empty slot) and direct entries of the item in that
//! slot, the main backpack. An entry's placement ordinal is the highest
//! existing ordinal plus one, taken under the `character_root` row lock; the
//! display order is newest first and nothing is renumbered.
//!
//! Capacity (D81): an entry-placing TRANSFER needs
//! `current_entry_count < definition_capacity`; weight is not checked here.
//! Stacks (D82): the stack maximum is the definition's proven value or 100,
//! never above 100; an unknown stack class is refused. Merge (D83): the
//! receiver is the compatible stack with room and the highest ordinal; full
//! merge retires the source, top-up keeps the remainder in a new entry, and
//! a top-up with no free entry is refused with nothing moved.
//!
//! Lifecycle of one logical TRANSFER, keyed by its full CommandRef:
//! 1. [`DurabilityRoot::freeze_item_transfer`] validates the intent, checks the
//!    complete current fence and that the TRANSFER is admissible now, then
//!    reserves TransactionId, EventId, the trusted timestamp and a zero
//!    DUR03-RL-08 budget in its own transaction. A refusal writes nothing.
//! 2. [`DurabilityRoot::commit_item_transfer`] re-checks the fence, rereads
//!    the authoritative before-state under the `character_root` row lock,
//!    materializes the plan and the exact event bytes, and commits every item,
//!    location, receipt and audit row together. After a proven non-commit the
//!    same TransactionId may re-materialize for the same intent (DUR-03
//!    §23.2); after commit the receipt and event bytes are terminal.
//! 3. [`DurabilityRoot::reconcile_item_transfer`] reads the receipt after an
//!    unknown outcome under the current recovery fence.
//!
//! The fence is the Character XP writer's complete fence with the CommandRef
//! in place of the XP occurrence (CHARACTER-REVISION-ITEM-TRANSACTION-
//! COMPOSITION-V1): recovery fence and admission-relation locks, the cause
//! lock and replay, then the reconnect-session row, the runtime-scope
//! assignment and current node incarnation, and the admission guards; the
//! CommandRef's GameSession is the fenced one and the fenced scope owns the
//! source Ground. `CharacterRevision` is never read as a fence nor written.

use super::character_authority::{
    ReconciledCharacterAuthority, SERVER_BUILD_ID, assert_recovery_fence,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_mint::{TypedDefinitionRef, uuid_text};
use super::item_mint_audit::{
    self as mint_audit, AuditError, ITEM_LIFECYCLE_LIVE, OneItemGroundV1, OneItemStateV1,
    OneItemTypedDefinitionRevisionV1, RL08_RETRY_WORK_UNITS_MAX,
};
use super::item_transfer_audit::{
    self as audit, GROUND_PICKUP_TYPED_CAUSE, ITEM_LIFECYCLE_RETIRED, OneItemCommandRefV1,
    OneItemContainerEntryV1, OneItemInventoryV1, OneItemReceiverV1, OneItemTransferCauseV1,
    OneItemTransferV1, TransferEventIdentity,
};
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation, scope_key};
use super::{DurabilityError, DurabilityRoot};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::domain::CharacterId;
use crate::foundation::{
    ChannelId, CommandRef, ConnectionGeneration, GameSessionId, RuntimeScopeRefV1,
    ScopeOwnershipGeneration, WorldId,
};
use sha2::{Digest, Sha256};
use sqlx::Row;

pub use super::item_transfer_audit::TransferShape;

type Result<T> = std::result::Result<T, ItemTransferError>;
type Pass<T> = std::result::Result<std::result::Result<T, ItemTransferError>, DurabilityError>;
const INTENT_BINDING_VERSION: u8 = 1;
const EVENT_TYPE_ID: i64 = mint_audit::EVENT_TYPE_ID as i64;
const EVENT_SCHEMA_REVISION: i64 = mint_audit::EVENT_SCHEMA_REVISION as i64;

/// GAMEITEM01-STACK-QUANTITY-MAX (D82).
pub const GAMEITEM01_STACK_QUANTITY_MAX: u32 = audit::GAMEITEM01_STACK_QUANTITY_MAX;
/// GAMEITEM01-CONTAINER-ENTRIES-MAX: a main backpack with a larger declared
/// capacity waits for a new decision.
pub const GAMEITEM01_CONTAINER_ENTRIES_MAX: u32 = 20;
/// GAMEITEM01-PLACEMENT-DEPTH: only direct entries of the main backpack.
pub const GAMEITEM01_PLACEMENT_DEPTH: u32 = 1;
/// GAMEITEM01-REACHABLE-ITEMS: the main backpack and its entries.
pub const GAMEITEM01_REACHABLE_ITEMS: u32 = 1 + GAMEITEM01_CONTAINER_ENTRIES_MAX;

/// Current gameplay authority supplied by the runtime owner at commit time.
/// Unlike the XP fence it carries no `expected_character_revision`: an
/// inventory-only TRANSFER neither reads nor advances CharacterRevision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CurrentCharacterItemFence {
    pub character_id: CharacterId,
    pub game_session_id: GameSessionId,
    pub connection_generation: ConnectionGeneration,
    pub character_lease_generation: u64,
    pub runtime_scope: RuntimeScopeRefV1,
    pub scope_ownership_generation: ScopeOwnershipGeneration,
}

/// GAME-ITEM-01 §4.1 stack class of a definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemStackClass {
    NonStackable,
    /// Stackable; `proven_maximum` is the definition's proven stack maximum,
    /// or `None` for the D82 default of 100.
    Stackable {
        proven_maximum: Option<u32>,
    },
    /// Not admitted for pickup (fail closed).
    Unknown,
}

/// Current compatible Content facts for one item definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemDefinitionFacts {
    pub definition: TypedDefinitionRef,
    pub stack: ItemStackClass,
    /// Declared container capacity (direct entries); `None` for a non-container.
    pub container_capacity: Option<u32>,
    /// The definition declares a complete equip pattern whose primary slot is
    /// `container` (GAME-ITEM-01 §6.2). Being a container is not enough.
    pub container_slot_equip_pattern: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemTransferDestination {
    /// CharacterEquipment slot `container`.
    ContainerSlot,
    /// A direct entry of the main backpack, including the D83 merge shapes.
    MainBackpack,
}

impl ItemTransferDestination {
    const fn kind(self) -> i16 {
        match self {
            Self::ContainerSlot => 1,
            Self::MainBackpack => 2,
        }
    }
}

/// Complete semantic intent of one player TRANSFER.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemTransferRequest {
    /// The actual FND-02 CommandRef; the DUR-03 cause of this TRANSFER.
    pub command: CommandRef,
    pub source_item_instance_id: [u8; 16],
    pub destination: ItemTransferDestination,
    /// Facts of the moved item's definition (and of any receiver stack).
    pub item: ItemDefinitionFacts,
    /// Facts of the main backpack's definition; `MainBackpack` only.
    pub backpack: Option<ItemDefinitionFacts>,
    pub content_revision: String,
    pub ruleset_revision: String,
    pub sim_revision: String,
}

/// Pickup refusal reasons (B3 §5). A refusal writes nothing and the item
/// stays on Ground.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemTransferRefusal {
    /// The source is not a live Ground item (for example already picked up).
    SourceNotOnGround,
    /// The supplied facts do not describe the stored item or main backpack.
    DefinitionMismatch,
    /// Unknown stack class (D82: fail closed).
    UnknownStackClass,
    /// A proven stack maximum of 0 or above GAMEITEM01-STACK-QUANTITY-MAX.
    UnsupportedStackMaximum,
    /// The stored quantity exceeds the definition's stack maximum.
    QuantityAboveStackMaximum,
    /// The item has no complete `container`-slot equip pattern.
    NotContainerSlotEquippable,
    ContainerSlotOccupied,
    /// A moved container must be empty (DUR03-RL-05 = 0).
    ContainerNotEmpty,
    /// No main backpack is equipped.
    NoMainBackpack,
    /// A main backpack capacity of 0 or above GAMEITEM01-CONTAINER-ENTRIES-MAX.
    UnsupportedContainerCapacity,
    /// No free entry, and no full merge is possible.
    MainBackpackFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerEntryPosition {
    pub parent_item_instance_id: [u8; 16],
    pub placement_ordinal: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferReceiver {
    pub item_instance_id: [u8; 16],
    pub quantity_before: u32,
    pub quantity_after: u32,
}

/// Terminal committed result of one CommandRef.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedItemTransfer {
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub envelope_sha256: [u8; 32],
    pub shape: TransferShape,
    pub source_item_instance_id: [u8; 16],
    pub source_quantity_before: u32,
    pub source_quantity_after: u32,
    pub receiver: Option<TransferReceiver>,
    /// The new entry of the source (NewEntry, TopUp).
    pub destination: Option<ContainerEntryPosition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemTransferOutcome {
    Committed(CommittedItemTransfer),
    /// The CommandRef already transferred; this is its original result.
    AlreadyCommitted(CommittedItemTransfer),
}

#[derive(Debug)]
pub enum ItemTransferError {
    /// Malformed or oversize input; never truncated.
    InvalidInput,
    /// A registered DUR-03 ceiling (including RL-08) was exceeded.
    CapacityExceeded,
    /// A current recovery, session, lease, scope, node or binding fence
    /// rejected the TRANSFER.
    AuthorityRejected,
    /// The TRANSFER is not admissible in the current state.
    Refused(ItemTransferRefusal),
    /// The CommandRef was already used with a different intent.
    ConflictingCause,
    /// A frozen identity is bound to a different cause.
    ConflictingCandidate,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for ItemTransferError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl From<AuditError> for ItemTransferError {
    fn from(error: AuditError) -> Self {
        match error {
            AuditError::InvalidInput => Self::InvalidInput,
            AuditError::CapacityExceeded => Self::CapacityExceeded,
        }
    }
}

impl std::fmt::Display for ItemTransferError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid item TRANSFER input"),
            Self::CapacityExceeded => formatter.write_str("item TRANSFER capacity exceeded"),
            Self::AuthorityRejected => formatter.write_str("item TRANSFER authority rejected"),
            Self::Refused(reason) => write!(formatter, "item TRANSFER refused: {reason:?}"),
            Self::ConflictingCause => {
                formatter.write_str("TRANSFER command was reused with different semantics")
            }
            Self::ConflictingCandidate => {
                formatter.write_str("TRANSFER identity is bound to a different command")
            }
            Self::Unavailable(error) => {
                write!(formatter, "item TRANSFER storage is unavailable: {error:?}")
            }
        }
    }
}

impl std::error::Error for ItemTransferError {}

/// Frozen candidate of one logical TRANSFER: a process-local view of its
/// durable reservation. Deliberately not `Clone`.
#[derive(Debug)]
pub struct ItemTransferCandidate {
    request: ItemTransferRequest,
    character_id: CharacterId,
    transaction_id: [u8; 16],
    event_id: [u8; 16],
    occurred_at_unix_ms: i64,
    intent_binding: [u8; 33],
    work_units_used: u8,
}

impl ItemTransferCandidate {
    #[must_use]
    pub const fn transaction_id(&self) -> &[u8; 16] {
        &self.transaction_id
    }
    #[must_use]
    pub const fn event_id(&self) -> &[u8; 16] {
        &self.event_id
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

/// One item as stored (live).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryItem {
    pub item_instance_id: [u8; 16],
    pub definition: TypedDefinitionRef,
    pub quantity: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackpackEntry {
    pub item: InventoryItem,
    pub placement_ordinal: u64,
}

/// The main backpack and its direct entries in display order (newest first).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterBackpack {
    pub backpack: InventoryItem,
    pub entries: Vec<BackpackEntry>,
}

/// Materialized D83 plan over the authoritative before-state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TransferPlan {
    ContainerSlot,
    NewEntry {
        position: ContainerEntryPosition,
    },
    FullMerge {
        receiver: TransferReceiver,
        receiver_position: ContainerEntryPosition,
    },
    TopUp {
        receiver: TransferReceiver,
        receiver_position: ContainerEntryPosition,
        position: ContainerEntryPosition,
        source_after: u32,
    },
}

impl TransferPlan {
    const fn shape(&self) -> TransferShape {
        match self {
            Self::ContainerSlot => TransferShape::ContainerSlot,
            Self::NewEntry { .. } => TransferShape::NewEntry,
            Self::FullMerge { .. } => TransferShape::FullMerge,
            Self::TopUp { .. } => TransferShape::TopUp,
        }
    }
}

/// Authoritative before-state read under the `character_root` row lock.
pub(crate) struct PlanInput<'a> {
    pub destination: ItemTransferDestination,
    pub item: &'a ItemDefinitionFacts,
    pub backpack: Option<&'a ItemDefinitionFacts>,
    pub source: &'a InventoryItem,
    pub source_has_entries: bool,
    pub slot: Option<&'a InventoryItem>,
    pub entries: &'a [BackpackEntry],
}

/// D82: the stack maximum of a definition, never above 100.
pub(crate) fn stack_maximum(
    stack: ItemStackClass,
) -> std::result::Result<(u32, bool), ItemTransferRefusal> {
    match stack {
        ItemStackClass::NonStackable => Ok((1, false)),
        ItemStackClass::Stackable {
            proven_maximum: None,
        } => Ok((GAMEITEM01_STACK_QUANTITY_MAX, true)),
        ItemStackClass::Stackable {
            proven_maximum: Some(maximum),
        } if (1..=GAMEITEM01_STACK_QUANTITY_MAX).contains(&maximum) => Ok((maximum, true)),
        ItemStackClass::Stackable { .. } => Err(ItemTransferRefusal::UnsupportedStackMaximum),
        ItemStackClass::Unknown => Err(ItemTransferRefusal::UnknownStackClass),
    }
}

/// Pure D80-D83 admission and plan. Never selects a receiver by UUID.
pub(crate) fn plan_transfer(
    input: &PlanInput<'_>,
) -> std::result::Result<TransferPlan, ItemTransferRefusal> {
    use ItemTransferRefusal as Refusal;
    if input.item.definition != input.source.definition {
        return Err(Refusal::DefinitionMismatch);
    }
    let (maximum, stackable) = stack_maximum(input.item.stack)?;
    if input.source.quantity > maximum {
        return Err(Refusal::QuantityAboveStackMaximum);
    }
    if input.source_has_entries {
        return Err(Refusal::ContainerNotEmpty);
    }
    match input.destination {
        ItemTransferDestination::ContainerSlot => {
            let Some(capacity) = input.item.container_capacity else {
                return Err(Refusal::NotContainerSlotEquippable);
            };
            if !input.item.container_slot_equip_pattern {
                return Err(Refusal::NotContainerSlotEquippable);
            }
            if capacity == 0 || capacity > GAMEITEM01_CONTAINER_ENTRIES_MAX {
                return Err(Refusal::UnsupportedContainerCapacity);
            }
            if input.slot.is_some() {
                return Err(Refusal::ContainerSlotOccupied);
            }
            Ok(TransferPlan::ContainerSlot)
        }
        ItemTransferDestination::MainBackpack => {
            let slot = input.slot.ok_or(Refusal::NoMainBackpack)?;
            let backpack = input.backpack.ok_or(Refusal::DefinitionMismatch)?;
            if backpack.definition != slot.definition {
                return Err(Refusal::DefinitionMismatch);
            }
            let capacity = backpack
                .container_capacity
                .ok_or(Refusal::DefinitionMismatch)?;
            if capacity == 0 || capacity > GAMEITEM01_CONTAINER_ENTRIES_MAX {
                return Err(Refusal::UnsupportedContainerCapacity);
            }
            let count = u32::try_from(input.entries.len()).unwrap_or(u32::MAX);
            let free = count < capacity;
            let next = input
                .entries
                .iter()
                .map(|entry| entry.placement_ordinal)
                .max()
                .unwrap_or(0)
                .checked_add(1)
                .ok_or(Refusal::MainBackpackFull)?;
            let position = ContainerEntryPosition {
                parent_item_instance_id: slot.item_instance_id,
                placement_ordinal: next,
            };
            let quantity = input.source.quantity;
            // D83: the compatible stack with room that comes first in display
            // order (the highest ordinal). Compatible = same definition key and
            // revision; quantities may differ.
            let receiver = stackable
                .then(|| {
                    input
                        .entries
                        .iter()
                        .filter(|entry| {
                            entry.item.definition == input.source.definition
                                && entry.item.quantity < maximum
                        })
                        .max_by_key(|entry| entry.placement_ordinal)
                })
                .flatten();
            if let Some(entry) = receiver {
                let receiver_position = ContainerEntryPosition {
                    parent_item_instance_id: slot.item_instance_id,
                    placement_ordinal: entry.placement_ordinal,
                };
                let room = maximum - entry.item.quantity;
                if quantity <= room {
                    return Ok(TransferPlan::FullMerge {
                        receiver: TransferReceiver {
                            item_instance_id: entry.item.item_instance_id,
                            quantity_before: entry.item.quantity,
                            quantity_after: entry.item.quantity + quantity,
                        },
                        receiver_position,
                    });
                }
                if !free {
                    return Err(Refusal::MainBackpackFull);
                }
                return Ok(TransferPlan::TopUp {
                    receiver: TransferReceiver {
                        item_instance_id: entry.item.item_instance_id,
                        quantity_before: entry.item.quantity,
                        quantity_after: maximum,
                    },
                    receiver_position,
                    position,
                    source_after: quantity - room,
                });
            }
            if !free {
                return Err(Refusal::MainBackpackFull);
            }
            Ok(TransferPlan::NewEntry { position })
        }
    }
}

/// Owned copy of the frozen candidate moved into one database pass.
struct FrozenTransfer {
    request: ItemTransferRequest,
    character_id: CharacterId,
    transaction_id: [u8; 16],
    event_id: [u8; 16],
    occurred_at_unix_ms: i64,
    intent_binding: [u8; 33],
}

impl FrozenTransfer {
    fn of(candidate: &ItemTransferCandidate) -> Self {
        Self {
            request: candidate.request.clone(),
            character_id: candidate.character_id,
            transaction_id: candidate.transaction_id,
            event_id: candidate.event_id,
            occurred_at_unix_ms: candidate.occurred_at_unix_ms,
            intent_binding: candidate.intent_binding,
        }
    }
}

/// The source item with its actual Ground.
struct SourceRow {
    item: InventoryItem,
    world_id: [u8; 16],
    ground: OneItemGroundV1,
}

struct Admitted {
    plan: TransferPlan,
    source: SourceRow,
}

impl DurabilityRoot {
    /// Validate the intent and reserve the logical TRANSFER of this CommandRef,
    /// or resume its reservation with the same identities and budget. A new
    /// reservation, and the resumption of one without a receipt, require the
    /// complete current fence and a TRANSFER admissible now; a refusal writes
    /// nothing. This grants nothing and moves nothing.
    pub async fn freeze_item_transfer(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ItemTransferRequest,
    ) -> Result<ItemTransferCandidate> {
        validate_request(&request)?;
        let intent_binding = intent_binding(&request, fence.character_id)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemTransferError::AuthorityRejected)?;
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
                            return Ok(Err(ItemTransferError::ConflictingCause));
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
                    let (world_id, channel_id) = match scope_of(&fence) {
                        Ok(scope) => scope,
                        Err(error) => return Ok(Err(error)),
                    };
                    let row = sqlx::query(
                        "SELECT game_character_uuid_v7()::text AS transaction_id, \
                                game_character_uuid_v7()::text AS event_id, \
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
        Ok(ItemTransferCandidate {
            request,
            character_id: reservation.character_id,
            transaction_id: reservation.transaction_id,
            event_id: reservation.event_id,
            occurred_at_unix_ms: reservation.occurred_at_unix_ms,
            intent_binding,
            work_units_used: reservation.work_units_used,
        })
    }

    /// Durably charge one DUR03-RL-08 work unit to the reservation in its own
    /// committed transaction before the pass does any work.
    async fn charge_item_transfer_work_unit(
        &self,
        recovery: CharacterRecoveryFenceV1,
        candidate: &mut ItemTransferCandidate,
    ) -> Result<()> {
        if candidate.work_units_used >= RL08_RETRY_WORK_UNITS_MAX {
            return Err(ItemTransferError::CapacityExceeded);
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
                        "UPDATE game_item_transfer_reservations \
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
                            "SELECT 1 FROM game_item_transfer_reservations \
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
                            ItemTransferError::CapacityExceeded
                        } else {
                            ItemTransferError::ConflictingCandidate
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
            Err(ItemTransferError::CapacityExceeded) => {
                candidate.work_units_used = RL08_RETRY_WORK_UNITS_MAX;
                Err(ItemTransferError::CapacityExceeded)
            }
            Err(error) => Err(error),
        }
    }

    /// Commit one frozen TRANSFER. A CommandRef that already transferred
    /// returns its original result under the current recovery fence without
    /// reacquiring session authority; a changed intent conflicts. A new
    /// TRANSFER commits only under the complete current fence and with the
    /// authoritative before-state admitting it.
    pub async fn commit_item_transfer(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        candidate: &mut ItemTransferCandidate,
    ) -> Result<ItemTransferOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemTransferError::AuthorityRejected)?;
        self.charge_item_transfer_work_unit(recovery.clone(), candidate)
            .await?;
        let frozen = FrozenTransfer::of(candidate);
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
                            return Ok(Err(ItemTransferError::ConflictingCause));
                        }
                        let committed = decode_receipt(&row)?;
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(ItemTransferOutcome::AlreadyCommitted(committed)));
                    }

                    let Some(row) = load_reservation(&mut tx, command).await? else {
                        return Ok(Err(ItemTransferError::ConflictingCandidate));
                    };
                    let stored: Vec<u8> = row.try_get("intent_binding")?;
                    if stored != frozen.intent_binding {
                        return Ok(Err(ItemTransferError::ConflictingCause));
                    }
                    let reservation = decode_reservation(&row)?;
                    if reservation.transaction_id != frozen.transaction_id
                        || reservation.event_id != frozen.event_id
                        || reservation.occurred_at_unix_ms != frozen.occurred_at_unix_ms
                        || reservation.character_id != frozen.character_id
                    {
                        return Ok(Err(ItemTransferError::ConflictingCandidate));
                    }
                    let identity_reused: bool = sqlx::query_scalar(
                        "SELECT EXISTS (SELECT 1 FROM game_item_transfer_receipts \
                                         WHERE transaction_id = encode($1,'hex')::uuid \
                                            OR event_id = encode($2,'hex')::uuid) \
                             OR EXISTS (SELECT 1 FROM game_item_audit_outbox \
                                         WHERE transaction_id = encode($1,'hex')::uuid \
                                            OR event_id = encode($2,'hex')::uuid)",
                    )
                    .bind(frozen.transaction_id.as_slice())
                    .bind(frozen.event_id.as_slice())
                    .fetch_one(&mut *tx)
                    .await?;
                    if identity_reused {
                        return Ok(Err(ItemTransferError::ConflictingCandidate));
                    }

                    let admitted =
                        match admit(&mut tx, &node, &fence, &frozen.request, &reservation).await? {
                            Ok(admitted) => admitted,
                            Err(error) => return Ok(Err(error)),
                        };
                    let message = transfer_message(&frozen.request, &fence, &admitted);
                    let envelope = match audit::encode_transfer_event(
                        TransferEventIdentity {
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
                    let committed = apply_transfer(&mut tx, &frozen, &admitted, &envelope).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(ItemTransferOutcome::Committed(committed)))
                })
            })
            .await?
    }

    /// Resolve an unknown outcome. The cause lock waits for any in-flight
    /// attempt, so `None` proves nothing committed for this CommandRef and the
    /// same candidate may be retried. Never reacquires session authority.
    pub async fn reconcile_item_transfer(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        candidate: &mut ItemTransferCandidate,
    ) -> Result<Option<CommittedItemTransfer>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemTransferError::AuthorityRejected)?;
        self.charge_item_transfer_work_unit(recovery.clone(), candidate)
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
                        return Ok(Err(ItemTransferError::ConflictingCause));
                    }
                    let committed = decode_receipt(&row)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(Some(committed)))
                })
            })
            .await?
    }

    /// Read the character's main backpack and its direct entries, newest
    /// first (at most GAMEITEM01-REACHABLE-ITEMS rows).
    pub async fn read_character_backpack(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
    ) -> Result<Option<CharacterBackpack>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemTransferError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let backpack = match load_slot(&mut tx, character_id).await? {
                        Some(slot) => {
                            let entries = load_entries(&mut tx, slot.item_instance_id).await?;
                            Some(CharacterBackpack {
                                backpack: slot,
                                entries,
                            })
                        }
                        None => None,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(backpack))
                })
            })
            .await?
    }
}

/// The durable reservation row of one CommandRef.
struct Reservation {
    character_id: CharacterId,
    world_id: [u8; 16],
    channel_id: [u8; 16],
    transaction_id: [u8; 16],
    event_id: [u8; 16],
    occurred_at_unix_ms: i64,
    work_units_used: u8,
}

pub(crate) fn scope_of(fence: &CurrentCharacterItemFence) -> Result<(WorldId, ChannelId)> {
    match fence.runtime_scope {
        RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } => Ok((world_id, channel_id)),
        _ => Err(ItemTransferError::AuthorityRejected),
    }
}

/// The complete current Character item fence in the XP writer's lock order,
/// shared by every CommandRef-keyed Character item transaction (B3-1
/// TRANSFER, CHEST-1 reward-claim MINT): the CommandRef belongs to the fenced
/// GameSession, the cause is keyed to the fenced Character and the fenced
/// scope is the reserved one; then the reconnect-session row, the
/// runtime-scope assignment and current node incarnation, the admission
/// guards, and finally the `character_root` row lock (never an UPDATE, so
/// CharacterRevision stays unchanged). `false` means the fence rejected.
/// Writes nothing.
pub(crate) async fn character_item_fence_is_current(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    reserved_character_id: CharacterId,
    reserved_world_id: [u8; 16],
    reserved_channel_id: [u8; 16],
) -> std::result::Result<bool, DurabilityError> {
    let Ok((world_id, channel_id)) = scope_of(fence) else {
        return Ok(false);
    };
    // Binding: the CommandRef belongs to the fenced GameSession, the cause is
    // keyed to the fenced Character and the fenced scope is the reserved one.
    if command.game_session_id() != fence.game_session_id
        || reserved_character_id != fence.character_id
        || reserved_world_id != *world_id.as_bytes()
        || reserved_channel_id != *channel_id.as_bytes()
    {
        return Ok(false);
    }
    let session = sqlx::query(
        "SELECT account_id::text FROM game_durability_reconnect_sessions \
         WHERE game_session_id = encode($1,'hex')::uuid \
           AND character_id = encode($2,'hex')::uuid \
           AND world_id = encode($3,'hex')::uuid \
           AND runtime_scope_kind = 1 \
           AND runtime_scope_world_id = encode($3,'hex')::uuid \
           AND runtime_scope_channel_id = encode($4,'hex')::uuid \
           AND runtime_scope_instance_id IS NULL \
           AND current_generation = $5::text::numeric(20,0) \
           AND character_lease_generation = $6::text::numeric(20,0) \
           AND scope_ownership_generation = $7::text::numeric(20,0) \
           AND session_state IN (1,2) FOR SHARE",
    )
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(fence.connection_generation.get().to_string())
    .bind(fence.character_lease_generation.to_string())
    .bind(fence.scope_ownership_generation.get().to_string())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(session) = session else {
        return Ok(false);
    };
    let account_text: String = session.try_get("account_id")?;

    let key = scope_key(world_id, channel_id);
    let fact = node.fact();
    let assignment = sqlx::query(
        "SELECT 1 FROM game_runtime_scope_assignments \
         WHERE scope_key = $1 AND world_id = encode($2,'hex')::uuid \
           AND channel_id = encode($3,'hex')::uuid AND state = 1 \
           AND ownership_generation = $4::text::numeric(20,0) \
           AND holder_node_id = encode($5,'hex')::uuid \
           AND holder_registration_revision = $6::text::numeric(20,0) \
         FOR SHARE",
    )
    .bind(key.as_slice())
    .bind(world_id.as_bytes().as_slice())
    .bind(channel_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(fact.node_id().as_bytes().as_slice())
    .bind(fact.registration_revision().to_string())
    .fetch_optional(&mut **tx)
    .await?;
    if assignment.is_none() || !prove_current_incarnation(tx, node).await? {
        return Ok(false);
    }

    let guards_ok: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 \
           FROM game_durability_admission_character_guards c \
           JOIN game_durability_admission_account_guards a \
             ON a.account_id = c.account_id \
           JOIN game_durability_admission_runtime_guards g \
             ON g.scope_key = $1 \
          WHERE c.character_id = encode($2,'hex')::uuid \
            AND c.account_id = $3::uuid \
            AND c.world_id = encode($4,'hex')::uuid \
            AND c.eligible \
            AND c.lease_generation = $5::text::numeric(20,0) \
            AND c.holder_game_session_id = encode($6,'hex')::uuid \
            AND a.presence_character_id = c.character_id \
            AND a.holder_game_session_id = c.holder_game_session_id \
            AND g.ready \
            AND g.ownership_generation = $7::text::numeric(20,0))",
    )
    .bind(key.as_slice())
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(&account_text)
    .bind(world_id.as_bytes().as_slice())
    .bind(fence.character_lease_generation.to_string())
    .bind(fence.game_session_id.as_bytes().as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .fetch_one(&mut **tx)
    .await?;
    if !guards_ok {
        return Ok(false);
    }

    // Per-Character serialization: a row lock, never an UPDATE, so the 0009
    // revision guard does not fire and CharacterRevision stays unchanged.
    let root = sqlx::query(
        "SELECT account_id::text, world_id::text FROM game_character_roots \
          WHERE character_id = encode($1,'hex')::uuid AND lifecycle = 1 FOR UPDATE",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(root) = root else {
        return Ok(false);
    };
    if root.try_get::<String, _>("account_id")? != account_text
        || uuid_text(root.try_get("world_id")?)? != *world_id.as_bytes()
    {
        return Ok(false);
    }
    Ok(true)
}

/// The complete current fence, the binding checks, the `character_root` row
/// lock, the authoritative before-state and the D80-D83 plan, in the XP
/// writer's lock order. Writes nothing.
async fn admit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    request: &ItemTransferRequest,
    reservation: &Reservation,
) -> Pass<Admitted> {
    let (world_id, channel_id) = match scope_of(fence) {
        Ok(scope) => scope,
        Err(error) => return Ok(Err(error)),
    };
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
        return Ok(Err(ItemTransferError::AuthorityRejected));
    }

    let Some(source) = load_source(tx, request.source_item_instance_id).await? else {
        return Ok(Err(ItemTransferError::Refused(
            ItemTransferRefusal::SourceNotOnGround,
        )));
    };
    // DUR-03 §32: the fenced runtime scope owns the source Ground.
    if source.world_id != *world_id.as_bytes() || source.ground.channel_id != channel_id.as_bytes()
    {
        return Ok(Err(ItemTransferError::AuthorityRejected));
    }
    let source_has_entries: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_item_container_entries \
                         WHERE parent_item_instance_id = encode($1,'hex')::uuid)",
    )
    .bind(request.source_item_instance_id.as_slice())
    .fetch_one(&mut **tx)
    .await?;
    let slot = load_slot(tx, fence.character_id).await?;
    let entries = match (request.destination, slot.as_ref()) {
        (ItemTransferDestination::MainBackpack, Some(slot)) => {
            load_entries(tx, slot.item_instance_id).await?
        }
        _ => Vec::new(),
    };
    let plan = match plan_transfer(&PlanInput {
        destination: request.destination,
        item: &request.item,
        backpack: request.backpack.as_ref(),
        source: &source.item,
        source_has_entries,
        slot: slot.as_ref(),
        entries: &entries,
    }) {
        Ok(plan) => plan,
        Err(refusal) => return Ok(Err(ItemTransferError::Refused(refusal))),
    };
    if let TransferPlan::FullMerge { receiver, .. } | TransferPlan::TopUp { receiver, .. } = plan {
        let locked = sqlx::query(
            "SELECT 1 FROM game_item_instances \
              WHERE item_instance_id = encode($1,'hex')::uuid AND lifecycle = 1 \
                AND quantity = $2 FOR UPDATE",
        )
        .bind(receiver.item_instance_id.as_slice())
        .bind(i64::from(receiver.quantity_before))
        .fetch_optional(&mut **tx)
        .await?;
        if locked.is_none() {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    if let Err(error) = plan.shape().usage().check() {
        return Ok(Err(error.into()));
    }
    Ok(Ok(Admitted { plan, source }))
}

async fn load_source(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    item_instance_id: [u8; 16],
) -> std::result::Result<Option<SourceRow>, DurabilityError> {
    let row = sqlx::query(
        "SELECT i.world_id::text, i.definition_family, i.definition_production_key, \
                i.definition_revision_ref, i.quantity, g.channel_id::text, \
                g.runtime_scope_ownership_generation::text, g.spatial_position, g.corpse_ref, \
                g.map_revision, g.content_revision, g.native_room_placement_context \
           FROM game_item_instances i \
           JOIN game_item_ground_locations g USING (item_instance_id, world_id) \
          WHERE i.item_instance_id = encode($1,'hex')::uuid AND i.lifecycle = 1 \
          FOR UPDATE OF i",
    )
    .bind(item_instance_id.as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    let world_id = uuid_text(row.try_get("world_id")?)?;
    Ok(Some(SourceRow {
        item: InventoryItem {
            item_instance_id,
            definition: decode_definition(&row)?,
            quantity: decode_quantity(&row)?,
        },
        world_id,
        ground: OneItemGroundV1 {
            world_id: world_id.to_vec(),
            channel_id: uuid_text(row.try_get("channel_id")?)?.to_vec(),
            spatial_position: row.try_get("spatial_position")?,
            corpse_ref: row.try_get("corpse_ref")?,
            map_revision: row.try_get("map_revision")?,
            content_revision: row.try_get("content_revision")?,
            native_room_placement_context: row.try_get("native_room_placement_context")?,
            runtime_scope_ownership_generation: row
                .try_get::<String, _>("runtime_scope_ownership_generation")?
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        },
    }))
}

pub(crate) async fn load_slot(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character_id: CharacterId,
) -> std::result::Result<Option<InventoryItem>, DurabilityError> {
    let row = sqlx::query(
        "SELECT s.item_instance_id::text, i.definition_family, i.definition_production_key, \
                i.definition_revision_ref, i.quantity \
           FROM game_item_container_slots s \
           JOIN game_item_instances i USING (item_instance_id, world_id) \
          WHERE s.character_id = encode($1,'hex')::uuid AND i.lifecycle = 1",
    )
    .bind(character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    row.map(|row| {
        Ok(InventoryItem {
            item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
            definition: decode_definition(&row)?,
            quantity: decode_quantity(&row)?,
        })
    })
    .transpose()
}

/// Direct entries of `parent`, newest first; bounded by the registered
/// container ceiling (a larger stored set is invalid state).
pub(crate) async fn load_entries(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    parent: [u8; 16],
) -> std::result::Result<Vec<BackpackEntry>, DurabilityError> {
    let rows = sqlx::query(
        "SELECT e.item_instance_id::text, e.placement_ordinal::text, i.definition_family, \
                i.definition_production_key, i.definition_revision_ref, i.quantity \
           FROM game_item_container_entries e \
           JOIN game_item_instances i USING (item_instance_id, world_id) \
          WHERE e.parent_item_instance_id = encode($1,'hex')::uuid AND i.lifecycle = 1 \
          ORDER BY e.placement_ordinal DESC LIMIT $2",
    )
    .bind(parent.as_slice())
    .bind(i64::from(GAMEITEM01_CONTAINER_ENTRIES_MAX) + 1)
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() > GAMEITEM01_CONTAINER_ENTRIES_MAX as usize {
        return Err(DurabilityError::InvalidStoredState);
    }
    rows.iter()
        .map(|row| {
            Ok(BackpackEntry {
                item: InventoryItem {
                    item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
                    definition: decode_definition(row)?,
                    quantity: decode_quantity(row)?,
                },
                placement_ordinal: row
                    .try_get::<String, _>("placement_ordinal")?
                    .parse()
                    .map_err(|_| DurabilityError::InvalidStoredState)?,
            })
        })
        .collect()
}

fn decode_definition(
    row: &sqlx::postgres::PgRow,
) -> std::result::Result<TypedDefinitionRef, DurabilityError> {
    Ok(TypedDefinitionRef {
        family: row.try_get("definition_family")?,
        production_key: row.try_get("definition_production_key")?,
        revision_ref: row.try_get("definition_revision_ref")?,
    })
}

fn decode_quantity(row: &sqlx::postgres::PgRow) -> std::result::Result<u32, DurabilityError> {
    u32::try_from(row.try_get::<i64, _>("quantity")?)
        .map_err(|_| DurabilityError::InvalidStoredState)
}

pub(crate) fn definition_message(value: &TypedDefinitionRef) -> OneItemTypedDefinitionRevisionV1 {
    OneItemTypedDefinitionRevisionV1 {
        family: value.family.clone(),
        production_key: value.production_key.clone(),
        revision_ref: value.revision_ref.clone(),
    }
}

pub(crate) fn entry_message(position: ContainerEntryPosition) -> OneItemContainerEntryV1 {
    OneItemContainerEntryV1 {
        parent_item_instance_id: position.parent_item_instance_id.to_vec(),
        placement_ordinal: position.placement_ordinal,
    }
}

/// The complete typed before/after evidence of the materialized plan.
fn transfer_message(
    request: &ItemTransferRequest,
    fence: &CurrentCharacterItemFence,
    admitted: &Admitted,
) -> OneItemTransferV1 {
    let source = &admitted.source;
    let state = |quantity: u32, lifecycle: u32| OneItemStateV1 {
        item_instance_id: source.item.item_instance_id.to_vec(),
        world_id: source.world_id.to_vec(),
        definition: Some(definition_message(&source.item.definition)),
        quantity,
        lifecycle,
    };
    let before = source.item.quantity;
    let (after, entry, slot, receiver) = match admitted.plan {
        TransferPlan::ContainerSlot => (state(before, ITEM_LIFECYCLE_LIVE), None, true, None),
        TransferPlan::NewEntry { position } => (
            state(before, ITEM_LIFECYCLE_LIVE),
            Some(entry_message(position)),
            false,
            None,
        ),
        TransferPlan::FullMerge {
            receiver,
            receiver_position,
        } => (
            state(0, ITEM_LIFECYCLE_RETIRED),
            None,
            false,
            Some((receiver, receiver_position)),
        ),
        TransferPlan::TopUp {
            receiver,
            receiver_position,
            position,
            source_after,
        } => (
            state(source_after, ITEM_LIFECYCLE_LIVE),
            Some(entry_message(position)),
            false,
            Some((receiver, receiver_position)),
        ),
    };
    OneItemTransferV1 {
        before: Some(state(before, ITEM_LIFECYCLE_LIVE)),
        after: Some(after),
        source: Some(source.ground.clone()),
        destination: Some(OneItemInventoryV1 {
            character_id: fence.character_id.as_bytes().to_vec(),
            expected_session_generation: fence.connection_generation.get(),
            expected_game_session_id: fence.game_session_id.as_bytes().to_vec(),
            expected_character_lease_generation: fence.character_lease_generation,
            container_entry: entry,
            equipment_container_slot: slot,
        }),
        cause: Some(OneItemTransferCauseV1 {
            typed_cause: GROUND_PICKUP_TYPED_CAUSE.into(),
            content_revision: request.content_revision.clone(),
            ruleset_revision: request.ruleset_revision.clone(),
            sim_revision: request.sim_revision.clone(),
            command_ref: Some(OneItemCommandRefV1 {
                game_session_id: request.command.game_session_id().as_bytes().to_vec(),
                command_id: request.command.command_id().get(),
            }),
        }),
        receiver: receiver.map(|(receiver, position)| OneItemReceiverV1 {
            item_instance_id: receiver.item_instance_id.to_vec(),
            position: Some(entry_message(position)),
            quantity_before: receiver.quantity_before,
            quantity_after: receiver.quantity_after,
        }),
    }
}

/// Apply every effect of the admitted plan with its receipt and audit event.
async fn apply_transfer(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    frozen: &FrozenTransfer,
    admitted: &Admitted,
    envelope: &[u8],
) -> std::result::Result<CommittedItemTransfer, DurabilityError> {
    let source = &admitted.source;
    let source_id = source.item.item_instance_id;
    let before = source.item.quantity;
    let (source_after, receiver, destination) = match admitted.plan {
        TransferPlan::ContainerSlot => (before, None, None),
        TransferPlan::NewEntry { position } => (before, None, Some(position)),
        TransferPlan::FullMerge { receiver, .. } => (0, Some(receiver), None),
        TransferPlan::TopUp {
            receiver,
            position,
            source_after,
            ..
        } => (source_after, Some(receiver), Some(position)),
    };
    let tx_id = frozen.transaction_id.as_slice();

    let removed = sqlx::query(
        "DELETE FROM game_item_ground_locations WHERE item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(source_id.as_slice())
    .execute(&mut **tx)
    .await?;
    if removed.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let lifecycle = if source_after == 0 {
        ITEM_LIFECYCLE_RETIRED
    } else {
        ITEM_LIFECYCLE_LIVE
    };
    let updated = sqlx::query(
        "UPDATE game_item_instances SET quantity = $2, lifecycle = $3, \
                last_transaction_id = encode($4,'hex')::uuid \
          WHERE item_instance_id = encode($1,'hex')::uuid AND lifecycle = 1 AND quantity = $5",
    )
    .bind(source_id.as_slice())
    .bind(i64::from(source_after))
    .bind(i16::try_from(lifecycle).map_err(|_| DurabilityError::InvalidStoredState)?)
    .bind(tx_id)
    .bind(i64::from(before))
    .execute(&mut **tx)
    .await?;
    if updated.rows_affected() != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    if let Some(receiver) = receiver {
        let grown = sqlx::query(
            "UPDATE game_item_instances SET quantity = $2, \
                    last_transaction_id = encode($3,'hex')::uuid \
              WHERE item_instance_id = encode($1,'hex')::uuid AND lifecycle = 1 \
                AND quantity = $4",
        )
        .bind(receiver.item_instance_id.as_slice())
        .bind(i64::from(receiver.quantity_after))
        .bind(tx_id)
        .bind(i64::from(receiver.quantity_before))
        .execute(&mut **tx)
        .await?;
        if grown.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    match admitted.plan {
        TransferPlan::ContainerSlot => {
            sqlx::query(
                "INSERT INTO game_item_container_slots(character_id, item_instance_id, world_id, \
                   placed_transaction_id) \
                 VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, \
                   encode($3,'hex')::uuid, encode($4,'hex')::uuid)",
            )
            .bind(frozen.character_id.as_bytes().as_slice())
            .bind(source_id.as_slice())
            .bind(source.world_id.as_slice())
            .bind(tx_id)
            .execute(&mut **tx)
            .await?;
        }
        TransferPlan::NewEntry { position } | TransferPlan::TopUp { position, .. } => {
            sqlx::query(
                "INSERT INTO game_item_container_entries(item_instance_id, world_id, \
                   character_id, parent_item_instance_id, placement_ordinal, \
                   placed_transaction_id) \
                 VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, \
                   encode($3,'hex')::uuid, encode($4,'hex')::uuid, $5::text::numeric(20,0), \
                   encode($6,'hex')::uuid)",
            )
            .bind(source_id.as_slice())
            .bind(source.world_id.as_slice())
            .bind(frozen.character_id.as_bytes().as_slice())
            .bind(position.parent_item_instance_id.as_slice())
            .bind(position.placement_ordinal.to_string())
            .bind(tx_id)
            .execute(&mut **tx)
            .await?;
        }
        TransferPlan::FullMerge { .. } => {}
    }
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
    .bind(source_id.as_slice())
    .bind(frozen.occurred_at_unix_ms)
    .bind(mint_audit::AUDIT_RETENTION_P90D_MS)
    .bind(envelope)
    .execute(&mut **tx)
    .await?;
    let envelope_sha256: [u8; 32] = Sha256::digest(envelope).into();
    let shape = admitted.plan.shape();
    let committed_at: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM statement_timestamp())*1000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let command = frozen.request.command;
    sqlx::query(
        "INSERT INTO game_item_transfer_receipts(game_session_id, command_id, character_id, \
           intent_binding, transaction_id, event_id, shape, source_item_instance_id, \
           source_quantity_before, source_quantity_after, receiver_item_instance_id, \
           receiver_quantity_before, receiver_quantity_after, \
           destination_parent_item_instance_id, destination_ordinal, occurred_at, \
           envelope_sha256, committed_at) \
         VALUES (encode($1,'hex')::uuid, $2::text::numeric(20,0), encode($3,'hex')::uuid, $4, \
           encode($5,'hex')::uuid, encode($6,'hex')::uuid, $7, encode($8,'hex')::uuid, $9, $10, \
           CASE WHEN $11::bytea IS NULL THEN NULL ELSE encode($11,'hex')::uuid END, $12, $13, \
           CASE WHEN $14::bytea IS NULL THEN NULL ELSE encode($14,'hex')::uuid END, \
           $15::text::numeric(20,0), $16, $17, $18)",
    )
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .bind(frozen.character_id.as_bytes().as_slice())
    .bind(frozen.intent_binding.as_slice())
    .bind(tx_id)
    .bind(frozen.event_id.as_slice())
    .bind(shape_code(shape))
    .bind(source_id.as_slice())
    .bind(i64::from(before))
    .bind(i64::from(source_after))
    .bind(receiver.map(|value| value.item_instance_id.to_vec()))
    .bind(receiver.map(|value| i64::from(value.quantity_before)))
    .bind(receiver.map(|value| i64::from(value.quantity_after)))
    .bind(destination.map(|value| value.parent_item_instance_id.to_vec()))
    .bind(destination.map(|value| value.placement_ordinal.to_string()))
    .bind(frozen.occurred_at_unix_ms)
    .bind(envelope_sha256.as_slice())
    .bind(committed_at)
    .execute(&mut **tx)
    .await?;
    Ok(CommittedItemTransfer {
        transaction_id: frozen.transaction_id,
        event_id: frozen.event_id,
        occurred_at_unix_ms: frozen.occurred_at_unix_ms,
        envelope_sha256,
        shape,
        source_item_instance_id: source_id,
        source_quantity_before: before,
        source_quantity_after: source_after,
        receiver,
        destination,
    })
}

const fn shape_code(shape: TransferShape) -> i16 {
    match shape {
        TransferShape::ContainerSlot => 1,
        TransferShape::NewEntry => 2,
        TransferShape::FullMerge => 3,
        TransferShape::TopUp => 4,
    }
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
         'oteryn:item-transfer:' || encode($1, 'hex'), 0))",
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
                transaction_id::text, event_id::text, occurred_at, work_units_used \
           FROM game_item_transfer_reservations \
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
        occurred_at_unix_ms: row.try_get("occurred_at")?,
        work_units_used: u8::try_from(row.try_get::<i16, _>("work_units_used")?)
            .map_err(invalid)?,
    })
}

async fn insert_reservation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &ItemTransferRequest,
    intent_binding: &[u8; 33],
    reservation: &Reservation,
) -> std::result::Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_item_transfer_reservations(game_session_id, command_id, \
           character_id, world_id, channel_id, source_item_instance_id, destination_kind, \
           intent_binding, transaction_id, event_id, occurred_at, work_units_used, \
           reserved_at) \
         VALUES (encode($1,'hex')::uuid, $2::text::numeric(20,0), encode($3,'hex')::uuid, \
           encode($4,'hex')::uuid, encode($5,'hex')::uuid, encode($6,'hex')::uuid, $7, $8, \
           encode($9,'hex')::uuid, encode($10,'hex')::uuid, $11, 0, \
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint)",
    )
    .bind(request.command.game_session_id().as_bytes().as_slice())
    .bind(request.command.command_id().get().to_string())
    .bind(reservation.character_id.as_bytes().as_slice())
    .bind(reservation.world_id.as_slice())
    .bind(reservation.channel_id.as_slice())
    .bind(request.source_item_instance_id.as_slice())
    .bind(request.destination.kind())
    .bind(intent_binding.as_slice())
    .bind(reservation.transaction_id.as_slice())
    .bind(reservation.event_id.as_slice())
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
        "SELECT intent_binding, transaction_id::text, event_id::text, shape, \
                source_item_instance_id::text, source_quantity_before, source_quantity_after, \
                receiver_item_instance_id::text, receiver_quantity_before, \
                receiver_quantity_after, destination_parent_item_instance_id::text, \
                destination_ordinal::text, occurred_at, envelope_sha256 \
           FROM game_item_transfer_receipts \
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
) -> std::result::Result<CommittedItemTransfer, DurabilityError> {
    let invalid = |_| DurabilityError::InvalidStoredState;
    let quantity = |column: &str| -> std::result::Result<Option<u32>, DurabilityError> {
        row.try_get::<Option<i64>, _>(column)?
            .map(|value| u32::try_from(value).map_err(invalid))
            .transpose()
    };
    let shape = match row.try_get::<i16, _>("shape")? {
        1 => TransferShape::ContainerSlot,
        2 => TransferShape::NewEntry,
        3 => TransferShape::FullMerge,
        4 => TransferShape::TopUp,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let receiver = match row.try_get::<Option<String>, _>("receiver_item_instance_id")? {
        Some(id) => Some(TransferReceiver {
            item_instance_id: uuid_text(&id)?,
            quantity_before: quantity("receiver_quantity_before")?
                .ok_or(DurabilityError::InvalidStoredState)?,
            quantity_after: quantity("receiver_quantity_after")?
                .ok_or(DurabilityError::InvalidStoredState)?,
        }),
        None => None,
    };
    let destination = match (
        row.try_get::<Option<String>, _>("destination_parent_item_instance_id")?,
        row.try_get::<Option<String>, _>("destination_ordinal")?,
    ) {
        (Some(parent), Some(ordinal)) => Some(ContainerEntryPosition {
            parent_item_instance_id: uuid_text(&parent)?,
            placement_ordinal: ordinal
                .parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        }),
        (None, None) => None,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let digest: Vec<u8> = row.try_get("envelope_sha256")?;
    Ok(CommittedItemTransfer {
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        occurred_at_unix_ms: row.try_get("occurred_at")?,
        envelope_sha256: digest
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        shape,
        source_item_instance_id: uuid_text(row.try_get("source_item_instance_id")?)?,
        source_quantity_before: quantity("source_quantity_before")?
            .ok_or(DurabilityError::InvalidStoredState)?,
        source_quantity_after: quantity("source_quantity_after")?
            .ok_or(DurabilityError::InvalidStoredState)?,
        receiver,
        destination,
    })
}

pub(crate) fn validate_facts(value: &ItemDefinitionFacts) -> Result<()> {
    mint_audit::check_technical_text(&value.definition.family)?;
    mint_audit::check_content_key(&value.definition.production_key)?;
    mint_audit::check_content_key(&value.definition.revision_ref)?;
    Ok(())
}

fn validate_request(request: &ItemTransferRequest) -> Result<()> {
    mint_audit::check_uuid_v7(&request.source_item_instance_id)?;
    validate_facts(&request.item)?;
    match (request.destination, request.backpack.as_ref()) {
        (ItemTransferDestination::ContainerSlot, None) => {}
        (ItemTransferDestination::MainBackpack, Some(backpack)) => validate_facts(backpack)?,
        _ => return Err(ItemTransferError::InvalidInput),
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

pub(crate) fn push_text(out: &mut Vec<u8>, value: &[u8]) -> Result<()> {
    let length = u16::try_from(value.len()).map_err(|_| ItemTransferError::InvalidInput)?;
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(value);
    Ok(())
}

pub(crate) fn push_facts(out: &mut Vec<u8>, value: &ItemDefinitionFacts) -> Result<()> {
    push_text(out, value.definition.family.as_bytes())?;
    push_text(out, value.definition.production_key.as_bytes())?;
    push_text(out, value.definition.revision_ref.as_bytes())?;
    let (class, maximum) = match value.stack {
        ItemStackClass::NonStackable => (1_u8, 0),
        ItemStackClass::Stackable { proven_maximum } => (2, proven_maximum.unwrap_or(0)),
        ItemStackClass::Unknown => (3, 0),
    };
    out.push(class);
    out.extend_from_slice(&maximum.to_be_bytes());
    out.push(u8::from(value.container_capacity.is_some()));
    out.extend_from_slice(&value.container_capacity.unwrap_or(0).to_be_bytes());
    out.push(u8::from(value.container_slot_equip_pattern));
    Ok(())
}

/// Version byte plus SHA-256 over the complete intent: the CommandRef, the
/// fenced Character, the source item, the destination kind, the definition
/// facts and the interpretation revisions. The connection generation is not
/// part of the intent (DUR-03 §31).
fn intent_binding(request: &ItemTransferRequest, character_id: CharacterId) -> Result<[u8; 33]> {
    let mut canonical = Vec::new();
    canonical.extend_from_slice(request.command.game_session_id().as_bytes());
    canonical.extend_from_slice(&request.command.command_id().get().to_be_bytes());
    canonical.extend_from_slice(character_id.as_bytes());
    canonical.extend_from_slice(&request.source_item_instance_id);
    canonical.extend_from_slice(&request.destination.kind().to_be_bytes());
    push_facts(&mut canonical, &request.item)?;
    match request.backpack.as_ref() {
        Some(backpack) => {
            canonical.push(1);
            push_facts(&mut canonical, backpack)?;
        }
        None => canonical.push(0),
    }
    for revision in [
        &request.content_revision,
        &request.ruleset_revision,
        &request.sim_revision,
    ] {
        push_text(&mut canonical, revision.as_bytes())?;
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
    use crate::foundation::CommandId;

    fn id(seed: u8) -> [u8; 16] {
        [
            seed, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, seed,
        ]
    }

    fn definition(key: &str) -> TypedDefinitionRef {
        TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "definition-r1".into(),
        }
    }

    fn facts(key: &str, stack: ItemStackClass) -> ItemDefinitionFacts {
        ItemDefinitionFacts {
            definition: definition(key),
            stack,
            container_capacity: None,
            container_slot_equip_pattern: false,
        }
    }

    fn backpack_facts(capacity: u32) -> ItemDefinitionFacts {
        ItemDefinitionFacts {
            definition: definition("fixture:b3.backpack"),
            stack: ItemStackClass::NonStackable,
            container_capacity: Some(capacity),
            container_slot_equip_pattern: true,
        }
    }

    const COIN: ItemStackClass = ItemStackClass::Stackable {
        proven_maximum: None,
    };

    fn item(seed: u8, key: &str, quantity: u32) -> InventoryItem {
        InventoryItem {
            item_instance_id: id(seed),
            definition: definition(key),
            quantity,
        }
    }

    fn entry(seed: u8, key: &str, quantity: u32, ordinal: u64) -> BackpackEntry {
        BackpackEntry {
            item: item(seed, key, quantity),
            placement_ordinal: ordinal,
        }
    }

    fn plan(
        source: &InventoryItem,
        stack: ItemStackClass,
        entries: &[BackpackEntry],
        capacity: u32,
    ) -> std::result::Result<TransferPlan, ItemTransferRefusal> {
        let slot = item(1, "fixture:b3.backpack", 1);
        let moved = facts(&source.definition.production_key, stack);
        let backpack = backpack_facts(capacity);
        plan_transfer(&PlanInput {
            destination: ItemTransferDestination::MainBackpack,
            item: &moved,
            backpack: Some(&backpack),
            source,
            source_has_entries: false,
            slot: Some(&slot),
            entries,
        })
    }

    fn at(ordinal: u64) -> ContainerEntryPosition {
        ContainerEntryPosition {
            parent_item_instance_id: id(1),
            placement_ordinal: ordinal,
        }
    }

    #[test]
    fn d83_shapes_pick_the_newest_compatible_stack_with_room() {
        let coin = |seed, quantity| item(seed, "fixture:b3.coin", quantity);
        // No compatible stack: a new entry with the next ordinal.
        assert_eq!(
            plan(&coin(9, 30), COIN, &[], 20),
            Ok(TransferPlan::NewEntry { position: at(1) })
        );
        // Full merge into the newest stack with room; quantities may differ.
        let entries = [
            entry(10, "fixture:b3.coin", 40, 1),
            entry(11, "fixture:b3.coin", 100, 3),
            entry(12, "fixture:b3.coin", 60, 2),
        ];
        assert_eq!(
            plan(&coin(9, 30), COIN, &entries, 20),
            Ok(TransferPlan::FullMerge {
                receiver: TransferReceiver {
                    item_instance_id: id(12),
                    quantity_before: 60,
                    quantity_after: 90,
                },
                receiver_position: at(2),
            })
        );
        // Top-up: the receiver reaches 100 and the remainder takes ordinal 4.
        assert_eq!(
            plan(&coin(9, 55), COIN, &entries, 20),
            Ok(TransferPlan::TopUp {
                receiver: TransferReceiver {
                    item_instance_id: id(12),
                    quantity_before: 60,
                    quantity_after: 100,
                },
                receiver_position: at(2),
                position: at(4),
                source_after: 15,
            })
        );
        // Exactly max: 60 + 40 = 100 is still a full merge.
        assert!(matches!(
            plan(&coin(9, 40), COIN, &entries, 20),
            Ok(TransferPlan::FullMerge { .. })
        ));
        // Top-up with no free entry is refused; a full merge still fits.
        assert_eq!(
            plan(&coin(9, 55), COIN, &entries, 3),
            Err(ItemTransferRefusal::MainBackpackFull)
        );
        assert!(matches!(
            plan(&coin(9, 30), COIN, &entries, 3),
            Ok(TransferPlan::FullMerge { .. })
        ));
        // Another revision is not compatible.
        let other_revision = [BackpackEntry {
            item: InventoryItem {
                definition: TypedDefinitionRef {
                    revision_ref: "definition-r2".into(),
                    ..definition("fixture:b3.coin")
                },
                ..coin(10, 10)
            },
            placement_ordinal: 5,
        }];
        assert_eq!(
            plan(&coin(9, 30), COIN, &other_revision, 20),
            Ok(TransferPlan::NewEntry { position: at(6) })
        );
        // A non-stackable item never merges.
        let stones = [entry(10, "fixture:b3.stone", 1, 1)];
        assert_eq!(
            plan(
                &item(9, "fixture:b3.stone", 1),
                ItemStackClass::NonStackable,
                &stones,
                20
            ),
            Ok(TransferPlan::NewEntry { position: at(2) })
        );
    }

    #[test]
    fn capacity_and_stack_ceilings_are_max_and_max_plus_one() {
        let full: Vec<BackpackEntry> = (1..=20)
            .map(|ordinal| entry(100 + ordinal, "fixture:b3.stone", 1, u64::from(ordinal)))
            .collect();
        let stone = item(9, "fixture:b3.stone", 1);
        // 19 entries: the 20th is placed; 20 entries: the 21st is refused.
        assert!(matches!(
            plan(&stone, ItemStackClass::NonStackable, &full[..19], 20),
            Ok(TransferPlan::NewEntry { .. })
        ));
        assert_eq!(
            plan(&stone, ItemStackClass::NonStackable, &full, 20),
            Err(ItemTransferRefusal::MainBackpackFull)
        );
        // Capacity 20 accepted, 21 refused.
        assert!(plan(&stone, ItemStackClass::NonStackable, &[], 20).is_ok());
        assert_eq!(
            plan(&stone, ItemStackClass::NonStackable, &[], 21),
            Err(ItemTransferRefusal::UnsupportedContainerCapacity)
        );
        // Stack ceiling: proven 100 accepted, 101 refused; quantity 100 / 101.
        let proven = |maximum| ItemStackClass::Stackable {
            proven_maximum: Some(maximum),
        };
        assert_eq!(stack_maximum(proven(100)), Ok((100, true)));
        assert_eq!(
            stack_maximum(proven(101)),
            Err(ItemTransferRefusal::UnsupportedStackMaximum)
        );
        assert_eq!(stack_maximum(COIN), Ok((100, true)));
        assert_eq!(
            stack_maximum(ItemStackClass::Unknown),
            Err(ItemTransferRefusal::UnknownStackClass)
        );
        let coin = |quantity| item(9, "fixture:b3.coin", quantity);
        assert!(plan(&coin(100), COIN, &[], 20).is_ok());
        assert_eq!(
            plan(&coin(101), COIN, &[], 20),
            Err(ItemTransferRefusal::QuantityAboveStackMaximum)
        );
        // A proven smaller maximum bounds the merge.
        let small = [entry(10, "fixture:b3.coin", 20, 1)];
        assert!(matches!(
            plan(&coin(10), proven(25), &small, 20),
            Ok(TransferPlan::TopUp {
                source_after: 5,
                ..
            })
        ));
        assert_eq!(GAMEITEM01_REACHABLE_ITEMS, 21);
        assert_eq!(GAMEITEM01_PLACEMENT_DEPTH, 1);
    }

    #[test]
    fn container_slot_and_backpack_refusals() {
        let bag = item(9, "fixture:b3.backpack", 1);
        let mut pattern = backpack_facts(20);
        let slot_plan =
            |facts: &ItemDefinitionFacts, slot: Option<&InventoryItem>, has_entries: bool| {
                plan_transfer(&PlanInput {
                    destination: ItemTransferDestination::ContainerSlot,
                    item: facts,
                    backpack: None,
                    source: &bag,
                    source_has_entries: has_entries,
                    slot,
                    entries: &[],
                })
            };
        assert_eq!(
            slot_plan(&pattern, None, false),
            Ok(TransferPlan::ContainerSlot)
        );
        let occupied = item(1, "fixture:b3.backpack", 1);
        assert_eq!(
            slot_plan(&pattern, Some(&occupied), false),
            Err(ItemTransferRefusal::ContainerSlotOccupied)
        );
        assert_eq!(
            slot_plan(&pattern, None, true),
            Err(ItemTransferRefusal::ContainerNotEmpty)
        );
        // A container-slot destination applies the same capacity bounds as
        // the main backpack: capacity 0 and 21 refused, 1 and 20 accepted.
        let zero_capacity = backpack_facts(0);
        assert_eq!(
            slot_plan(&zero_capacity, None, false),
            Err(ItemTransferRefusal::UnsupportedContainerCapacity)
        );
        let above_max_capacity = backpack_facts(21);
        assert_eq!(
            slot_plan(&above_max_capacity, None, false),
            Err(ItemTransferRefusal::UnsupportedContainerCapacity)
        );
        let min_capacity = backpack_facts(1);
        assert_eq!(
            slot_plan(&min_capacity, None, false),
            Ok(TransferPlan::ContainerSlot)
        );
        let max_capacity = backpack_facts(20);
        assert_eq!(
            slot_plan(&max_capacity, None, false),
            Ok(TransferPlan::ContainerSlot)
        );
        pattern.container_slot_equip_pattern = false;
        assert_eq!(
            slot_plan(&pattern, None, false),
            Err(ItemTransferRefusal::NotContainerSlotEquippable)
        );
        let stone = item(9, "fixture:b3.stone", 1);
        let stone_facts = facts("fixture:b3.stone", ItemStackClass::NonStackable);
        let backpack = backpack_facts(20);
        assert_eq!(
            plan_transfer(&PlanInput {
                destination: ItemTransferDestination::MainBackpack,
                item: &stone_facts,
                backpack: Some(&backpack),
                source: &stone,
                source_has_entries: false,
                slot: None,
                entries: &[],
            }),
            Err(ItemTransferRefusal::NoMainBackpack)
        );
        let wrong = facts("fixture:b3.other", ItemStackClass::NonStackable);
        assert_eq!(
            plan(&stone, ItemStackClass::Unknown, &[], 20),
            Err(ItemTransferRefusal::UnknownStackClass)
        );
        let slot = item(1, "fixture:b3.backpack", 1);
        assert_eq!(
            plan_transfer(&PlanInput {
                destination: ItemTransferDestination::MainBackpack,
                item: &wrong,
                backpack: Some(&backpack),
                source: &stone,
                source_has_entries: false,
                slot: Some(&slot),
                entries: &[],
            }),
            Err(ItemTransferRefusal::DefinitionMismatch)
        );
    }

    fn request() -> ItemTransferRequest {
        ItemTransferRequest {
            command: CommandRef::new(
                GameSessionId::decode(&id(50)).expect("session"),
                CommandId::new(7).expect("command"),
            ),
            source_item_instance_id: id(9),
            destination: ItemTransferDestination::MainBackpack,
            item: facts("fixture:b3.coin", COIN),
            backpack: Some(backpack_facts(20)),
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        }
    }

    #[test]
    fn intent_binding_covers_the_whole_intent_but_not_the_connection() {
        let character = CharacterId::from_bytes(id(41)).expect("character");
        let base = intent_binding(&request(), character).expect("binding");
        assert_eq!(base[0], INTENT_BINDING_VERSION);
        let mutations: [fn(&mut ItemTransferRequest); 7] = [
            |r| {
                r.command = CommandRef::new(
                    r.command.game_session_id(),
                    CommandId::new(8).expect("command"),
                );
            },
            |r| r.source_item_instance_id = id(10),
            |r| {
                r.destination = ItemTransferDestination::ContainerSlot;
                r.backpack = None;
            },
            |r| r.item.stack = ItemStackClass::NonStackable,
            |r| {
                r.backpack = Some(backpack_facts(19));
            },
            |r| r.sim_revision = "sim-2".into(),
            |r| r.item.definition.revision_ref = "definition-r2".into(),
        ];
        for mutate in mutations {
            let mut changed = request();
            mutate(&mut changed);
            assert_ne!(intent_binding(&changed, character).expect("binding"), base);
        }
        let other = CharacterId::from_bytes(id(42)).expect("character");
        assert_ne!(intent_binding(&request(), other).expect("binding"), base);
    }

    #[test]
    fn invalid_input_fails_before_database_work() {
        assert!(validate_request(&request()).is_ok());
        let mut missing = request();
        missing.backpack = None;
        assert!(matches!(
            validate_request(&missing),
            Err(ItemTransferError::InvalidInput)
        ));
        let mut extra = request();
        extra.destination = ItemTransferDestination::ContainerSlot;
        assert!(matches!(
            validate_request(&extra),
            Err(ItemTransferError::InvalidInput)
        ));
        let mut long = request();
        long.item.definition.production_key = "k".repeat(513);
        assert!(matches!(
            validate_request(&long),
            Err(ItemTransferError::InvalidInput)
        ));
        let mut nil = request();
        nil.source_item_instance_id = [0; 16];
        assert!(matches!(
            validate_request(&nil),
            Err(ItemTransferError::InvalidInput)
        ));
    }
}

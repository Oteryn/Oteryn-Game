//! ITEM-LOOP-1 (CP D1438): the live item loop writer. One fenced receipt per CommandRef moves,
//! merges, splits, drops or picks up one item of the Character's carried tree:
//! - ITEM-MOVE-2a: a main-backpack entry or a nested entry to the main backpack or to any
//!   container of the carried tree (BAGS-1: nesting depth <= 8, at most min(capacity, 20)
//!   entries per container, at most 500 items below the main backpack).
//! - STACK-1: a whole move tops up the newest compatible stack with room (a full merge retires
//!   the source); a partial move (`count` below the stack) splits a fresh stack off the source.
//! - ITEM-MOVE-2b: a drop puts the item (or a split part) on a Ground cell of the Channel with a
//!   loop drop reference; a pickup takes a whole loop drop or unchanged, receipt-proven
//!   map-MINT Ground item into the carried tree (MAP-PICKUP-1, migration 0130).
//! - B3-3 weight: a pickup weighs the whole carried graph and the incoming stack in centi-oz
//!   against the capacity derived from the vocation and level progression; an unknown weight
//!   fails closed ([`LoopMoveRefusal::WeightUnknown`]).
//!
//! The writer holds the complete current Character item fence of the CommandRef (the equipment
//! owner's authority, including the active Content digest), reads every touched item under its
//! row lock and writes the receipt and its lines before any custody write, so the 0128 proofs
//! (immediate line admission, deferred line and receipt proofs) decide the same facts again.
//! A refusal writes nothing; an unknown outcome is answered by [`DurabilityRoot::read_item_loop_move_receipt`].

use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_equipment::{EquipmentError, assert_current_equipment_in_transaction};
use super::ground_cell_limit::{
    GroundCellAdmissionError, GroundCellLimitError, check_ground_cell_root_insert_with_recovery,
};
use oteryn_gameplay_items::item_capacity::{CapacityMove, CapacityRefusal, IncomingWeight, plan_capacity};
use super::item_mint::{GroundItemInstance, GroundPlacement, TypedDefinitionRef, uuid_text};
use super::item_transfer::{
    BackpackEntry, CurrentCharacterItemFence, GAMEITEM01_CONTAINER_ENTRIES_MAX, InventoryItem,
    ItemDefinitionFacts, ItemWeightLookup, ItemWeightRefused, load_entries, load_slot,
    read_derived_transfer_capacity_centi_oz, scope_of, stack_maximum,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_item_transaction::SpellItemError;
use super::spell_owner_commit::SpellLanePermit;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use crate::foundation::{ChannelId, CommandRef, WorldId};
use serde::Serialize;
use serde_json::json;
use sqlx::{Postgres, Row, Transaction};

/// BAGS-1: the deepest admitted placement (the main backpack's direct entries are depth 1).
pub const ITEM_LOOP_DEPTH_MAX: i32 = 8;
/// BAGS-1: the most items below one Character's main backpack.
pub const ITEM_LOOP_TREE_ITEMS_MAX: i64 = 500;
/// ITEMMOVE1-RL-02: the most loop-dropped Ground items of one channel.
pub const ITEM_LOOP_CHANNEL_DROPS_MAX: i64 = 20_000;
/// The prefix of every loop drop's Ground `corpse_ref` (0128 `game_item_loop_drop_ref`).
pub const ITEM_LOOP_DROP_REF_PREFIX: &[u8] = b"OTERYN_ITEM_LOOP_DROP/v1:";

const KIND_NONE: i16 = 0;
const KIND_MAIN: i16 = 1;
const KIND_NESTED: i16 = 2;
const KIND_GROUND: i16 = 3;
const KIND_UNCHANGED: i16 = 9;

const OPERATION_MOVE: i16 = 1;
const OPERATION_MERGE: i16 = 2;
const OPERATION_SPLIT: i16 = 3;
const OPERATION_DROP: i16 = 4;
const OPERATION_PICKUP: i16 = 5;

/// A Ground cell of the fenced Channel, bound to the live map and Content pins of the caller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoopGround {
    /// The 10-byte cell (x i32 BE, y i32 BE, floor i16 BE).
    pub spatial_position: Vec<u8>,
    pub map_revision: String,
    pub content_revision: String,
    pub placement_context: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum LoopDestination {
    /// A direct entry of the Character's main backpack.
    MainBackpack,
    /// A direct entry of this container of the carried tree (the main backpack included).
    Container([u8; 16]),
    Ground(LoopGround),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LoopMoveRequest {
    pub item: [u8; 16],
    /// `None` moves the whole stack; a smaller count splits it.
    pub count: Option<u32>,
    /// A pickup names the Ground cell the caller found the item on (and checked reach to); a
    /// carried source names none. A source elsewhere is `Stale`.
    pub pickup_cell: Option<Vec<u8>>,
    pub destination: LoopDestination,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMoveRefusal {
    /// The item, its location or the destination container is not the current one.
    Stale,
    NoBackpack,
    /// The destination container, the carried tree or the Ground cell is full.
    NoRoom,
    /// The Ground cell does not admit the item (incompatible map, Content or context pins).
    Blocked,
    /// A shape the loop does not host (a non-container destination, a cycle, a deeper tree, a
    /// unproven Ground item, a partial pickup, a non-empty container to the Ground).
    NotSupported,
    Overweight,
    WeightUnknown,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMoveReplay {
    Absent,
    Moved,
    Conflicting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoopMoveReceipt {
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    /// 1 move, 2 merge, 3 split, 4 drop, 5 pickup.
    pub operation: i16,
    pub replayed: bool,
}

/// One container of a carried tree: the item, its parent container (`None` for the main
/// backpack) and its direct entries, newest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopContainer {
    pub container: InventoryItem,
    pub parent: Option<[u8; 16]>,
    pub entries: Vec<BackpackEntry>,
}

/// One live loop drop or map-MINT Ground item with its persisted append ordinal, for the overlay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopGroundItem {
    pub stack_ordinal: u64,
    pub item: GroundItemInstance,
}

/// The admitted Content generation the move is decided against: the facts of every touched
/// definition and their unit weights, bound to the active Content digest.
pub trait ItemLoopContent: ItemWeightLookup {
    fn loop_content_digest(&self) -> [u8; 32];
    fn loop_facts(&self, definition: &TypedDefinitionRef) -> Option<ItemDefinitionFacts>;
}

#[derive(Debug)]
pub(super) enum LoopError {
    Refused(LoopMoveRefusal),
    Durability(DurabilityError),
    Database(sqlx::Error),
}

impl From<DurabilityError> for LoopError {
    fn from(error: DurabilityError) -> Self {
        Self::Durability(error)
    }
}

impl From<sqlx::Error> for LoopError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl From<EquipmentError> for LoopError {
    fn from(error: EquipmentError) -> Self {
        match error {
            EquipmentError::Rejected(_) => Self::Refused(LoopMoveRefusal::Rejected),
            EquipmentError::Database(error) => Self::Database(error),
            EquipmentError::Durability(error) => Self::Durability(error),
        }
    }
}

type LoopResult<T> = std::result::Result<T, LoopError>;

fn refuse<T>(refusal: LoopMoveRefusal) -> LoopResult<T> {
    Err(LoopError::Refused(refusal))
}

/// A refusal or an integrity violation wrote nothing; any other database error leaves the
/// outcome to the replay.
fn loop_error(error: LoopError) -> std::result::Result<LoopMoveRefusal, DurabilityError> {
    match error {
        LoopError::Refused(refusal) => Ok(refusal),
        LoopError::Database(sqlx::Error::Database(database))
            if database.code().is_some_and(|code| code.starts_with("23")) =>
        {
            Ok(LoopMoveRefusal::Rejected)
        }
        LoopError::Database(_) => Err(DurabilityError::Unavailable),
        LoopError::Durability(error) => Err(error),
    }
}

fn ground_error(error: GroundCellAdmissionError) -> LoopError {
    match error {
        GroundCellAdmissionError::Limit(GroundCellLimitError::NotEnoughRoom) => {
            LoopError::Refused(LoopMoveRefusal::NoRoom)
        }
        GroundCellAdmissionError::Limit(GroundCellLimitError::IncompatibleInput)
        | GroundCellAdmissionError::Source(SpellItemError::Rejected(_)) => {
            LoopError::Refused(LoopMoveRefusal::Blocked)
        }
        GroundCellAdmissionError::Source(SpellItemError::Durability(error)) => {
            LoopError::Durability(error)
        }
        GroundCellAdmissionError::Source(SpellItemError::Database(error)) => {
            LoopError::Database(error)
        }
        GroundCellAdmissionError::Source(SpellItemError::Training(_)) => {
            LoopError::Durability(DurabilityError::Unavailable)
        }
    }
}

fn optional_uuid(value: Option<String>) -> LoopResult<Option<[u8; 16]>> {
    Ok(value.as_deref().map(uuid_text).transpose()?)
}

fn stored_u32(value: i64) -> LoopResult<u32> {
    u32::try_from(value).map_err(|_| LoopError::Durability(DurabilityError::InvalidStoredState))
}

fn stored_u64(value: String) -> LoopResult<u64> {
    value
        .parse()
        .map_err(|_| LoopError::Durability(DurabilityError::InvalidStoredState))
}

fn definition_of(row: &sqlx::postgres::PgRow) -> LoopResult<TypedDefinitionRef> {
    Ok(TypedDefinitionRef {
        family: row.try_get("definition_family")?,
        production_key: row.try_get("definition_production_key")?,
        revision_ref: row.try_get("definition_revision_ref")?,
    })
}

/// The per-CommandRef serialization point of the loop writer and its replay read.
async fn lock_loop_command(
    tx: &mut Transaction<'_, Postgres>,
    command: CommandRef,
) -> std::result::Result<(), sqlx::Error> {
    let mut key = Vec::with_capacity(24);
    key.extend_from_slice(command.game_session_id().as_bytes());
    key.extend_from_slice(&command.command_id().get().to_be_bytes());
    sqlx::query(
        "SELECT pg_advisory_xact_lock(hashtextextended(\
         'oteryn:item-loop-move:' || encode($1, 'hex'), 0))",
    )
    .bind(key.as_slice())
    .execute(&mut **tx)
    .await?;
    Ok(())
}

fn intent_bytes(character: CharacterId, request: &LoopMoveRequest) -> LoopResult<Vec<u8>> {
    serde_json::to_vec(&json!({"character": character.as_bytes(), "request": request}))
        .map_err(|_| LoopError::Durability(DurabilityError::InvalidStoredState))
}

/// The loop receipt of `command` under its lock: `None` when absent, the replayed receipt when
/// it is this Character's same request, a refusal for anything else (an equipment move of the
/// same CommandRef included).
async fn committed_loop_move(
    tx: &mut Transaction<'_, Postgres>,
    command: CommandRef,
    character: CharacterId,
    intent: &[u8],
) -> LoopResult<Option<std::result::Result<LoopMoveReceipt, LoopMoveRefusal>>> {
    let row = sqlx::query(
        "SELECT transaction_id::text, event_id::text, character_id::text, operation, intent \
           FROM game_item_loop_move_receipts \
          WHERE game_session_id = encode($1,'hex')::uuid AND command_id = $2::text::numeric(20,0)",
    )
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .fetch_optional(&mut **tx)
    .await?;
    if let Some(row) = row {
        let owner = uuid_text(row.try_get("character_id")?)?;
        let stored: Vec<u8> = row.try_get("intent")?;
        if owner != *character.as_bytes() || stored != intent {
            return Ok(Some(Err(LoopMoveRefusal::Rejected)));
        }
        return Ok(Some(Ok(LoopMoveReceipt {
            transaction_id: uuid_text(row.try_get("transaction_id")?)?,
            event_id: uuid_text(row.try_get("event_id")?)?,
            operation: row.try_get("operation")?,
            replayed: true,
        })));
    }
    let equipment: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_character_equipment_receipts \
          WHERE game_session_id = encode($1,'hex')::uuid AND command_id = $2::text::numeric(20,0))",
    )
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .fetch_one(&mut **tx)
    .await?;
    Ok(equipment.then_some(Err(LoopMoveRefusal::Rejected)))
}

/// The owner and depth of `item` in a carried tree (`None` when it is in none).
async fn loop_owner(
    tx: &mut Transaction<'_, Postgres>,
    item: [u8; 16],
) -> LoopResult<Option<([u8; 16], i32)>> {
    let row = sqlx::query(
        "SELECT owner_character::text AS owner_character, depth \
           FROM game_item_loop_owner(encode($1,'hex')::uuid)",
    )
    .bind(item.as_slice())
    .fetch_one(&mut **tx)
    .await?;
    let owner = optional_uuid(row.try_get("owner_character")?)?;
    let depth: Option<i32> = row.try_get("depth")?;
    Ok(owner.zip(depth))
}

async fn loop_subtree(
    tx: &mut Transaction<'_, Postgres>,
    item: [u8; 16],
) -> LoopResult<(i32, i64)> {
    let row =
        sqlx::query("SELECT height, size FROM game_item_loop_subtree(encode($1,'hex')::uuid)")
            .bind(item.as_slice())
            .fetch_one(&mut **tx)
            .await?;
    Ok((
        row.try_get::<Option<i32>, _>("height")?.unwrap_or(0),
        row.try_get::<Option<i64>, _>("size")?.unwrap_or(1),
    ))
}

/// Direct nested entries of `parent`, newest first, bounded by the container ceiling.
async fn load_nested_entries(
    tx: &mut Transaction<'_, Postgres>,
    parent: [u8; 16],
) -> LoopResult<Vec<BackpackEntry>> {
    let rows = sqlx::query(
        "SELECT n.item_instance_id::text, n.placement_ordinal::text, i.definition_family, \
                i.definition_production_key, i.definition_revision_ref, i.quantity \
           FROM game_item_nested_entries n \
           JOIN game_item_instances i USING (item_instance_id, world_id) \
          WHERE n.parent_item_instance_id = encode($1,'hex')::uuid AND i.lifecycle = 1 \
          ORDER BY n.placement_ordinal DESC LIMIT $2",
    )
    .bind(parent.as_slice())
    .bind(i64::from(GAMEITEM01_CONTAINER_ENTRIES_MAX) + 1)
    .fetch_all(&mut **tx)
    .await?;
    if rows.len() > GAMEITEM01_CONTAINER_ENTRIES_MAX as usize {
        return Err(DurabilityError::InvalidStoredState.into());
    }
    rows.iter()
        .map(|row| {
            Ok(BackpackEntry {
                item: InventoryItem {
                    item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
                    definition: definition_of(row)?,
                    quantity: stored_u32(row.try_get("quantity")?)?,
                },
                placement_ordinal: stored_u64(row.try_get("placement_ordinal")?)?,
            })
        })
        .collect()
}

/// The live item under its row lock, with its state revision.
async fn lock_live_item(
    tx: &mut Transaction<'_, Postgres>,
    item: [u8; 16],
    world: [u8; 16],
) -> LoopResult<Option<(TypedDefinitionRef, u32, u64)>> {
    let row = sqlx::query(
        "SELECT definition_family, definition_production_key, definition_revision_ref, quantity, \
                state_revision::text AS state_revision \
           FROM game_item_instances \
          WHERE item_instance_id = encode($1,'hex')::uuid AND world_id = encode($2,'hex')::uuid \
            AND lifecycle = 1 FOR UPDATE",
    )
    .bind(item.as_slice())
    .bind(world.as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    row.map(|row| {
        Ok((
            definition_of(&row)?,
            stored_u32(row.try_get("quantity")?)?,
            stored_u64(row.try_get("state_revision")?)?,
        ))
    })
    .transpose()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceKind {
    Main,
    Nested { parent: [u8; 16] },
    Ground,
}

impl SourceKind {
    const fn code(self) -> i16 {
        match self {
            Self::Main => KIND_MAIN,
            Self::Nested { .. } => KIND_NESTED,
            Self::Ground => KIND_GROUND,
        }
    }
}

/// Where the moved item (or its split part) lands.
enum Target {
    Container {
        kind: i16,
        parent: [u8; 16],
        parent_depth: i32,
        capacity: u32,
        entries: Vec<BackpackEntry>,
    },
    Ground(GroundPlacement),
}

struct Line {
    item: [u8; 16],
    from: i16,
    to: i16,
    parent: Option<[u8; 16]>,
    capacity: Option<i16>,
    position: Option<Vec<u8>>,
    before: u32,
    after: u32,
    revision: Option<u64>,
}

/// The carried weight in centi-oz: every equipped item and the main backpack tree, nested
/// entries included, each instance locked.
pub(super) async fn carried_weight(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
    world: [u8; 16],
    weights: &dyn ItemWeightLookup,
) -> LoopResult<u64> {
    let rows = sqlx::query(
        "WITH RECURSIVE roots AS ( \
           SELECT item_instance_id FROM game_item_container_slots \
            WHERE character_id = encode($1,'hex')::uuid \
           UNION ALL \
           SELECT item_instance_id FROM game_character_equipment_slots \
            WHERE character_id = encode($1,'hex')::uuid \
         ), carried(item_instance_id, path, cycle) AS ( \
           SELECT item_instance_id, ARRAY[item_instance_id], false FROM roots \
           UNION ALL \
           SELECT e.item_instance_id, c.path || e.item_instance_id, \
                  e.item_instance_id = ANY(c.path) \
             FROM carried c JOIN ( \
                  SELECT item_instance_id, parent_item_instance_id FROM game_item_container_entries \
                  UNION ALL \
                  SELECT item_instance_id, parent_item_instance_id FROM game_item_nested_entries \
             ) e ON e.parent_item_instance_id = c.item_instance_id WHERE NOT c.cycle \
         ) \
         SELECT i.item_instance_id::text, i.definition_family, i.definition_production_key, \
                i.definition_revision_ref, i.quantity, i.lifecycle, i.world_id::text AS world, \
                c.cycle \
           FROM carried c JOIN game_item_instances i USING (item_instance_id) \
          ORDER BY i.item_instance_id FOR UPDATE OF i",
    )
    .bind(character.as_bytes().as_slice())
    .fetch_all(&mut **tx)
    .await?;
    let mut seen = std::collections::BTreeSet::new();
    let mut total = 0_u64;
    for row in rows {
        let item = uuid_text(row.try_get("item_instance_id")?)?;
        if row.try_get::<bool, _>("cycle")?
            || row.try_get::<i16, _>("lifecycle")? != 1
            || uuid_text(row.try_get("world")?)? != world
            || !seen.insert(item)
        {
            return Err(DurabilityError::InvalidStoredState.into());
        }
        let unit = match weights.weight_of(&definition_of(&row)?) {
            Ok(unit) => unit,
            Err(ItemWeightRefused::Unknown) => return refuse(LoopMoveRefusal::WeightUnknown),
            Err(ItemWeightRefused::DefinitionMismatch) => return refuse(LoopMoveRefusal::Rejected),
        };
        let quantity = stored_u32(row.try_get("quantity")?)?;
        total = total
            .checked_add(u64::from(unit) * u64::from(quantity))
            .ok_or(DurabilityError::InvalidStoredState)?;
    }
    Ok(total)
}

async fn new_uuid(tx: &mut Transaction<'_, Postgres>) -> LoopResult<[u8; 16]> {
    let bytes: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(game_character_uuid_v7())")
        .fetch_one(&mut **tx)
        .await?;
    bytes
        .try_into()
        .map_err(|_| LoopError::Durability(DurabilityError::InvalidStoredState))
}

/// Shared destination admission for a live move and a not-yet-materialized map leaf.
struct LoopPlan<'a> {
    character: CharacterId,
    world: [u8; 16],
    item: Option<[u8; 16]>,
    definition: &'a TypedDefinitionRef,
    facts: &'a ItemDefinitionFacts,
    quantity: u32,
    count: u32,
    partial: bool,
    source: SourceKind,
    height: i32,
    size: i64,
    destination: &'a LoopDestination,
}

struct PlannedDestination {
    target: Target,
    receiver: Option<([u8; 16], u32, u32, u64)>,
    rest: u32,
}

async fn plan_loop_destination<C: ItemLoopContent>(
    tx: &mut Transaction<'_, Postgres>,
    content: &C,
    plan: LoopPlan<'_>,
) -> LoopResult<PlannedDestination> {
    let LoopPlan { character, world, item, definition, facts, quantity, count, partial,
        source, height, size, destination } = plan;
    let Some(slot) = load_slot(tx, character).await? else {
        return refuse(LoopMoveRefusal::NoBackpack);
    };

    let target = match destination {
        LoopDestination::Ground(ground) => {
            if source == SourceKind::Ground || height > 0 {
                return refuse(LoopMoveRefusal::NotSupported);
            }
            Target::Ground(GroundPlacement {
                spatial_position: ground.spatial_position.clone(),
                corpse_ref: Vec::new(),
                map_revision: ground.map_revision.clone(),
                content_revision: ground.content_revision.clone(),
                native_room_placement_context: ground.placement_context.clone(),
            })
        }
        LoopDestination::MainBackpack => main_target(tx, content, &slot).await?,
        LoopDestination::Container(parent) if *parent == slot.item_instance_id => {
            main_target(tx, content, &slot).await?
        }
        LoopDestination::Container(parent) => {
            let parent = *parent;
            if Some(parent) == item {
                return refuse(LoopMoveRefusal::NotSupported);
            }
            let parent_depth = match loop_owner(tx, parent).await? {
                Some((owner, depth)) if owner == *character.as_bytes() && depth >= 1 => depth,
                _ => return refuse(LoopMoveRefusal::Stale),
            };
            let Some((parent_definition, _, _)) = lock_live_item(tx, parent, world).await? else {
                return refuse(LoopMoveRefusal::Stale);
            };
            let Some(capacity) = content
                .loop_facts(&parent_definition)
                .and_then(|facts| facts.container_capacity)
            else {
                return refuse(LoopMoveRefusal::NotSupported);
            };
            if let Some(item) = item {
            let inside: bool = sqlx::query_scalar(
                "WITH RECURSIVE up(item, hops) AS ( \
                   SELECT encode($1,'hex')::uuid, 0 \
                   UNION ALL \
                   SELECT n.parent_item_instance_id, up.hops + 1 \
                     FROM up JOIN game_item_nested_entries n ON n.item_instance_id = up.item \
                    WHERE up.hops < 16) \
                 SELECT EXISTS (SELECT 1 FROM up WHERE item = encode($2,'hex')::uuid)",
            )
            .bind(parent.as_slice())
            .bind(item.as_slice())
            .fetch_one(&mut **tx)
            .await?;
            if inside {
                return refuse(LoopMoveRefusal::NotSupported);
            }
            }
            Target::Container {
                kind: KIND_NESTED,
                parent,
                parent_depth,
                capacity: capacity.min(GAMEITEM01_CONTAINER_ENTRIES_MAX),
                entries: load_nested_entries(tx, parent).await?,
            }
        }
    };

    // BAGS-1 ceilings and the destination's room.
    let fresh = partial;
    if let Target::Container { parent_depth, .. } = &target
        && parent_depth + 1 + height > ITEM_LOOP_DEPTH_MAX
    {
        return refuse(LoopMoveRefusal::NotSupported);
    }
    let Ok((maximum, stackable)) = stack_maximum(facts.stack) else {
        return refuse(LoopMoveRefusal::NotSupported);
    };
    if quantity > maximum {
        return Err(DurabilityError::InvalidStoredState.into());
    }

    // STACK-1: the newest compatible stack with room receives first; never on the Ground.
    let mut receiver = None;
    if let Target::Container { entries, .. } = &target
        && stackable
        && let Some(entry) = entries.iter().find(|entry| {
            Some(entry.item.item_instance_id) != item
                && entry.item.definition == *definition
                && entry.item.quantity < maximum
        })
    {
        let Some((_, current, receiver_revision)) =
            lock_live_item(tx, entry.item.item_instance_id, world).await?
        else {
            return Err(DurabilityError::InvalidStoredState.into());
        };
        let topup = count.min(maximum - current);
        receiver = Some((
            entry.item.item_instance_id,
            current,
            topup,
            receiver_revision,
        ));
    }
    let topup = receiver.map_or(0, |(_, _, topup, _)| topup);
    let rest = count - topup;
    let placed = rest > 0;
    if let Target::Container {
        parent,
        capacity,
        entries,
        ..
    } = &target
    {
        let already_here = !partial
            && match source {
                SourceKind::Main => *parent == slot.item_instance_id,
                SourceKind::Nested { parent: from } => from == *parent,
                SourceKind::Ground => false,
            };
        let occupied = entries.len() - usize::from(already_here);
        if placed && occupied + 1 > *capacity as usize {
            return refuse(LoopMoveRefusal::NoRoom);
        }
        // The carried tree grows only by what is placed: a pickup's subtree or a split's fresh
        // stack. A full merge retires its source and adds nothing, so it passes at the ceiling.
        let added = if placed {
            (if source == SourceKind::Ground {
                size
            } else {
                0
            }) + i64::from(fresh)
        } else {
            0
        };
        if added > 0 {
            let tree: i64 =
                sqlx::query_scalar("SELECT game_item_loop_tree_size(encode($1,'hex')::uuid)")
                    .bind(character.as_bytes().as_slice())
                    .fetch_one(&mut **tx)
                    .await?;
            if tree + added > ITEM_LOOP_TREE_ITEMS_MAX {
                return refuse(LoopMoveRefusal::NoRoom);
            }
        }
    }

    // B3-3: a pickup brings the whole stack into the Character.
    if source == SourceKind::Ground {
        let unit = match content.weight_of(definition) {
            Ok(unit) => unit,
            Err(ItemWeightRefused::Unknown) => return refuse(LoopMoveRefusal::WeightUnknown),
            Err(ItemWeightRefused::DefinitionMismatch) => return refuse(LoopMoveRefusal::Rejected),
        };
        let carried = carried_weight(tx, character, world, content).await?;
        let capacity = read_derived_transfer_capacity_centi_oz(tx, character).await?;
        let incoming_item = InventoryItem {
            // A preflight has no durable identity yet. The pure weight planner needs only
            // one unique incoming root; this sentinel is never stored or used as a source.
            item_instance_id: item.unwrap_or([0; 16]),
            definition: definition.clone(),
            quantity,
        };
        let incoming = [IncomingWeight {
            item: &incoming_item,
            weight_definition: definition,
            unit_weight: Some(unit),
        }];
        match plan_capacity(CapacityMove::IntoCharacter {
            carried_weight: Some(carried),
            capacity: Some(capacity),
            incoming: &incoming,
        }) {
            Ok(_) => {}
            Err(CapacityRefusal::Overweight { .. }) => return refuse(LoopMoveRefusal::Overweight),
            Err(_) => return Err(DurabilityError::InvalidStoredState.into()),
        }
    }

    Ok(PlannedDestination { target, receiver, rest })
}

#[allow(clippy::too_many_arguments)]
async fn apply_loop_move<C: ItemLoopContent>(
    tx: &mut Transaction<'_, Postgres>,
    permit: &SpellLanePermit,
    record: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    command: CommandRef,
    content: &C,
    request: &LoopMoveRequest,
    intent: &[u8],
) -> LoopResult<LoopMoveReceipt> {
    let character = fence.character_id;
    let (world_id, channel_id) =
        scope_of(fence).map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
    let world = *world_id.as_bytes();
    let channel = *channel_id.as_bytes();
    let digest = content.loop_content_digest();
    assert_current_equipment_in_transaction(tx, record, node, fence, Some(command), digest).await?;

    // The source: locked first, then read in its own statement so the location is current.
    let Some((definition, quantity, revision)) = lock_live_item(tx, request.item, world).await?
    else {
        return refuse(LoopMoveRefusal::Stale);
    };
    let row = sqlx::query(
        "SELECT e.character_id::text AS entry_character, \
                n.parent_item_instance_id::text AS nested_parent, \
                g.channel_id::text AS ground_channel, g.spatial_position AS ground_cell, \
                COALESCE(game_item_loop_drop_ref(g.corpse_ref), false) \
                OR game_item_loop_map_mint_source(i.item_instance_id) AS loop_pickup \
           FROM game_item_instances i \
           LEFT JOIN game_item_container_entries e ON e.item_instance_id = i.item_instance_id \
           LEFT JOIN game_item_nested_entries n ON n.item_instance_id = i.item_instance_id \
           LEFT JOIN game_item_ground_locations g ON g.item_instance_id = i.item_instance_id \
          WHERE i.item_instance_id = encode($1,'hex')::uuid",
    )
    .bind(request.item.as_slice())
    .fetch_one(&mut **tx)
    .await?;
    let entry_character = optional_uuid(row.try_get("entry_character")?)?;
    let nested_parent = optional_uuid(row.try_get("nested_parent")?)?;
    let ground_channel = optional_uuid(row.try_get("ground_channel")?)?;
    let ground_cell: Option<Vec<u8>> = row.try_get("ground_cell")?;
    if ground_channel.is_some() != request.pickup_cell.is_some()
        || (ground_channel.is_some() && ground_cell != request.pickup_cell)
    {
        return refuse(LoopMoveRefusal::Stale);
    }
    let source = if let Some(owner) = entry_character {
        if owner != *character.as_bytes() {
            return refuse(LoopMoveRefusal::Stale);
        }
        SourceKind::Main
    } else if let Some(parent) = nested_parent {
        match loop_owner(tx, request.item).await? {
            Some((owner, depth)) if owner == *character.as_bytes() && depth >= 2 => {
                SourceKind::Nested { parent }
            }
            _ => return refuse(LoopMoveRefusal::Stale),
        }
    } else if let Some(ground) = ground_channel {
        if ground != channel {
            return refuse(LoopMoveRefusal::Stale);
        }
        if !row.try_get::<bool, _>("loop_pickup")? {
            return refuse(LoopMoveRefusal::NotSupported);
        }
        SourceKind::Ground
    } else {
        return refuse(LoopMoveRefusal::NotSupported);
    };

    let Some(facts) = content.loop_facts(&definition) else {
        return refuse(LoopMoveRefusal::NotSupported);
    };
    if facts.definition != definition {
        return refuse(LoopMoveRefusal::Rejected);
    }
    let count = request.count.unwrap_or(quantity);
    if count == 0 || count > quantity {
        return refuse(LoopMoveRefusal::Rejected);
    }
    let partial = count < quantity;
    if partial && source == SourceKind::Ground {
        return refuse(LoopMoveRefusal::NotSupported);
    }
    let (height, size) = loop_subtree(tx, request.item).await?;
    let PlannedDestination { target, receiver, rest } = plan_loop_destination(tx, content, LoopPlan {
        character, world, item: Some(request.item), definition: &definition, facts: &facts,
        quantity, count, partial, source, height, size, destination: &request.destination,
    }).await?;
    let fresh = partial;
    let topup = count - rest;
    let placed = rest > 0;

    let transaction_id = new_uuid(tx).await?;
    let event_id = new_uuid(tx).await?;
    let fresh_id = if fresh {
        Some(new_uuid(tx).await?)
    } else {
        None
    };
    let placed_item = fresh_id.unwrap_or(request.item);

    let (to_kind, to_parent, to_capacity, to_position) = match &target {
        Target::Container {
            kind,
            parent,
            capacity,
            ..
        } => (
            *kind,
            Some(*parent),
            Some(i16::try_from(*capacity).map_err(|_| DurabilityError::InvalidStoredState)?),
            None,
        ),
        Target::Ground(ground) => (
            KIND_GROUND,
            None,
            None,
            Some(ground.spatial_position.clone()),
        ),
    };
    let target = match target {
        Target::Ground(mut ground) => {
            // ITEMMOVE1-RL-02: the channel's row lock serializes its drops, so the count read
            // after it is exact; a drop over the cap is BLOCKED.
            sqlx::query(
                "INSERT INTO game_item_loop_drop_channels (world_id, channel_id) \
                 VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid) \
                 ON CONFLICT (world_id, channel_id) DO UPDATE SET world_id = EXCLUDED.world_id",
            )
            .bind(world.as_slice())
            .bind(channel.as_slice())
            .execute(&mut **tx)
            .await?;
            let dropped: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_item_ground_locations \
                  WHERE world_id = encode($1,'hex')::uuid AND channel_id = encode($2,'hex')::uuid \
                    AND game_item_loop_drop_ref(corpse_ref)",
            )
            .bind(world.as_slice())
            .bind(channel.as_slice())
            .fetch_one(&mut **tx)
            .await?;
            if dropped + 1 > ITEM_LOOP_CHANNEL_DROPS_MAX {
                return refuse(LoopMoveRefusal::Blocked);
            }
            ground.corpse_ref = [ITEM_LOOP_DROP_REF_PREFIX, transaction_id.as_slice()].concat();
            check_ground_cell_root_insert_with_recovery(
                tx,
                permit,
                record,
                node,
                (fence.runtime_scope, fence.scope_ownership_generation.get()),
                &ground,
                &placed_item,
            )
            .await
            .map_err(ground_error)?;
            Target::Ground(ground)
        }
        target => target,
    };

    let mut lines = Vec::with_capacity(3);
    if let Some((item, current, topup, receiver_revision)) = receiver {
        lines.push(Line {
            item,
            from: KIND_UNCHANGED,
            to: KIND_UNCHANGED,
            parent: None,
            capacity: None,
            position: None,
            before: current,
            after: current + topup,
            revision: Some(receiver_revision),
        });
    }
    if partial {
        lines.push(Line {
            item: request.item,
            from: KIND_UNCHANGED,
            to: KIND_UNCHANGED,
            parent: None,
            capacity: None,
            position: None,
            before: quantity,
            after: quantity - count,
            revision: Some(revision),
        });
    } else if placed {
        lines.push(Line {
            item: request.item,
            from: source.code(),
            to: to_kind,
            parent: to_parent,
            capacity: to_capacity,
            position: to_position.clone(),
            before: quantity,
            after: rest,
            revision: Some(revision),
        });
    } else {
        lines.push(Line {
            item: request.item,
            from: source.code(),
            to: KIND_NONE,
            parent: None,
            capacity: None,
            position: None,
            before: quantity,
            after: 0,
            revision: Some(revision),
        });
    }
    if let Some(fresh_id) = fresh_id.filter(|_| placed) {
        lines.push(Line {
            item: fresh_id,
            from: KIND_NONE,
            to: to_kind,
            parent: to_parent,
            capacity: to_capacity,
            position: to_position,
            before: 0,
            after: rest,
            revision: None,
        });
    }
    let operation = if source == SourceKind::Ground {
        OPERATION_PICKUP
    } else if matches!(target, Target::Ground(_)) {
        OPERATION_DROP
    } else if partial {
        OPERATION_SPLIT
    } else if topup > 0 {
        OPERATION_MERGE
    } else {
        OPERATION_MOVE
    };

    sqlx::query(
        "INSERT INTO game_item_loop_move_receipts (transaction_id, event_id, character_id, \
                game_session_id, command_id, connection_generation, lease_generation, world_id, \
                channel_id, scope_generation, content_digest, operation, line_count, intent, \
                intent_sha256) \
         VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
                 encode($4,'hex')::uuid, $5::text::numeric(20,0), $6::text::numeric(20,0), \
                 $7::text::numeric(20,0), encode($8,'hex')::uuid, encode($9,'hex')::uuid, \
                 $10::text::numeric(20,0), $11, $12, $13, $14, sha256($14))",
    )
    .bind(transaction_id.as_slice())
    .bind(event_id.as_slice())
    .bind(character.as_bytes().as_slice())
    .bind(command.game_session_id().as_bytes().as_slice())
    .bind(command.command_id().get().to_string())
    .bind(fence.connection_generation.get().to_string())
    .bind(fence.character_lease_generation.to_string())
    .bind(world.as_slice())
    .bind(channel.as_slice())
    .bind(fence.scope_ownership_generation.get().to_string())
    .bind(digest.as_slice())
    .bind(operation)
    .bind(i16::try_from(lines.len()).map_err(|_| DurabilityError::InvalidStoredState)?)
    .bind(intent)
    .execute(&mut **tx)
    .await?;
    for (index, line) in lines.iter().enumerate() {
        sqlx::query(
            "INSERT INTO game_item_loop_move_lines (transaction_id, line_ordinal, item_instance_id, \
                    definition_family, definition_production_key, definition_revision_ref, \
                    from_kind, to_kind, to_parent_item_instance_id, to_capacity, \
                    to_spatial_position, quantity_before, quantity_after, state_revision_before) \
             VALUES (encode($1,'hex')::uuid, $2, encode($3,'hex')::uuid, $4, $5, $6, $7, $8, \
                     encode($9,'hex')::uuid, $10, $11, $12, $13, $14::text::numeric(20,0))",
        )
        .bind(transaction_id.as_slice())
        .bind(i16::try_from(index + 1).map_err(|_| DurabilityError::InvalidStoredState)?)
        .bind(line.item.as_slice())
        .bind(&definition.family)
        .bind(&definition.production_key)
        .bind(&definition.revision_ref)
        .bind(line.from)
        .bind(line.to)
        .bind(line.parent.map(|parent| parent.to_vec()))
        .bind(line.capacity)
        .bind(line.position.as_deref())
        .bind(i64::from(line.before))
        .bind(i64::from(line.after))
        .bind(line.revision.map(|revision| revision.to_string()))
        .execute(&mut **tx)
        .await?;
    }

    // Custody: the source leaves its location when it moves whole.
    if !partial {
        let delete: &'static str = match source {
            SourceKind::Main => {
                "DELETE FROM game_item_container_entries \
                  WHERE item_instance_id = encode($1,'hex')::uuid"
            }
            SourceKind::Nested { .. } => {
                "DELETE FROM game_item_nested_entries \
                  WHERE item_instance_id = encode($1,'hex')::uuid"
            }
            SourceKind::Ground => {
                "DELETE FROM game_item_ground_locations \
                  WHERE item_instance_id = encode($1,'hex')::uuid"
            }
        };
        sqlx::query(delete)
            .bind(request.item.as_slice())
            .execute(&mut **tx)
            .await?;
    }
    for line in lines.iter().filter(|line| line.from != KIND_NONE) {
        sqlx::query(
            "UPDATE game_item_instances SET quantity = $1, lifecycle = $2, \
                    last_transaction_id = encode($3,'hex')::uuid \
              WHERE item_instance_id = encode($4,'hex')::uuid",
        )
        .bind(i64::from(line.after))
        .bind(if line.to == KIND_NONE { 2_i16 } else { 1_i16 })
        .bind(transaction_id.as_slice())
        .bind(line.item.as_slice())
        .execute(&mut **tx)
        .await?;
    }
    if let Some(fresh_id) = fresh_id.filter(|_| placed) {
        sqlx::query(
            "INSERT INTO game_item_instances (item_instance_id, world_id, definition_family, \
                    definition_production_key, definition_revision_ref, quantity, lifecycle, \
                    minted_transaction_id) \
             VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3, $4, $5, $6, 1, \
                     encode($7,'hex')::uuid)",
        )
        .bind(fresh_id.as_slice())
        .bind(world.as_slice())
        .bind(&definition.family)
        .bind(&definition.production_key)
        .bind(&definition.revision_ref)
        .bind(i64::from(rest))
        .bind(transaction_id.as_slice())
        .execute(&mut **tx)
        .await?;
    }
    if placed {
        match &target {
            Target::Container {
                kind: KIND_MAIN,
                parent,
                ..
            } => {
                sqlx::query(
                    "INSERT INTO game_item_container_entries (item_instance_id, world_id, \
                            character_id, parent_item_instance_id, placement_ordinal, \
                            placed_transaction_id) \
                     SELECT encode($1,'hex')::uuid, encode($2,'hex')::uuid, \
                            encode($3,'hex')::uuid, encode($4,'hex')::uuid, \
                            COALESCE(max(placement_ordinal), 0) + 1, encode($5,'hex')::uuid \
                       FROM game_item_container_entries \
                      WHERE parent_item_instance_id = encode($4,'hex')::uuid",
                )
                .bind(placed_item.as_slice())
                .bind(world.as_slice())
                .bind(character.as_bytes().as_slice())
                .bind(parent.as_slice())
                .bind(transaction_id.as_slice())
                .execute(&mut **tx)
                .await?;
            }
            Target::Container { parent, .. } => {
                sqlx::query(
                    "INSERT INTO game_item_nested_entries (item_instance_id, world_id, \
                            parent_item_instance_id, placement_ordinal, placed_transaction_id) \
                     SELECT encode($1,'hex')::uuid, encode($2,'hex')::uuid, \
                            encode($3,'hex')::uuid, COALESCE(max(placement_ordinal), 0) + 1, \
                            encode($4,'hex')::uuid \
                       FROM game_item_nested_entries \
                      WHERE parent_item_instance_id = encode($3,'hex')::uuid",
                )
                .bind(placed_item.as_slice())
                .bind(world.as_slice())
                .bind(parent.as_slice())
                .bind(transaction_id.as_slice())
                .execute(&mut **tx)
                .await?;
            }
            Target::Ground(ground) => {
                sqlx::query(
                    "INSERT INTO game_item_ground_locations (item_instance_id, world_id, \
                            channel_id, runtime_scope_ownership_generation, spatial_position, \
                            corpse_ref, map_revision, content_revision, \
                            native_room_placement_context) \
                     VALUES (encode($1,'hex')::uuid, encode($2,'hex')::uuid, \
                             encode($3,'hex')::uuid, $4::text::numeric(20,0), $5, $6, $7, $8, $9)",
                )
                .bind(placed_item.as_slice())
                .bind(world.as_slice())
                .bind(channel.as_slice())
                .bind(fence.scope_ownership_generation.get().to_string())
                .bind(ground.spatial_position.as_slice())
                .bind(ground.corpse_ref.as_slice())
                .bind(&ground.map_revision)
                .bind(&ground.content_revision)
                .bind(ground.native_room_placement_context.as_slice())
                .execute(&mut **tx)
                .await?;
            }
        }
    }
    // Run the deferred proofs inside the attempt, so a violation is a refusal, not a commit error.
    sqlx::query("SET CONSTRAINTS ALL IMMEDIATE")
        .execute(&mut **tx)
        .await?;
    Ok(LoopMoveReceipt {
        transaction_id,
        event_id,
        operation,
        replayed: false,
    })
}

async fn main_target<C: ItemLoopContent>(
    tx: &mut Transaction<'_, Postgres>,
    content: &C,
    slot: &InventoryItem,
) -> LoopResult<Target> {
    let Some(capacity) = content
        .loop_facts(&slot.definition)
        .and_then(|facts| facts.container_capacity)
    else {
        return refuse(LoopMoveRefusal::NotSupported);
    };
    Ok(Target::Container {
        kind: KIND_MAIN,
        parent: slot.item_instance_id,
        parent_depth: 0,
        capacity: capacity.min(GAMEITEM01_CONTAINER_ENTRIES_MAX),
        entries: load_entries(tx, slot.item_instance_id).await?,
    })
}

impl DurabilityRoot {
    /// MAP-PICKUP-1: read-only destination/weight admission before reserving a MINT.
    /// Reuses the live loop's planner; the move rechecks everything under its own fence.
    pub async fn preflight_map_item_loop_pickup<C: ItemLoopContent>(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &CurrentCharacterItemFence,
        content: &C,
        mint: &super::map_item_mint::MapItemMintRequest,
        destination: &LoopDestination,
    ) -> std::result::Result<std::result::Result<(), LoopMoveRefusal>, DurabilityError> {
        let record = recovery.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let fence = *fence;
        let mut context = (content, mint, destination);
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| {
            Box::pin(async move {
                let (content, mint, destination) = *context;
                let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                let checked: LoopResult<()> = async {
                    assert_current_equipment_in_transaction(&mut tx, &record, &node, &fence,
                        Some(mint.command), content.loop_content_digest()).await?;
                    let (world, _) = scope_of(&fence).map_err(|_| LoopError::Refused(LoopMoveRefusal::Rejected))?;
                    if matches!(destination, LoopDestination::Ground(_))
                        || mint.item.container_capacity.is_some()
                        || content.loop_facts(&mint.item.definition).as_ref() != Some(&mint.item)
                    {
                        return refuse(LoopMoveRefusal::NotSupported);
                    }
                    plan_loop_destination(&mut tx, content, LoopPlan {
                        character: fence.character_id, world: *world.as_bytes(), item: None,
                        definition: &mint.item.definition, facts: &mint.item,
                        quantity: mint.quantity, count: mint.quantity, partial: false,
                        source: SourceKind::Ground, height: 0, size: 1, destination,
                    }).await?;
                    Ok(())
                }.await;
                // Even a successful preflight commits no reservation or custody state.
                match checked {
                    Ok(()) => Ok(Ok(())),
                    Err(error) => loop_error(error).map(Err),
                }
            })
        }).await
    }

    /// ITEM-LOOP-1: one move, merge, split, drop or pickup of `request` under the current item
    /// fence of `command` in one semantic transaction. `Ok(Err(_))` is a refusal that wrote
    /// nothing; `Err(_)` leaves the outcome to [`Self::read_item_loop_move_receipt`].
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_item_loop_move<C: ItemLoopContent>(
        &self,
        permit: &SpellLanePermit,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: &CurrentCharacterItemFence,
        command: CommandRef,
        content: &C,
        request: LoopMoveRequest,
    ) -> std::result::Result<std::result::Result<LoopMoveReceipt, LoopMoveRefusal>, DurabilityError>
    {
        permit.check_scope(fence.runtime_scope)?;
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let fence = *fence;
        let mut context = (permit, content);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let (permit, content) = *context;
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    let attempt: LoopResult<std::result::Result<LoopMoveReceipt, LoopMoveRefusal>> =
                        async {
                            let intent = intent_bytes(fence.character_id, &request)?;
                            lock_loop_command(&mut tx, command).await?;
                            if let Some(replay) =
                                committed_loop_move(&mut tx, command, fence.character_id, &intent)
                                    .await?
                            {
                                return Ok(replay);
                            }
                            apply_loop_move(
                                &mut tx, permit, &record, &node, &fence, command, content,
                                &request, &intent,
                            )
                            .await
                            .map(Ok)
                        }
                        .await;
                    match attempt {
                        Ok(Ok(receipt)) => {
                            super::db::commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(receipt))
                        }
                        // Dropping the transaction rolls it back: a refusal writes nothing.
                        Ok(Err(refusal)) => Ok(Err(refusal)),
                        Err(error) => loop_error(error).map(Err),
                    }
                })
            })
            .await
    }

    /// ITEM-LOOP-1 replay first: the loop receipt of `command`, read by its CommandRef alone.
    pub async fn read_item_loop_move_receipt(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
        command: CommandRef,
    ) -> std::result::Result<LoopMoveReplay, DurabilityError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &record).await?;
                    // An attempt in flight holds this lock, so `Absent` proves none is.
                    lock_loop_command(&mut tx, command).await?;
                    let row = sqlx::query(
                        "SELECT character_id::text FROM game_item_loop_move_receipts \
                          WHERE game_session_id = encode($1,'hex')::uuid \
                            AND command_id = $2::text::numeric(20,0)",
                    )
                    .bind(command.game_session_id().as_bytes().as_slice())
                    .bind(command.command_id().get().to_string())
                    .fetch_optional(&mut *tx)
                    .await?;
                    let replay = match row {
                        None => LoopMoveReplay::Absent,
                        Some(row) => {
                            if uuid_text(row.try_get("character_id")?)? == *character_id.as_bytes()
                            {
                                LoopMoveReplay::Moved
                            } else {
                                LoopMoveReplay::Conflicting
                            }
                        }
                    };
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(replay)
                })
            })
            .await
    }

    /// `container` with its parent and its direct contents (newest first) when it is in
    /// `character_id`'s carried tree, the main backpack included; `None` otherwise.
    pub async fn read_item_loop_container(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        character_id: CharacterId,
        container: [u8; 16],
    ) -> std::result::Result<Option<LoopContainer>, DurabilityError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &record).await?;
                    let read: LoopResult<Option<LoopContainer>> = async {
                        let slot = load_slot(&mut tx, character_id).await?;
                        let main = slot.is_some_and(|slot| slot.item_instance_id == container);
                        if !main {
                            match loop_owner(&mut tx, container).await? {
                                Some((owner, depth))
                                    if owner == *character_id.as_bytes() && depth >= 1 => {}
                                _ => return Ok(None),
                            }
                        }
                        let Some(row) = sqlx::query(
                            "SELECT i.definition_family, i.definition_production_key, \
                                    i.definition_revision_ref, i.quantity, \
                                    COALESCE(n.parent_item_instance_id, \
                                             e.parent_item_instance_id)::text AS parent \
                               FROM game_item_instances i \
                               LEFT JOIN game_item_nested_entries n \
                                 ON n.item_instance_id = i.item_instance_id \
                               LEFT JOIN game_item_container_entries e \
                                 ON e.item_instance_id = i.item_instance_id \
                              WHERE i.item_instance_id = encode($1,'hex')::uuid \
                                AND i.lifecycle = 1",
                        )
                        .bind(container.as_slice())
                        .fetch_optional(&mut *tx)
                        .await?
                        else {
                            return Ok(None);
                        };
                        let entries = if main {
                            load_entries(&mut tx, container).await?
                        } else {
                            load_nested_entries(&mut tx, container).await?
                        };
                        Ok(Some(LoopContainer {
                            container: InventoryItem {
                                item_instance_id: container,
                                definition: definition_of(&row)?,
                                quantity: stored_u32(row.try_get("quantity")?)?,
                            },
                            parent: if main {
                                None
                            } else {
                                optional_uuid(row.try_get("parent")?)?
                            },
                            entries,
                        }))
                    }
                    .await;
                    let read = match read {
                        Ok(read) => read,
                        Err(LoopError::Durability(error)) => return Err(error),
                        Err(LoopError::Database(error)) => return Err(error.into()),
                        Err(LoopError::Refused(_)) => None,
                    };
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(read)
                })
            })
            .await
    }

    /// Live loop drops and unchanged map-MINT Ground rows of the Channel, for the map
    /// overlay of the Channel.
    pub async fn read_item_loop_ground(
        &self,
        world: WorldId,
        channel: ChannelId,
    ) -> std::result::Result<Vec<LoopGroundItem>, DurabilityError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    let rows = sqlx::query(
                        "SELECT i.item_instance_id::text, i.world_id::text, g.channel_id::text, \
                                i.definition_family, i.definition_production_key, \
                                i.definition_revision_ref, i.quantity, \
                                i.minted_transaction_id::text, \
                                g.runtime_scope_ownership_generation::text, g.spatial_position, \
                                g.corpse_ref, g.map_revision, g.content_revision, \
                                g.native_room_placement_context, g.stack_ordinal \
                           FROM game_item_ground_locations g \
                           JOIN game_item_instances i USING (item_instance_id, world_id) \
                          WHERE g.world_id = encode($1,'hex')::uuid \
                            AND g.channel_id = encode($2,'hex')::uuid AND i.lifecycle = 1 \
                            AND (game_item_loop_drop_ref(g.corpse_ref) \
                                 OR game_item_loop_map_mint_source(i.item_instance_id)) \
                          ORDER BY g.stack_ordinal, i.item_instance_id \
                          LIMIT $3",
                    )
                    .bind(world.as_bytes().as_slice())
                    .bind(channel.as_bytes().as_slice())
                    .bind(ITEM_LOOP_CHANNEL_DROPS_MAX + 1)
                    .fetch_all(&mut *tx)
                    .await?;
                    // Keep the bounded Ground read fail-closed, including pending map pickups.
                    // A MINT uses the same positive Ground INSERT ordinal as a loop drop;
                    // never substitute the base placement ordinal or invent a missing one.
                    let bound = usize::try_from(ITEM_LOOP_CHANNEL_DROPS_MAX)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    if rows.len() > bound {
                        return Err(DurabilityError::InvalidStoredState);
                    }
                    let invalid = |_| DurabilityError::InvalidStoredState;
                    let mut items = Vec::with_capacity(rows.len());
                    for row in &rows {
                        let stack_ordinal = row
                            .try_get::<Option<i64>, _>("stack_ordinal")?
                            .and_then(|ordinal| u64::try_from(ordinal).ok())
                            .ok_or(DurabilityError::InvalidStoredState)?;
                        items.push(LoopGroundItem {
                            stack_ordinal,
                            item: GroundItemInstance {
                                item_instance_id: uuid_text(row.try_get("item_instance_id")?)?,
                                world_id: WorldId::decode(&uuid_text(row.try_get("world_id")?)?)
                                    .map_err(invalid)?,
                                channel_id: ChannelId::decode(&uuid_text(
                                    row.try_get("channel_id")?,
                                )?)
                                .map_err(invalid)?,
                                definition: TypedDefinitionRef {
                                    family: row.try_get("definition_family")?,
                                    production_key: row.try_get("definition_production_key")?,
                                    revision_ref: row.try_get("definition_revision_ref")?,
                                },
                                quantity: u32::try_from(row.try_get::<i64, _>("quantity")?)
                                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                                runtime_scope_ownership_generation: row
                                    .try_get::<String, _>("runtime_scope_ownership_generation")?
                                    .parse()
                                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                                ground: GroundPlacement {
                                    spatial_position: row.try_get("spatial_position")?,
                                    corpse_ref: row.try_get("corpse_ref")?,
                                    map_revision: row.try_get("map_revision")?,
                                    content_revision: row.try_get("content_revision")?,
                                    native_room_placement_context: row
                                        .try_get("native_room_placement_context")?,
                                },
                                minted_transaction_id: uuid_text(
                                    row.try_get("minted_transaction_id")?,
                                )?,
                            },
                        });
                    }
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(items)
                })
            })
            .await
    }
}

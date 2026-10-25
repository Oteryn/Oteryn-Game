//! ITEM-MOVE-1: command 9 (`ItemMoveIntentV1`, ITEM-MOVE-WIRE-0 §5) taking an entry of the open
//! corpse into the Character's main backpack, and (EQUIP-WIRE-1, ITEM-MOVE-WIRE-1 §4) equipping
//! a main backpack entry into a slot or unequipping a slot's item into the main backpack.
//!
//! - **Replay first.** The committed TRANSFER of the CommandRef, then its equipment receipt, then
//!   its item loop receipt, is looked up before the handle is resolved, so a command that committed before a reconnect
//!   answers `MOVED` although its handle is now stale.
//! - **Source.** Only an entry of the open corpse, shown in it and in reach on the Channel owner's
//!   observation (`corpse_open::within_reach`); the TRANSFER (`settle_corpse_pickup`) decides the
//!   rest.
//! - **Item loop (ITEM-LOOP-1, CP D1438).** Every other durable source goes to the loop owner
//!   ([`LoopMove`]): a main-backpack or nested entry into the main backpack or into a visible
//!   container of the carried tree (capability 14), whole or merged into the newest compatible
//!   stack (ITEM-MOVE-2a, BAGS-1, STACK-1); a carried item onto a Ground cell in reach (capability
//!   12 or 11), and a loop Ground item of the map view in reach back into the carried tree
//!   (ITEM-MOVE-2b). The owner decides the rest, the B3-3 weight included, in one transaction.
//! - **Equipment (capability 12).** A main backpack entry to a slot, or a slot's item to the main
//!   backpack, through the equipment owner in one transaction. Into an occupied slot the owner
//!   swaps the whole items atomically (ITEM-MOVE-2a) or refuses. A slot-to-slot move and any other
//!   source answer `NOT_SUPPORTED`; the main backpack itself is a container (`SLOT_MISMATCH`).
//! - **Deltas.** After `MOVED` the open corpse's domain 11 delta and then the domain 9 delta
//!   follow the result; under capability 12 domain 9 carries the equipment.
//! - **Unknown outcome.** When the TRANSFER or the replay read cannot tell whether the command
//!   committed, or a committed view cannot be read, the connection ends before its CommandId is
//!   sequenced: the client resends it and replay first answers.

use super::connection::{FreshAdmissionAuthority, UseCommand};
use super::item_view::corpse_open::within_reach;
use super::item_view::{
    InventoryItems, ItemKey, ItemTargetObservation, ItemViewDelta, SessionItemView, UseItemTarget,
};
use crate::combat_pickup::GroundPickupError;
pub use crate::durability::character_equipment::{EquipmentMoveRefusal, EquipmentMoveReplay};
pub use crate::durability::item_loop_move::{LoopMoveRefusal, LoopMoveReplay};
use crate::durability::item_transfer::{
    ItemTransferError, ItemTransferOutcome, ItemTransferRefusal,
};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_protocol_oteryn::item_view::{
    EquipmentSlot, ItemMoveDestination, ItemMoveOutcome, ItemMoveSelection, WorldTilePosition,
    decode_item_move_intent_for, decode_item_move_intent_with_count_for,
};

/// One equipment move of command 9 (ITEM-MOVE-WIRE-1 §4), by the item's server-internal key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentMove {
    /// A main backpack entry into a free slot.
    Equip { item: ItemKey, to: EquipmentSlot },
    /// A slot's item into the main backpack.
    Unequip { item: ItemKey, from: EquipmentSlot },
}

/// Where one item loop move of command 9 puts its source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopTarget {
    /// A direct entry of the main backpack.
    MainBackpack,
    /// A direct entry of this visible container of the carried tree.
    Container(ItemKey),
    /// The Ground cell, in reach of the actor.
    Ground(WorldTilePosition),
}

/// One item loop move of command 9 (ITEM-LOOP-1), by the source's server-internal key. The whole
/// stack moves when `count` is absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoopMove {
    pub item: ItemKey,
    /// Explicit requested quantity, bound into the existing durable receipt.
    pub count: Option<u32>,
    pub target: LoopTarget,
}

/// What the loop owner did with one [`LoopMove`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMoveResult {
    Committed,
    /// Nothing was written.
    Refused(LoopMoveRefusal),
    /// The Ground cell of a drop or a pickup is out of the actor's reach; nothing was written.
    TooFar,
    /// Whether it committed is unknown: the replay decides on the resend.
    Unknown,
}

/// The §4 result of one refused loop move. The wire has no weight result: an overweight or
/// unknown-weight pickup is `REJECTED`.
pub(super) const fn loop_outcome(refusal: LoopMoveRefusal) -> ItemMoveOutcome {
    match refusal {
        LoopMoveRefusal::Stale => ItemMoveOutcome::Stale,
        LoopMoveRefusal::NoBackpack => ItemMoveOutcome::NoBackpack,
        LoopMoveRefusal::NoRoom => ItemMoveOutcome::NoRoom,
        LoopMoveRefusal::Blocked => ItemMoveOutcome::Blocked,
        LoopMoveRefusal::NotSupported => ItemMoveOutcome::NotSupported,
        LoopMoveRefusal::Overweight
        | LoopMoveRefusal::WeightUnknown
        | LoopMoveRefusal::Rejected => ItemMoveOutcome::Rejected,
    }
}

/// What the equipment owner did with one [`EquipmentMove`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipmentMoveResult {
    Committed,
    /// Nothing was written.
    Refused(EquipmentMoveRefusal),
    /// Whether it committed is unknown: the replay decides on the resend.
    Unknown,
}

/// The §4 result of one refused equipment move.
pub(super) const fn equipment_outcome(refusal: EquipmentMoveRefusal) -> ItemMoveOutcome {
    match refusal {
        EquipmentMoveRefusal::Stale => ItemMoveOutcome::Stale,
        EquipmentMoveRefusal::NoBackpack => ItemMoveOutcome::NoBackpack,
        EquipmentMoveRefusal::NoRoom => ItemMoveOutcome::NoRoom,
        EquipmentMoveRefusal::SlotMismatch => ItemMoveOutcome::SlotMismatch,
        EquipmentMoveRefusal::RequirementNotMet => ItemMoveOutcome::RequirementNotMet,
        EquipmentMoveRefusal::Occupied | EquipmentMoveRefusal::TimedNotHosted => {
            ItemMoveOutcome::NotSupported
        }
        EquipmentMoveRefusal::Rejected => ItemMoveOutcome::Rejected,
    }
}

/// How many times [`observe_inventory`] reads the two custody halves before it gives up.
pub(super) const INVENTORY_SNAPSHOT_ATTEMPTS: usize = 3;

/// Domain 9's content for `view`: the main backpack and its entries, and under capability 12 the
/// occupied equipment slots. The two halves are separate durable reads, so under capability 12
/// the equipment is read before and after the backpack. The pair is kept only when both equipment
/// reads agree and no item is in both halves; otherwise an equipment move committed between the
/// reads, and the pair is read again. `None` when a half cannot be read or the halves never
/// agree within [`INVENTORY_SNAPSHOT_ATTEMPTS`].
pub(super) async fn observe_inventory<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    game_session_id: GameSessionId,
    view: &SessionItemView,
) -> Option<InventoryItems> {
    if !view.equip_drop() {
        return authority
            .observe_character_inventory(actor, game_session_id)
            .await;
    }
    for _ in 0..INVENTORY_SNAPSHOT_ATTEMPTS {
        let before = authority
            .observe_character_equipment(actor, game_session_id)
            .await?;
        let mut inventory = authority
            .observe_character_inventory(actor, game_session_id)
            .await?;
        let after = authority
            .observe_character_equipment(actor, game_session_id)
            .await?;
        let worn_in_backpack = after.iter().any(|(_, worn)| {
            inventory
                .main_backpack
                .iter()
                .chain(&inventory.entries)
                .any(|entry| entry.key == worn.key)
        });
        if before == after && !worn_in_backpack {
            inventory.equipment = after;
            return Some(inventory);
        }
    }
    None
}

/// What the connection does with one command 9.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum ItemMoveStep {
    /// The result, then the view deltas in order.
    Result(ItemMoveOutcome, Vec<ItemViewDelta>),
    /// The outcome is unknown or a committed view cannot be shown: end the connection without
    /// sequencing the CommandId.
    Disconnect,
}

/// The WIRE-0 §5 result of one corpse-entry TRANSFER.
pub(super) fn outcome_of(
    result: &Result<ItemTransferOutcome, GroundPickupError>,
) -> ItemMoveOutcome {
    use ItemTransferRefusal as Refusal;
    match result {
        Ok(ItemTransferOutcome::Committed(_) | ItemTransferOutcome::AlreadyCommitted(_)) => {
            ItemMoveOutcome::Moved
        }
        Err(GroundPickupError::SourceMismatch) => ItemMoveOutcome::Stale,
        Err(GroundPickupError::Content(_)) => ItemMoveOutcome::Rejected,
        Err(GroundPickupError::Transfer(error)) => match error {
            ItemTransferError::Refused(refusal) => match refusal {
                Refusal::SourceNotOnGround => ItemMoveOutcome::Stale,
                Refusal::NoMainBackpack => ItemMoveOutcome::NoBackpack,
                Refusal::MainBackpackFull => ItemMoveOutcome::NoRoom,
                Refusal::CorpseExclusiveWindow => ItemMoveOutcome::NotOwner,
                Refusal::CorpseNotPickupable => ItemMoveOutcome::NotPickupable,
                Refusal::ContainerNotEmpty => ItemMoveOutcome::NotSupported,
                Refusal::WeightUnknown
                | Refusal::WeightGenerationMismatch
                | Refusal::WeightDefinitionMismatch
                | Refusal::Overweight
                | Refusal::DefinitionMismatch
                | Refusal::UnknownStackClass
                | Refusal::UnsupportedStackMaximum
                | Refusal::QuantityAboveStackMaximum
                | Refusal::NotContainerSlotEquippable
                | Refusal::ContainerSlotOccupied
                | Refusal::UnsupportedContainerCapacity => ItemMoveOutcome::Rejected,
            },
            ItemTransferError::InvalidInput
            | ItemTransferError::CapacityExceeded
            | ItemTransferError::AuthorityRejected
            | ItemTransferError::ConflictingCause
            | ItemTransferError::ConflictingCandidate
            | ItemTransferError::Unavailable(_) => ItemMoveOutcome::Rejected,
        },
    }
}

/// One command 9 of an admitted session that selected capability 4.
pub(super) async fn item_move<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    command: UseCommand,
    payload: &[u8],
    selection: ItemMoveSelection,
    stack_count: bool,
    view: &mut SessionItemView,
) -> ItemMoveStep {
    let decoded = if stack_count {
        decode_item_move_intent_with_count_for(payload, selection)
    } else {
        decode_item_move_intent_for(payload, selection).map(|intent| (intent, None))
    };
    let Ok((intent, count)) = decoded else {
        return ItemMoveStep::Result(ItemMoveOutcome::Rejected, Vec::new());
    };
    match authority.committed_item_move(actor, command).await {
        Ok(Some(_)) => return moved(authority, actor, command, view).await,
        Ok(None) => {}
        Err(ItemTransferError::Unavailable(_)) => return ItemMoveStep::Disconnect,
        Err(_) => return ItemMoveStep::Result(ItemMoveOutcome::Rejected, Vec::new()),
    }
    match authority.committed_equipment_move(actor, command).await {
        Some(EquipmentMoveReplay::Moved) => return moved(authority, actor, command, view).await,
        Some(EquipmentMoveReplay::Absent) => {}
        Some(EquipmentMoveReplay::Conflicting) => {
            return ItemMoveStep::Result(ItemMoveOutcome::Rejected, Vec::new());
        }
        None => return ItemMoveStep::Disconnect,
    }
    match authority.committed_loop_move(actor, command).await {
        Some(LoopMoveReplay::Moved) => return moved(authority, actor, command, view).await,
        Some(LoopMoveReplay::Absent) => {}
        Some(LoopMoveReplay::Conflicting) => {
            return ItemMoveStep::Result(ItemMoveOutcome::Rejected, Vec::new());
        }
        None => return ItemMoveStep::Disconnect,
    }
    let Some(source) = view.resolve(intent.source) else {
        return ItemMoveStep::Result(ItemMoveOutcome::Stale, Vec::new());
    };
    if count.is_some()
        && (matches!(intent.destination, ItemMoveDestination::Equipment(_))
            || !matches!(source, ItemKey::Instance(_))
            || view.equipped_slot(&source).is_some()
            || view.open_corpse_of(&source).is_some())
    {
        return ItemMoveStep::Result(ItemMoveOutcome::NotSupported, Vec::new());
    }
    let request = match intent.destination {
        ItemMoveDestination::Equipment(to) => {
            if view.is_main_backpack(&source) {
                return ItemMoveStep::Result(ItemMoveOutcome::SlotMismatch, Vec::new());
            }
            // An occupied slot reaches the owner, which swaps atomically or refuses
            // (ITEM-MOVE-2a); a view that does not show the slots never moves into one.
            if !view.equip_drop() || !view.is_backpack_entry(&source) {
                return ItemMoveStep::Result(ItemMoveOutcome::NotSupported, Vec::new());
            }
            EquipmentMove::Equip { item: source, to }
        }
        ItemMoveDestination::MainBackpack => match view.equipped_slot(&source) {
            Some(from) => EquipmentMove::Unequip { item: source, from },
            None if view.open_corpse_of(&source).is_some() => {
                return take_from_corpse(authority, actor, command, source, view).await;
            }
            None => {
                let target = LoopTarget::MainBackpack;
                return loop_move(
                    authority, actor, command, selection, source, target, count, view,
                )
                .await;
            }
        },
        ItemMoveDestination::Container(handle) => {
            let Some(container) = view.resolve(handle) else {
                return ItemMoveStep::Result(ItemMoveOutcome::Stale, Vec::new());
            };
            let target = if view.is_main_backpack(&container) {
                LoopTarget::MainBackpack
            } else {
                LoopTarget::Container(container)
            };
            return loop_move(
                authority, actor, command, selection, source, target, count, view,
            )
            .await;
        }
        ItemMoveDestination::Ground(at) => {
            let target = LoopTarget::Ground(at);
            return loop_move(
                authority, actor, command, selection, source, target, count, view,
            )
            .await;
        }
    };
    match authority.move_equipment(actor, command, request).await {
        EquipmentMoveResult::Committed => moved(authority, actor, command, view).await,
        EquipmentMoveResult::Unknown => ItemMoveStep::Disconnect,
        EquipmentMoveResult::Refused(refusal) => {
            ItemMoveStep::Result(admitted(equipment_outcome(refusal), selection), Vec::new())
        }
    }
}

/// `outcome`, or `REJECTED` when the session's selection does not admit it: the results 10 to
/// 12 are never sent without capability 12 (`BLOCKED` also with capability 11).
fn admitted(outcome: ItemMoveOutcome, selection: ItemMoveSelection) -> ItemMoveOutcome {
    let admitted = match outcome {
        ItemMoveOutcome::SlotMismatch | ItemMoveOutcome::RequirementNotMet => selection.equip_drop,
        ItemMoveOutcome::Blocked => selection.equip_drop || selection.world_interaction,
        _ => true,
    };
    if admitted {
        outcome
    } else {
        ItemMoveOutcome::Rejected
    }
}

/// ITEM-LOOP-1 / MAP-PICKUP-1: a durable item or eligible base-map entry into the carried
/// tree through the loop owner. Base-to-Ground, MapAdded, equipped sources and the main
/// backpack itself remain unsupported.
#[allow(clippy::too_many_arguments)]
async fn loop_move<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    command: UseCommand,
    selection: ItemMoveSelection,
    source: ItemKey,
    target: LoopTarget,
    count: Option<u32>,
    view: &mut SessionItemView,
) -> ItemMoveStep {
    let supported = matches!(source, ItemKey::Instance(_))
        || matches!(
            (source, target),
            (
                ItemKey::MapBase { .. },
                LoopTarget::MainBackpack | LoopTarget::Container(ItemKey::Instance(_))
            )
        );
    if !supported || view.is_main_backpack(&source) || view.equipped_slot(&source).is_some() {
        return ItemMoveStep::Result(ItemMoveOutcome::NotSupported, Vec::new());
    }
    let request = LoopMove {
        item: source,
        count,
        target,
    };
    match authority.move_loop_item(actor, command, request).await {
        LoopMoveResult::Committed => moved(authority, actor, command, view).await,
        LoopMoveResult::TooFar => ItemMoveStep::Result(ItemMoveOutcome::TooFar, Vec::new()),
        LoopMoveResult::Unknown => ItemMoveStep::Disconnect,
        LoopMoveResult::Refused(refusal) => {
            ItemMoveStep::Result(admitted(loop_outcome(refusal), selection), Vec::new())
        }
    }
}

/// ITEM-MOVE-1: an entry of the open corpse into the main backpack.
async fn take_from_corpse<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    command: UseCommand,
    entry: ItemKey,
    view: &mut SessionItemView,
) -> ItemMoveStep {
    let Some(corpse) = view.open_corpse_of(&entry) else {
        return ItemMoveStep::Result(ItemMoveOutcome::NotSupported, Vec::new());
    };
    match authority.observe_item_target(actor, corpse).await {
        Some(ItemTargetObservation {
            actor: position,
            target:
                UseItemTarget::Corpse {
                    position: at,
                    contents,
                },
        }) if contents.iter().any(|item| item.key == entry) => {
            if !within_reach(position, at) {
                return ItemMoveStep::Result(ItemMoveOutcome::TooFar, Vec::new());
            }
        }
        _ => return ItemMoveStep::Result(ItemMoveOutcome::Stale, Vec::new()),
    }
    let result = authority
        .take_corpse_entry(actor, command, corpse, entry)
        .await;
    if matches!(
        result,
        Err(GroundPickupError::Transfer(ItemTransferError::Unavailable(
            _
        )))
    ) {
        return ItemMoveStep::Disconnect;
    }
    match outcome_of(&result) {
        ItemMoveOutcome::Moved => moved(authority, actor, command, view).await,
        outcome => ItemMoveStep::Result(outcome, Vec::new()),
    }
}

/// `MOVED`, then the open corpse's domain 11 delta and the domain 9 delta. The backpack view is
/// updated first: an entry taken whole keeps its ItemKey, so it moves from the corpse view into
/// the backpack view and keeps its handle instead of leaving every view in between.
async fn moved<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    command: UseCommand,
    view: &mut SessionItemView,
) -> ItemMoveStep {
    let Some(inventory) = observe_inventory(authority, actor, command.game_session_id, view).await
    else {
        return ItemMoveStep::Disconnect;
    };
    let Ok(inventory_delta) = view.inventory_committed(inventory) else {
        return ItemMoveStep::Disconnect;
    };
    let mut deltas = Vec::new();
    if let Some(corpse) = view.continuity().open_corpse {
        let observation = authority.observe_item_target(actor, corpse).await;
        match view.open_corpse_committed(observation) {
            Ok(delta) => deltas.extend(delta),
            Err(_) => return ItemMoveStep::Disconnect,
        }
    }
    deltas.extend(inventory_delta);
    ItemMoveStep::Result(ItemMoveOutcome::Moved, deltas)
}

#[cfg(test)]
#[path = "item_move_tests.rs"]
mod tests;

//! ITEM-MOVE-1: command 9 (`ItemMoveIntentV1`, ITEM-MOVE-WIRE-0 §5) taking an entry of the open
//! corpse into the Character's main backpack, and (EQUIP-WIRE-1, ITEM-MOVE-WIRE-1 §4) equipping
//! a main backpack entry into a free slot or unequipping a slot's item into the main backpack.
//!
//! - **Replay first.** The committed TRANSFER of the CommandRef, then its equipment receipt, is
//!   looked up before the handle is resolved, so a command that committed before a reconnect
//!   answers `MOVED` although its handle is now stale.
//! - **Source.** Only an entry of the open corpse, shown in it and in reach on the Channel owner's
//!   observation (`corpse_open::within_reach`); the TRANSFER (`settle_corpse_pickup`) decides the
//!   rest. Ground and container destinations answer `NOT_SUPPORTED`.
//! - **Equipment (capability 12).** A main backpack entry to a free slot, or a slot's item to the
//!   main backpack, through the equipment owner in one transaction. An occupied slot (the swap),
//!   a slot-to-slot move and any other source answer `NOT_SUPPORTED`; the main backpack itself is
//!   a container (`SLOT_MISMATCH`).
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
pub(crate) use crate::durability::character_equipment::{
    EquipmentMoveRefusal, EquipmentMoveReplay,
};
use crate::durability::item_transfer::{
    ItemTransferError, ItemTransferOutcome, ItemTransferRefusal,
};
use crate::foundation::{ExactActorRef, GameSessionId};
use oteryn_protocol_oteryn::item_view::{
    EquipmentSlot, ItemMoveDestination, ItemMoveOutcome, ItemMoveSelection,
    decode_item_move_intent_for,
};

/// One equipment move of command 9 (ITEM-MOVE-WIRE-1 §4), by the item's server-internal key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EquipmentMove {
    /// A main backpack entry into a free slot.
    Equip { item: ItemKey, to: EquipmentSlot },
    /// A slot's item into the main backpack.
    Unequip { item: ItemKey, from: EquipmentSlot },
}

/// What the equipment owner did with one [`EquipmentMove`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EquipmentMoveResult {
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
        EquipmentMoveRefusal::Occupied => ItemMoveOutcome::NotSupported,
        EquipmentMoveRefusal::Rejected => ItemMoveOutcome::Rejected,
    }
}

/// Domain 9's content for `view`: the main backpack and its entries, and under capability 12 the
/// occupied equipment slots read after it. `None` when either cannot be read.
pub(super) async fn observe_inventory<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    game_session_id: GameSessionId,
    view: &SessionItemView,
) -> Option<InventoryItems> {
    let mut inventory = authority
        .observe_character_inventory(actor, game_session_id)
        .await?;
    if view.equip_drop() {
        inventory.equipment = authority
            .observe_character_equipment(actor, game_session_id)
            .await?;
    }
    Some(inventory)
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
                Refusal::DefinitionMismatch
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
    view: &mut SessionItemView,
) -> ItemMoveStep {
    let Ok(intent) = decode_item_move_intent_for(payload, selection) else {
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
    if matches!(
        intent.destination,
        ItemMoveDestination::Ground(_) | ItemMoveDestination::Container(_)
    ) {
        return ItemMoveStep::Result(ItemMoveOutcome::NotSupported, Vec::new());
    }
    let Some(source) = view.resolve(intent.source) else {
        return ItemMoveStep::Result(ItemMoveOutcome::Stale, Vec::new());
    };
    let request = match intent.destination {
        ItemMoveDestination::Equipment(to) => {
            if view.is_main_backpack(&source) {
                return ItemMoveStep::Result(ItemMoveOutcome::SlotMismatch, Vec::new());
            }
            // Swap (an occupied slot) is ITEM-MOVE-2a's next slice; a view that does not show
            // the slots never moves into one.
            if !view.equip_drop()
                || !view.is_backpack_entry(&source)
                || view.equipped_item(to).is_some()
            {
                return ItemMoveStep::Result(ItemMoveOutcome::NotSupported, Vec::new());
            }
            EquipmentMove::Equip { item: source, to }
        }
        ItemMoveDestination::MainBackpack => match view.equipped_slot(&source) {
            Some(from) => EquipmentMove::Unequip { item: source, from },
            None => return take_from_corpse(authority, actor, command, source, view).await,
        },
        ItemMoveDestination::Ground(_) | ItemMoveDestination::Container(_) => {
            return ItemMoveStep::Result(ItemMoveOutcome::NotSupported, Vec::new());
        }
    };
    match authority.move_equipment(actor, command, request).await {
        EquipmentMoveResult::Committed => moved(authority, actor, command, view).await,
        EquipmentMoveResult::Unknown => ItemMoveStep::Disconnect,
        EquipmentMoveResult::Refused(refusal) => {
            let outcome = equipment_outcome(refusal);
            // The results 10 to 12 are never sent without capability 12.
            let needs_equip_drop = matches!(
                outcome,
                ItemMoveOutcome::SlotMismatch
                    | ItemMoveOutcome::RequirementNotMet
                    | ItemMoveOutcome::Blocked
            );
            let outcome = if needs_equip_drop && !selection.equip_drop {
                ItemMoveOutcome::Rejected
            } else {
                outcome
            };
            ItemMoveStep::Result(outcome, Vec::new())
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

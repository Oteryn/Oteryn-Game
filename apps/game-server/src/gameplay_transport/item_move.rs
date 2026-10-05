//! ITEM-MOVE-1: command 9 (`ItemMoveIntentV1`, ITEM-MOVE-WIRE-0 §5) taking an entry of the open
//! corpse into the Character's main backpack.
//!
//! - **Replay first.** The committed TRANSFER of the CommandRef is looked up before the handle is
//!   resolved, so a command that committed before a reconnect answers `MOVED` although its handle
//!   is now stale.
//! - **Source.** Only an entry of the open corpse, shown in it and in reach on the Channel owner's
//!   observation (`corpse_open::within_reach`); the TRANSFER (`settle_corpse_pickup`) decides the
//!   rest. Equipment, Ground and container destinations answer `NOT_SUPPORTED`.
//! - **Deltas.** After `MOVED` the open corpse's domain 11 delta and then the domain 9 delta
//!   follow the result.
//! - **Unknown outcome.** When the TRANSFER or the replay read cannot tell whether the command
//!   committed, or a committed view cannot be read, the connection ends before its CommandId is
//!   sequenced: the client resends it and replay first answers.

use super::connection::{FreshAdmissionAuthority, UseCommand};
use super::item_view::corpse_open::within_reach;
use super::item_view::{ItemTargetObservation, ItemViewDelta, SessionItemView, UseItemTarget};
use crate::combat_pickup::GroundPickupError;
use crate::durability::item_transfer::{
    ItemTransferError, ItemTransferOutcome, ItemTransferRefusal,
};
use crate::foundation::ExactActorRef;
use oteryn_protocol_oteryn::item_view::{
    ItemMoveDestination, ItemMoveOutcome, ItemMoveSelection, decode_item_move_intent_for,
};

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
    if intent.destination != ItemMoveDestination::MainBackpack {
        return ItemMoveStep::Result(ItemMoveOutcome::NotSupported, Vec::new());
    }
    let Some(entry) = view.resolve(intent.source) else {
        return ItemMoveStep::Result(ItemMoveOutcome::Stale, Vec::new());
    };
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

/// `MOVED`, then the open corpse's domain 11 delta and the domain 9 delta.
async fn moved<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    command: UseCommand,
    view: &mut SessionItemView,
) -> ItemMoveStep {
    let mut deltas = Vec::new();
    if let Some(corpse) = view.continuity().open_corpse {
        let observation = authority.observe_item_target(actor, corpse).await;
        match view.open_corpse_committed(observation) {
            Ok(delta) => deltas.extend(delta),
            Err(_) => return ItemMoveStep::Disconnect,
        }
    }
    let Some(inventory) = authority
        .observe_character_inventory(actor, command.game_session_id)
        .await
    else {
        return ItemMoveStep::Disconnect;
    };
    match view.inventory_committed(inventory) {
        Ok(delta) => deltas.extend(delta),
        Err(_) => return ItemMoveStep::Disconnect,
    }
    ItemMoveStep::Result(ItemMoveOutcome::Moved, deltas)
}

#[cfg(test)]
#[path = "item_move_tests.rs"]
mod tests;

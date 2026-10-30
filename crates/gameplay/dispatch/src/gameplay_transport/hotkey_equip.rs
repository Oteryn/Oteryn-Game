//! PKG5 I17: server-selected equipment intent through the existing fenced owner.
use super::ComposedFreshAdmission;
use super::connection::UseCommand;
use super::equipment_move::{self, read_current_equipment};
use super::item_move::{EquipmentMove, EquipmentMoveReplay, EquipmentMoveResult};
use super::item_ref_admission::pinned_index;
use super::item_view::ItemKey;
use crate::durability::equipment_policy_abi::EquipmentPolicyLookup;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::ExactActorRef;
use oteryn_protocol_oteryn::hotkey_equip::Request;
use oteryn_protocol_oteryn::item_view::{ItemMoveOutcome, wire_slot};

macro_rules! authority_methods {
    () => {
        fn hotkey_equip(
            &self,
            _actor: $crate::foundation::ExactActorRef,
            _command: $crate::gameplay_transport::connection::UseCommand,
            _request: oteryn_protocol_oteryn::hotkey_equip::Request,
        ) -> impl std::future::Future<Output = Option<oteryn_protocol_oteryn::item_view::ItemMoveOutcome>> {
            async { Some(oteryn_protocol_oteryn::item_view::ItemMoveOutcome::Rejected) }
        }
    };
}
pub(super) use authority_methods;
macro_rules! composed_methods {
    () => {
        async fn hotkey_equip(
            &self,
            actor: $crate::foundation::ExactActorRef,
            command: $crate::gameplay_transport::connection::UseCommand,
            request: oteryn_protocol_oteryn::hotkey_equip::Request,
        ) -> Option<oteryn_protocol_oteryn::item_view::ItemMoveOutcome> {
            $crate::gameplay_transport::hotkey_equip::execute(self, actor, command, request).await
        }
    };
}
pub(super) use composed_methods;

pub(super) async fn execute(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    request: Request,
) -> Option<ItemMoveOutcome> {
    // Foundation binds command id, type and payload before this route. Replay must precede
    // lookup: the selected item has already left its original location after a commit.
    match equipment_move::committed_equipment_move(owner, command).await? {
        EquipmentMoveReplay::Moved => return Some(ItemMoveOutcome::Moved),
        EquipmentMoveReplay::Conflicting => return Some(ItemMoveOutcome::Rejected),
        EquipmentMoveReplay::Absent => {}
    }
    let Some(fence) = command.item_fence else {
        return Some(ItemMoveOutcome::Rejected);
    };
    let (definition, slot) = {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let native = equipment_move::native_for_move(
            &runtime,
            owner.active_generation,
            states.is_dead(actor),
            states.has_pending_spell_commit(actor, command.game_session_id),
        )?;
        runtime
            .player_control_facts(actor, command.game_session_id)
            .ok()?;
        let index = pinned_index(&runtime, owner.active_generation)?;
        let Some((key, revision)) = index.definition_at(request.definition) else {
            return Some(ItemMoveOutcome::Stale);
        };
        let Some(policy) = native.equipment_policy(key, revision) else {
            return Some(ItemMoveOutcome::SlotMismatch);
        };
        let Some((slot, _)) = policy
            .claims()
            .iter()
            .find(|(slot, claim)| *slot != 10 && claim.is_ok())
        else {
            return Some(ItemMoveOutcome::SlotMismatch);
        };
        let definition = TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: revision.into(),
        };
        (definition, *slot)
    };
    let inventory = owner
        .root
        .read_item_use_inventory(owner.character, owner.holder, fence)
        .await
        .ok()?;
    let equipment = read_current_equipment(owner, command.game_session_id).await?;
    let Some(slot_wire) = wire_slot(slot) else {
        return Some(ItemMoveOutcome::SlotMismatch);
    };
    // Crystal 63b93e2c game.cpp:3747-3754 searches the backpack first and toggles
    // the worn item only when no carried replacement exists. Never consume a source.
    let direct = inventory.backpack.as_ref().and_then(|bag| {
        bag.entries
            .iter()
            .find(|entry| entry.item.definition == definition)
    });
    let request = if let Some(entry) = direct {
        EquipmentMove::Equip {
            item: ItemKey::Instance(entry.item.item_instance_id),
            to: slot_wire,
        }
    } else if inventory
        .nested
        .iter()
        .any(|(_, entry)| entry.item.definition == definition)
    {
        // The current equipment receipt does not admit a nested source. Do not silently
        // toggle the worn item or stage a non-atomic intermediate move.
        return Some(ItemMoveOutcome::NotSupported);
    } else if let Some(worn) = equipment
        .items
        .iter()
        .find(|item| item.slot == slot && item.definition == definition)
    {
        EquipmentMove::Unequip {
            item: ItemKey::Instance(worn.item_instance_id),
            from: slot_wire,
        }
    } else {
        return Some(if inventory.backpack.is_none() {
            ItemMoveOutcome::NoBackpack
        } else {
            ItemMoveOutcome::Stale
        });
    };
    match equipment_move::move_equipment(owner, actor, command, request).await {
        EquipmentMoveResult::Committed => Some(ItemMoveOutcome::Moved),
        EquipmentMoveResult::Refused(refusal) => Some(super::item_move::equipment_outcome(refusal)),
        EquipmentMoveResult::Unknown => None,
    }
}

//! EQUIP-WIRE-1 (ITEM-MOVE-WIRE-1 §4, ITEM-MOVE-2a slice): the production equipment owner seam
//! of command type 9. Equip moves one direct main-backpack entry into a free equipment slot;
//! unequip moves one equipped item into the main backpack. Both commit in one fenced semantic
//! transaction through `apply_equipment_in_transaction`, keyed by the command's CommandRef, under
//! the spell lane (key-33 writer) and with the Channel content pin of the active generation.
//!
//! Timed items (TIMED-RT-1b, ITEM-MOVE-WIRE-1 §4 slot wiring): the Game node runs no
//! `TimedItemHost` yet, so no equipped item has a live lane. An equip of a timed definition is
//! refused (`NOT_SUPPORTED`) before any write instead of entering a slot non-live; an unequip
//! moves an item that has no lane, so there is no accrued state to stop, checkpoint or drain.
//!
//! Domain 9 shows the occupied slots under capability 12, read through the same current-fence
//! reader movement uses, and names each item by the pinned generation's Item definition index.

use super::ComposedFreshAdmission;
use super::connection::UseCommand;
use super::item_move::{
    EquipmentMove, EquipmentMoveRefusal, EquipmentMoveReplay, EquipmentMoveResult,
};
use super::item_ref_admission::{definition_ref, pinned_index};
use super::item_view::{ItemKey, ViewItem};
use super::{FreshAdmissionStore, GameSessionId, GameSessionState};
use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{ReferenceItemField, ReferenceItemTarget};
use crate::domain::timed_item_host::resolve_timed_definition;
use crate::durability::character_equipment::{self, EquipmentMoveContent};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::item_view::EquipmentSlot;

/// A plain item names a Known definition with container semantics `NotApplicable`; a container
/// or an item with unknown container semantics never enters a slot in this slice.
impl EquipmentMoveContent for NativeGameplayState {
    fn is_plain_item(&self, production_key: &str, revision_ref: &str) -> bool {
        self.item_policy(production_key, revision_ref)
            .is_some_and(|policy| {
                matches!(
                    policy.record().semantics.container,
                    ReferenceItemField::NotApplicable
                )
            })
    }

    /// The hosting runtime's own reading of a timed definition (TIMED-RT-1b): a definition it
    /// would run, or one whose timed content conflicts, is timed.
    fn is_timed_item(&self, production_key: &str, revision_ref: &str) -> bool {
        let Some(policy) = self.item_policy(production_key, revision_ref) else {
            return true;
        };
        let Ok(definition) = ReferenceItemTarget::new(production_key, revision_ref) else {
            return true;
        };
        !matches!(
            resolve_timed_definition(&definition, &policy.record().semantics),
            Ok(None)
        )
    }
}

/// The durable slot code of a wire slot, by its semantic key (ITEM-MOVE-WIRE-1 §2).
pub(super) fn durable_slot(slot: EquipmentSlot) -> Option<u8> {
    Some(match slot.semantic_key() {
        "HEAD" => 1,
        "TORSO" => 2,
        "LEGS" => 3,
        "FEET" => 4,
        "WEAPON" => 5,
        "SHIELD" => 6,
        "AMULET" => 7,
        "RING" => 8,
        "EXTRA" => 10,
        _ => return None,
    })
}

/// The wire slot of a durable slot code; the backpack slot 9 and unknown codes have none.
pub(super) fn wire_slot(code: u8) -> Option<EquipmentSlot> {
    [
        EquipmentSlot::Head,
        EquipmentSlot::Necklace,
        EquipmentSlot::Armor,
        EquipmentSlot::RightHand,
        EquipmentSlot::LeftHand,
        EquipmentSlot::Legs,
        EquipmentSlot::Feet,
        EquipmentSlot::Ring,
        EquipmentSlot::Ammo,
    ]
    .into_iter()
    .find(|slot| durable_slot(*slot) == Some(code))
}

/// The durable move of one wire request: only an `INSTANCE` handle names a durable item.
pub(super) fn durable_move(request: EquipmentMove) -> Option<character_equipment::EquipmentMove> {
    let (key, from_slot, to_slot) = match request {
        EquipmentMove::Equip { item, to } => (item, None, Some(durable_slot(to)?)),
        EquipmentMove::Unequip { item, from } => (item, Some(durable_slot(from)?), None),
    };
    let ItemKey::Instance(item) = key else {
        return None;
    };
    Some(character_equipment::EquipmentMove {
        item,
        from_slot,
        to_slot,
    })
}

/// The pre-durable checks of one move, under the caller's runtime lock: a living actor with no
/// pending spell commit, and a Channel content pin that is the generation whose policies decide
/// the move. `None` refuses it before any durable read.
pub(super) fn native_for_move<'g>(
    runtime: &ChannelRuntimeV1,
    generation: Option<&'g crate::content::ActiveGeneration>,
    dead: bool,
    pending_spell_commit: bool,
) -> Option<&'g NativeGameplayState> {
    if dead || pending_spell_commit {
        return None;
    }
    pinned_index(runtime, generation)?;
    let native = generation?.native_gameplay()?;
    (native.source_digest() == runtime.content_pin().server_artifact_digest()).then_some(native)
}

/// Domain 9's occupied equipment slots of `game_session_id`'s Character, in ascending wire slot
/// order. `None` when the session is not current, the Channel pin is another generation, the
/// equipment owner cannot be read or an item is not named by the pinned index.
pub(super) async fn observe_character_equipment(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    game_session_id: GameSessionId,
) -> Option<Vec<(EquipmentSlot, ViewItem)>> {
    let current = FreshAdmissionStore::from_root(admission.root.clone())
        .current_session_at(game_session_id)
        .await
        .ok()?
        .0;
    if current.session_state() != GameSessionState::Active {
        return None;
    }
    let fence = super::item_fence_of(current)?;
    let digest = {
        let runtime = admission.runtime.lock().await;
        if fence.runtime_scope
            != RuntimeScopeRefV1::channel(
                runtime.binding().world_id(),
                runtime.binding().channel_id(),
            )
            || fence.scope_ownership_generation != runtime.binding().scope_generation()
        {
            return None;
        }
        let digest = runtime.content_pin().server_artifact_digest();
        pinned_index(&runtime, admission.active_generation)?;
        if admission
            .active_generation?
            .native_gameplay()?
            .source_digest()
            != digest
        {
            return None;
        }
        digest
    };
    let read = admission
        .root
        .read_movement_equipment(admission.character, admission.holder, &fence, digest)
        .await
        .ok()?;
    if read.fence() != &fence || read.equipment().content_digest != digest {
        return None;
    }
    let index = pinned_index(
        &*admission.runtime.lock().await,
        admission.active_generation,
    )?;
    let mut equipment = read
        .equipment()
        .items
        .iter()
        .map(|item| {
            Some((
                wire_slot(item.slot)?,
                ViewItem {
                    key: ItemKey::Instance(item.item_instance_id),
                    item_definition_ref: definition_ref(index, &item.definition)?,
                    count: std::num::NonZeroU32::new(item.quantity)?,
                    sub_type: 0,
                },
            ))
        })
        .collect::<Option<Vec<_>>>()?;
    equipment.sort_by_key(|(slot, _)| *slot);
    Some(equipment)
}

/// The equipment receipt of command 9's CommandRef. A session without an item fence has none.
pub(super) async fn committed_equipment_move(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    command: UseCommand,
) -> Option<EquipmentMoveReplay> {
    let Some(fence) = command.item_fence else {
        return Some(EquipmentMoveReplay::Absent);
    };
    let command_id = CommandId::new(command.command_id).ok()?;
    admission
        .root
        .read_equipment_move_receipt(
            admission.character,
            fence.character_id,
            CommandRef::new(command.game_session_id, command_id),
        )
        .await
        .ok()
}

/// One equip or unequip of command 9 through the equipment owner. A dead actor, a pending spell
/// commit, another Channel generation, a session without an item fence or a handle that is not a
/// durable instance is refused before any durable write; a durable failure is `Unknown`.
pub(super) async fn move_equipment(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    command: UseCommand,
    request: EquipmentMove,
) -> EquipmentMoveResult {
    let refused = EquipmentMoveResult::Refused(EquipmentMoveRefusal::Rejected);
    // Lock order (SPELL-LOCK-2 §1.2): the lane before any Channel guard; the move is a key-33
    // writer. `None` is retryable with no side effect while the lane is fenced.
    let Some(_permit) = admission.spell_lane_permit().await else {
        return EquipmentMoveResult::Unknown;
    };
    let native = {
        let runtime = admission.runtime.lock().await;
        let states = admission.spell_states.lock().await;
        let dead = states.is_dead(actor);
        let pending = states.has_pending_spell_commit(actor, command.game_session_id);
        native_for_move(&runtime, admission.active_generation, dead, pending)
    };
    let Some(native) = native else {
        return refused;
    };
    let Some(fence) = command.item_fence else {
        return refused;
    };
    let Some(durable) = durable_move(request) else {
        return refused;
    };
    let Ok(command_id) = CommandId::new(command.command_id) else {
        return refused;
    };
    match admission
        .root
        .commit_equipment_move(
            admission.character,
            admission.holder,
            &fence,
            CommandRef::new(command.game_session_id, command_id),
            native,
            durable,
        )
        .await
    {
        Ok(Ok(_receipt)) => EquipmentMoveResult::Committed,
        Ok(Err(refusal)) => EquipmentMoveResult::Refused(refusal),
        Err(_) => EquipmentMoveResult::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLOTS: [EquipmentSlot; 9] = [
        EquipmentSlot::Head,
        EquipmentSlot::Necklace,
        EquipmentSlot::Armor,
        EquipmentSlot::RightHand,
        EquipmentSlot::LeftHand,
        EquipmentSlot::Legs,
        EquipmentSlot::Feet,
        EquipmentSlot::Ring,
        EquipmentSlot::Ammo,
    ];

    #[test]
    #[allow(clippy::expect_used)]
    fn a_definition_the_generation_does_not_name_is_timed_and_never_enters_a_slot() {
        // The fixture generation has no Item profiles: no definition is known to be untimed.
        let controller = crate::content::native_gameplay::tests::activated_with_item_keys(None);
        let native = controller
            .active()
            .and_then(|generation| generation.native_gameplay())
            .expect("native gameplay");
        assert!(native.is_timed_item("fixture:unknown", "definition-r1"));
        assert!(native.is_timed_item("", ""));
    }

    #[test]
    fn every_wire_slot_maps_to_its_durable_code_and_back() {
        let codes: Vec<Option<u8>> = SLOTS.into_iter().map(durable_slot).collect();
        assert_eq!(
            codes,
            [1, 7, 2, 5, 6, 3, 4, 8, 10].map(Some).to_vec(),
            "HEAD AMULET TORSO WEAPON SHIELD LEGS FEET RING EXTRA"
        );
        for slot in SLOTS {
            assert_eq!(durable_slot(slot).and_then(wire_slot), Some(slot));
        }
        // The backpack slot is never an equipment destination.
        assert_eq!(wire_slot(9), None);
        assert_eq!(wire_slot(0), None);
        assert_eq!(wire_slot(11), None);
    }

    #[test]
    fn only_an_instance_handle_names_a_durable_move() {
        let item = [0xA5; 16];
        assert_eq!(
            durable_move(EquipmentMove::Equip {
                item: ItemKey::Instance(item),
                to: EquipmentSlot::RightHand,
            }),
            Some(character_equipment::EquipmentMove {
                item,
                from_slot: None,
                to_slot: Some(5),
            })
        );
        assert_eq!(
            durable_move(EquipmentMove::Unequip {
                item: ItemKey::Instance(item),
                from: EquipmentSlot::Armor,
            }),
            Some(character_equipment::EquipmentMove {
                item,
                from_slot: Some(2),
                to_slot: None,
            })
        );
        assert_eq!(
            durable_move(EquipmentMove::Equip {
                item: ItemKey::Entity([7; super::super::world_spatial::ENTITY_IDENTITY_BYTES]),
                to: EquipmentSlot::Head,
            }),
            None
        );
    }
}

//! MAP-ITEM-REF-1 (ARCH-MAP-TRACK-PACKETS-V1 §1.6): the production fresh admission's capability 4
//! offer and its domain 9 read. Capability 4 is offered only once [`ITEM_VIEW_OFFERED`] is set and
//! only when the active generation pins a non-empty Item key set. Domain 9 names each durable backpack definition by that generation's
//! Item definition index, read only while the Channel content pin is the same generation; any
//! definition the index does not name fails closed.
//!
//! The index is read from the active generation rather than carried on `ChannelContentPin`:
//! `foundation` is also compiled into the durability integration tests without `content`.

use super::ComposedFreshAdmission;
use super::capabilities::{
    ITEM_VIEW_OFFERED_CAPABILITIES, OfferedCapability, PRODUCTION_OFFERED_CAPABILITIES,
};
use super::connection::UseCommand;
use super::item_view::{
    InventoryItems, ItemKey, ItemTargetObservation, UseItemTarget, ViewItem,
    inventory_from_backpack,
};
use super::world_spatial::{ActorPosition, ChannelEntity, VisibleKind};
use super::{FreshAdmissionStore, GameSessionId, GameSessionState};
use crate::combat_pickup::{CorpsePickupRequest, GroundPickupError, settle_corpse_pickup};
use crate::content::ActiveGeneration;
use crate::content::item_ref::ItemDefinitionIndex;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    ItemTransferDestination, ItemTransferError, ItemTransferOutcome,
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef};
use std::num::NonZeroU32;

/// The non-empty Item definition index of the active generation, if it pins one.
fn item_index(generation: Option<&ActiveGeneration>) -> Option<&ItemDefinitionIndex> {
    generation?
        .native_gameplay()?
        .item_index()
        .map(|index| &**index)
        .filter(|index| !index.is_empty())
}

/// MAP-ITEM-REF-1 Part B: set atomically with the composed corpse observation
/// ([`visible_corpses`]), [`observe_item_target`] and [`take_corpse_entry`] (CP ruling on Codex
/// 4201178872).
const ITEM_VIEW_OFFERED: bool = true;

pub(super) fn offered_capabilities(
    active_generation: Option<&ActiveGeneration>,
) -> &'static [OfferedCapability] {
    offered_capabilities_when(ITEM_VIEW_OFFERED, active_generation)
}

fn offered_capabilities_when(
    item_view_offered: bool,
    active_generation: Option<&ActiveGeneration>,
) -> &'static [OfferedCapability] {
    if item_view_offered && item_index(active_generation).is_some() {
        ITEM_VIEW_OFFERED_CAPABILITIES
    } else {
        PRODUCTION_OFFERED_CAPABILITIES
    }
}

/// The Item definition index of `admission`'s generation while the Channel content pin is that
/// same generation; `None` otherwise, and then no item is named at all.
fn pinned_index<'g>(
    runtime: &ChannelRuntimeV1,
    generation: Option<&'g ActiveGeneration>,
) -> Option<&'g ItemDefinitionIndex> {
    let generation = generation?;
    let index = item_index(Some(generation))?;
    (runtime.content_pin().server_artifact_digest()
        == generation.identity().server_artifact_digest())
    .then_some(index)
}

fn definition_ref(
    index: &ItemDefinitionIndex,
    definition: &TypedDefinitionRef,
) -> Option<NonZeroU32> {
    index.definition_ref(
        &definition.family,
        &definition.production_key,
        &definition.revision_ref,
    )
}

/// MAP-ITEM-REF-1 Part B: the bound corpses of the Channel as D85 objects (entity generation 0),
/// read under the caller's runtime lock. A corpse whose definition the pinned index does not name
/// is left out; with capability 4 not offered, or no pinned index, there are none.
pub(super) fn visible_corpses(
    runtime: &ChannelRuntimeV1,
    generation: Option<&ActiveGeneration>,
) -> Vec<ChannelEntity> {
    if !ITEM_VIEW_OFFERED {
        return Vec::new();
    }
    let Some(index) = pinned_index(runtime, generation) else {
        return Vec::new();
    };
    runtime
        .visible_corpses()
        .into_iter()
        .filter_map(|corpse| {
            let item_definition_ref = index.definition_ref(
                &corpse.item.family,
                &corpse.item.production_key,
                &corpse.item.revision_ref,
            )?;
            Some(ChannelEntity {
                kind: VisibleKind::Corpse {
                    item_definition_ref,
                },
                identity: corpse.identity,
                generation: 0,
                position: ActorPosition {
                    x: corpse.position.x,
                    y: corpse.position.y,
                    floor: corpse.position.floor,
                },
                revision: corpse.revision,
            })
        })
        .collect()
}

/// MAP-ITEM-REF-1 Part B: `USE` on a domain-1 object (ITEM-MOVE-WIRE-0 §4.3). Only a bound corpse
/// is an item target; an unbound or vanished one is `Gone`, so no corpse Item id is ever made up.
/// Its contents are the durable corpse entries, each named by the pinned index; any it does not
/// name, or a failed read, is `None` (`STALE`).
pub(super) async fn observe_item_target(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    target: ItemKey,
) -> Option<ItemTargetObservation> {
    let ItemKey::Entity(identity) = target else {
        return None;
    };
    let (position, corpse, index) = {
        let mut runtime = admission.runtime.lock().await;
        let index = pinned_index(&runtime, admission.active_generation)?;
        let position = runtime
            .borrow_movement_position()
            .read(actor)
            .ok()?
            .position();
        (position, runtime.bound_corpse(identity), index)
    };
    let actor_position = ActorPosition {
        x: position.x,
        y: position.y,
        floor: position.floor,
    };
    let Some(corpse) = corpse else {
        return Some(ItemTargetObservation {
            actor: actor_position,
            target: UseItemTarget::Gone,
        });
    };
    let Some(entries) = admission
        .root
        .read_corpse_contents(admission.character, corpse.item.item_instance_id)
        .await
        .ok()?
    else {
        return Some(ItemTargetObservation {
            actor: actor_position,
            target: UseItemTarget::Gone,
        });
    };
    let contents = entries
        .iter()
        .map(|entry| {
            Some(ViewItem {
                key: ItemKey::Instance(entry.item.item_instance_id),
                item_definition_ref: definition_ref(index, &entry.item.definition)?,
                count: NonZeroU32::new(entry.item.quantity)?,
                sub_type: 0,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    Some(ItemTargetObservation {
        actor: actor_position,
        target: UseItemTarget::Corpse {
            position: ActorPosition {
                x: corpse.position.x,
                y: corpse.position.y,
                floor: corpse.position.floor,
            },
            contents,
        },
    })
}

/// MAP-ITEM-REF-1 Part B: command 9's corpse-entry TRANSFER into the main backpack, one durable
/// `settle_corpse_pickup` keyed by the command's CommandRef. A dead actor, an unbound corpse or a
/// session without an item fence is refused before any durable write; an entry that is not in
/// the corpse's durable contents is `SourceMismatch`.
/// The Channel half of a corpse take, decided before any durable call: the corpse is a bound
/// D85 entity and the entry an Item instance, and a dead actor takes nothing. Gives the corpse's
/// bound Item id and the entry id. An unbound corpse is refused as `AuthorityRejected`; no Item
/// id is ever derived for it.
fn corpse_for_take(
    runtime: &ChannelRuntimeV1,
    dead: bool,
    corpse: ItemKey,
    entry: ItemKey,
) -> Result<([u8; 16], [u8; 16]), GroundPickupError> {
    let rejected = || GroundPickupError::Transfer(ItemTransferError::AuthorityRejected);
    let (ItemKey::Entity(identity), ItemKey::Instance(entry_id)) = (corpse, entry) else {
        return Err(rejected());
    };
    if dead {
        return Err(rejected());
    }
    let corpse = runtime.bound_corpse(identity).ok_or_else(rejected)?;
    Ok((corpse.item.item_instance_id, entry_id))
}

pub(super) async fn take_corpse_entry(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    corpse: ItemKey,
    entry: ItemKey,
) -> Result<ItemTransferOutcome, GroundPickupError> {
    use crate::interaction_chest_use::entry_chest;
    let (corpse, entry_id) = {
        let runtime = admission.runtime.lock().await;
        let dead = admission.spell_states.lock().await.is_dead(actor);
        corpse_for_take(&runtime, dead, corpse, entry)?
    };
    let fence = command
        .item_fence
        .ok_or(GroundPickupError::Transfer(ItemTransferError::AuthorityRejected))?;
    let command_id = crate::foundation::CommandId::new(command.command_id)
        .map_err(|_| GroundPickupError::Transfer(ItemTransferError::InvalidInput))?;
    let contents = admission
        .root
        .read_corpse_contents(admission.character, corpse)
        .await?
        .ok_or(GroundPickupError::SourceMismatch)?;
    let source = contents
        .into_iter()
        .find(|candidate| candidate.item.item_instance_id == entry_id)
        .ok_or(GroundPickupError::SourceMismatch)?;
    let session = crate::combat::DurabilitySession {
        root: admission.root,
        authority: admission.character,
        node: admission.holder,
    };
    settle_corpse_pickup(
        &session,
        admission.chest,
        fence,
        CorpsePickupRequest {
            command: crate::foundation::CommandRef::new(command.game_session_id, command_id),
            corpse_item_instance_id: corpse,
            source_item_instance_id: entry_id,
            source_definition: source.item.definition,
            destination: ItemTransferDestination::MainBackpack,
            content_revision: entry_chest::CONTENT_REVISION.to_owned(),
            ruleset_revision: entry_chest::RULESET_REVISION.to_owned(),
            sim_revision: entry_chest::SIM_REVISION.to_owned(),
        },
    )
    .await
}

pub(super) async fn observe_character_inventory(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    game_session_id: GameSessionId,
) -> Option<InventoryItems> {
    let current = FreshAdmissionStore::from_root(admission.root.clone())
        .current_session_at(game_session_id)
        .await
        .ok()?
        .0;
    if current.session_state() != GameSessionState::Active {
        return None;
    }
    let character_id =
        crate::domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes()).ok()?;
    let backpack = admission
        .root
        .read_character_backpack(admission.character, character_id)
        .await
        .ok()?;
    let index = pinned_index(
        &*admission.runtime.lock().await,
        admission.active_generation,
    )?;
    inventory_from_backpack(backpack.as_ref(), |definition| {
        definition_ref(index, definition)
    })
}

#[cfg(test)]
#[path = "item_ref_admission_tests.rs"]
mod tests;

//! MAP-ITEM-REF-1 (ARCH-MAP-TRACK-PACKETS-V1 §1.6): the production fresh admission's capability 4
//! offer and its domain 9 read. Capability 4 is offered only when the active generation pins a
//! non-empty Item key set. Domain 9 names each durable backpack definition by that generation's
//! Item definition index, read only while the Channel content pin is the same generation; any
//! definition the index does not name fails closed.
//!
//! The index is read from the active generation rather than carried on `ChannelContentPin`:
//! `foundation` is also compiled into the durability integration tests without `content`.

use super::ComposedFreshAdmission;
use super::capabilities::{
    OfferedCapability, PRODUCTION_OFFERED_CAPABILITIES,
    PRODUCTION_OFFERED_CAPABILITIES_WITHOUT_ITEM_VIEW,
};
use super::item_view::{InventoryItems, inventory_from_backpack};
use super::{FreshAdmissionStore, GameSessionId, GameSessionState};
use crate::content::ActiveGeneration;
use crate::content::item_ref::ItemDefinitionIndex;

/// The non-empty Item definition index of the active generation, if it pins one.
fn item_index(generation: Option<&ActiveGeneration>) -> Option<&ItemDefinitionIndex> {
    generation?
        .native_gameplay()?
        .item_index()
        .map(|index| &**index)
        .filter(|index| !index.is_empty())
}

pub(super) fn offered_capabilities(
    active_generation: Option<&ActiveGeneration>,
) -> &'static [OfferedCapability] {
    if item_index(active_generation).is_some() {
        PRODUCTION_OFFERED_CAPABILITIES
    } else {
        PRODUCTION_OFFERED_CAPABILITIES_WITHOUT_ITEM_VIEW
    }
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
    let generation = admission.active_generation?;
    let index = item_index(Some(generation))?;
    let pinned = admission
        .runtime
        .lock()
        .await
        .content_pin()
        .server_artifact_digest();
    if pinned != generation.identity().server_artifact_digest() {
        return None;
    }
    inventory_from_backpack(backpack.as_ref(), |definition| {
        index.definition_ref(
            &definition.family,
            &definition.production_key,
            &definition.revision_ref,
        )
    })
}

#[cfg(test)]
#[path = "item_ref_admission_tests.rs"]
mod tests;

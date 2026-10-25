//! PARITY-ITEM-TRANSFORMS-1: fixed, unbound map furniture rotates through ordinary USE.
//! Durable instances and house interiors remain with their existing custody owners.

use crate::content::native_gameplay::NativeGameplayState;
use crate::content::{ReferenceItemField, ReferenceTransformKind};
use crate::map::boot::BundleMap;
use crate::map::overlay::TilePos;
use crate::map::view::{self, MapFacts};
use oteryn_protocol_oteryn::world_map::MapDefinition;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

use super::world_map::{MapTargetRefusal, MapViewSource, resolve_map_target};

/// Called under the Channel runtime lock, after current session/death and world-action gates.
/// `None` leaves ordinary doors and unsupported objects to their existing route.
pub(super) fn rotate(
    map: &BundleMap,
    native: &NativeGameplayState,
    generation: [u8; 32],
    actor: TilePos,
    target: &WorldObjectTarget,
) -> Option<UseDisposition> {
    let facts = map.facts.pinned();
    let source = MapViewSource {
        overlay: &map.overlay,
        facts: &facts,
        content_generation: generation,
        reset_epoch: 0,
    };
    let resolved = match resolve_map_target(&source, target, |_| None::<()>) {
        Ok(resolved) => resolved,
        Err(MapTargetRefusal::StateRevisionMismatch) => return Some(UseDisposition::StaleState),
        Err(MapTargetRefusal::Use(disposition)) => return Some(disposition),
    };
    let tile = view::compose_tile(&map.overlay, &facts, resolved.pos).ok()??;
    let before = tile
        .entries
        .iter()
        .find(|entry| entry.source.key() == view::MapEntryKey::Base(resolved.placement_key))?
        .facts;
    let MapDefinition::Item(reference) = before.definition else {
        return None;
    };
    let index = native.item_index()?;
    let definition = super::item_look::typed_definition(index, reference)?;
    let policy = native
        .item_policy(&definition.production_key, &definition.revision_ref)?
        .record();
    let ReferenceItemField::Known(transforms) = &policy.semantics.use_transform else {
        return None;
    };
    let to = transforms
        .targets
        .iter()
        .find(|entry| entry.kind == ReferenceTransformKind::Rotate)?;
    let ReferenceItemField::Known(to) = &to.target else {
        return None;
    };
    if !super::map_door::reachable(actor, resolved.pos) {
        return Some(UseDisposition::TooFar);
    }
    let output = native.item_policy(&to.key, &to.revision)?.record();
    // Rotation in this slice preserves every collision fact. Containers, beds, doors, floor
    // changes and other special types need their own owner and cannot be used as furniture.
    let plain = |record: &crate::content::native_gameplay::ItemProfileRecord| {
        matches!(&record.semantics.classification, ReferenceItemField::Known(classification)
            if classification.item_type == ReferenceItemField::NotApplicable)
            && matches!(&record.semantics.physical, ReferenceItemField::Known(physical)
                if physical.pickupable == ReferenceItemField::Known(false))
            && matches!(&record.semantics.stack, ReferenceItemField::Known(stack)
                if stack.stackable == ReferenceItemField::Known(false))
            && record.semantics.container == ReferenceItemField::NotApplicable
            && record.semantics.charges == ReferenceItemField::NotApplicable
            && record.semantics.temporal == ReferenceItemField::NotApplicable
    };
    if facts.house_tile(resolved.pos)
        || before.bound
        || !plain(policy)
        || !plain(output)
        || policy.attributes.blocks_movement.is_none()
        || policy.attributes.blocks_movement != output.attributes.blocks_movement
        || policy.attributes.blocks_projectile.is_none()
        || policy.attributes.blocks_projectile != output.attributes.blocks_projectile
        || policy.attributes.has_height != output.attributes.has_height
        || crate::map::floor::palette_mask(oteryn_world_bundle::bundle::Family::Item, &to.key) != 0
    {
        return Some(UseDisposition::NothingToUse);
    }
    let after = view::EntryFacts {
        definition: MapDefinition::Item(index.definition_ref("Item", &to.key, &to.revision)?),
        appearance_id: view::appearance_id(&to.key, None),
        ..before
    };
    Some(map.facts.rotate_base(
        resolved.placement_key,
        target.expected_revision,
        before,
        after,
    ))
}

//! Crystal 201189f8 data-global/scripts/quests/the_ancient_tombs/actions_oasis_lever_door.lua:26-45.
use super::*;
use oteryn_gameplay_quests::quest::world_callbacks::UseChance;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

pub(in crate::gameplay_transport) fn wins(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    command: CommandRef,
    callback: &str,
    placement: u64,
    chance: &UseChance,
) -> Option<bool> {
    let occurrence = Sha256::new()
        .chain_update(actor.placement_identity())
        .chain_update(command.game_session_id().as_bytes())
        .chain_update(command.command_id().get().to_be_bytes())
        .chain_update(placement.to_be_bytes())
        .chain_update(callback.as_bytes())
        .finalize();
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&occurrence[..16]);
    let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
    let draw = deterministic_decision_u64(
        &root,
        DecisionOccurrenceId::from_bytes(bytes),
        "quest.callback.use.chance",
        0,
    )
    .ok()?;
    let face = crate::spell::uniform_draw(draw, 1, i64::from(chance.faces));
    Some(face <= i64::from(chance.winning_faces))
}

// Crystal actions_oasis_lever_door.lua:26-34. The temporary pickup's custody
// is anchored to this successful lever occurrence, not its visual entry id.
#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_pickup(
    runtime: &ChannelRuntimeV1,
    map: &BundleMap,
    native: &crate::content::native_gameplay::NativeGameplayState,
    source: TilePos,
    key: u64,
    revision: u64,
    cause: QuestCause,
    item: world_callbacks::RelativePickup,
    expires: u64,
) -> Option<crate::map::facts::QuestPickup> {
    let QuestCause::Use(command) = cause else {
        return None;
    };
    let at = TilePos {
        x: source.x.checked_add_signed(i16::from(item.dx))?,
        y: source.y.checked_add_signed(i16::from(item.dy))?,
        ..source
    };
    map.overlay.base().tile(at.x, at.y, at.floor)?;
    if map.facts.house_tile(at) {
        return None;
    }
    let policy = native
        .item_policy_for_source_id(u32::from(item.appearance))?
        .record();
    let definition = crate::map::boot::native_item_definition(
        Some(native),
        &policy.production_definition.production_key,
    )?;
    if !policy.admission.materializable || !definition.pickupable || definition.solid != Some(false)
    {
        return None;
    }
    let facts = crate::map::view::EntryFacts {
        definition: oteryn_protocol_oteryn::world_map::MapDefinition::Item(definition.reference),
        terrain_kind: None,
        appearance_id: item.appearance,
        blocks_projectile: definition.blocks_projectile,
        pickupable: true,
        bound: false,
        count: 1,
        sub_type: 0,
    };
    let occurrence = Sha256::new()
        .chain_update(b"oteryn:quest-temporary-pickup:v1")
        .chain_update(runtime.content_pin().frame_binding_digest())
        .chain_update(runtime.binding().scope_generation().get().to_be_bytes())
        .chain_update(command.game_session_id().as_bytes())
        .chain_update(command.command_id().get().to_be_bytes())
        .chain_update(key.to_be_bytes())
        .chain_update(revision.to_be_bytes())
        .finalize()
        .into();
    Some(crate::map::facts::QuestPickup {
        occurrence,
        at,
        facts,
        expires,
    })
}

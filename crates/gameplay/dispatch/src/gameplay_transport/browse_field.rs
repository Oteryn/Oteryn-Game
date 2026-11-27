//! I18: session-fenced tile observation for the existing ephemeral container owner.

use super::ComposedFreshAdmission;
use super::container_view::{ContainerObservation, browse_projection, browse_rules};
use super::item_ref_admission::pinned_index;
use super::item_view::ItemKey;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::ExactActorRef;
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::item_view::WorldTilePosition;

pub(super) async fn observe(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    fence: Option<CurrentCharacterItemFence>,
    key: ItemKey,
) -> Result<Option<ContainerObservation>, crate::durability::DurabilityError> {
    let (ItemKey::BrowseField { x, y, floor }, Some(fence)) = (key, fence) else {
        return Ok(None);
    };
    let position = WorldTilePosition { x, y, floor };
    let (Ok(x), Ok(y), Some(floor)) = (
        u16::try_from(x), u16::try_from(y), floor.checked_neg().and_then(|floor| i8::try_from(floor).ok()),
    ) else { return Ok(None); };
    let pos = TilePos { x, y, floor };
    if !(-15..=0).contains(&pos.floor) || !admission.ensure_source_map_initialized().await {
        return Ok(Some(ContainerObservation::Gone));
    }
    // The lane serializes the actor's movement and Ground projection. It is not a
    // durable browse lock: each item move independently proves custody and its fence.
    let Some(_permit) = admission.spell_lane_permit().await else {
        return Err(crate::durability::DurabilityError::RootUnavailable);
    };
    if !current(admission, fence).await? { return Ok(Some(ContainerObservation::Gone)); }
    if admission.states.spell_states.lock().await.is_dead(actor) {
        return Ok(Some(ContainerObservation::Gone));
    }
    let Some(map) = admission.bundle_map() else { return Ok(None); };
    let index = {
        let mut runtime = admission.runtime.lock().await;
        if runtime.player_control_facts(actor, fence.game_session_id).is_err() {
            return Ok(Some(ContainerObservation::Gone));
        }
        let Some(at) = super::bundle_map_position(&mut runtime, admission.movement_cells(), actor) else {
            return Ok(None);
        };
        let here = WorldTilePosition { x: at.at.x, y: at.at.y, floor: -at.at.floor };
        match browse_rules::reach(here, position) {
            browse_rules::Reach::Adjacent => {},
            browse_rules::Reach::Walk | browse_rules::Reach::WrongFloor => return Ok(Some(ContainerObservation::TooFar)),
        }
        pinned_index(&runtime, admission.active_generation)
    };
    let Some(index) = index else { return Ok(None); };
    if map.overlay.base().tile(pos.x, pos.y, pos.floor).is_none() {
        return Ok(Some(ContainerObservation::Gone));
    }
    let Some(native) = admission.active_generation.and_then(|generation| generation.native_gameplay()) else { return Ok(None); };
    let Some(ground) = super::loop_move::loop_ground_result(admission, map.overlay.base(), Some(index)).await? else { return Ok(None); };
    let facts = map.facts.pinned();
    let Some(entries) = browse_projection::project(ground.overlay(), &ground.facts(&facts), native, index, pos, admission.reset_epoch) else { return Ok(None); };
    if entries.len() > usize::from(u16::MAX) { return Ok(Some(ContainerObservation::Gone)); }
    // Durable custody reads may wait while reconnect advances the generation.
    if !current(admission, fence).await?
        || admission.runtime.lock().await.player_control_facts(actor, fence.game_session_id).is_err()
    { return Ok(Some(ContainerObservation::Gone)); }
    let observed = admission.states.browse_fields.observe(
        map.overlay.base().digest(), admission.reset_epoch, pos, entries,
    );
    Ok(observed.map(|(entries, lease)| ContainerObservation::BrowseField { position, entries, lease: Some(lease) }))
}

async fn current(admission: &ComposedFreshAdmission<'_, '_, '_>, fence: CurrentCharacterItemFence) -> Result<bool, crate::durability::DurabilityError> {
    let session = super::FreshAdmissionStore::from_root(admission.root.clone())
        .current_session_at(fence.game_session_id).await?.0;
    Ok(session.session_state() == super::GameSessionState::Active && super::item_fence_of(session) == Some(fence))
}

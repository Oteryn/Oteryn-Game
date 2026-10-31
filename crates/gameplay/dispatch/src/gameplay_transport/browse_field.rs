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
) -> Option<ContainerObservation> {
    let ItemKey::BrowseField { x, y, floor } = key else { return None; };
    let fence = fence?;
    let position = WorldTilePosition { x, y, floor };
    let pos = TilePos { x: u16::try_from(x).ok()?, y: u16::try_from(y).ok()?, floor: i8::try_from(floor).ok()? };
    if !(-15..=0).contains(&pos.floor) || !admission.ensure_source_map_initialized().await {
        return Some(ContainerObservation::Gone);
    }
    // The lane serializes the actor's movement and Ground projection. It is not a
    // durable browse lock: each item move independently proves custody and its fence.
    let _permit = admission.spell_lane_permit().await?;
    if !current(admission, fence).await? { return Some(ContainerObservation::Gone); }
    if admission.states.spell_states.lock().await.is_dead(actor) {
        return Some(ContainerObservation::Gone);
    }
    let map = admission.bundle_map()?;
    let index = {
        let mut runtime = admission.runtime.lock().await;
        if runtime.player_control_facts(actor, fence.game_session_id).is_err() {
            return Some(ContainerObservation::Gone);
        }
        let at = super::bundle_map_position(&mut runtime, admission.movement_cells, actor)?;
        let here = WorldTilePosition { x: at.at.x, y: at.at.y, floor: at.at.floor };
        match browse_rules::reach(here, position) {
            browse_rules::Reach::Adjacent => {},
            browse_rules::Reach::Walk | browse_rules::Reach::WrongFloor => return Some(ContainerObservation::TooFar),
        }
        pinned_index(&runtime, admission.active_generation)?
    };
    if map.overlay.base().tile(pos.x, pos.y, pos.floor).is_none() {
        return Some(ContainerObservation::Gone);
    }
    let native = admission.active_generation?.native_gameplay()?;
    let ground = super::loop_move::loop_ground(admission, map.overlay.base(), Some(index)).await?;
    let facts = map.facts.pinned();
    let entries = browse_projection::project(ground.overlay(), &ground.facts(&facts), native, index, pos, admission.reset_epoch)?;
    if entries.len() > usize::from(u16::MAX) { return Some(ContainerObservation::Gone); }
    // Durable custody reads may wait while reconnect advances the generation.
    if !current(admission, fence).await?
        || admission.runtime.lock().await.player_control_facts(actor, fence.game_session_id).is_err()
    { return Some(ContainerObservation::Gone); }
    let (entries, lease) = admission.states.browse_fields.observe(
        map.overlay.base().digest(), admission.reset_epoch, pos, entries,
    )?;
    Some(ContainerObservation::BrowseField { position, entries, lease: Some(lease) })
}

async fn current(admission: &ComposedFreshAdmission<'_, '_, '_>, fence: CurrentCharacterItemFence) -> Option<bool> {
    let session = super::FreshAdmissionStore::from_root(admission.root.clone())
        .current_session_at(fence.game_session_id).await.ok()?.0;
    Some(session.session_state() == super::GameSessionState::Active && super::item_fence_of(session) == Some(fence))
}

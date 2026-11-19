//! Exercise the placed egg and altars, then enter the retained magic forcefield.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::{CharacterId, CharacterLease};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_move::{LoopMove, LoopMoveResult, LoopTarget};
use crate::gameplay_transport::item_view::ItemKey;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
    decode_use_result, encode_item_use_intent, encode_use_intent,
};
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use std::num::NonZeroU64;

const EGG: Position = Position { x: 32144, y: 31912, floor: -8 };
pub(super) const START: Position = Position { x: 32143, ..EGG };
const BEFORE_PORTAL: Position = Position { x: 32111, y: 31936, floor: -8 };
const DESTINATION: Position = Position { x: 32124, y: 31938, floor: -8 };

pub(super) fn matches_world(name: &str) -> bool {
    name.starts_with("qc-lizard-sanctuary-altar-")
}

fn placed(owner: &ComposedFreshAdmission<'_, '_, '_>, at: Position, id: u16) -> TestResult<u64> {
    let tile = owner.bundle_map().ok_or("map")?.overlay.base()
        .tile(at.x, at.y, at.floor).ok_or("source tile")?;
    let ordinal = super::qc_pinned_world::ordinal(tile, id, None)?;
    oteryn_world_bundle::bundle::placement_key(at.floor, at.x, at.y, ordinal)
        .ok_or_else(|| "source placement key".into())
}

async fn position(owner: &ComposedFreshAdmission<'_, '_, '_>, a: &AdmittedSession) -> TestResult<crate::foundation::MovementLocalPosition> {
    let actor = a.runtime_actor.ok_or("actor")?;
    Ok(owner.runtime.lock().await.read_actor_position(actor).map_err(debug)?.position())
}

async fn stand_at(owner: &ComposedFreshAdmission<'_, '_, '_>, a: &AdmittedSession, at: Position) -> TestResult {
    let actor = a.runtime_actor.ok_or("actor")?;
    let mut runtime = owner.runtime.lock().await;
    let current = runtime.read_actor_position(actor).map_err(debug)?;
    let map = owner.bundle_map().ok_or("map")?;
    let doors = crate::map::floor::BundleDoors(map.facts.doors());
    let proof = crate::map::floor::prepare_quest_callback_teleport(
        &runtime, owner.movement_cells(), actor, a.game_session_id, current,
        super::quest_world_callbacks_fixtures::local(at), false,
        &doors,
    ).map_err(debug)?;
    runtime.commit_source_step(proof).map_err(debug)?;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    callback: &Callback,
    altar: Position,
) -> TestResult {
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("capabilities")?;
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "fence")?.ok_or("fence")?;
    owner.runtime.lock().await.bind_attacker_lease(
        actor, admitted.game_session_id,
        CharacterLease::new(CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                            fence.character_lease_generation).map_err(debug)?,
    ).map_err(debug)?;
    let map = owner.bundle_map().ok_or("map")?;
    let digest = map.overlay.base().digest();
    let egg = placed(owner, EGG, 21469)?;
    let key = placed(owner, altar, callback.selector.appearance.ok_or("altar appearance")?)?;
    placed(owner, Position { x: 32112, ..BEFORE_PORTAL }, 1949)?;
    let mut placement = digest.to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget { placement, expected_revision: map.facts.object_revision(key) };
    let move_result = owner.move_loop_item(actor, UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 10_001,
        item_fence: Some(fence.item_fence()),
    }, LoopMove {
        item: ItemKey::MapBase { bundle_digest: digest, placement_key: egg, reset_epoch: 0 },
        count: None,
        target: LoopTarget::MainBackpack,
    }).await;
    assert_eq!(move_result, LoopMoveResult::Committed);
    let custody = async || super::questpc7_outfit_items_fixtures::quantity(h, 21469).await;
    assert_eq!(custody().await?, 1);

    stand_at(owner, admitted, BEFORE_PORTAL).await?;
    super::questpc7_ferumbras_fixtures::step(owner, admitted, StepDirection::East).await?;
    assert_eq!(position(owner, admitted).await?, super::quest_world_callbacks_fixtures::local(BEFORE_PORTAL));

    stand_at(owner, admitted, Position { y: altar.y + 1, ..altar }).await?;
    let bare = encode_use_intent(&target).map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let result = super::quest_world_callbacks_fixtures::wire(owner, admitted, command, 2, bare).await?;
    assert_eq!(decode_use_result(&result).map_err(debug)?, UseDisposition::Rejected);
    assert_eq!(custody().await?, 1, "bare USE must leave the egg in the backpack");
    let selected = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(NonZeroU64::MIN),
        use_with: None,
        use_with_position: None,
        use_with_map_item: Some(MapUseTarget::Object(target)),
    }).map_err(debug)?;
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let command = admitted.continuity.next_command_id;
    let egg_definition = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i21469", "definition-r1"))
        .ok_or("sacred snake egg definition")?;
    let (next, result, actual) = super::rust_lottery_fixtures::exchange(
        owner, *admitted, command, &selected, Some(egg_definition), None,
    ).await?;
    *admitted = next;
    assert_eq!(result, UseDisposition::Committed);
    assert_eq!(custody().await?, 0);
    let discovered: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
        .bind(&callback.storage[0].track).fetch_one(&h.pool).await?;
    assert_eq!(discovered, 1);
    let (next, replay, _) = super::rust_lottery_fixtures::exchange(
        owner, *admitted, command, &actual, None, None,
    ).await?;
    *admitted = next;
    assert_eq!(replay, UseDisposition::Committed);
    assert_eq!(custody().await?, 0);

    stand_at(owner, admitted, BEFORE_PORTAL).await?;
    super::questpc7_ferumbras_fixtures::step(owner, admitted, StepDirection::East).await?;
    assert_eq!(position(owner, admitted).await?, super::quest_world_callbacks_fixtures::local(DESTINATION));
    Ok(())
}

//! Project the committed seal removal into the current native creature owner.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

const DEMON: &str = "oteryn:creature.demon";
// Crystal 201189f8, parchment_room/parchment.lua:1-6.
const SPAWNS: [(u16, u16); 4] = [
    (33060, 31623),
    (33066, 31623),
    (33060, 31627),
    (33066, 31627),
];

/// The caller holds the Channel lane across custody, receipt and publication.
pub(super) async fn publish(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
) -> bool {
    let Some(map) = admission.bundle_map() else {
        return true;
    };
    if map
        .facts
        .base_text(oteryn_gameplay_items::map_pickup::PARCHMENT_SEAL_PLACEMENT)
        .is_none()
    {
        return true;
    }
    let Ok(removal) = admission
        .root
        .read_parchment_removal(
            fence,
            command,
            map.overlay.base().digest(),
            admission.reset_epoch,
        )
        .await
    else {
        return false;
    };
    let Some(removal) = removal else {
        return true;
    };
    let mut runtime = admission.runtime.lock().await;
    if runtime.owner_fence().is_err()
        || runtime.encounter_owner_scope()
            != (fence.runtime_scope, fence.scope_ownership_generation)
    {
        return false;
    }
    let Ok(mut cache) = admission.states.loop_ground.state.lock() else {
        return false;
    };
    if cache.published_parchment_removal == Some(removal) {
        return true;
    }
    let Some(native) = admission
        .active_generation
        .and_then(|active| active.native_gameplay())
    else {
        return false;
    };
    if native.source_digest() != runtime.content_pin().server_artifact_digest()
        || native.install_companion_policies(&mut runtime).is_err()
    {
        return false;
    }
    let Ok(policy) = runtime.companion_policy(DEMON) else {
        return false;
    };
    let health = policy.maximum_health;
    let Ok(occupants) = runtime.positioned_melee_census() else {
        return false;
    };
    for (x, y) in SPAWNS {
        let cell = TilePos { x, y, floor: -15 };
        let at = MovementLocalPosition {
            x: i32::from(x),
            y: i32::from(y),
            floor: 15,
        };
        let cells = admission.movement_cells();
        if cells.index().lookup(
            cells.scope(),
            LogicalCell {
                x: at.x,
                y: at.y,
                z: 15,
            },
        ) == Ok(CollisionClass::Walkable)
            && !map
                .facts
                .quest_callback_blocked(map.overlay.base(), cell, false)
            && !occupants
                .iter()
                .any(|(_, position, _)| position.position() == at)
        {
            // The donor attempts each spawn independently. An occupied cell does not
            // cancel the other three attempts.
            let _ = runtime.commit_quest_creature_spawn(&[(DEMON, health, at)]);
        }
    }
    cache.published_parchment_removal = Some(removal);
    true
}

//! Crystal movement callbacks, called only after the real monster position commit.
use super::*;
use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
use crate::map::boot::BundleCollisionIndexExt;
use crate::map::{overlay::TilePos, view};

pub fn step_in(
    runtime: &mut ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    at: MovementLocalPosition,
) {
    let NativeMovementCollisionIndex::Bundle(collision) = cells.index() else {
        return;
    };
    let map = collision.map();
    if map.overlay.runtime_scope() != runtime.binding().runtime_scope()
        || !runtime.contains_live_creature(actor)
    {
        return;
    }
    let Some(pos) = (|| {
        Some(TilePos {
            x: at.x.try_into().ok()?,
            y: at.y.try_into().ok()?,
            floor: i8::try_from(at.floor).ok()?.checked_neg()?,
        })
    })() else {
        return;
    };
    let Some(tile) = map.overlay.base().tile(pos.x, pos.y, pos.floor) else {
        return;
    };
    let Ok(Some(stack)) = view::compose_tile(&map.overlay, &map.facts, pos) else {
        return;
    };
    // Crystal movements_extraction_spot.lua:26-49 and startup/tables/tile.lua:
    // 1111-1153: reserve the actual Cake step before any later AI movement.
    crate::movement::cake_extraction::reserve_machine_step(
        runtime,
        actor,
        stack
            .entries
            .iter()
            .any(|entry| entry.facts.appearance_id == 8886),
    );
    // Crystal thornfire_wolf.lua:3-12: removal plus a fresh, fully healthy Crystal Wolf.
    // This callback has neither the trap's protection-zone check nor its jaw-trap guard.
    const THORNFIRE_WOLF: &str = "oteryn:creature.thornfire_wolf";
    if stack.entries.iter().any(|e| e.facts.appearance_id == 12721)
        && runtime.matches_live_creature_identity(actor, THORNFIRE_WOLF.as_bytes())
    {
        let Ok(policy) = runtime.companion_policy("oteryn:creature.crystal_wolf") else {
            return;
        };
        if runtime.remove_encounter_actor(actor).is_ok() {
            map.facts.emit_movement_effect(pos, 44);
            if let Ok(created) =
                runtime.admit_encounter_actor(&policy.definition_key, policy.maximum_health, at)
            {
                let _ = runtime.install_creature_policy(created, &policy.definition_key);
            }
        }
        return;
    }
    // Crystal trap.lua:23-29: protection and stacked jaw traps suppress the callback.
    const STARVING_WOLF: &str = "oteryn:creature.starving_wolf";
    if stack.entries.iter().any(|e| e.facts.appearance_id == 12368)
        && !tile.protection_zone()
        && stack
            .entries
            .iter()
            .filter(|e| e.facts.appearance_id == 3482)
            .count()
            <= 1
        && runtime.matches_live_creature_identity(actor, STARVING_WOLF.as_bytes())
    {
        // trap.lua:39-42: do not transform the trap; administrative removal has no loot/XP.
        map.facts.create_movement_item(pos, 12369, 32, || {
            runtime.remove_encounter_actor(actor).is_ok()
        });
    }
}

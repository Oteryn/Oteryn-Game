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
    if let Some(prepared) = prepare_step_in(runtime, cells, actor, at) {
        prepared.publish(runtime);
    }
}

/// The actual original tile DATA is copied before a detached Item callback.
/// A later ACK must not select another trap, replacement or machine from a
/// fresh tile scan. Ordinary movement prepares and publishes in one turn.
#[derive(Clone)]
pub(super) struct PreparedStepIn {
    cells: NativeEntryMovementCells,
    actor: ExactActorRef,
    at: MovementLocalPosition,
    position: TilePos,
    machine_present: bool,
    reaction: Option<Reaction>,
}
#[derive(Clone)]
enum Reaction {
    Thornfire(
        Option<
            std::sync::Arc<
                oteryn_gameplay_content::content::creature_policy::CompiledCreaturePolicy,
            >,
        >,
    ),
    Starving,
}

pub(super) fn prepare_step_in(
    runtime: &ChannelRuntimeV1,
    cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    at: MovementLocalPosition,
) -> Option<PreparedStepIn> {
    let NativeMovementCollisionIndex::Bundle(collision) = cells.index() else {
        return None;
    };
    let map = collision.map();
    if map.overlay.runtime_scope() != runtime.binding().runtime_scope()
        || !runtime.contains_live_creature(actor)
    {
        return None;
    }
    let Some(pos) = (|| {
        Some(TilePos {
            x: at.x.try_into().ok()?,
            y: at.y.try_into().ok()?,
            floor: i8::try_from(at.floor).ok()?.checked_neg()?,
        })
    })() else {
        return None;
    };
    let Some(tile) = map.overlay.base().tile(pos.x, pos.y, pos.floor) else {
        return None;
    };
    let Ok(Some(stack)) = view::compose_tile(&map.overlay, &map.facts, pos) else {
        return None;
    };
    let machine_present = stack
        .entries
        .iter()
        .any(|entry| entry.facts.appearance_id == 8886);
    // Crystal thornfire_wolf.lua:3-12: removal plus a fresh, fully healthy Crystal Wolf.
    // This callback has neither the trap's protection-zone check nor its jaw-trap guard.
    const THORNFIRE_WOLF: &str = "oteryn:creature.thornfire_wolf";
    let reaction = if stack.entries.iter().any(|e| e.facts.appearance_id == 12721)
        && runtime.matches_live_creature_identity(actor, THORNFIRE_WOLF.as_bytes())
    {
        Some(Reaction::Thornfire(
            runtime
                .companion_policy("oteryn:creature.crystal_wolf")
                .ok(),
        ))
    } else {
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
            Some(Reaction::Starving)
        } else {
            None
        }
    };
    Some(PreparedStepIn {
        cells: cells.clone(),
        actor,
        at,
        position: pos,
        machine_present,
        reaction,
    })
}

impl PreparedStepIn {
    pub(super) fn publish(&self, runtime: &mut ChannelRuntimeV1) {
        let NativeMovementCollisionIndex::Bundle(collision) = self.cells.index() else {
            return;
        };
        let map = collision.map();
        if map.overlay.runtime_scope() != runtime.binding().runtime_scope()
            || !runtime.contains_live_creature(self.actor)
        {
            return;
        }
        // The original order is unchanged: Cake registration, then the
        // mutually exclusive Thornfire/Starving callback captured on entry.
        crate::movement::cake_extraction::reserve_machine_step(
            runtime,
            self.actor,
            self.machine_present,
        );
        match &self.reaction {
            Some(Reaction::Thornfire(Some(policy))) => {
                if runtime.remove_encounter_actor(self.actor).is_ok() {
                    map.facts.emit_movement_effect(self.position, 44);
                    if let Ok(created) = runtime.admit_encounter_actor(
                        &policy.definition_key,
                        policy.maximum_health,
                        self.at,
                    ) {
                        let _ = runtime.install_creature_policy(created, &policy.definition_key);
                    }
                }
            }
            Some(Reaction::Starving) => {
                // trap.lua:39-42: no trap transformation, loot or XP.
                map.facts
                    .create_movement_item(self.position, 12369, 32, || {
                        runtime.remove_encounter_actor(self.actor).is_ok()
                    });
            }
            Some(Reaction::Thornfire(None)) | None => {}
        }
    }
}

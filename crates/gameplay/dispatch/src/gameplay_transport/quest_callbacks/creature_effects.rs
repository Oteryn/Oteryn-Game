//! Realize source creature effects with the current Channel's native policies.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) fn spawn_quest_creatures(
        &self,
        runtime: &mut ChannelRuntimeV1,
        origin: MovementLocalPosition,
        keys: &[&str],
    ) -> Vec<ExactActorRef> {
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return Vec::new();
        };
        if runtime.owner_fence().is_err()
            || native.source_digest() != runtime.content_pin().server_artifact_digest()
            || native.install_companion_policies(runtime).is_err()
        {
            return Vec::new();
        }

        // Try the source cell, then the nearest free cells in a bounded area.
        let mut cells = (0..=2_i32).flat_map(|radius| {
            (-radius..=radius).flat_map(move |dx| {
                (-radius..=radius)
                    .filter(move |dy| dx.abs().max(dy.abs()) == radius)
                    .map(move |dy| MovementLocalPosition {
                        x: origin.x + dx,
                        y: origin.y + dy,
                        ..origin
                    })
            })
        });
        let map = self.bundle_map();
        let mut spawned = Vec::new();
        for key in keys {
            let Ok(policy) = runtime.companion_policy(key) else {
                continue;
            };
            let health = policy.maximum_health;
            let Some(at) = cells.find(|at| {
                runtime.native_summon_cell_free(*at)
                    && self.movement_cells.index().lookup(
                        self.movement_cells.scope(),
                        LogicalCell {
                            x: at.x,
                            y: at.y,
                            z: i32::from(at.floor),
                        },
                    ) == Ok(CollisionClass::Walkable)
                    && map.as_ref().is_none_or(|map| {
                        let (Ok(x), Ok(y), Ok(floor)) = (
                            u16::try_from(at.x),
                            u16::try_from(at.y),
                            i8::try_from(-at.floor),
                        ) else {
                            return false;
                        };
                        !map.facts.quest_callback_blocked(
                            map.overlay.base(),
                            tile(world_callbacks::Position { x, y, floor }),
                            false,
                        )
                    })
            }) else {
                break;
            };
            if let Ok(actors) = runtime.commit_quest_creature_spawn(&[(key, health, at)]) {
                spawned.extend(actors);
            }
        }
        spawned
    }
}

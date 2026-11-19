//! Crystal 201189f the_dream_courts/movements_spider_summon.lua:20-24.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::map::facts::QuestCallbackProjection;
use crate::map::view::EntryFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::StepSpawn;
use sha2::{Digest, Sha256};

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "binds the spawn to the exact landed STEP and source revision"
    )]
    pub(super) fn step_spawn(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &ChannelSpellStates,
        map: &BundleMap,
        spawn: &StepSpawn,
        key: u64,
        revision: u64,
        entry: EntryFacts,
        action: u16,
        at: MovementLocalPosition,
        command: CommandRef,
    ) -> UseDisposition {
        let Some(pos) = native(at) else {
            return UseDisposition::Rejected;
        };
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return UseDisposition::Rejected;
        };
        if map.facts.action_at(key) != Some(action)
            || runtime.owner_fence().is_err()
            || map.facts.house_tile(pos)
            || native.install_companion_policies(runtime).is_err()
        {
            return UseDisposition::Rejected;
        }
        let Ok(policy) = runtime.companion_policy(&spawn.creature) else {
            return UseDisposition::Rejected;
        };
        let health = policy.maximum_health;
        let digest = Sha256::new()
            .chain_update(b"oteryn:quest-step-rearm:v1")
            .chain_update(command.game_session_id().as_bytes())
            .chain_update(command.command_id().get().to_be_bytes())
            .chain_update(key.to_be_bytes())
            .finalize();
        let mut roll = [0u8; 8];
        roll.copy_from_slice(&digest[..8]);
        let steps = u64::from(spawn.rearm_steps);
        if steps == 0 {
            return UseDisposition::Rejected;
        }
        let delay_us =
            (1 + u64::from_be_bytes(roll) % steps) * u64::from(spawn.rearm_step_ms) * 1_000;
        let Some(due) = self.owner_now().get().checked_add(delay_us) else {
            return UseDisposition::Exhausted;
        };
        let projection = || {
            map.facts.apply_quest_callback(
                key,
                revision,
                entry,
                &[],
                &[],
                false,
                Some(&QuestCallbackProjection::TimedAction {
                    due,
                    restore: entry,
                    action,
                }),
            )
        };
        let landing = match self.quest_creature_landing(runtime, map, at, &digest) {
            Ok(landing) => landing,
            Err(result) => return result,
        };
        if let Some(landing) = landing {
            if runtime
                .commit_quest_creature_spawn_with_projection(
                    &[(spawn.creature.as_str(), health, landing)],
                    projection,
                )
                .is_err()
            {
                return UseDisposition::Rejected;
            }
        } else if !projection() {
            return UseDisposition::Rejected;
        }
        states.owner_wake.notify_one();
        UseDisposition::Committed
    }

    /// Crystal src/map/map.cpp:234-306 tries the requested tile first and then
    /// shuffles normal adjacent offsets, skipping teleports in the fallback.
    pub(super) fn quest_creature_landing(
        &self,
        runtime: &ChannelRuntimeV1,
        map: &BundleMap,
        at: MovementLocalPosition,
        seed: &[u8],
    ) -> Result<Option<MovementLocalPosition>, UseDisposition> {
        let census = runtime
            .positioned_melee_census()
            .map_err(|_| UseDisposition::Rejected)?;
        let mut neighbours = [
            (-1i8, -1i8),
            (0, -1),
            (1, -1),
            (-1, 0),
            (1, 0),
            (-1, 1),
            (0, 1),
            (1, 1),
        ];
        neighbours.sort_by_cached_key(|(x, y)| {
            Sha256::new()
                .chain_update(seed)
                .chain_update([x.to_ne_bytes()[0], y.to_ne_bytes()[0]])
                .finalize()
        });
        Ok(std::iter::once((0, 0))
            .chain(neighbours)
            .map(|(dx, dy)| MovementLocalPosition {
                x: at.x + i32::from(dx),
                y: at.y + i32::from(dy),
                floor: at.floor,
            })
            .find(|landing| {
                let cell = LogicalCell {
                    x: landing.x,
                    y: landing.y,
                    z: i32::from(landing.floor),
                };
                let cells = self.movement_cells();
                cells.index().lookup(cells.scope(), cell) == Ok(CollisionClass::Walkable)
                    // Crystal src/map/map.cpp:293-295 excludes teleport tiles
                    // from the neighbouring positions considered for a spawn.
                    && (*landing == at || !crate::map::floor::is_teleport(cells, cell))
                    && native_position_open(map, *landing)
                    && !census
                        .iter()
                        .any(|(_, position, _)| position.position() == *landing)
            }))
    }
}

fn native_position_open(map: &BundleMap, landing: MovementLocalPosition) -> bool {
    native(landing).is_some_and(|pos| {
        !map.facts.house_tile(pos)
            && !map
                .facts
                .quest_callback_blocked(map.overlay.base(), pos, false)
    })
}

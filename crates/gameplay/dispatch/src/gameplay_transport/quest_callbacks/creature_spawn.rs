//! Immediate source Game.createMonster and exact-incarnation removal.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::CreatureSpawn;
use oteryn_gameplay_world::movement::source_placement::{SourcePlacementCell, SourcePlacementPlan};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};

pub(super) struct Expiry {
    actor: ExactActorRef,
    pub(super) due: u64,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    // Crystal 201189f8 data-global/scripts/lib/register_actions.lua:1158-1167;
    // data-global/scripts/quests/what_a_foolish_quest/action_cat_basket.lua:32-35;
    // data-global/scripts/quests/the_gravedigger_of_drefia/movements_necromancer_servant.lua:9-11;
    // data-global/scripts/quests/the_gravedigger_of_drefia/actions_sacrifice.lua:3-10.
    pub(in crate::gameplay_transport) fn spawn_quest_creature(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &ChannelSpellStates,
        map: &BundleMap,
        spawn: &CreatureSpawn,
    ) -> bool {
        let Some(native) = self.active_generation.and_then(|g| g.native_gameplay()) else {
            return false;
        };
        if runtime.owner_fence().is_err() {
            return false;
        }
        let Ok(census) = runtime.positioned_melee_census() else {
            return false;
        };
        let center = local(spawn.position);
        let now = self.owner_now().get();
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let mut occurrence = [0; 16];
        occurrence[..2].copy_from_slice(&spawn.position.x.to_be_bytes());
        occurrence[2..4].copy_from_slice(&spawn.position.y.to_be_bytes());
        occurrence[4] = spawn.position.floor.to_be_bytes()[0];
        occurrence[8..].copy_from_slice(&now.to_be_bytes());
        let occurrence = DecisionOccurrenceId::from_bytes(occurrence);
        let mut ordinal = 0;
        // Crystal 201189f8 src/map/map.cpp:234-313 tries the center, then
        // shuffles the eight neighbors for ordinary Game.createMonster.
        let Ok(plan) = SourcePlacementPlan::prepare(center, false, false, |bound| {
            let limit = u64::MAX - u64::MAX % u64::from(bound);
            for _ in 0..64 {
                let draw = deterministic_decision_u64(
                    &root,
                    occurrence,
                    "quest_source_placement",
                    ordinal,
                );
                ordinal += 1;
                if let Ok(value) = draw
                    && value < limit
                {
                    return (value % u64::from(bound)) as u32;
                }
            }
            bound
        }) else {
            return false;
        };
        let cells = self.movement_cells();
        let Ok(Some(position)) = plan.select(
            |at| {
                let pos = super::native(at)?;
                let cell = LogicalCell {
                    x: at.x,
                    y: at.y,
                    z: i32::from(at.floor),
                };
                let base = map.overlay.base().tile(pos.x, pos.y, pos.floor);
                let protection_zone = base.as_ref().is_some_and(|base| base.protection_zone());
                let teleport = crate::map::floor::is_teleport(cells, cell);
                Some(SourcePlacementCell {
                    exists: base.is_some(),
                    // Crystal 201189f8 src/items/tile.cpp:624-630 rejects
                    // masterless monsters on floor changes, portals and PZs.
                    query_add: !protection_zone
                        && !teleport
                        && !cells.spell_tiles().lookup(cells.scope(), cell).map_or_else(
                            |_| map.floors.contains(pos),
                            |tile| tile.flags().floor_change,
                        )
                        && cells.index().lookup(cells.scope(), cell)
                            == Ok(CollisionClass::Walkable)
                        && !map
                            .facts
                            .quest_callback_blocked(map.overlay.base(), pos, false)
                        && census.iter().all(|(_, placed, _)| placed.position() != at),
                    protection_zone,
                    teleport,
                })
            },
            |_, _| Some(true),
        ) else {
            return false;
        };
        if native.install_companion_policies(runtime).is_err() {
            return false;
        }
        let Ok(policy) = runtime.companion_policy(&spawn.creature) else {
            return false;
        };
        let mut expiry = match spawn.lifetime_ms {
            Some(ms) => {
                let Some(due) = now.checked_add(u64::from(ms) * 1000) else {
                    return false;
                };
                let Ok(mut held) = self.states.quest_callbacks.source_creature_expiry.lock() else {
                    return false;
                };
                if held.len() >= super::timed_encounter::MAX_TIMED_ENCOUNTERS
                    || held.try_reserve(1).is_err()
                {
                    return false;
                }
                Some((held, due))
            }
            None => None,
        };
        let Ok(actors) = runtime.commit_quest_creature_spawn(&[(
            &spawn.creature,
            policy.maximum_health,
            position,
        )]) else {
            return false;
        };
        if let Some((held, due)) = expiry.as_mut()
            && let Some(actor) = actors.first()
        {
            held.push(Expiry {
                actor: *actor,
                due: *due,
            });
            states.owner_wake.notify_one();
        }
        true
    }

    pub(super) fn expire_quest_creatures(&self, runtime: &mut ChannelRuntimeV1, now: u64) {
        if let Ok(mut held) = self.states.quest_callbacks.source_creature_expiry.lock() {
            held.retain(|expiry| {
                runtime.contains_live_creature(expiry.actor)
                    && (now < expiry.due
                        || runtime.assert_actor_spell_unreserved(expiry.actor).is_err()
                        || runtime.remove_encounter_actor(expiry.actor).is_err())
            });
        }
    }
}

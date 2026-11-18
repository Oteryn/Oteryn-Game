//! Immediate source Game.createMonster and exact-incarnation removal.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::CreatureSpawn;

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
        let position = local(spawn.position);
        if runtime.owner_fence().is_err()
            || self.movement_cells.index().lookup(
                self.movement_cells.scope(),
                LogicalCell {
                    x: position.x,
                    y: position.y,
                    z: i32::from(position.floor),
                },
            ) != Ok(CollisionClass::Walkable)
            || map
                .facts
                .quest_callback_blocked(map.overlay.base(), tile(spawn.position), false)
            || !runtime
                .positioned_melee_census()
                .is_ok_and(|census| census.iter().all(|(_, at, _)| at.position() != position))
            || native.install_companion_policies(runtime).is_err()
        {
            return false;
        }
        let Ok(policy) = runtime.companion_policy(&spawn.creature) else {
            return false;
        };
        let mut expiry = match spawn.lifetime_ms {
            Some(ms) => {
                let Some(due) = self.owner_now().get().checked_add(u64::from(ms) * 1000) else {
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

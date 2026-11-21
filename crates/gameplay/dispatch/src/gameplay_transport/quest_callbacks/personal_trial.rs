//! Preflight a personal STEP encounter before its durable progress receipt.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn personal_trial_ready(
        &self,
        runtime: &mut ChannelRuntimeV1,
        callback: &Callback,
        key: u64,
    ) -> bool {
        let Some(encounter) = &callback.effects.encounter else {
            return false;
        };
        let Some(native) = self.active_generation.and_then(|a| a.native_gameplay()) else {
            return false;
        };
        if native.install_companion_policies(runtime).is_err() || runtime.owner_fence().is_err() {
            return false;
        }
        let Ok(pending) = self.states.quest_callbacks.timed.lock() else {
            return false;
        };
        if pending.len() >= super::timed_encounter::MAX_TIMED_ENCOUNTERS
            || pending.iter().any(|p| p.source_key() == key)
        {
            return false;
        }
        let Some(map) = self.bundle_map() else {
            return false;
        };
        let Ok(census) = runtime.positioned_melee_census() else {
            return false;
        };
        encounter.monsters.iter().all(|monster| {
            let at = local(monster.position);
            runtime.companion_policy(&monster.creature).is_ok()
                && !map.facts.quest_callback_blocked(
                    map.overlay.base(),
                    tile(monster.position),
                    false,
                )
                && !census
                    .iter()
                    .any(|(_, position, _)| position.position() == at)
                && self.movement_cells.index().lookup(
                    self.movement_cells.scope(),
                    LogicalCell {
                        x: at.x,
                        y: at.y,
                        z: i32::from(at.floor),
                    },
                ) == Ok(CollisionClass::Walkable)
        })
    }
}

pub(super) fn cleanup(runtime: &mut ChannelRuntimeV1, spawned: &mut Vec<ExactActorRef>) {
    for actor in spawned.drain(..) {
        let _ = runtime.remove_encounter_actor(actor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn personal_trial_cleanup_preserves_neighbours_players_and_reused_slots() -> Result<(), String>
    {
        let (mut runtime, player, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_capacity(93, 8);
        let at = MovementLocalPosition {
            x: 32850,
            y: 32673,
            floor: 7,
        };
        let owned = runtime
            .admit_encounter_actor("oteryn:creature.terrified_elephant", 100, at)
            .map_err(|e| format!("{e:?}"))?;
        let neighbour = runtime
            .admit_encounter_actor(
                "oteryn:creature.rat",
                20,
                MovementLocalPosition { x: at.x + 1, ..at },
            )
            .map_err(|e| format!("{e:?}"))?;
        let mut spawned = vec![owned];
        cleanup(&mut runtime, &mut spawned);
        assert!(spawned.is_empty());
        assert!(runtime.read_actor_position(owned).is_err());
        assert!(runtime.read_actor_position(neighbour).is_ok());
        assert!(runtime.player_control_facts(player, session).is_ok());
        let replacement = runtime
            .admit_encounter_actor("oteryn:creature.rat", 20, at)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(replacement.actor_local_id(), owned.actor_local_id());
        assert_ne!(
            replacement.actor_local_generation(),
            owned.actor_local_generation()
        );
        // A late timer carrying a retired exact generation cannot delete a reused slot.
        cleanup(&mut runtime, &mut vec![owned]);
        assert!(runtime.read_actor_position(replacement).is_ok());
        assert!(runtime.read_actor_position(neighbour).is_ok());
        Ok(())
    }
}

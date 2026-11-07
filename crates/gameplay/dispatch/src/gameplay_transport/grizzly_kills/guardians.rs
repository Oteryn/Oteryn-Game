//! One native world guardian slot per bounded authored objective, never per refresh.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::foundation::MovementLocalPosition;
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::death_objectives;

#[derive(Debug)]
pub(super) struct GuardianSlot {
    key: String,
    actor: Option<ExactActorRef>,
    retry_at: u64,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Copy admission already checked the Character fence. Only present players
    /// with an active objective request a spawn. Occupancy and actor exhaustion
    /// retry; they never fabricate a kill or write quest progress.
    pub(in crate::gameplay_transport) async fn refresh_quest_guardians(
        &self,
        session: GameSessionId,
    ) -> bool {
        let (Ok(objectives), Some(map)) = (death_objectives::load(), self.bundle_map()) else {
            return false;
        };
        let mut runtime = self.runtime.lock().await;
        let Some((player, _, _)) = runtime.positioned_actor_census().ok().and_then(|actors| {
            actors
                .into_iter()
                .find(|(_, _, observed)| *observed == Some(session))
        }) else {
            return false;
        };
        let Ok(position) = runtime.read_actor_position(player) else {
            return false;
        };
        let position = position.position();
        if runtime.player_control_facts(player, session).is_err() {
            return false;
        }
        let requested: Vec<_> = self
            .states
            .quest_sessions
            .lock()
            .ok()
            .and_then(|sessions| {
                sessions
                    .get(&session)
                    .and_then(|entry| entry.copy.as_ref())
                    .map(|copy| {
                        objectives
                            .iter()
                            .filter(|objective| {
                                objective.guardian.is_some()
                                    && objective.active(copy.tracks())
                                    && objective.matches(
                                        &objective.creature,
                                        position.x,
                                        position.y,
                                        position.floor,
                                    )
                            })
                            .collect::<Vec<_>>()
                    })
            })
            .unwrap_or_default();
        let dead = self.states.spell_states.lock().await.is_dead(player);
        let now = self.owner_now().get();
        let attack = self.states.attack.lock().await;
        let Ok(mut cache) = attack.kills().1.lock() else {
            return !requested.is_empty();
        };
        // A native death keeps its actor until the existing corpse/kill queue
        // releases it. Never retire a pinned death to make room for another spawn.
        for slot in &mut cache.guardians {
            if let Some(actor) = slot.actor
                && runtime.live_creature_health(actor).is_none()
                && !runtime.corpse_projection_pinned(actor)
            {
                if runtime.read_actor_position(actor).is_ok()
                    && runtime.remove_encounter_actor(actor).is_err()
                {
                    continue;
                }
                slot.actor = None;
                let delay = objectives
                    .iter()
                    .find(|objective| objective.key == slot.key)
                    .and_then(|objective| objective.guardian.as_ref())
                    .map_or(90_000_000, |guardian| u64::from(guardian.respawn_ms) * 1000);
                slot.retry_at = now.saturating_add(delay);
            }
        }
        if dead {
            return false;
        }
        for objective in &requested {
            let Some(guardian) = &objective.guardian else {
                continue;
            };
            let index = if let Some(index) = cache
                .guardians
                .iter()
                .position(|slot| slot.key == objective.key)
            {
                index
            } else {
                if cache.guardians.len() >= 32 {
                    continue;
                }
                cache.guardians.push(GuardianSlot {
                    key: objective.key.clone(),
                    actor: None,
                    retry_at: 0,
                });
                cache.guardians.len() - 1
            };
            let slot = &mut cache.guardians[index];
            if slot.actor.is_some() || now < slot.retry_at {
                continue;
            }
            slot.retry_at = now.saturating_add(5_000_000);
            let at = MovementLocalPosition {
                x: i32::from(guardian.position.x),
                y: i32::from(guardian.position.y),
                floor: -i16::from(guardian.position.floor),
            };
            let tile = TilePos {
                x: guardian.position.x,
                y: guardian.position.y,
                floor: guardian.position.floor,
            };
            if map.facts.house_tile(tile)
                || self.movement_cells.index().lookup(
                    self.movement_cells.scope(),
                    LogicalCell {
                        x: at.x,
                        y: at.y,
                        z: i32::from(at.floor),
                    },
                ) != Ok(CollisionClass::Walkable)
                || map
                    .facts
                    .quest_callback_blocked(map.overlay.base(), tile, false)
                || !runtime.positioned_melee_census().is_ok_and(|actors| {
                    actors
                        .iter()
                        .all(|(_, position, _)| position.position() != at)
                })
            {
                continue;
            }
            let Ok(policy) = runtime.companion_policy(&objective.creature) else {
                continue;
            };
            let health = policy.maximum_health;
            let Ok(actor) = runtime.admit_encounter_actor(&objective.creature, health, at) else {
                continue;
            };
            if runtime
                .install_creature_policy(actor, &objective.creature)
                .is_err()
            {
                let _ = runtime.remove_encounter_actor(actor);
                continue;
            }
            slot.actor = Some(actor);
        }
        !requested.is_empty()
    }

    #[cfg(test)]
    pub(in crate::gameplay_transport) async fn quest_guardian_for_test(
        &self,
        key: &str,
    ) -> Option<ExactActorRef> {
        let runtime = self.runtime.lock().await;
        let attack = self.states.attack.lock().await;
        let cache = attack.kills().1.lock().ok()?;
        cache
            .guardians
            .iter()
            .find(|slot| slot.key == key)
            .and_then(|slot| slot.actor)
            .filter(|actor| runtime.live_creature_health(*actor).is_some())
    }
}

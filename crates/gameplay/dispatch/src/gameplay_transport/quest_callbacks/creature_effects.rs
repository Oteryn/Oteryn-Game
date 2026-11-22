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

pub(super) struct TemporaryActor {
    scope: (
        crate::foundation::RuntimeScopeRefV1,
        crate::foundation::ScopeOwnershipGeneration,
    ),
    actor: ExactActorRef,
    pub(super) expires_at: u64,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn publish_carried_quest_creature(
        &self,
        actor: ExactActorRef,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
        effect: &oteryn_gameplay_quests::quest::reward_callbacks::TemporaryCreature,
    ) {
        let mut runtime = self.runtime.lock().await;
        if runtime.encounter_owner_scope()
            != (fence.runtime_scope, fence.scope_ownership_generation)
            || !runtime
                .bound_attacker_lease(actor, fence.game_session_id)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
        {
            return;
        }
        let Ok(position) = runtime.read_actor_position(actor) else {
            return;
        };
        let origin = position.position();
        let Some(expires_at) = self
            .owner_now()
            .get()
            .checked_add(u64::from(effect.lifetime_ms) * 1000)
        else {
            return;
        };
        {
            let Ok(mut owned) = self.states.quest_callbacks.temporary.lock() else {
                return;
            };
            if owned.len() >= super::timed_encounter::MAX_TIMED_ENCOUNTERS {
                return;
            }
            let scope = runtime.encounter_owner_scope();
            owned.extend(
                self.spawn_quest_creatures(&mut runtime, origin, &[&effect.creature])
                    .into_iter()
                    .map(|actor| TemporaryActor {
                        scope,
                        actor,
                        expires_at,
                    }),
            );
        }
        drop(runtime);
        self.states
            .spell_states
            .lock()
            .await
            .owner_wake
            .notify_one();
    }

    pub(super) fn drain_temporary_quest_creatures(&self, runtime: &mut ChannelRuntimeV1) {
        if runtime.owner_fence().is_err() {
            return;
        }
        let now = self.owner_now().get();
        if let Ok(mut owned) = self.states.quest_callbacks.temporary.lock() {
            owned.retain(|entry| {
                if entry.scope != runtime.encounter_owner_scope() {
                    return false;
                }
                if now < entry.expires_at {
                    return true;
                }
                let _ = runtime.remove_encounter_actor(entry.actor);
                false
            });
        }
    }
}

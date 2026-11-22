//! Realize source creature effects with the current Channel's native policies.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::reward_callbacks::TemporaryCreature;

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

// A pending presentation is local to its original Channel and expires even
// when the client never retries the committed command.
const TEMPORARY_CREATURE_RECOVERY_US: u64 = 60_000_000;

pub(super) struct TemporaryActor {
    scope: (
        crate::foundation::RuntimeScopeRefV1,
        crate::foundation::ScopeOwnershipGeneration,
    ),
    state: TemporaryActorState,
    pub(super) expires_at: u64,
}

enum TemporaryActorState {
    Pending {
        actor: ExactActorRef,
        command: CommandRef,
        fence: CurrentCharacterItemFence,
        effect: TemporaryCreature,
    },
    Spawned(ExactActorRef),
}

impl TemporaryActor {
    fn matches_command(&self, command: CommandRef) -> bool {
        matches!(self.state, TemporaryActorState::Pending { command: pending, .. } if pending == command)
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Share the same capacity bound between retained commands and live creatures.
    /// Return whether this call reserved a new command.
    pub(in crate::gameplay_transport) fn reserve_carried_quest_creature(
        &self,
        actor: ExactActorRef,
        command: CommandRef,
        fence: CurrentCharacterItemFence,
        effect: TemporaryCreature,
    ) -> Option<bool> {
        let expires_at = self
            .owner_now()
            .get()
            .checked_add(TEMPORARY_CREATURE_RECOVERY_US)?;
        let mut owned = self.states.quest_callbacks.temporary.lock().ok()?;
        if let Some(entry) = owned.iter().find(|entry| entry.matches_command(command)) {
            return matches!(&entry.state, TemporaryActorState::Pending {
                actor: pending_actor, fence: pending_fence, effect: pending_effect, ..
            } if *pending_actor == actor && *pending_fence == fence && *pending_effect == effect)
            .then_some(false);
        }
        if owned.len() >= super::timed_encounter::MAX_TIMED_ENCOUNTERS {
            return None;
        }
        owned.push(TemporaryActor {
            scope: (fence.runtime_scope, fence.scope_ownership_generation),
            state: TemporaryActorState::Pending {
                actor,
                command,
                fence,
                effect,
            },
            expires_at,
        });
        Some(true)
    }

    pub(in crate::gameplay_transport) async fn resume_carried_quest_creature(
        &self,
        command: CommandRef,
        actor: ExactActorRef,
    ) -> Result<(), ()> {
        let pending = self
            .states
            .quest_callbacks
            .temporary
            .lock()
            .map_err(|_| ())?
            .iter()
            .any(|entry| entry.matches_command(command));
        if pending {
            let _permit = self.spell_lane_permit().await.ok_or(())?;
            self.settle_carried_quest_creature(command, actor, true)
                .await;
        }
        Ok(())
    }

    /// The caller holds the Channel lane and has qualified the exact command's receipt.
    pub(in crate::gameplay_transport) async fn settle_carried_quest_creature(
        &self,
        command: CommandRef,
        actor: ExactActorRef,
        committed: bool,
    ) {
        let mut runtime = self.runtime.lock().await;
        {
            let Ok(mut owned) = self.states.quest_callbacks.temporary.lock() else {
                return;
            };
            let Some(index) = owned
                .iter()
                .position(|entry| entry.matches_command(command))
            else {
                return;
            };
            let pending = owned.swap_remove(index);
            let TemporaryActorState::Pending {
                actor: pending_actor,
                fence,
                effect,
                ..
            } = pending.state
            else {
                return;
            };
            let now = self.owner_now().get();
            if !committed
                || now >= pending.expires_at
                || pending_actor != actor
                || runtime.encounter_owner_scope() != pending.scope
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
            let Some(expires_at) = now.checked_add(u64::from(effect.lifetime_ms) * 1000) else {
                return;
            };
            owned.extend(
                self.spawn_quest_creatures(&mut runtime, position.position(), &[&effect.creature])
                    .into_iter()
                    .map(|actor| TemporaryActor {
                        scope: pending.scope,
                        state: TemporaryActorState::Spawned(actor),
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
                if let TemporaryActorState::Spawned(actor) = entry.state {
                    let _ = runtime.remove_encounter_actor(actor);
                }
                false
            });
        }
    }
}

//! Source boss portals use the existing movement, quest and native creature owners.
//! A durable room visit retains the original exit and deadline across relog/restart.
use super::{ComposedFreshAdmission, actor_spell::ChannelSpellStates};
use crate::durability::{
    character_progression::CharacterProgressionError,
    quest_state::grizzly_boss::{GrizzlyBossEntry, GrizzlyBossEntryContext, GrizzlyBossEntryState},
    quest_state::{QuestCause, QuestTransitionOutcome},
    quest_world_callback::QuestWorldContext,
    spell_owner_commit::SpellLanePermit,
};
use crate::foundation::{
    ChannelRuntimeV1, CommandId, CommandRef, ExactActorRef, GameSessionId, MovementLocalPosition,
    MovementPositionSnapshot,
};
use crate::map::boot::BundleMap;
use oteryn_gameplay_quests::quest::grizzly_adams::{self as quest, GrizzlyPosition};
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, OnceLock};
mod cleanup;
mod retire;

#[derive(Clone)]
struct Pending {
    actor: ExactActorRef,
    session: GameSessionId,
    origin: MovementPositionSnapshot,
    entered: MovementPositionSnapshot,
    cause: QuestCause,
    context: GrizzlyBossEntryContext,
}

#[derive(Default)]
pub(super) struct GrizzlyBossSessions {
    pending: Mutex<HashMap<GameSessionId, Pending>>,
    checked: Mutex<HashSet<GameSessionId>>,
    returns: Mutex<HashMap<GameSessionId, (ExactActorRef, GrizzlyBossEntry)>>,
}
impl GrizzlyBossSessions {
    pub(super) fn has_pending(&self, session: GameSessionId) -> bool {
        self.pending
            .lock()
            .map_or(true, |records| records.contains_key(&session))
            || self
                .returns
                .lock()
                .map_or(true, |records| records.contains_key(&session))
    }
    pub(super) fn forget(&self, session: GameSessionId) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.remove(&session);
        }
        if let Ok(mut checked) = self.checked.lock() {
            checked.remove(&session);
        }
        if let Ok(mut returns) = self.returns.lock() {
            returns.remove(&session);
        }
    }
}

#[cfg(test)]
#[path = "grizzly_boss/tests.rs"]
mod tests;

#[derive(serde::Deserialize)]
struct Room {
    boss_id: u8,
    qualified: bool,
    bounds: Option<[u16; 5]>,
}
#[derive(serde::Deserialize)]
struct Rooms {
    rooms: Vec<Room>,
}
fn room(id: u8) -> Option<&'static Room> {
    static DATA: OnceLock<Vec<Room>> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str::<Rooms>(include_str!(
            "../../../../../content/quests/grizzly-boss-rooms.json"
        ))
        .map(|data| data.rooms)
        .unwrap_or_default()
    })
    .iter()
    .find(|room| room.boss_id == id && room.qualified && room.bounds.is_some())
}
impl Room {
    fn contains(&self, position: MovementLocalPosition) -> bool {
        self.bounds.is_some_and(|[left, top, right, bottom, z]| {
            position.floor == z as i16
                && (i32::from(left)..=i32::from(right)).contains(&position.x)
                && (i32::from(top)..=i32::from(bottom)).contains(&position.y)
        })
    }
}
fn local(position: GrizzlyPosition) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(position.x),
        y: i32::from(position.y),
        floor: i16::from(position.z),
    }
}
fn step_cause(
    actor: ExactActorRef,
    session: GameSessionId,
    revision: u64,
    epoch: u64,
) -> Option<QuestCause> {
    use sha2::{Digest, Sha256};
    let digest = Sha256::new()
        .chain_update(b"grizzly-boss-step-v1")
        .chain_update(actor.placement_identity())
        .chain_update(revision.to_be_bytes())
        .chain_update(epoch.to_be_bytes())
        .finalize();
    let id = u64::from_be_bytes(digest[..8].try_into().ok()?).max(1);
    Some(QuestCause::Step(CommandRef::new(
        session,
        CommandId::new(id).ok()?,
    )))
}
fn clear(admission: &ComposedFreshAdmission<'_, '_, '_>, session: GameSessionId) {
    if let Ok(mut pending) = admission.states.grizzly_boss.pending.lock() {
        pending.remove(&session);
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn step_grizzly_boss(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
    ) -> MovementPositionSnapshot {
        let Ok(data) = quest::catalogue() else {
            return entered;
        };
        let Some(boss) = data
            .bosses
            .iter()
            .find(|boss| local(boss.portal) == entered.position())
        else {
            return entered;
        };
        let Some(map) = self.bundle_map() else {
            return entered;
        };
        let Some(cause) = step_cause(actor, session, entered.revision(), self.reset_epoch) else {
            return entered;
        };
        let context = {
            let runtime = self.runtime.lock().await;
            GrizzlyBossEntryContext {
                boss_id: boss.id,
                exit: origin.position(),
                world: QuestWorldContext {
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_revision: format!(
                        "sha256:{}",
                        runtime
                            .content_pin()
                            .server_artifact_digest()
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<String>()
                    ),
                    placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                    reset_epoch: self.reset_epoch,
                    actor_placement: actor.placement_identity(),
                    movement_revision: entered.revision(),
                },
            }
        };
        let pending = Pending {
            actor,
            session,
            origin,
            entered,
            cause,
            context,
        };
        if let Ok(mut records) = self.states.grizzly_boss.pending.lock() {
            if records.contains_key(&session) {
                return entered;
            }
            records.insert(session, pending.clone());
        } else {
            return entered;
        }
        self.execute_grizzly_boss(permit, &pending).await;
        self.runtime
            .lock()
            .await
            .read_actor_position(actor)
            .unwrap_or(entered)
    }

    /// Unknown durable commits retain the exact physical predecessor and context. Input
    /// admission invokes this before accepting another step, exactly as other callbacks do.
    async fn execute_grizzly_boss(&self, permit: &SpellLanePermit, pending: &Pending) -> bool {
        let Some(map) = self.bundle_map() else {
            return false;
        };
        let Some(boss) = quest::boss(pending.context.boss_id) else {
            return false;
        };
        let Ok(Some(fence)) = self.current_quest_fence(pending.session).await else {
            return false;
        };
        let Some(catalogue) = self.quest_catalogue.as_ref() else {
            return false;
        };
        if !self
            .reclaim_grizzly_entry_rooms(permit, pending, fence.character_id)
            .await
        {
            return false;
        }
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let optional_pvp = self
            .special_movement_policy(permit, pending.actor, pending.session)
            .await;
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime.read_actor_position(pending.actor) != Ok(pending.entered)
            || runtime.owner_fence().is_err()
            || states.is_dead(pending.actor)
            || !states.spell_idle(&runtime, pending.actor, pending.session)
            || !runtime
                .bound_attacker_lease(pending.actor, pending.session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
        {
            return false;
        }
        let qualified = room(boss.id);
        let census = runtime.positioned_melee_census();
        let occupied = census.as_ref().map_or(true, |actors| {
            actors.iter().any(|(_, at, session)| {
                session.is_some() && qualified.is_some_and(|room| room.contains(at.position()))
            })
        });
        let at = local(boss.boss_position);
        let spawn_open = self.movement_cells.index().lookup(
            self.movement_cells.scope(),
            crate::content::LogicalCell {
                x: at.x,
                y: at.y,
                z: i32::from(at.floor),
            },
        ) == Ok(crate::content::CollisionClass::Walkable)
            && census.as_ref().is_ok_and(|actors| {
                actors
                    .iter()
                    .all(|(_, position, _)| position.position() != at)
            });
        let policy = runtime
            .companion_policy(&boss.creature_key)
            .ok()
            .filter(|policy| policy.definition_key == boss.creature_key);
        let closed = crate::map::floor::BundleDoors(map.facts.doors());
        let teleport = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            self.movement_cells,
            pending.actor,
            pending.session,
            pending.entered,
            local(boss.player_position),
            false,
            &closed,
        );
        if qualified.is_none()
            || occupied
            || !spawn_open
            || policy.is_none()
            || teleport.is_err()
            || !self.states.player_trades.physical(pending.session, None)
        {
            self.grizzly_return(
                &mut runtime,
                &mut states,
                map,
                pending.actor,
                pending.session,
                pending.origin.position(),
                optional_pvp,
            );
            clear(self, pending.session);
            self.states.chat.push_quest_feedback(
                pending.session,
                "This boss room is not available. Your assignment has not been consumed.",
            );
            return true;
        }
        let outcome = slot
            .commit_grizzly_boss_entry(
                self.root,
                self.character,
                self.holder,
                fence,
                pending.cause,
                std::sync::Arc::clone(catalogue),
                permit,
                pending.context.clone(),
            )
            .await;
        match outcome {
            Ok(
                QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_),
            ) => {}
            Err(CharacterProgressionError::Unavailable(_)) => return false,
            _ => {
                self.grizzly_return(
                    &mut runtime,
                    &mut states,
                    map,
                    pending.actor,
                    pending.session,
                    pending.origin.position(),
                    optional_pvp,
                );
                clear(self, pending.session);
                self.states
                    .chat
                    .push_quest_feedback(pending.session, "You cannot enter this boss room yet.");
                return true;
            }
        }
        // A delayed receipt replay cannot resurrect a room already retired by its
        // deadline or by an observed early exit. The exact visit must still own it.
        let current_visit = match self
            .root
            .pending_grizzly_boss_entry(self.character, fence.character_id)
            .await
        {
            Ok(current) => current,
            Err(_) => return false,
        };
        let live_visit = current_visit.is_some_and(|entry| {
            pending.cause == QuestCause::Step(entry.cause)
                && entry.boss_id == boss.id
                && entry.state == GrizzlyBossEntryState::Active
                && !entry.expired
                && entry.world_id == runtime.binding().world_id()
                && Some(entry.channel_id) == runtime.binding().channel_id()
                && entry.scope_generation == runtime.binding().scope_generation()
        });
        if !live_visit {
            self.grizzly_return(
                &mut runtime,
                &mut states,
                map,
                pending.actor,
                pending.session,
                pending.origin.position(),
                optional_pvp,
            );
            clear(self, pending.session);
            drop(states);
            drop(runtime);
            drop(slot);
            self.refresh_quest_session(pending.session, false).await;
            return true;
        }
        // The exact active policy and native collision were checked under this same owner
        // borrow. This is the existing physical creature admission, never a second spawn table.
        let spawned = policy.and_then(|policy| {
            runtime
                .admit_encounter_actor(&boss.creature_key, policy.maximum_health, at)
                .ok()
        });
        let mut installed = spawned.is_some_and(|actor| {
            runtime
                .install_creature_policy(actor, &boss.creature_key)
                .is_ok()
        });
        if installed {
            installed = false;
            if let Ok(proof) = teleport {
                let before = pending.entered.position();
                if let Ok(after) = runtime.commit_source_step(proof) {
                    self.grizzly_movement(
                        &runtime,
                        &mut states,
                        map,
                        pending.actor,
                        pending.session,
                        before,
                        after.position(),
                        optional_pvp,
                    );
                    installed = true;
                }
            }
        }
        if !installed {
            if let Some(actor) = spawned {
                let _ = runtime.remove_encounter_actor(actor);
            }
            self.grizzly_return(
                &mut runtime,
                &mut states,
                map,
                pending.actor,
                pending.session,
                pending.origin.position(),
                optional_pvp,
            );
        }
        clear(self, pending.session);
        drop(states);
        drop(runtime);
        drop(slot);
        self.refresh_quest_session(pending.session, false).await;
        self.states.chat.push_quest_feedback(pending.session, if installed {
            "You have ten minutes to kill and loot this boss. After that you will be returned to the entrance."
        } else { "The boss could not appear. Report to Grizzly Adams before trying again." });
        true
    }

    #[allow(clippy::too_many_arguments)]
    fn grizzly_movement(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        actor: ExactActorRef,
        session: GameSessionId,
        before: MovementLocalPosition,
        after: MovementLocalPosition,
        optional_pvp: Option<bool>,
    ) {
        states.stage_special_movement_step(
            runtime,
            map,
            actor,
            session,
            self.owner_now().get(),
            Some(before),
            after,
            optional_pvp,
        );
        states.stage_underwater_step(
            runtime,
            actor,
            session,
            self.owner_now().get(),
            super::paced_movement::underwater_drown::underwater(map, before),
            super::paced_movement::underwater_drown::underwater(map, after),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn grizzly_return(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        actor: ExactActorRef,
        session: GameSessionId,
        exit: MovementLocalPosition,
        optional_pvp: Option<bool>,
    ) -> bool {
        let Ok(before) = runtime.read_actor_position(actor) else {
            return false;
        };
        if before.position() == exit {
            return true;
        }
        let closed = crate::map::floor::BundleDoors(map.facts.doors());
        let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
            runtime,
            self.movement_cells,
            actor,
            session,
            before,
            exit,
            false,
            &closed,
        ) else {
            return false;
        };
        let Ok(after) = runtime.commit_source_step(proof) else {
            return false;
        };
        self.grizzly_movement(
            runtime,
            states,
            map,
            actor,
            session,
            before.position(),
            after.position(),
            optional_pvp,
        );
        true
    }
}

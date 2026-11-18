//! The survival timer holds the exact admitted player through the shared encounter deadline loop.
use super::*;
use oteryn_gameplay_quests::quest::the_new_frontier::{
    CUROS, CUROS_ARENA, CUROS_RADIUS, CUROS_SURVIVAL_US,
};

#[derive(Clone, Copy)]
pub(super) struct SurvivalTrial {
    actor: ExactActorRef,
    session: GameSessionId,
    key: u64,
    cause: QuestCause,
    due: u64,
}

impl SurvivalTrial {
    pub(super) fn due(&self) -> u64 {
        self.due
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "STEP binds the exact source, session and movement snapshots"
    )]
    pub(super) async fn enter_curos_trial(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        origin: MovementPositionSnapshot,
        entered: MovementPositionSnapshot,
    ) -> MovementPositionSnapshot {
        let optional_pvp = self.special_movement_policy(permit, actor, session).await;
        let eligible = self
            .curos_gate(permit, actor, session, map, callback, entered)
            .await;
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime.read_actor_position(actor) != Ok(entered) {
            return entered;
        }
        let arena = local(CUROS_ARENA);
        let now = self.owner_now();
        if eligible
            && !states.is_dead(actor)
            && states.spell_idle(&runtime, actor, session)
            && self.states.player_trades.physical(session, None)
            && runtime.positioned_melee_census().is_ok_and(|census| {
                !census.iter().any(|(_, position, session)| {
                    let pos = position.position();
                    session.is_some()
                        && pos.floor == arena.floor
                        && pos.x.abs_diff(arena.x) <= u32::from(CUROS_RADIUS)
                        && pos.y.abs_diff(arena.y) <= u32::from(CUROS_RADIUS)
                })
            })
            && let Some(command) =
                step_command(actor, session, entered.revision(), self.reset_epoch)
            && let Some(due) = now.get().checked_add(CUROS_SURVIVAL_US)
            && let Ok(mut trials) = self.states.quest_callbacks.curos.lock()
            && !trials.iter().any(|trial| trial.session == session)
            && trials.len() < super::timed_encounter::MAX_TIMED_ENCOUNTERS
            && trials.try_reserve(1).is_ok()
            && let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
                &runtime,
                self.movement_cells(),
                actor,
                session,
                entered,
                arena,
                super::super::pvp_live::pz_blocked(&states, actor, session, now),
                &crate::map::floor::BundleDoors(&map.facts),
            )
            && let Ok(snapshot) = runtime.commit_source_step(proof)
        {
            trials.push(SurvivalTrial {
                actor,
                session,
                key,
                cause: QuestCause::Step(command),
                due,
            });
            states.owner_wake.notify_one();
            states.stage_special_movement_step(
                &runtime,
                map,
                actor,
                session,
                now.get(),
                Some(entered.position()),
                arena,
                optional_pvp,
            );
            states.stage_underwater_step(
                &runtime,
                actor,
                session,
                now.get(),
                underwater(map, entered.position()),
                underwater(map, arena),
            );
            map.facts.emit_movement_effect(tile(CUROS_ARENA), 11);
            return snapshot;
        }
        self.refuse_quest_step(
            optional_pvp,
            &mut runtime,
            &mut states,
            actor,
            session,
            map,
            callback,
            entered,
            origin,
        );
        runtime.read_actor_position(actor).unwrap_or(entered)
    }

    async fn curos_gate(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &BundleMap,
        callback: &Callback,
        entered: MovementPositionSnapshot,
    ) -> bool {
        let (Ok(Some(fence)), Some(catalogue)) = (
            self.current_quest_fence(session).await,
            self.quest_catalogue.as_ref(),
        ) else {
            return false;
        };
        let context = {
            let runtime = self.runtime.lock().await;
            if runtime.read_actor_position(actor) != Ok(entered) {
                return false;
            }
            let pin = runtime.content_pin();
            QuestWorldContext {
                map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                content_revision: format!("sha256:{}", hex(&pin.server_artifact_digest())),
                placement_context: pin.frame_binding_digest().to_vec(),
                reset_epoch: self.reset_epoch,
                actor_placement: actor.placement_identity(),
                movement_revision: entered.revision(),
                party: Vec::new(),
            }
        };
        matches!(
            self.root
                .check_character_world_callback(
                    self.character,
                    self.holder,
                    fence,
                    &callback.key,
                    catalogue.clone(),
                    permit,
                    context,
                )
                .await,
            Ok(true)
        )
    }

    pub(super) async fn drain_curos_trial(&self, permit: &SpellLanePermit) {
        let now = self.owner_now().get();
        let trials = {
            let Ok(mut trials) = self.states.quest_callbacks.curos.lock() else {
                return;
            };
            let (due, pending): (Vec<_>, Vec<_>) =
                trials.drain(..).partition(|trial| trial.due <= now);
            *trials = pending;
            due
        };
        let Some(map) = self.bundle_map() else { return };
        let Some(callback) = self
            .callback_content()
            .iter()
            .find(|callback| callback.key == CUROS)
        else {
            return;
        };
        for trial in trials {
            let SurvivalTrial {
                actor,
                session,
                key,
                cause,
                ..
            } = trial;
            let Ok(Some(fence)) = self.current_quest_fence(session).await else {
                continue;
            };
            let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
            let optional_pvp = self.special_movement_policy(permit, actor, session).await;
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            Box::pin(self.execute_quest_callback(
                permit,
                &mut runtime,
                &mut states,
                actor,
                session,
                map,
                callback,
                key,
                map.facts.object_revision(key),
                cause,
                Some(fence),
                Some(&mut slot),
                None,
                optional_pvp,
                None,
                None,
            ))
            .await;
        }
    }
}

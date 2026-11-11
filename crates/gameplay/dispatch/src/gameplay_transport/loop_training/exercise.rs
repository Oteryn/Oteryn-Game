use super::*;
use crate::domain::timed_item::TimedValues;
use crate::durability::character_build::BuildOccurrence;
use crate::durability::exercise_training::{
    ExerciseCheckpoint, ExerciseCommitted, ExerciseDummyKey, TrainingWeapon,
};
use crate::durability::item_timed_state::TimedDefinitionFacts;
use oteryn_protocol_oteryn::loop_training::ExerciseIntent;

pub(super) struct Run {
    actor: ExactActorRef,
    fence: CurrentCharacterItemFence,
    weapon: [u8; 16],
    definition: TimedDefinitionFacts,
    family: usize,
    gain_per_charge: u64,
    dummy: ItemKey,
    house_dummy: Option<ExerciseDummyKey>,
    durable: TrainingWeapon,
    spent: u32,
    next_tick: u64,
    checkpoint_due: u64,
    pending: Option<Checkpoint>,
}

/// One retained occurrence: a known receipt replaces its write request, so a
/// later readback never replays the checkpoint or accrues the same ticks again.
enum Checkpoint {
    Pending(CurrentCharacterGameplayFence, ExerciseCheckpoint),
    Committed {
        result: ExerciseCommitted,
        checkpoint_due: u64,
    },
}

#[cfg(test)]
tokio::task_local! {
    pub(in crate::gameplay_transport) static EXERCISE_READBACK_BARRIER:
        std::sync::Arc<(tokio::sync::Notify, tokio::sync::Notify)>;
}

// Crystal 201189f exercise_training_weapons.lua:1,200-203,211.
const EXERCISE_START_EXHAUST_US: u64 = 10_000_000;
impl Run {
    pub(super) fn matches_session(&self, admitted: AdmittedSession) -> Option<bool> {
        (admitted.runtime_actor == Some(self.actor) && admitted.item_fence == Some(self.fence))
            .then_some(true)
    }

    fn remaining_charges(&self) -> u32 {
        match self.pending.as_ref() {
            Some(Checkpoint::Committed { result, .. }) => result.weapon.charges,
            _ => remaining_charges(self.durable.charges, self.spent),
        }
    }

    fn latest_weapon(&self) -> &TrainingWeapon {
        match self.pending.as_ref() {
            Some(Checkpoint::Committed { result, .. }) => &result.weapon,
            _ => &self.durable,
        }
    }
}
fn remaining_charges(durable: u32, pending: u32) -> u32 {
    durable.saturating_sub(pending)
}

fn occurrence() -> Option<BuildOccurrence> {
    let mut bytes = crate::gameplay_transport::SecureIdentifiers::random::<16>()?;
    bytes[6] = (bytes[6] & 15) | 0x70;
    bytes[8] = (bytes[8] & 63) | 0x80;
    BuildOccurrence::from_bytes(bytes).ok()
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Cadence and every actor-end save share the same pending checkpoint/reconciliation owner.
    pub(in crate::gameplay_transport) async fn training_before_save(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        stop: bool,
    ) -> bool {
        self.training_before_save_bound(actor, session, stop, None)
            .await
    }
    pub(in crate::gameplay_transport) async fn loop_training_activity(
        &self,
        admitted: AdmittedSession,
        kind: u32,
    ) -> bool {
        let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
            return true;
        };
        self.training_before_save_bound(
            actor,
            admitted.game_session_id,
            !matches!(kind, 13 | 30 | 67),
            Some(fence),
        )
        .await
    }
    async fn training_before_save_bound(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        stop: bool,
        bound: Option<CurrentCharacterItemFence>,
    ) -> bool {
        let mut sessions = self.states.training.sessions.lock().await;
        let Some(state) = sessions.get_mut(&session) else {
            return true;
        };
        let Some(mut run) = state.exercise.take() else {
            return true;
        };
        if run.actor != actor || bound.is_some_and(|fence| fence != run.fence) {
            state.exercise = Some(run);
            return false;
        }
        let result = self.advance_exercise(&mut run, stop).await;
        match result {
            Some(running) => {
                state.last_weapon = Some(run.latest_weapon().clone());
                if running {
                    state.exercise = Some(run);
                } else {
                    self.release_house_dummy(&run).await;
                }
                true
            }
            None => {
                state.exercise = Some(run);
                false
            }
        }
    }
    async fn release_house_dummy(&self, run: &Run) {
        let Some(key) = run.house_dummy else { return };
        let Some(fence) = self.training_fence(run.fence).await else {
            return;
        };
        let _ = self
            .root
            .release_exercise_dummy(self.character, self.holder, fence, key)
            .await;
    }
    async fn advance_exercise(&self, run: &mut Run, stop: bool) -> Option<bool> {
        let context = self.training_context()?;
        let permit = self.spell_lane_permit().await?;
        let mut slot = self
            .revision_sequencer
            .acquire(run.fence.character_id)
            .await;
        let command = UseCommand {
            game_session_id: run.fence.game_session_id,
            command_id: 1,
            item_fence: Some(run.fence),
        };
        let qualified = self
            .training_target(
                run.actor,
                command,
                run.dummy,
                if matches!(run.family, 0 | 5) { 7 } else { 1 },
            )
            .await
            .and_then(|target| context.rates.dummies.get(&target.production_key))
            .is_some_and(|percentage| {
                context.rates.exercise_gain(run.family, *percentage) == Ok(run.gain_per_charge)
            });
        if matches!(run.pending, Some(Checkpoint::Committed { .. })) {
            return self.finish_exercise_checkpoint(run, stop, qualified).await;
        }
        let current = self.training_fence_durable(run.fence).await.ok()?;
        if current.is_none() && run.pending.is_none() {
            return Some(false);
        }
        let now = self.owner_now().get();
        if qualified && run.pending.is_none() {
            let fence = current?;
            let live = self
                .root
                .read_training_weapon(
                    self.character,
                    self.holder,
                    fence,
                    run.weapon,
                    run.definition.clone(),
                )
                .await
                .ok()?;
            if live != run.durable {
                return Some(false);
            }
            let build = self
                .root
                .read_character_build_state(self.character, run.fence.character_id)
                .await
                .ok()?;
            let old = if run.family == 0 {
                build.magic().0
            } else {
                build.skills()[run.family - 1].0
            };
            // At most one 60-second checkpoint interval is materialized in one cadence.
            for _ in 0..30 {
                if now < run.next_tick || run.spent >= run.durable.charges {
                    break;
                }
                run.spent += 1;
                run.next_tick = run.next_tick.checked_add(2_000_000)?;
                let gain = run.gain_per_charge.checked_mul(u64::from(run.spent))?;
                let next = oteryn_gameplay_progression::loop_training::add_gain(
                    &build,
                    &context.formula,
                    run.family,
                    gain,
                )
                .ok()?;
                let level = if run.family == 0 {
                    next.magic().0
                } else {
                    next.skills()[run.family - 1].0
                };
                if level != old {
                    break;
                }
            }
            let gain = run.gain_per_charge.checked_mul(u64::from(run.spent))?;
            let level_due = if gain > 0 {
                let next = oteryn_gameplay_progression::loop_training::add_gain(
                    &build,
                    &context.formula,
                    run.family,
                    gain,
                )
                .ok()?;
                if run.family == 0 {
                    next.magic().0 != old
                } else {
                    next.skills()[run.family - 1].0 != old
                }
            } else {
                false
            };
            if !stop && !level_due && run.spent < run.durable.charges && now < run.checkpoint_due {
                return Some(true);
            }
        }
        if run.spent == 0 && run.pending.is_none() {
            return Some(qualified && !stop);
        }
        if run.pending.is_none() {
            let event_id = *occurrence()?.as_bytes();
            let occurrence = occurrence()?;
            run.pending = Some(Checkpoint::Pending(
                current?,
                ExerciseCheckpoint {
                    occurrence,
                    event_id,
                    weapon: run.weapon,
                    definition: run.definition.clone(),
                    before: run.durable.clone(),
                    spent: run.spent,
                    family: run.family,
                    gain_per_charge: run.gain_per_charge,
                },
            ));
        }
        let Checkpoint::Pending(fence, request) = run.pending.as_ref()? else {
            return None;
        };
        let checkpoint_due = now.checked_add(60_000_000)?;
        let result = self
            .root
            .commit_exercise_training(
                self.character,
                self.holder,
                *fence,
                context,
                request.clone(),
            )
            .await;
        slot.observe_loop_training();
        #[cfg(test)]
        if let Err(error) = &result {
            eprintln!("LOOP-TRAIN-1 exercise checkpoint: {error:?}");
        }
        run.pending = Some(Checkpoint::Committed {
            result: result.ok()?,
            checkpoint_due,
        });
        #[cfg(test)]
        if let Ok(barrier) = EXERCISE_READBACK_BARRIER.try_with(std::sync::Arc::clone) {
            barrier.0.notify_one();
            barrier.1.notified().await;
        }
        let finished = self.finish_exercise_checkpoint(run, stop, qualified).await;
        drop(permit);
        finished
    }

    /// Complete only the retained receipt. Failed reads keep it in the same
    /// session owner; there is no retry wait or checkpoint write in this path.
    async fn finish_exercise_checkpoint(
        &self,
        run: &mut Run,
        stop: bool,
        qualified: bool,
    ) -> Option<bool> {
        let Checkpoint::Committed {
            result,
            checkpoint_due,
        } = run.pending.as_ref()?
        else {
            return None;
        };
        let runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime.owner_fence().is_err()
            || runtime
                .player_control_facts(run.actor, run.fence.game_session_id)
                .is_err()
            || runtime.binding().runtime_scope() != run.fence.runtime_scope
            || runtime.binding().scope_generation() != run.fence.scope_ownership_generation
        {
            return Some(false);
        }
        let state = states.get_mut(&runtime, run.actor, run.fence.game_session_id)?;
        let Some(current) = self.training_fence_durable(run.fence).await.ok()? else {
            return Some(false);
        };
        if current.expected_character_revision != result.revision {
            // A receipt can outlive an unrelated Character write. It may still
            // install only if its build is current under an unchanged fence.
            let build = self
                .root
                .read_character_build_state(self.character, run.fence.character_id)
                .await
                .ok()?;
            if build != result.build {
                return Some(false);
            }
            if self.training_fence_durable(run.fence).await.ok()? != Some(current) {
                return None;
            }
        }
        // Resolve session authority last: the revision/build reads above are
        // historical evidence and cannot authorize a successor connection.
        let fresh = FreshAdmissionStore::from_root(self.root.clone())
            .current_session(run.fence.game_session_id)
            .await
            .ok()?;
        if fresh.session_state() == crate::foundation::GameSessionState::Terminal
            || super::super::item_fence_of(fresh) != Some(run.fence)
        {
            return Some(false);
        }
        if let Some(key) = run.house_dummy
            && !self
                .root
                .reserve_exercise_dummy(self.character, self.holder, current, key)
                .await
                .ok()?
        {
            return Some(false);
        }
        state.rebase_after_death(&result.build);
        run.durable = result.weapon.clone();
        run.spent = 0;
        run.checkpoint_due = *checkpoint_due;
        run.pending = None;
        Some(!stop && qualified && run.durable.charges > 0)
    }

    pub(in crate::gameplay_transport) async fn exercise_training(
        &self,
        admitted: AdmittedSession,
        command: UseCommand,
        intent: ExerciseIntent,
        weapon: Option<ItemKey>,
        dummy: Option<ItemKey>,
    ) -> Option<TrainingResult> {
        let Some(context) = self.training_context() else {
            return Some(TrainingResult::REFUSED);
        };
        let (Some(actor), Some(item)) = (admitted.runtime_actor, admitted.item_fence) else {
            return Some(TrainingResult::REFUSED);
        };
        if command.item_fence != Some(item) || self.training_fence(item).await.is_none() {
            return Some(TrainingResult::REFUSED);
        }
        let stop = matches!(intent, ExerciseIntent::Stop | ExerciseIntent::Start { .. });
        if !self
            .training_before_save_bound(actor, admitted.game_session_id, stop, Some(item))
            .await
        {
            return None;
        }
        if let ExerciseIntent::Start { .. } = intent {
            if matches!(
                self.save_spell_training(actor, admitted.game_session_id, true)
                    .await,
                super::super::actor_spell::TrainingSave::Unknown
            ) {
                return None;
            }
            let (Some(ItemKey::Instance(weapon)), Some(dummy)) = (weapon, dummy) else {
                return Some(TrainingResult::REFUSED);
            };
            let now = self.owner_now().get();
            let mut sessions = self.states.training.sessions.lock().await;
            let state = sessions.entry(admitted.game_session_id).or_default();
            if now < state.ready_at_us {
                return Some(TrainingResult::REFUSED);
            }
            let Some(fence) = self.training_fence(item).await else {
                return Some(TrainingResult::REFUSED);
            };
            let inventory = self
                .root
                .read_item_use_inventory(self.character, self.holder, item)
                .await
                .ok()?;
            let Some(entry) = inventory
                .backpack
                .iter()
                .flat_map(|bag| &bag.entries)
                .find(|entry| entry.item.item_instance_id == weapon)
            else {
                return Some(TrainingResult::REFUSED);
            };
            let definition = entry.item.definition.clone();
            let Some(weapon_profile) = context.rates.weapons.get(&definition.production_key) else {
                return Some(TrainingResult::REFUSED);
            };
            let family = weapon_profile.family;
            let Some(target) = self
                .training_target(
                    actor,
                    command,
                    dummy,
                    if matches!(family, 0 | 5) { 7 } else { 1 },
                )
                .await
            else {
                return Some(TrainingResult::REFUSED);
            };
            let Some(gain_per_charge) = context
                .rates
                .dummies
                .get(&target.production_key)
                .and_then(|percentage| context.rates.exercise_gain(family, *percentage).ok())
            else {
                return Some(TrainingResult::REFUSED);
            };
            let native = self.active_generation?.native_gameplay()?;
            let record = native
                .item_policy(&definition.production_key, &definition.revision_ref)?
                .record();
            let crate::content::ReferenceItemField::Known(charges) = &record.semantics.charges
            else {
                return Some(TrainingResult::REFUSED);
            };
            let crate::content::ReferenceItemField::Known(charges) = charges.count else {
                return Some(TrainingResult::REFUSED);
            };
            // Crystal 201189f data/items/items.xml:53481-53552,65926-65996. The
            // pinned native item and donor exercise profile must agree before use.
            if charges != weapon_profile.charges {
                return Some(TrainingResult::REFUSED);
            }
            let definition = TimedDefinitionFacts {
                definition,
                full: TimedValues::new(Some(charges), None).ok()?,
                lit_continuous: false,
            };
            let durable = self
                .root
                .read_training_weapon(
                    self.character,
                    self.holder,
                    fence,
                    weapon,
                    definition.clone(),
                )
                .await
                .ok()?;
            if let Some(key) = target.house_dummy {
                match self
                    .root
                    .reserve_exercise_dummy(self.character, self.holder, fence, key)
                    .await
                {
                    Ok(true) => {}
                    Ok(false) => return Some(TrainingResult::REFUSED),
                    Err(_) => return None,
                }
            }
            state.ready_at_us = now.checked_add(EXERCISE_START_EXHAUST_US)?;
            state.exercise = Some(Run {
                actor,
                fence: item,
                weapon,
                definition,
                family,
                gain_per_charge,
                dummy,
                house_dummy: target.house_dummy,
                durable,
                spent: 0,
                next_tick: now.checked_add(2_000_000)?,
                checkpoint_due: now.checked_add(60_000_000)?,
                pending: None,
            });
        }
        let root = self
            .root
            .read_current_character(self.character, item.character_id)
            .await
            .ok()?;
        let sessions = self.states.training.sessions.lock().await;
        let run = sessions
            .get(&admitted.game_session_id)
            .and_then(|state| state.exercise.as_ref());
        Some(TrainingResult {
            disposition: if run.is_some() { 1 } else { 2 },
            revision: root.revision.get(),
            pool_seconds: 0,
            charges: run.map_or_else(
                || {
                    sessions
                        .get(&admitted.game_session_id)
                        .and_then(|state| state.last_weapon.as_ref())
                        .map_or(0, |weapon| weapon.charges)
                },
                Run::remaining_charges,
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn exercise_start_exhaust_is_crystals_ten_seconds() {
        assert_eq!(super::EXERCISE_START_EXHAUST_US, 10_000_000);
    }

    #[test]
    fn status_charge_count_includes_uncheckpointed_ticks() {
        assert_eq!(super::remaining_charges(500, 0), 500);
        assert_eq!(super::remaining_charges(500, 1), 499);
        assert_eq!(super::remaining_charges(1, 2), 0);
    }
}

//! Copied source death handlers run through the canonical FIFO cursor.
//! Invoke exactly one deferred task after the existing timer owner arbiter
//! selects it using the notice's actual committed enqueue owner clock.
use super::actor_spell;
use super::attack;
use super::soul_war_malice;
use super::*;
use crate::movement::monster_map::CurrentMonsterMap;

/// One retained action payload for the canonical head's current copied event.
/// This carries no independent ordering: only the carrier's FIFO/cursor selects it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PreparedSourceDeathAction {
    begin: oteryn_gameplay_entities::carrier::native_source_death::SourceCreatureDeathSeal,
    index: usize,
    now_us: u64,
}
impl PreparedSourceDeathAction {
    fn capture(
        runtime: &ChannelRuntimeV1,
        begin: &oteryn_gameplay_entities::carrier::native_source_death::SourceCreatureDeathSeal,
        now_us: u64,
    ) -> Result<Self, ()> {
        let index = runtime
            .source_creature_death_event_index(begin)
            .map_err(|_| ())?
            .ok_or(())?;
        Ok(Self {
            begin: begin.clone(),
            index,
            now_us,
        })
    }
    pub(super) fn begin(
        &self,
    ) -> &oteryn_gameplay_entities::carrier::native_source_death::SourceCreatureDeathSeal {
        &self.begin
    }
    pub(super) fn event(&self) -> &crate::content::QualifiedNativeDeathEvent {
        &self.begin.events()[self.index]
    }
    pub(super) fn now_us(&self) -> u64 {
        self.now_us
    }
    pub(super) fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        self.begin.current_for(runtime)
            && runtime.source_creature_death_event_index(&self.begin).ok() == Some(Some(self.index))
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Execute a retained copied action after the read Map transaction and all
    /// volatile guards have ended. Unknown outcomes keep the exact payload.
    pub(super) async fn finish_prepared_source_death_action(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> bool {
        let (action, callbacks) = {
            let runtime = self.runtime.lock().await;
            let Ok(states) = self.states.spell_states.try_lock() else {
                return false;
            };
            let Some(action) = states.source_death_action.as_ref() else {
                return true;
            };
            if !states.source_death_action_ready
                || !action.current_for(&runtime)
                || states
                    .source_death_action_reservation
                    .as_ref()
                    .is_none_or(|reservation| {
                        runtime
                            .validate_source_death_action_reservation(reservation)
                            .is_err()
                    })
            {
                return false;
            }
            (action.clone(), states.native_callbacks.clone())
        };
        if !matches!(
            callbacks
                .finish_source_death_action(self, permit, &action)
                .await,
            Ok(true)
        ) {
            return false;
        }
        let mut runtime = self.runtime.lock().await;
        let Ok(mut states) = self.states.spell_states.try_lock() else {
            return false;
        };
        if states.source_death_action.as_ref() != Some(&action) || !action.current_for(&runtime) {
            return false;
        }
        let Some(reservation) = states.source_death_action_reservation.as_ref().cloned() else {
            return false;
        };
        if runtime
            .validate_source_death_action_reservation(&reservation)
            .is_err()
            || !matches!(
                callbacks.publish_source_death_action(&mut runtime, &action),
                Ok(true)
            )
        {
            return false;
        }
        if runtime
            .release_source_death_action_reservation(reservation)
            .is_err()
        {
            return false;
        }
        states.source_death_action_reservation = None;
        states.source_death_action_ready = false;
        if runtime
            .acknowledge_source_creature_death_event(action.begin(), action.event())
            .is_err()
        {
            return false;
        }
        states.source_death_action = None;
        states.source_death_action_ready = false;
        true
    }

    /// Pure preparation needs no Map/SQL transaction when all older owners
    /// have already completed. Their timers remain selected by original due.
    pub(super) fn source_death_can_begin_without_timer_read(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &actor_spell::ChannelSpellStates,
        now: u64,
    ) -> Result<bool, ()> {
        let Some(notice) = runtime.next_source_creature_death().map_err(|_| ())? else {
            return Ok(false);
        };
        let cutoff = notice.enqueued_at_us().ok_or(())?;
        Ok(!states
            .soul_war_callbacks
            .next_native_due()
            .into_iter()
            .chain(states.primal_callbacks.next_due_us())
            .chain(states.rotten_blood_callbacks.next_due_time())
            .chain(states.native_callbacks.next_due_time())
            .any(|due| due <= cutoff && due <= now))
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_source_death_head(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut actor_spell::ChannelSpellStates,
        attack: &mut attack::ChannelAttackStates,
        room: &crate::content::QualifiedNativeEntryRoom,
        native: &crate::content::native_gameplay::NativeGameplayState,
        now: u64,
    ) -> Result<bool, ()> {
        let Some(notice) = runtime.next_source_creature_death().map_err(|_| ())? else {
            return Ok(true);
        };
        let begin = runtime
            .begin_source_creature_death(&notice)
            .map_err(|_| ())?;
        #[cfg(test)]
        eprintln!(
            "[DBG-W11-DEATH] begin source={} events={}",
            begin.definition_key(),
            begin.events().len()
        );
        attack
            .kills()
            .freeze_source_drop_profile(runtime, &begin, native)?;
        while let Some(index) = runtime
            .source_creature_death_event_index(&begin)
            .map_err(|_| ())?
        {
            let event = &begin.events()[index];
            if let Some(action) = &states.source_death_action {
                if !action.current_for(runtime) {
                    return Err(());
                }
            } else {
                states.source_death_action =
                    Some(PreparedSourceDeathAction::capture(runtime, &begin, now)?);
            }
            let action_now = states.source_death_action.as_ref().ok_or(())?.now_us();
            // Source clocks/Quest player eligibility are captured at this
            // SPECIFIC copied handler, after all previous handler effects.
            // Retain the chosen values in the same canonical event state before
            // any async side effect; replay must never resample them.
            let handled = if event.name() == "SoulCageDeath" {
                soul_war_malice::consume_cage_death(
                    runtime, states, room, native, &begin, event, action_now,
                )?
            } else {
                let callbacks = states.native_callbacks.clone();
                let action = states.source_death_action.as_ref().ok_or(())?;
                #[cfg(test)]
                eprintln!(
                    "[DBG-W11-DEATH] event={} source={}",
                    event.name(),
                    begin.definition_key()
                );
                let prepared =
                    callbacks.prepare_source_death_action(runtime, room, native, action)?;
                if prepared {
                    if states.source_death_action_reservation.is_none() {
                        states.source_death_action_reservation = Some(
                            runtime
                                .reserve_source_death_action(action.begin(), action.event(), &[])
                                .map_err(|_| ())?,
                        );
                    }
                    states.source_death_action_ready = true;
                }
                // Preparation never acknowledges a copied event. Its physical
                // writer runs only in the detached outer phase above.
                false
            };
            if !handled {
                return Ok(false);
            }
            runtime
                .acknowledge_source_creature_death_event(&begin, event)
                .map_err(|_| ())?;
            states.source_death_action = None;
            states.source_death_action_ready = false;
        }
        // A rowless corpse needs the closed uncredited physical port. Do not
        // discard its only completion capability before that pending work is
        // retained; ordinary credited entries retain it in their existing queue.
        if !attack.kills().has_source_drop(&begin)
            && native
                .reward_table()
                .row(begin.definition_key())
                .ok()
                .is_some_and(|row| row.corpse_item.is_some())
        {
            return Ok(false);
        }
        let completed = runtime
            .finish_source_creature_death_task(&begin)
            .map_err(|_| ())?;
        attack.kills().complete_source_drop(runtime, &completed)?;
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) async fn drive_one_source_creature_death(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut actor_spell::ChannelSpellStates,
        attack: &mut attack::ChannelAttackStates,
        room: &crate::content::QualifiedNativeEntryRoom,
        native: &crate::content::native_gameplay::NativeGameplayState,
        map: &mut CurrentMonsterMap<'_, '_>,
        parties: &super::monster_profile_cycle::physical_spell::ReferenceParties,
        now: u64,
    ) -> Result<bool, ()> {
        let Some(notice) = runtime.next_source_creature_death().map_err(|_| ())? else {
            return Ok(true);
        };
        let cutoff = notice.enqueued_at_us().ok_or(())?;
        #[cfg(test)]
        eprintln!("[DBG-W11-DEATH] source-head cutoff={cutoff} now={now}");
        // Each existing timer owner must select individual original deadlines.
        // A synthetic wake value or an unclocked legacy hit cannot establish
        // task ordering. Keep the head until that owner supplies its cutoff port.
        if states
            .soul_war_callbacks
            .next_native_due()
            .into_iter()
            .chain(states.primal_callbacks.next_due_us())
            .chain(states.rotten_blood_callbacks.next_due_time())
            .any(|due| due <= cutoff && due <= now)
        {
            #[cfg(test)]
            eprintln!("[DBG-W11-DEATH] source-head waits for older non-W01 timer");
            return Ok(false);
        }
        let active = self.active_generation.ok_or(())?;
        let callbacks = states.native_callbacks.clone();
        callbacks
            .drain_due_through(
                active, runtime, states, attack, room, native, map, parties, cutoff, now,
            )
            .await?;
        if !map.validate_transaction(runtime).await {
            return Err(());
        }
        self.prepare_source_death_head(runtime, states, attack, room, native, now)
    }
}

//! Copied source death handlers run through the canonical FIFO cursor.
//! Invoke exactly one deferred task after the existing timer owner arbiter
//! selects it using the notice's actual committed enqueue owner clock.
use super::actor_spell;
use super::attack;
use super::soul_war_callbacks::megalomania::phases::CopiedDeath as MegalomaniaDeath;
use super::soul_war_malice;
use super::*;
use crate::movement::monster_map::CurrentMonsterMap;

mod creature_step;
mod heart;
mod rotten;
mod soul_war;
mod uncredited;
pub(super) use soul_war::SoulWarAction;
pub(super) use uncredited::PendingSourceCorpse;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SourceWorkFamily {
    Native,
    TaintOne,
    SoulWarAttack,
    Greed,
    Physical,
    RottenBlood,
    CreatureStep,
    Detached,
    Profile,
}

/// Compare original causal deadlines. Scheduler retry wakes and busy-lock
/// sentinels must never become a source event's place before the death FIFO.
pub(super) fn next_source_work(
    runtime: &ChannelRuntimeV1,
    states: &actor_spell::ChannelSpellStates,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> Result<Option<(SourceWorkFamily, u64)>, ()> {
    let detached = states
        .soul_war_callbacks
        .next_native_due_without_inline_callbacks()
        .into_iter()
        .chain(
            states
                .soul_war_callbacks
                .next_splinter_due(runtime, native)?,
        )
        .chain(states.spite_global.next_due_us())
        .chain(states.primal_callbacks.next_source_due_time()?)
        .min();
    Ok([
        (
            SourceWorkFamily::Native,
            states.native_callbacks.next_source_due_time()?,
        ),
        (
            SourceWorkFamily::TaintOne,
            states.soul_war_callbacks.next_taint_one_due(),
        ),
        (
            SourceWorkFamily::SoulWarAttack,
            states.soul_war_callbacks.next_attack_due(),
        ),
        (
            SourceWorkFamily::Greed,
            super::soul_war_callbacks::greed::next_source_due_time(runtime, states)?,
        ),
        (
            SourceWorkFamily::Physical,
            super::monster_profile_cycle::physical_spell::next_source_due_time(runtime, states)?,
        ),
        (
            SourceWorkFamily::RottenBlood,
            states
                .rotten_blood_callbacks
                .next_source_due_time(runtime)?,
        ),
        (
            SourceWorkFamily::CreatureStep,
            states
                .spite_steps
                .creatures
                .next_due_us(runtime)
                .map_err(|_| ())?,
        ),
        (SourceWorkFamily::Detached, detached),
        (
            SourceWorkFamily::Profile,
            states.native_profile_pass.next_due(runtime)?,
        ),
    ]
    .into_iter()
    .filter_map(|(family, due)| due.map(|due| (family, due)))
    .min_by_key(|(_, due)| *due))
}

/// Original boundaries come from the existing death, native-pass and physical
/// movement owners. No scheduler retry wake supplies a new causal deadline.
pub(super) fn source_work_cutoff(
    runtime: &ChannelRuntimeV1,
    states: &actor_spell::ChannelSpellStates,
) -> Result<Option<u64>, ()> {
    let death = runtime
        .next_source_creature_death()
        .map_err(|_| ())?
        .map(|notice| notice.enqueued_at_us().ok_or(()))
        .transpose()?;
    Ok(death
        .into_iter()
        .chain(states.native_profile_pass.next_due(runtime)?)
        .chain(
            states
                .spite_steps
                .creatures
                .next_due_us(runtime)
                .map_err(|_| ())?,
        )
        .min())
}

/// One retained action payload for the canonical head's current copied event.
/// This carries no independent ordering: only the carrier's FIFO/cursor selects it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PreparedSourceDeathAction {
    begin: oteryn_gameplay_entities::carrier::native_source_death::SourceCreatureDeathSeal,
    index: usize,
    now_us: u64,
    heart: Option<heart::HeartAction>,
    soul_war: Option<SoulWarAction>,
    megalomania: Option<MegalomaniaDeath>,
    rotten: Option<rotten::RottenDeathAction>,
}
impl PreparedSourceDeathAction {
    #[allow(clippy::too_many_arguments)]
    fn capture(
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        runtime: &mut ChannelRuntimeV1,
        states: &actor_spell::ChannelSpellStates,
        attack: &attack::ChannelAttackStates,
        native: &crate::content::native_gameplay::NativeGameplayState,
        begin: &oteryn_gameplay_entities::carrier::native_source_death::SourceCreatureDeathSeal,
        now_us: u64,
    ) -> Result<Self, ()> {
        let index = runtime
            .source_creature_death_event_index(begin)
            .map_err(|_| ())?
            .ok_or(())?;
        let heart = if begin.events().get(index).ok_or(())?.name() == "HeartBossDeath" {
            Some(heart::HeartAction::capture(owner, runtime, begin)?)
        } else {
            None
        };
        let soul_war = SoulWarAction::capture(runtime, attack, begin, index, now_us)?;
        let megalomania = MegalomaniaDeath::capture(runtime, begin, &begin.events()[index])?;
        let rotten = rotten::RottenDeathAction::capture(runtime, states, native, begin, index)?;
        Ok(Self {
            begin: begin.clone(),
            index,
            now_us,
            heart,
            soul_war,
            megalomania,
            rotten,
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
    pub(super) fn source_profile_pass_can_resume(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &actor_spell::ChannelSpellStates,
        native: &crate::content::native_gameplay::NativeGameplayState,
        now: u64,
    ) -> Result<bool, ()> {
        let Some(due) = states.native_profile_pass.next_due(runtime)? else {
            return Ok(false);
        };
        if due > now {
            return Ok(false);
        }
        if let Some(notice) = runtime.next_source_creature_death().map_err(|_| ())?
            && due > notice.enqueued_at_us().ok_or(())?
        {
            return Ok(false);
        }
        Ok(next_source_work(runtime, states, native)? == Some((SourceWorkFamily::Profile, due)))
    }

    /// Drain one genuinely older Malice callback without retaining any volatile
    /// owner guard across SQL. Other timer families retain their own FIFO gate.
    pub(super) async fn drain_detached_malice_before_source_death(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
        now: u64,
    ) -> Result<bool, ()> {
        if self.drain_detached_megalomania_timer(permit, now).await? {
            return Ok(true);
        }
        let room = self.effective_rooms().gate_monster_ai().ok_or(())?;
        let native = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or(())?;
        let (notice, profile, step, timer, map_plan, names_plan, scope, generation) = {
            let runtime = self.runtime.lock().await;
            let states = self.states.spell_states.try_lock().map_err(|_| ())?;
            let notice = runtime.next_source_creature_death().map_err(|_| ())?;
            let profile = if notice.is_none() {
                states.native_profile_pass.pending()
            } else {
                None
            };
            let step = if notice.is_none() && profile.is_none() {
                states.spite_steps.creatures.front().map_err(|_| ())?
            } else {
                None
            };
            let cutoff = source_work_cutoff(&runtime, &states)?.unwrap_or(now);
            let Some(timer) = soul_war_malice::prepare_due_timer(&states, cutoff, now) else {
                return Ok(false);
            };
            let due = states
                .soul_war_callbacks
                .malice
                .as_ref()
                .ok_or(())?
                .next_due_ms()
                .and_then(|due| due.checked_mul(1_000))
                .ok_or(())?;
            if states
                .native_callbacks
                .next_source_due_time()?
                .into_iter()
                .chain(states.soul_war_callbacks.next_native_due_without_malice())
                .chain(
                    states
                        .soul_war_callbacks
                        .next_splinter_due(&runtime, native)?,
                )
                .chain(states.primal_callbacks.next_source_due_time()?)
                .chain(
                    states
                        .spite_steps
                        .creatures
                        .next_due_us(&runtime)
                        .map_err(|_| ())?,
                )
                .chain(
                    super::monster_profile_cycle::physical_spell::next_source_due_time(
                        &runtime, &states,
                    )?,
                )
                .chain(states.spite_global.next_due_us())
                .chain(
                    states
                        .rotten_blood_callbacks
                        .next_source_due_time(&runtime)?,
                )
                .any(|other| other <= due && other <= now)
                || states
                    .native_profile_pass
                    .next_due(&runtime)?
                    .is_some_and(|other| other < due && other <= now)
            {
                return Ok(false);
            }
            let objects = self.door.try_lock().map_err(|_| ())?;
            let map_plan = crate::movement::monster_map::PreparedMonsterMapRead::prepare(
                &runtime,
                room,
                &objects,
                &soul_war_malice::due_timer_cells(),
            )
            .ok_or(())?;
            let names = self.prepare_source_names(&runtime, &["Soul Cage", "Goshnar's Malice"])?;
            (
                notice,
                profile,
                step,
                timer,
                map_plan,
                names,
                runtime.binding().runtime_scope(),
                runtime.binding().scope_generation().get(),
            )
        };
        let names = self.read_source_names(permit, &names_plan).await?;
        let mut context = (
            self,
            permit,
            map_plan,
            Some(names),
            notice,
            profile,
            step,
            timer,
            room,
            native,
            scope,
            generation,
            now,
        );
        self.root.try_issue_semantic_pass().map_err(|_| ())?
            .run_with_context(&mut context, |holder, deadline, context| {
                Box::pin(async move {
                    let (owner, permit, map_plan, names, notice, profile, step, timer, room, native, scope, generation, now) = context;
                    let mut tx = crate::durability::spell_item_transaction::begin_spell_owner_transaction(holder, deadline).await?;
                    let authority = crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
                        &mut tx, permit, owner.root, owner.character, owner.holder, *scope, *generation,
                    ).await.map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                    let read = map_plan.read(&mut tx, &authority).await
                        .ok_or(crate::durability::DurabilityError::Unavailable)?;
                    {
                        let mut runtime = owner.runtime.lock().await;
                        let mut states = owner.states.spell_states.try_lock()
                            .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                        let objects = owner.door.try_lock()
                            .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                        if runtime.next_source_creature_death()
                            .map_err(|_| crate::durability::DurabilityError::Unavailable)?.as_ref() != notice.as_ref()
                            || profile.as_ref().is_some_and(|original| states.native_profile_pass.pending()
                                .as_ref().is_none_or(|current| !std::sync::Arc::ptr_eq(original,current)))
                            || step.as_ref().is_some_and(|original| !states.spite_steps.creatures
                                .retains_head(&runtime, original).unwrap_or(false)) {
                            return Err(crate::durability::DurabilityError::Unavailable);
                        }
                        let view = read.into_current_view(&runtime, room, &objects, native)
                            .ok_or(crate::durability::DurabilityError::Unavailable)?;
                        let names = owner.seal_source_names(&runtime, names.take().ok_or(crate::durability::DurabilityError::Unavailable)?)
                            .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                        soul_war_malice::publish_due_timer(&mut runtime, &mut states, room, native,
                            &view, timer, &names, *now)
                            .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                    }
                    tx.rollback().await.map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                    Ok(true)
                })
            }).await.map_err(|_| ())
    }

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
        let completed = if let Some(heart) = &action.heart {
            heart.execute(self, &action).await.is_ok()
        } else if let Some(soul_war) = &action.soul_war {
            soul_war.execute(self, permit).await.is_ok()
        } else if let Some(megalomania) = &action.megalomania {
            megalomania.execute(self, permit, &action).await.is_ok()
        } else if let Some(rotten) = &action.rotten {
            rotten.execute(self, &action).await.is_ok()
        } else {
            matches!(
                callbacks
                    .finish_source_death_action(self, permit, &action)
                    .await,
                Ok(true)
            )
        };
        if !completed {
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
            || if let Some(heart) = &action.heart {
                heart.publish_vortex(self).is_err()
            } else if let Some(soul_war) = &action.soul_war {
                !soul_war.current_for(&runtime)
            } else if let Some(megalomania) = &action.megalomania {
                !megalomania.completed(&states, &runtime, &action)
            } else if let Some(rotten) = &action.rotten {
                rotten.publish(&runtime, &mut states).is_err()
            } else {
                !matches!(
                    callbacks.publish_source_death_action(&mut runtime, &action),
                    Ok(true)
                )
            }
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
        drop(states);
        drop(runtime);
        if let Some(heart) = &action.heart {
            heart.refresh_after_release(self).await;
        } else if let Some(soul_war) = &action.soul_war {
            soul_war.refresh_after_release(self).await;
        }
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
        let native = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .ok_or(())?;
        Ok(next_source_work(runtime, states, native)?
            .is_none_or(|(_, due)| due > cutoff || due > now))
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
                let action = PreparedSourceDeathAction::capture(
                    self, runtime, states, attack, native, &begin, now,
                )?;
                if (action.heart.is_some() || action.rotten.is_some())
                    && !begin.has_reward_damage_credit()
                    && !attack.kills().has_source_drop(&begin)
                    && attack
                        .kills()
                        .source_drop_refusal(runtime, &begin)?
                        .is_none()
                {
                    // Keep the projected death and its per-player occurrences through
                    // asynchronous copied effects, before any durable side effect runs.
                    PendingSourceCorpse::request(native, &begin)?;
                    runtime
                        .pin_uncredited_source_corpse_projection(&begin)
                        .map_err(|_| ())?;
                }
                states.source_death_action = Some(action);
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
            } else if matches!(event.name(), "GreedMonsterDeath" | "SoulSphereDeath") {
                super::soul_war_callbacks::greed::consume_death(
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
                let prepared = if action.heart.is_some()
                    || action.soul_war.is_some()
                    || action.megalomania.is_some()
                    || action.rotten.is_some()
                {
                    true
                } else {
                    callbacks.prepare_source_death_action(runtime, room, native, action)?
                };
                if prepared {
                    if states.source_death_action_reservation.is_none() {
                        states.source_death_action_reservation = Some(
                            runtime
                                .reserve_source_death_action(
                                    action.begin(),
                                    action.event(),
                                    action.heart.as_ref().map_or_else(
                                        || {
                                            action
                                                .soul_war
                                                .as_ref()
                                                .map_or(&[] as &[ExactActorRef], |soul_war| {
                                                    soul_war.actors()
                                                })
                                        },
                                        |heart| heart.actors(),
                                    ),
                                )
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
        let refused = attack
            .kills()
            .source_drop_refusal(runtime, &begin)?
            .is_some();
        // A rowless corpse needs the closed uncredited physical port. Do not
        // discard its only completion capability before that pending work is
        // retained; ordinary credited entries retain it in their existing queue.
        // An original terminal reward refusal instead finishes without a MINT.
        if !attack.kills().has_source_drop(&begin)
            && !refused
            && native
                .reward_table()
                .row(begin.definition_key())
                .ok()
                .is_some_and(|row| row.corpse_item.is_some())
        {
            if states.source_death_corpse.is_some() {
                return Err(());
            }
            let request = PendingSourceCorpse::request(native, &begin)?;
            let rotten_loot = states
                .rotten_blood_callbacks
                .pending_bakragore_death_loot(&begin)?;
            if (begin.definition_key() == "oteryn:creature.bakragore") != rotten_loot.is_some()
                || rotten_loot
                    .as_ref()
                    .is_some_and(|source| !source.matches_source(&begin))
            {
                return Err(());
            }
            runtime
                .pin_uncredited_source_corpse_projection(&begin)
                .map_err(|_| ())?;
            let completed = runtime
                .finish_source_creature_death_task(&begin)
                .map_err(|_| ())?;
            let mut pending = PendingSourceCorpse::new(completed, request);
            if let Some(source) = rotten_loot {
                pending = pending.with_rotten_loot(source);
            }
            states.source_death_corpse =
                Some(std::sync::Arc::new(tokio::sync::Mutex::new(pending)));
            states
                .rotten_blood_callbacks
                .clear_bakragore_death_loot(&begin)?;
            return Ok(true);
        }
        let completed = runtime
            .finish_source_creature_death_task(&begin)
            .map_err(|_| ())?;
        attack.kills().complete_source_drop(runtime, &completed)?;
        if refused {
            runtime.unpin_corpse_projection(begin.actor());
        }
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
        native_health_names: &crate::gameplay_transport::source_names::NativeHealthNamesData,
    ) -> Result<bool, ()> {
        let notice = runtime.next_source_creature_death().map_err(|_| ())?;
        let cutoff = source_work_cutoff(runtime, states)?.unwrap_or(now);
        #[cfg(test)]
        eprintln!("[DBG-W11-DEATH] source-head cutoff={cutoff} now={now}");
        let selected = next_source_work(runtime, states, native)?
            .filter(|(_, due)| *due <= cutoff && *due <= now);
        let active = self.active_generation.ok_or(())?;
        if matches!(
            selected,
            Some((
                SourceWorkFamily::Native
                    | SourceWorkFamily::TaintOne
                    | SourceWorkFamily::SoulWarAttack
                    | SourceWorkFamily::Greed
                    | SourceWorkFamily::Physical,
                _
            ))
        ) {
            let published = active.qualify_published_world_context(room, runtime.content_pin());
            super::soul_war_callbacks::refresh_taints(
                self.root,
                runtime,
                states,
                room,
                native,
                map,
                now,
                published.as_ref(),
            )
            .await?;
        }
        match selected {
            Some((SourceWorkFamily::Native, due)) => {
                let callbacks = states.native_callbacks.clone();
                callbacks
                    .drain_due_through(
                        active,
                        runtime,
                        states,
                        attack,
                        room,
                        native,
                        map,
                        parties,
                        due,
                        now,
                        native_health_names,
                    )
                    .await?;
            }
            Some((SourceWorkFamily::TaintOne, due)) => {
                super::soul_war_callbacks::drive_taint_one_due_through(
                    runtime, states, room, native, map, due, now,
                )
                .await?;
            }
            Some((SourceWorkFamily::SoulWarAttack, due)) => {
                super::soul_war_callbacks::drive_attack_due_through(
                    runtime, states, room, native, map, due, now,
                )
                .await?;
            }
            Some((SourceWorkFamily::Greed, due)) => {
                super::soul_war_callbacks::greed::drive_due_through(
                    runtime, states, room, native, map, due, now,
                )
                .await?;
            }
            Some((SourceWorkFamily::Physical, due)) => {
                super::monster_profile_cycle::physical_spell::drain_due_through(
                    runtime,
                    states,
                    attack,
                    room,
                    native,
                    map,
                    parties,
                    due,
                    now,
                    self.imported_charms,
                )
                .await?;
            }
            Some((SourceWorkFamily::RottenBlood, due)) => {
                super::rotten_blood_callbacks::refresh_taints(self.root, runtime, states, map)
                    .await?;
                super::rotten_blood_callbacks::drive_due_before(
                    runtime,
                    states,
                    attack,
                    &self.states.server_messages,
                    room,
                    native,
                    map,
                    due,
                    now,
                )
                .await?;
            }
            // Detached source owners keep their exact head and reservation;
            // their qualified consumer must run before beginning this death.
            Some(_) => return Ok(false),
            None => {}
        }
        if !map.validate_transaction(runtime).await {
            return Err(());
        }
        // A retained native pass also has an original causal deadline when no
        // death notice exists. Yield after its selected predecessor; the same
        // pass and its Item tail remain owned until the next selected turn.
        if notice.is_none() {
            return Ok(true);
        }
        if !self.source_death_can_begin_without_timer_read(runtime, states, now)? {
            return Ok(false);
        }
        self.prepare_source_death_head(runtime, states, attack, room, native, now)
    }
}

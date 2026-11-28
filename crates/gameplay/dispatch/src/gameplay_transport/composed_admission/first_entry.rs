use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// [`Self::initialize_first_entry`], repeated from fresh reads while it is refused before
    /// the Channel-owner write: on a busy holder under `budget` (NODEBOOT-1), on stale authority
    /// for at most [`RECONCILE_ATTEMPTS`] rounds. A refused round wrote no runtime state.
    pub(in crate::gameplay_transport) async fn enter_committed_actor(
        &self,
        budget: &AdmissionRetryBudget<'_>,
        request: &FreshAdmissionCommitRequestV1,
        game_session_id: GameSessionId,
        transport: AuthenticatedTransportRefV1,
        actor: ExactActorRef,
    ) -> (
        FirstEntryOutcome,
        Option<CurrentCharacterItemFence>,
        Option<last_position::AdmissionPosition>,
    ) {
        let phase = EntryPhase::default();
        enter_with_retry(budget, &phase, async || {
            self.initialize_first_entry_traced(
                request,
                game_session_id,
                transport,
                actor,
                &phase,
                budget,
            )
            .await
        })
        .await
    }

    #[cfg(test)]
    pub(in crate::gameplay_transport) async fn initialize_first_entry(
        &self,
        request: &FreshAdmissionCommitRequestV1,
        game_session_id: GameSessionId,
        transport: AuthenticatedTransportRefV1,
        actor: ExactActorRef,
    ) -> EntryResult {
        let budget = AdmissionRetryBudget::for_root(&game_session_id, self.root);
        self.initialize_first_entry_traced(
            request,
            game_session_id,
            transport,
            actor,
            &EntryPhase::default(),
            &budget,
        )
        .await
    }

    /// Immediately before the position write, resolve independently current
    /// authority (#935): the current GameSession, the current Character guard
    /// (owner, World, eligibility, lease generation and holder) and the current
    /// assignment of this runtime. The immutable request supplies only the
    /// expected values those current reads are compared against. A positioned
    /// actor also gets its session's item fence (C2), from that current read.
    async fn initialize_first_entry_traced(
        &self,
        request: &FreshAdmissionCommitRequestV1,
        game_session_id: GameSessionId,
        transport: AuthenticatedTransportRefV1,
        actor: ExactActorRef,
        phase: &EntryPhase,
        budget: &AdmissionRetryBudget<'_>,
    ) -> (
        FirstEntryOutcome,
        Option<CurrentCharacterItemFence>,
        Option<last_position::AdmissionPosition>,
    ) {
        #[cfg(test)]
        let stage = |name: &str| {
            if std::env::var_os("OTERYN_SEAM_SPELL_MANIFEST").is_some() {
                eprintln!("SEAM_EVIDENCE spell_first_entry stage={name}");
            }
        };
        #[cfg(test)]
        stage("resolve_session");
        phase.set("resolve_session");
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let current = match store.reconcile(request.operation()).await {
            Ok(FreshReconciliation::Committed(snapshot)) => snapshot.current_session,
            Ok(_) => return (FirstEntryOutcome::RefusedStaleAuthority, None, None),
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
        phase.set("reserve_precondition");
        match self.reserve_precondition().await {
            Ok(()) => {}
            Err(AdmissionRefusal::Unavailable) => {
                return (FirstEntryOutcome::RefusedUnavailable, None, None);
            }
            Err(_) => return (FirstEntryOutcome::RefusedStaleAuthority, None, None),
        }
        // SPELL-D8 §8.2: a new runtime actor loads the durable Harmony and remaining forced
        // Serene time, read before the Channel-owner lock; a failed or corrupt load fails closed.
        let character_id =
            match domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes()) {
                Ok(character_id) => character_id,
                Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
            };
        phase.set("conditions");
        let saved_conditions = match self
            .root
            .read_character_conditions(self.character, character_id)
            .await
        {
            Ok(saved) => saved,
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
        phase.set("gourmet");
        let gourmet = match self
            .root
            .read_gourmet_conditions(self.character, character_id)
            .await
        {
            Ok(saved) => saved,
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
        phase.set("daily_streak");
        let daily_streak = match self
            .root
            .read_daily_reward_streak(self.character, character_id, self.world_id)
            .await
        {
            Ok(streak) => streak,
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
        phase.set("cast_facts");
        let (facts, character_revision) = match spell_character_facts::load_character_cast_facts(
            self.root,
            self.character,
            character_id,
        )
        .await
        {
            spell_character_facts::CastFactsLoad::Ready {
                facts,
                character_revision,
            }
            | spell_character_facts::CastFactsLoad::NoVocation {
                facts,
                character_revision,
            } => (facts, character_revision),
            spell_character_facts::CastFactsLoad::Unavailable => {
                return (FirstEntryOutcome::RefusedUnavailable, None, None);
            }
        };
        phase.set("known_spells");
        let learned = match self
            .root
            .read_character_known_spells(self.character, character_id)
            .await
        {
            Ok(learned) => learned,
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
        #[cfg(test)]
        stage("character_facts_loaded");
        phase.set("monk");
        let Some(monk) = self.load_monk_state(character_id).await else {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
        };
        phase.set("build");
        let build = match self
            .root
            .read_character_build_state(self.character, character_id)
            .await
        {
            Ok(build) => build,
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
        phase.set("stamina");
        let stamina = match self
            .root
            .read_hunting_stamina(self.character, character_id)
            .await
        {
            Ok(minutes) => minutes,
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
        phase.set("stance");
        let stance = match self
            .root
            .read_character_stance(self.character, character_id)
            .await
        {
            Ok(stance) => stance,
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
        if let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        {
            let Some(fence) = item_fence_of(current) else {
                return (FirstEntryOutcome::RefusedStaleAuthority, None, None);
            };
            #[cfg(test)]
            stage("equipment_initialize");
            phase.set("equipment");
            if let Err(_error) = self
                .root
                .initialize_admission_equipment(
                    self.character,
                    self.holder,
                    &fence,
                    native.source_digest(),
                    request.operation(),
                )
                .await
            {
                #[cfg(test)]
                stage(&format!("equipment_refused:{_error:?}"));
                return (FirstEntryOutcome::RefusedUnavailable, None, None);
            }
            if let Err(outcome) = self
                .initialize_starter_items(
                    native,
                    character_id,
                    fence,
                    &build,
                    facts.vocation,
                    phase,
                )
                .await
            {
                return (outcome, None, None);
            }
            let gameplay =
                crate::durability::character_progression::CurrentCharacterGameplayFence {
                    character_id,
                    game_session_id,
                    connection_generation: current.current_connection_generation(),
                    character_lease_generation: current.current_character_lease().generation(),
                    runtime_scope: current.current_runtime_scope(),
                    scope_ownership_generation: current.current_scope_generation(),
                    expected_character_revision: character_revision,
                };
            #[cfg(test)]
            stage("familiar_initialize");
            phase.set("familiar_group");
            if let Err(_error) = self
                .root
                .initialize_familiar_group(self.character, self.holder, gameplay)
                .await
            {
                #[cfg(test)]
                stage(&format!("familiar_refused:{_error:?}"));
                return (FirstEntryOutcome::RefusedUnavailable, None, None);
            }
        }
        // EQUIP-0 §4 / SPELL-LOCK-2: acquire the lane before final authority revalidation.
        phase.set("spell_lane");
        let Some(permit) = self.spell_lane_permit().await else {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
        };
        // Character facts, monk and stance loads are data reads, never current session authority.
        // Reject a changed root and resolve session, guard and assignment again after them.
        phase.set("revalidate_character");
        let Ok(latest_character) = self
            .root
            .read_current_character(self.character, character_id)
            .await
        else {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
        };
        if latest_character.revision != character_revision {
            return (FirstEntryOutcome::RefusedStaleAuthority, None, None);
        }
        phase.set("revalidate_session");
        let current = match store.reconcile(request.operation()).await {
            Ok(FreshReconciliation::Committed(snapshot)) => snapshot.current_session,
            Ok(_) => return (FirstEntryOutcome::RefusedStaleAuthority, None, None),
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
        if current.commit().character_id().as_bytes() != character_id.as_bytes() {
            return (FirstEntryOutcome::RefusedStaleAuthority, None, None);
        }
        phase.set("authority_guard");
        let key = AdmissionAuthorityGuardKeyV1::Character(current.commit().character_id());
        let character = match AdmissionGuardStore::from_root(self.root.clone())
            .load(&[key])
            .await
        {
            Ok(rows) => rows.into_iter().next().flatten().map(|row| row.state),
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
        // CHAR-POSITION-0 §3.3: the durable position replaces `entry_start` as the input of the
        // owner step below, so it is read now, under this admission's session fence and before
        // the Channel-owner lock: party presence and the familiar's login placement, which run
        // after that step, then read the placed cell. A failed read leaves the actor unpositioned.
        let position_fence =
            crate::durability::character_progression::CurrentCharacterGameplayFence {
                character_id,
                game_session_id,
                connection_generation: current.current_connection_generation(),
                character_lease_generation: current.current_character_lease().generation(),
                runtime_scope: current.current_runtime_scope(),
                scope_ownership_generation: current.current_scope_generation(),
                expected_character_revision: character_revision,
            };
        phase.set("death_items");
        if self
            .resume_pending_player_death_items(&permit, position_fence)
            .await
            .is_err()
        {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
        }
        phase.set("home_town");
        if self
            .bundle_map()
            .and_then(|map| map.overlay.base().subareas.default_town)
            .is_some()
            && self.home_town(game_session_id, None).await.is_none()
        {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
        }
        phase.set("tutorial_position");
        let Ok(decided) = self.read_tutorial_admission_position(position_fence).await else {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
        };
        // P20: Crystal's login check (`login.lua:8-15`) runs before the owner places the actor.
        let account = super::super::store::uuid_bytes(&request.binding().account_id);
        phase.set("premium_temple");
        let temple = match account {
            Some(account) => self.premium_ended_temple(account, game_session_id).await,
            None => None,
        };
        let relocated = temple.map(|temple| decided.relocated(temple, self.world_id));
        let placement = relocated.as_ref().unwrap_or(&decided);
        #[cfg(test)]
        stage("durable_position_read");
        phase.set("reserve_precondition");
        match self.reserve_precondition().await {
            Ok(()) => {}
            Err(AdmissionRefusal::Unavailable) => {
                return (FirstEntryOutcome::RefusedUnavailable, None, None);
            }
            Err(_) => return (FirstEntryOutcome::RefusedStaleAuthority, None, None),
        }
        // Hold the lane through installation; the equipment reader runs before runtime locks.
        phase.set("equipment_effects");
        let Ok(equipment_effects) = self.read_admission_effects(game_session_id).await else {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
        };
        // One Channel-owner lock covers the binding comparison and the write.
        phase.set("channel_owner");
        let mut runtime = self.runtime.lock().await;
        let expected = FirstEntryExpectation {
            game_session_id,
            transport,
            world_id: self.world_id,
            channel_id: self.channel_id,
            account_id: &request.binding().account_id,
            scope_generation: runtime.binding().scope_generation(),
        };
        if !first_entry_authority_is_current(&expected, current, character.as_ref()) {
            return (FirstEntryOutcome::RefusedStaleAuthority, None, None);
        }
        let Some(prepared_conditions) = super::super::condition_persistence::prepare(
            saved_conditions.as_ref(),
            current.current_character_lease().generation(),
            runtime.content_pin().server_artifact_digest(),
            self.owner_now().get(),
        ) else {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
        };
        // A2 (D295 item 4): bind the admitted session's lease to its slot in this owner step,
        // from the read that just proved it current. No damage write is admitted before.
        if runtime
            .bind_attacker_lease(actor, game_session_id, current.current_character_lease())
            .is_err()
        {
            return (FirstEntryOutcome::RefusedByChannel, None, None);
        }
        // CHAR-POSITION-0 §3.3: initialized and placed at the durable position in this one
        // owner step, before the lock is released and any owner reads the actor's position.
        #[cfg(test)]
        let _ = FAMILIAR_LOGIN_CHECKOUT.try_with(|probe| {
            probe.placement.set(probe.placement.get() + 1);
        });
        let outcome = match self
            .initialize_admitted_position(&mut runtime, actor, game_session_id, placement)
            .await
        {
            Ok(FirstEntryPosition::Initialized(_)) => FirstEntryOutcome::Positioned,
            Ok(FirstEntryPosition::Reconciled(_)) => FirstEntryOutcome::Reconciled,
            Err(_) => return (FirstEntryOutcome::RefusedByChannel, None, None),
        };
        #[cfg(test)]
        stage("position_initialized");
        // C2: the item fence of the admitted session, from the same current session read that
        // just proved it is this admission's own.
        let item_fence = item_fence_of(current);
        // Spell cast §4: the vitals and cooldowns are created in this same owner step, before the
        // actor's first command, and only from Character-owned facts; a monk's Serene is
        // evaluated here too (§8.2). CHAR-VITALS-1: a newly placed actor whose last-position row
        // saved its health and mana takes them, capped at its maxima, in this same step.
        let saved_vitals = (outcome == FirstEntryOutcome::Positioned)
            .then(|| last_position::admitted_vitals(placement, self.world_id))
            .flatten();
        let admitted_rest = (outcome == FirstEntryOutcome::Positioned)
            .then(|| bed_regen::admitted_rest(placement, self.world_id))
            .flatten();
        let gourmet_created = !self
            .states
            .spell_states
            .lock()
            .await
            .actors
            .iter()
            .any(|(present, session, _)| *present == actor && *session == game_session_id);
        if self
            .states
            .spell_states
            .lock()
            .await
            .initialize_admitted(
                &runtime,
                actor,
                game_session_id,
                facts,
                monk,
                self.owner_now(),
                saved_vitals,
                admitted_rest,
            )
            .is_none()
        {
            return (FirstEntryOutcome::RefusedByChannel, None, None);
        }
        if let Some(state) =
            self.states
                .spell_states
                .lock()
                .await
                .get_mut(&runtime, actor, game_session_id)
        {
            state.install_learned_spells(learned);
        }
        if gourmet_created {
            let mut states = self.states.spell_states.lock().await;
            let Some(state) = states.get_mut(&runtime, actor, game_session_id) else {
                return (FirstEntryOutcome::RefusedByChannel, None, None);
            };
            if super::super::condition_persistence::restore(
                state,
                prepared_conditions.as_ref(),
                gourmet,
                self.owner_now().get(),
                admitted_rest,
            )
            .is_none()
            {
                return (FirstEntryOutcome::RefusedByChannel, None, None);
            }
            super::super::item_use::gourmet::remember(
                &mut states,
                actor,
                game_session_id,
                gourmet,
                self.owner_now().get(),
            );
        }
        if self
            .install_admission_suppressions(&runtime, actor, game_session_id, equipment_effects)
            .await
            .is_none()
        {
            return (FirstEntryOutcome::RefusedByChannel, None, None);
        }
        if !self.states.spell_states.lock().await.load_owned_stance(
            &runtime,
            actor,
            game_session_id,
            &stance,
            self.spells,
        ) {
            return (FirstEntryOutcome::RefusedByChannel, None, None);
        }
        if !self.states.spell_states.lock().await.load_owned_build(
            &runtime,
            actor,
            game_session_id,
            &build,
        ) {
            return (FirstEntryOutcome::RefusedByChannel, None, None);
        }
        if let Some(formula) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .and_then(|native| native.training_formula())
        {
            let mut states = self.states.spell_states.lock().await;
            if states
                .get_mut(&runtime, actor, game_session_id)
                .is_none_or(|state| state.enable_owned_training(&build, formula).is_err())
            {
                return (FirstEntryOutcome::RefusedByChannel, None, None);
            }
        }
        if !self
            .restore_pve_logout_block(&runtime, actor, game_session_id, saved_conditions.as_ref())
            .await
        {
            return (FirstEntryOutcome::RefusedByChannel, None, None);
        }
        self.states.spell_states.lock().await.load_stamina_at(
            &runtime,
            actor,
            game_session_id,
            stamina,
            self.owner_now().get(),
        );
        self.states
            .resting_areas
            .install(game_session_id, actor, daily_streak);
        let relocation = if temple.is_some() {
            let Ok(position) = runtime.read_actor_position(actor) else {
                return (FirstEntryOutcome::RefusedByChannel, None, None);
            };
            let mut placed = self.placed_record(&runtime, position.position());
            placed.record.vitals = last_position::admitted_vitals(placement, self.world_id)
                .and_then(|(health, mana)| {
                    crate::durability::character_last_position::LastVitals::new(health, mana)
                });
            placed.record.rest = bed_regen::admitted_rest(placement, self.world_id);
            Some(placed)
        } else {
            None
        };
        drop(runtime);
        phase.set("premium_relocation");
        if let Some(placed) = relocation
            && !self
                .commit_premium_ended_relocation(game_session_id, actor, &decided, placed)
                .await
        {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
        }
        let decided = relocated.unwrap_or(decided);
        drop(permit);
        #[cfg(test)]
        stage("runtime_facts_initialized");
        phase.set("party_presence");
        if self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .is_some()
        {
            let _ = self
                .refresh_source_party_presence(actor, game_session_id)
                .await;
        }
        // ARCH-PROGRESSION-SOURCE-0 §1.5: the committed, positioned session now has its
        // gameplay fence. Familiar Login reads the durable progression this step owns.
        phase.set("progression");
        if budget
            .retry_unavailable(async || self.bind_positioned_progression(game_session_id).await)
            .await
            .is_err()
        {
            return (
                FirstEntryOutcome::RefusedUnavailable,
                item_fence,
                Some(decided),
            );
        }
        // A Character without a vocation has no familiar; its absent build row is valid.
        if facts.vocation != crate::spell::Vocation::None
            && let Some(native) = self
                .active_generation
                .and_then(|active| active.native_gameplay())
            && let Some(config) = native.familiar_config()
        {
            phase.set("familiar_login");
            // The test seeds its durable Familiar checkpoint with the positioned fence.
            #[cfg(test)]
            if let Ok(Some(barrier)) = FAMILIAR_LOGIN_CHECKOUT.try_with(|probe| {
                (!probe.seed_started.replace(true)).then(|| probe.seed_barrier.clone())
            }) {
                barrier.0.notify_one();
                barrier.1.notified().await;
            }
            let login_budget =
                AdmissionRetryBudget::for_foreground_root(&game_session_id, self.root)
                    .unwrap_or_else(|_| {
                        AdmissionRetryBudget::for_root(&game_session_id, self.root)
                    });
            let access = self
                .refresh_and_apply_spell_access_with_budget(
                    actor,
                    game_session_id,
                    Some(&login_budget),
                )
                .await;
            // #2199: a familiar Login that did not complete restored neither the stored familiar
            // nor its cooldowns, so the entry fails closed like the other post-position owners.
            let (login, login_attempts) = Box::pin(familiar_login::run(
                async || {
                    Box::pin(self.initialize_familiar_login(
                        &login_budget,
                        actor,
                        game_session_id,
                        request,
                        native,
                        &access,
                        config,
                    ))
                    .await
                    .map(|_| ())
                },
                |error| error.retryable(),
                async || {
                    login_budget
                        .retries_unavailable(AdmissionRefusal::Unavailable)
                        .await
                        && tokio::time::Instant::now() < login_budget.deadline
                },
                |error, attempt| error.report(attempt),
            ))
            .await;
            phase.set_familiar_login_attempts(login_attempts);
            if let Err(error) = login {
                // Retain the placement marker so the outer first-entry retry cannot
                // replay placement and post-entry writes stay gated on unavailable storage.
                if error.unavailable() {
                    return (FirstEntryOutcome::RefusedUnavailable, None, Some(decided));
                }
                return (familiar_login_first_entry(outcome, false), None, None);
            }
        }
        (outcome, item_fence, Some(decided))
    }
}

mod familiar_login;
mod retry;
mod starter;

#[cfg(test)]
use retry::EntryResult;
use retry::{EntryPhase, enter_with_retry};

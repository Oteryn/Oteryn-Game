use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Immediately before the position write, resolve independently current
    /// authority (#935): the current GameSession, the current Character guard
    /// (owner, World, eligibility, lease generation and holder) and the current
    /// assignment of this runtime. The immutable request supplies only the
    /// expected values those current reads are compared against. A positioned
    /// actor also gets its session's item fence (C2), from that current read.
    pub(in crate::gameplay_transport) async fn initialize_first_entry(
        &self,
        request: &FreshAdmissionCommitRequestV1,
        game_session_id: GameSessionId,
        transport: AuthenticatedTransportRefV1,
        actor: ExactActorRef,
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
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let current = match store.reconcile(request.operation()).await {
            Ok(FreshReconciliation::Committed(snapshot)) => snapshot.current_session,
            Ok(_) => return (FirstEntryOutcome::RefusedStaleAuthority, None, None),
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
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
        #[cfg(test)]
        stage("character_facts_loaded");
        let Some(monk) = self.load_monk_state(character_id).await else {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
        };
        let build = match self
            .root
            .read_character_build_state(self.character, character_id)
            .await
        {
            Ok(build) => build,
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
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
            // STARTER-1 (CP D1438): the starter grant of the Character's first entry, once per
            // Character and template; a replay writes nothing.
            if self
                .root
                .grant_starter_items(
                    self.character,
                    self.holder,
                    &fence,
                    &item_ref_admission::NativeItemWeights(native),
                    &crate::durability::starter_grant::StarterTemplate::main(),
                )
                .await
                .is_err()
            {
                return (FirstEntryOutcome::RefusedUnavailable, None, None);
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
        let Some(permit) = self.spell_lane_permit().await else {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
        };
        // Character facts, monk and stance loads are data reads, never current session authority.
        // Reject a changed root and resolve session, guard and assignment again after them.
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
        let current = match store.reconcile(request.operation()).await {
            Ok(FreshReconciliation::Committed(snapshot)) => snapshot.current_session,
            Ok(_) => return (FirstEntryOutcome::RefusedStaleAuthority, None, None),
            Err(_) => return (FirstEntryOutcome::RefusedUnavailable, None, None),
        };
        if current.commit().character_id().as_bytes() != character_id.as_bytes() {
            return (FirstEntryOutcome::RefusedStaleAuthority, None, None);
        }
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
        let Ok(decided) = self.read_tutorial_admission_position(position_fence).await else {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
        };
        #[cfg(test)]
        stage("durable_position_read");
        match self.reserve_precondition().await {
            Ok(()) => {}
            Err(AdmissionRefusal::Unavailable) => {
                return (FirstEntryOutcome::RefusedUnavailable, None, None);
            }
            Err(_) => return (FirstEntryOutcome::RefusedStaleAuthority, None, None),
        }
        // Hold the lane through installation; the equipment reader runs before runtime locks.
        let Ok(equipment_effects) = self.read_admission_effects(game_session_id).await else {
            return (FirstEntryOutcome::RefusedUnavailable, None, None);
        };
        // One Channel-owner lock covers the binding comparison and the write.
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
        let outcome = match self
            .initialize_admitted_position(&mut runtime, actor, game_session_id, &decided)
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
            .then(|| last_position::admitted_vitals(&decided, self.world_id))
            .flatten();
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
                (outcome == FirstEntryOutcome::Positioned)
                    .then(|| bed_regen::admitted_rest(&decided, self.world_id))
                    .flatten(),
            )
            .is_none()
        {
            return (FirstEntryOutcome::RefusedByChannel, None, None);
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
        drop(runtime);
        drop(permit);
        #[cfg(test)]
        stage("runtime_facts_initialized");
        if self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .is_some()
        {
            let _ = self
                .refresh_source_party_presence(actor, game_session_id)
                .await;
        }
        if let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            && let Some(config) = native.familiar_config()
        {
            let access = self
                .refresh_and_apply_spell_access(actor, game_session_id)
                .await;
            // #2199: a familiar Login that did not complete restored neither the stored familiar
            // nor its cooldowns, so the entry fails closed like the other post-position owners.
            let initialized = self
                .initialize_familiar_login(actor, game_session_id, request, native, &access, config)
                .await
                .is_ok();
            let outcome = familiar_login_first_entry(outcome, initialized);
            if outcome == FirstEntryOutcome::RefusedByChannel {
                return (outcome, None, None);
            }
        }
        (outcome, item_fence, Some(decided))
    }
}

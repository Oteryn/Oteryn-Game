impl<'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    async fn enter_native_activity_inner(
        &self,
        mut request: ActivityEnterRequest,
        boss_room: bool,
        soulpit: Option<crate::durability::soulpit::SoulpitEntryRequest>,
        boss_pad_wire: Option<[u8; 32]>,
    ) -> Result<NativeActivityOutcome, ()> {
        let refuse = |reason| Ok(NativeActivityOutcome::Refused(reason));
        if request.command_id == 0
            || request.participants.is_empty()
            || request.participants.len() > 15
        {
            return refuse(ActivityRefusal::InvalidInput);
        }
        request
            .participants
            .sort_by_key(|participant| *participant.source.character_id.as_bytes());
        if request
            .participants
            .windows(2)
            .any(|pair| pair[0].source.character_id == pair[1].source.character_id)
            || request
                .participants
                .iter()
                .enumerate()
                .any(|(index, participant)| {
                    request.participants[..index].iter().any(|earlier| {
                        earlier.source.game_session_id == participant.source.game_session_id
                            || earlier.destination_game_session_id
                                == participant.destination_game_session_id
                    })
                })
        {
            return refuse(ActivityRefusal::InvalidInput);
        }
        let Some(initiator) = request.participants.iter().find(|participant| {
            participant.source.character_id.as_bytes() == request.initiating_character_id.as_bytes()
        }) else {
            return refuse(ActivityRefusal::InvalidInput);
        };
        let key = ActivityAttemptKey {
            session: initiator.source.game_session_id,
            connection_generation: initiator.source.connection_generation.get(),
            command: request.command_id,
        };
        let _transition = self.activity_runtimes.transitions.lock().await;
        let existing = self
            .activity_runtimes
            .attempts
            .lock()
            .map_err(|_| ())?
            .get(&key)
            .cloned();
        if let Some(pending) = existing {
            let attempt = pending.lock().await;
            if attempt.request.activity_key != request.activity_key
                || attempt.boss_room != boss_room
                || attempt.soulpit != soulpit
                || attempt.boss_pad_wire != boss_pad_wire
                || attempt
                    .soulpit
                    .as_ref()
                    .map(|core| (&core.funding, &core.intent))
                    != soulpit.as_ref().map(|core| (&core.funding, &core.intent))
            {
                return refuse(ActivityRefusal::ReplayConflict);
            }
            drop(attempt);
            return self.finish_native_activity(pending).await;
        }
        let first = request.participants[0].source;
        if !matches!(
            (first.runtime_scope, request.destination_scope),
            (
                RuntimeScopeRefV1::Channel { .. },
                RuntimeScopeRefV1::Instance { .. }
            )
        ) || request.participants.iter().any(|participant| {
            participant.source.runtime_scope != first.runtime_scope
                || participant.source.scope_ownership_generation != first.scope_ownership_generation
        }) {
            return refuse(ActivityRefusal::InvalidInput);
        }
        let Some(source) = self
            .activity_runtimes
            .get(first.runtime_scope, first.scope_ownership_generation.get())
        else {
            return refuse(ActivityRefusal::AuthorityRejected);
        };
        let Some(destination) = self.activity_runtimes.get(
            request.destination_scope,
            request.destination_generation.get(),
        ) else {
            return refuse(ActivityRefusal::NoCapacity);
        };
        if destination.assignment_source_revision != request.destination_assignment_source_revision
            || destination
                .template
                .as_ref()
                .is_none_or(|template| template.revision != request.template_revision)
        {
            return refuse(ActivityRefusal::ContentChanged);
        }
        let origin_pin = {
            let runtime = source.runtime.lock().await;
            ActivityContentPin {
                coordinate_frame_digest: runtime.content_pin().frame_binding_digest(),
                map_revision_digest: runtime.map_revision_digest(),
                server_artifact_digest: runtime.content_pin().server_artifact_digest(),
            }
        };
        let boss_definition = if boss_room {
            let owner = ActivityHandoffOwner::new(self.root.clone());
            let Some(definition) = owner
                .read_boss_room_with_pin(
                    self.character,
                    self.holder,
                    initiator.source,
                    &request.activity_key,
                    origin_pin,
                )
                .await
                .map_err(|_| ())?
            else {
                return refuse(ActivityRefusal::ContentUnavailable);
            };
            if destination.template.as_ref().is_none_or(|template| {
                template.key != definition.room_template_key
                    || template.revision != definition.template_revision
            }) {
                return refuse(ActivityRefusal::ContentChanged);
            }
            Some(definition)
        } else {
            None
        };
        let mut sources = Vec::new();
        sources
            .try_reserve_exact(request.participants.len())
            .map_err(|_| ())?;
        let mut members = Vec::new();
        members
            .try_reserve_exact(request.participants.len())
            .map_err(|_| ())?;
        // Reserve the registry entry before taking any owned state out of a runtime.
        self.activity_runtimes
            .attempts
            .lock()
            .map_err(|_| ())?
            .try_reserve(1)
            .map_err(|_| ())?;
        #[cfg(test)]
        if soulpit.is_some() {
            eprintln!("SRX3_SOULPIT HANDOFF_STAGE prepared_destination");
        }
        let source_view = self.runtime_view(&source);
        let destination_view = self.runtime_view(&destination);
        // A shared lethal may be queued under another contributor's principal.
        // Drain every dependency before extracting live state or holding revision
        // slots; the frozen check below closes a capture between this and prepare.
        let mut principals = std::collections::BTreeSet::new();
        for participant in &request.participants {
            principals.insert(participant.source.game_session_id);
            principals.extend(
                source
                    .states
                    .attack
                    .lock()
                    .await
                    .kills()
                    .proficiency_principals(participant.source.game_session_id),
            );
        }
        for principal in principals {
            if source_view.drain_kill_rewards(principal).await == DrainOutcome::Deferred {
                return Err(());
            }
        }
        // Shared-XP settlement acquires its lane before recipient revisions.
        // Both scope lanes must precede every participant slot here as well.
        let source_first = ScopeKey::new(source.scope, source.generation)
            < ScopeKey::new(destination.scope, destination.generation);
        let (_first_lane, _second_lane) = if source_first {
            (
                source.lane.acquire().await.map_err(|_| ())?,
                destination.lane.acquire().await.map_err(|_| ())?,
            )
        } else {
            (
                destination.lane.acquire().await.map_err(|_| ())?,
                source.lane.acquire().await.map_err(|_| ())?,
            )
        };
        for participant in &request.participants {
            let session = participant.source.game_session_id;
            let Some(admitted) = self.activity_runtimes.connection(session) else {
                return refuse(ActivityRefusal::AuthorityRejected);
            };
            let Some(actor) = admitted
                .runtime_actor
                .filter(|_| admitted.item_fence == Some(participant.source))
            else {
                return refuse(ActivityRefusal::AuthorityRejected);
            };
            if !source_view.settle_activity_equipment(actor, session).await {
                return Err(());
            }
        }
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(request.participants.len())
            .map_err(|_| ())?;
        for participant in &request.participants {
            slots.push(
                self.revision_sequencer
                    .acquire(participant.source.character_id)
                    .await,
            );
        }
        let (mut from, mut to) = if source_first {
            let from = source.runtime.lock().await;
            let to = destination.runtime.lock().await;
            (from, to)
        } else {
            let to = destination.runtime.lock().await;
            let from = source.runtime.lock().await;
            (from, to)
        };
        from.owner_fence().map_err(|_| ())?;
        to.owner_fence().map_err(|_| ())?;
        let actual_pin = ActivityContentPin {
            coordinate_frame_digest: to.content_pin().frame_binding_digest(),
            map_revision_digest: to.map_revision_digest(),
            server_artifact_digest: to.content_pin().server_artifact_digest(),
        };
        if request.destination_content != actual_pin {
            return refuse(ActivityRefusal::ContentChanged);
        }
        if boss_definition
            .as_ref()
            .is_some_and(|definition| !boss_room_native_ready(&to, definition))
        {
            return refuse(ActivityRefusal::ContentUnavailable);
        }
        let doors = destination.door.lock().await;
        let mut spells = source.states.spell_states.lock().await;
        let mut destination_spells = destination.states.spell_states.lock().await;
        for (ordinal, participant) in request.participants.iter_mut().enumerate() {
            let Some(admitted) = self
                .activity_runtimes
                .connection(participant.source.game_session_id)
            else {
                return refuse(ActivityRefusal::AuthorityRejected);
            };
            let (Some(actor), Some(controller)) = (admitted.runtime_actor, admitted.controller)
            else {
                return refuse(ActivityRefusal::AuthorityRejected);
            };
            if admitted.item_fence != Some(participant.source)
                || participant.selected_capabilities.as_slice()
                    != admitted.continuity.selected_capabilities.as_slice()
                || admitted.continuity.connection_generation
                    != participant.source.connection_generation.get()
                || participant
                    .destination_transport
                    .is_none_or(|next| next == controller.transport)
                || sources.iter().any(|old: &AdmittedSession| {
                    old.item_fence
                        .is_some_and(|fence| fence.character_id == participant.source.character_id)
                })
                || source
                    .states
                    .player_trades
                    .transitioning(admitted.game_session_id)
                || source.states.newhaven_travel.held(admitted.game_session_id)
                || source
                    .states
                    .dawnport_departures
                    .held(admitted.game_session_id)
                || source
                    .states
                    .dawnport_trials
                    .lock()
                    .map_err(|_| ())?
                    .contains(&admitted.game_session_id)
            {
                return refuse(ActivityRefusal::AuthorityRejected);
            }
            let lease = from
                .bound_attacker_lease(actor, admitted.game_session_id)
                .map_err(|_| ())?;
            if lease.character_id().as_bytes() != participant.source.character_id.as_bytes()
                || lease.generation() != participant.source.character_lease_generation
                || from
                    .player_control_facts(actor, admitted.game_session_id)
                    .map_err(|_| ())?
                    .control_loss
                    .is_some()
            {
                return refuse(ActivityRefusal::AuthorityRejected);
            }
            let current = from.read_actor_position(actor).map_err(|_| ())?.position();
            let mut saved = source_view.placed_record(&from, current);
            if saved.record.cell != participant.source_position.cell
                || saved.record.coordinate_frame_digest
                    != participant.source_position.coordinate_frame_digest
                || saved.record.map_revision_digest
                    != participant.source_position.map_revision_digest
                || saved.record.server_artifact_digest
                    != participant.source_position.server_artifact_digest
                || participant.exit_position.world_id != source.scope.world_id()
                || participant.exit_position.coordinate_frame_digest
                    != saved.record.coordinate_frame_digest
                || participant.exit_position.map_revision_digest != saved.record.map_revision_digest
                || participant.exit_position.server_artifact_digest
                    != saved.record.server_artifact_digest
            {
                return refuse(ActivityRefusal::ContentChanged);
            }
            let Some((_, _, state)) = spells
                .actors
                .iter()
                .find(|(held, session, _)| *held == actor && *session == admitted.game_session_id)
            else {
                return refuse(ActivityRefusal::AuthorityRejected);
            };
            // Handoffs require an explicit current snapshot even at full pools.
            // saved_vitals() uses CHAR-VITALS-1's compact NULL-at-maxima encoding,
            // which is valid for a future login but not an activity receipt.
            let vitals = state.vitals();
            saved.record.vitals = LastVitals::new(vitals.health, vitals.mana);
            saved.record.rest = Some(state.bed_rest_snapshot(self.owner_now().get()));
            participant.source_position = saved.record;
            participant.source_position_write_sequence = saved.write_sequence;
            participant.exit_position.vitals = saved.record.vitals;
            participant.exit_position.rest = saved.record.rest;
            let cell = MovementLocalPosition {
                x: participant.destination_cell.x,
                y: participant.destination_cell.y,
                floor: participant.destination_cell.floor,
            };
            if !super::respawn_cell_admissible(
                destination_view.movement_cells().index(),
                destination_view.movement_cells().scope(),
                destination.scope.world_id(),
                actual_pin.server_artifact_digest,
                doors.blocking_cells(),
                cell,
            ) {
                return refuse(ActivityRefusal::Position);
            }
            let next_lease = CharacterLease::new(
                lease.character_id(),
                lease.generation().checked_add(1).ok_or(())?,
            )
            .map_err(|_| ())?;
            members.push(ActivityTransferMember {
                source_actor: actor,
                source_session: admitted.game_session_id,
                source_lease: lease,
                destination_session: participant.destination_game_session_id,
                destination_lease: next_lease,
                destination_cell: cell,
            });
            sources.push(admitted);
            debug_assert_eq!(sources.len(), ordinal + 1);
        }
        // Native membership and both owner lanes are frozen before source
        // onUseExtra. Reconstruct authored pad order inside the checked reader;
        // Character sorting above remains solely the durable lock order.
        let boss_pad_prefix =
            if let (Some(wire), Some(definition)) = (boss_pad_wire, boss_definition.as_ref()) {
                match source_view
                    .qualify_source_pad_prefix(&from, &to, definition, &request, wire)
                    .await?
                {
                    Ok(Some(prefix)) => {
                        if let Some(reason) = source_view
                            .apply_source_pad_prefix(
                                &mut from,
                                &mut spells,
                                &to,
                                &mut destination_spells,
                                &prefix,
                            )
                            .await?
                        {
                            return refuse(reason);
                        }
                        Some(prefix)
                    }
                    Ok(None) => None,
                    Err(reason) => return refuse(reason),
                }
            } else {
                None
            };
        // Acquire the leaf registry guard before extracting owned state; no
        // fallible registry acquisition or await remains before retention.
        let (mut attack, mut destination_attack) = if source_first {
            let attack = source.states.attack.lock().await;
            let destination_attack = destination.states.attack.lock().await;
            (attack, destination_attack)
        } else {
            let destination_attack = destination.states.attack.lock().await;
            let attack = source.states.attack.lock().await;
            (attack, destination_attack)
        };
        let pending = {
            let mut attempts = self.activity_runtimes.attempts.lock().map_err(|_| ())?;
            attempts.try_reserve(1).map_err(|_| ())?;
            let mut kill_marks = seal_activity_kills(attack.kills(), &members)?;
            let mut held = match spells.prepare_activity_states(&mut destination_spells, &members) {
                Ok(value) => value,
                Err(()) => {
                    finish_activity_kills(attack.kills(), &mut kill_marks, ReleaseEnd::Retryable);
                    return refuse(ActivityRefusal::AuthorityRejected);
                }
            };
            let mut held_attacks =
                match attack.prepare_activity_attacks(&from, &mut destination_attack, &members) {
                    Ok(value) => value,
                    Err(()) => {
                        spells.abort_activity_states(&mut held);
                        finish_activity_kills(
                            attack.kills(),
                            &mut kill_marks,
                            ReleaseEnd::Retryable,
                        );
                        return refuse(ActivityRefusal::AuthorityRejected);
                    }
                };
            let prepared = if boss_room {
                from.prepare_activity_group_entry(&mut to, &members)
            } else {
                from.prepare_activity_transfer(&mut to, &members)
            };
            let physical = match prepared {
                Ok(value) => value,
                Err(_) => {
                    spells.abort_activity_states(&mut held);
                    attack.abort_activity_attacks(&mut held_attacks);
                    finish_activity_kills(attack.kills(), &mut kill_marks, ReleaseEnd::Retryable);
                    return refuse(ActivityRefusal::NoCapacity);
                }
            };
            let pending = Arc::new(tokio::sync::Mutex::new(NativeActivityAttempt {
                source: source.clone(),
                destination: destination.clone(),
                request,
                boss_room,
                soulpit,
                boss_pad_wire,
                boss_pad_prefix,
                sources,
                physical,
                spells: held,
                attacks: held_attacks,
                kill_marks,
                receipt: None,
                moved: None,
                entry_effect_starts: None,
                states_installed: false,
                outcome: None,
            }));
            // No await occurs between extraction and retention. A cancelled durable
            // future only releases the attempt mutex; its unique tokens remain owned.
            attempts.insert(key, Arc::clone(&pending));
            pending
        };
        drop(destination_attack);
        drop(attack);
        drop(destination_spells);
        drop(spells);
        drop(doors);
        drop(to);
        drop(from);
        drop(slots);
        drop(_second_lane);
        drop(_first_lane);
        #[cfg(test)]
        if pending.lock().await.soulpit.is_some() {
            eprintln!("SRX3_SOULPIT HANDOFF_STAGE retained_attempt");
        }
        self.finish_native_activity(pending).await
    }
}

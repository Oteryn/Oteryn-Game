impl<'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    async fn prepare_native_activity_return(
        &self,
        admitted: AdmittedSession,
        command: Option<u64>,
        reason: ActivityReturnReason,
        source_exit: Option<super::source_boss_exit::SourceBossExitProof>,
    ) -> Result<NativeActivityOutcome, ()> {
        let refuse = |reason| Ok(NativeActivityOutcome::Refused(reason));
        if matches!(reason, ActivityReturnReason::OwnerLost) {
            return refuse(ActivityRefusal::InvalidInput);
        }
        let Some(fence) = admitted.item_fence else {
            return refuse(ActivityRefusal::AuthorityRejected);
        };
        if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Instance { .. })
            || fence.game_session_id != admitted.game_session_id
            || fence.connection_generation.get() != admitted.continuity.connection_generation
        {
            return refuse(ActivityRefusal::AuthorityRejected);
        }
        let key = ActivityReturnAttemptKey {
            session: admitted.game_session_id,
            connection_generation: admitted.continuity.connection_generation,
            command,
            server_reason: if command.is_some() {
                0
            } else {
                match reason {
                    ActivityReturnReason::Exit if source_exit.is_some() => 1,
                    ActivityReturnReason::Timeout => 2,
                    ActivityReturnReason::Defeated => 3,
                    ActivityReturnReason::Death { .. } => 5,
                    ActivityReturnReason::Logout => 6,
                    ActivityReturnReason::IdleLogout => 7,
                    ActivityReturnReason::Abandoned { .. } => 8,
                    ActivityReturnReason::CapabilityMismatch { .. } => 9,
                    ActivityReturnReason::StaffKick { .. } => 10,
                    ActivityReturnReason::StaffKickLost { .. } => 11,
                    ActivityReturnReason::SoulpitTerminal => 52,
                    _ => return refuse(ActivityRefusal::InvalidInput),
                }
            },
            native_step_occurrence: source_exit.as_ref().map(|proof| proof.movement().binding()),
            death_occurrence: match reason {
                ActivityReturnReason::Death { occurrence } => Some(occurrence),
                _ => None,
            },
        };
        let _transition = self.activity_runtimes.transitions.lock().await;
        // A cancelled socket may still own an undecided recovery COMMIT.
        // Its admission-serialized reconcile must finish before moving state.
        if self.pending_activity_recovery(admitted.game_session_id) {
            return Err(());
        }
        let existing = self
            .activity_runtimes
            .returns
            .lock()
            .map_err(|_| ())?
            .get(&key)
            .cloned();
        if let Some(pending) = existing {
            let held = pending.lock().await;
            if held.trigger_reason != reason
                || held.source_exit != source_exit
                || held.admitted.controller != admitted.controller
                || held.admitted.item_fence != admitted.item_fence
            {
                return refuse(ActivityRefusal::ReplayConflict);
            }
            let retry_server_refusal = command.is_none()
                && held.receipt.is_none()
                && matches!(held.outcome, Some(NativeActivityOutcome::Refused(_)));
            drop(held);
            if retry_server_refusal {
                let mut entries = self.activity_runtimes.returns.lock().map_err(|_| ())?;
                if entries
                    .get(&key)
                    .is_some_and(|current| Arc::ptr_eq(current, &pending))
                {
                    entries.remove(&key);
                }
            } else {
                return self.finish_native_activity_return(pending).await;
            }
        }
        let Some(source) = self
            .activity_runtimes
            .get(fence.runtime_scope, fence.scope_ownership_generation.get())
        else {
            return refuse(ActivityRefusal::AuthorityRejected);
        };
        let (current, now) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(admitted.game_session_id)
            .await
            .map_err(|_| ())?;
        if super::item_fence_of(current) != Some(fence) {
            return refuse(ActivityRefusal::AuthorityRejected);
        }
        let online = current.session_state() == GameSessionState::Active;
        if online {
            if admitted
                .controller
                .is_none_or(|controller| Some(controller.transport) != current.current_transport())
            {
                return refuse(ActivityRefusal::AuthorityRejected);
            }
        } else if current.session_state() != GameSessionState::Reconnectable
            || command.is_some()
            || current.current_transport().is_some()
            || current.current_control_loss_epoch().is_none()
            || current.current_original_grace_deadline().is_none()
        {
            return refuse(ActivityRefusal::AuthorityRejected);
        }
        if reason == ActivityReturnReason::IdleLogout {
            use super::connection::FreshAdmissionAuthority;
            let idle_now = self.gameplay_idle_now();
            if command.is_some()
                || !online
                || admitted.continuity.gameplay_idle.elapsed(idle_now)
                    < std::time::Duration::from_millis(self.gameplay_idle_policy().removal_after_ms)
            {
                return refuse(ActivityRefusal::NotDue);
            }
        }
        match reason {
            ActivityReturnReason::Abandoned { transport }
            | ActivityReturnReason::StaffKick { transport }
                if command.is_some()
                    || !online
                    || current.current_transport() != Some(transport)
                    || admitted
                        .controller
                        .is_none_or(|controller| controller.transport != transport) =>
            {
                return refuse(ActivityRefusal::AuthorityRejected);
            }
            ActivityReturnReason::StaffKickLost { epoch }
                if command.is_some()
                    || online
                    || current.current_control_loss_epoch() != Some(epoch) =>
            {
                return refuse(ActivityRefusal::AuthorityRejected);
            }
            ActivityReturnReason::CapabilityMismatch { epoch }
                if command.is_some()
                    || online
                    || current.current_control_loss_epoch() != Some(epoch)
                    || current
                        .current_original_grace_deadline()
                        .is_none_or(|deadline| deadline <= now) =>
            {
                return refuse(ActivityRefusal::AuthorityRejected);
            }
            _ => {}
        }
        if reason == ActivityReturnReason::Logout
            && command.is_none()
            && (online
                || current
                    .current_original_grace_deadline()
                    .is_none_or(|deadline| deadline > now))
        {
            return refuse(ActivityRefusal::NotDue);
        }
        let trigger_reason = reason;
        let source_view = self.runtime_view(&source);
        // A disconnected participant has no online respawn cadence. Preserve
        // the original transport/epoch/deadline checks above before choosing a
        // proven Death return; a stale release must not inherit control of a
        // new Channel successor merely because the actor happens to be dead.
        let reason = if command.is_none() && !matches!(reason, ActivityReturnReason::Death { .. }) {
            let native_death = if let Some(actor) = admitted.runtime_actor {
                let runtime = source.runtime.lock().await;
                let states = source.states.spell_states.lock().await;
                states.player_death(&runtime, actor, admitted.game_session_id)
            } else {
                None
            };
            if let Some(death) = native_death {
                let actor = admitted.runtime_actor.ok_or(())?;
                // Finish the death and selected Item work with the ordinary
                // training fence. Post-loss equipment is refreshed only after
                // the committed Channel return, before native respawn.
                let (settled, _) = source_view
                    .settle_death(actor, admitted.game_session_id, death, false)
                    .await;
                if settled != super::SessionDeath::Durable {
                    return Err(());
                }
                ActivityReturnReason::Death {
                    occurrence: *death.occurrence.as_bytes(),
                }
            } else {
                reason
            }
        } else {
            reason
        };
        let owner = ActivityHandoffOwner::new(self.root.clone());
        let placement = match reason {
            ActivityReturnReason::Death { occurrence } => {
                owner
                    .read_death_return_placement(self.character, self.holder, fence, occurrence)
                    .await
            }
            _ => {
                owner
                    .read_return_placement(self.character, self.holder, fence)
                    .await
            }
        }
        .map_err(|_| ())?;
        let Some(placement) = placement else {
            return refuse(ActivityRefusal::AuthorityRejected);
        };
        let Some(destination) = self.activity_runtimes.get(
            placement.destination_scope,
            placement.destination_generation.get(),
        ) else {
            return refuse(ActivityRefusal::NoCapacity);
        };
        if !matches!(destination.scope, RuntimeScopeRefV1::Channel { .. }) {
            return refuse(ActivityRefusal::InvalidInput);
        }
        let destination_session = SecureIdentifiers.game_session_id().ok_or(())?;
        let return_id = *SecureIdentifiers.game_session_id().ok_or(())?.as_bytes();
        let destination_transport = if online {
            Some(SecureIdentifiers.transport_ref().ok_or(())?)
        } else {
            None
        };
        self.activity_runtimes
            .returns
            .lock()
            .map_err(|_| ())?
            .try_reserve(1)
            .map_err(|_| ())?;
        let destination_view = self.runtime_view(&destination);
        // Complete every already captured shared lethal before reading live
        // vitals. The queue seal below closes the capture-to-freeze race.
        let mut principals: std::collections::BTreeSet<_> = source
            .states
            .attack
            .lock()
            .await
            .kills()
            .proficiency_principals(admitted.game_session_id)
            .into_iter()
            .collect();
        principals.insert(admitted.game_session_id);
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
        let Some(actor) = admitted.runtime_actor else {
            return refuse(ActivityRefusal::AuthorityRejected);
        };
        if !source_view
            .settle_activity_equipment(actor, admitted.game_session_id)
            .await
        {
            return Err(());
        }
        let slot = self.revision_sequencer.acquire(fence.character_id).await;
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
        let lease = from
            .bound_attacker_lease(actor, admitted.game_session_id)
            .map_err(|_| ())?;
        let control = from
            .player_control_facts(actor, admitted.game_session_id)
            .map_err(|_| ())?;
        if lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || lease.generation() != fence.character_lease_generation
            || online != control.control_loss.is_none()
            || (!online
                && control.control_loss.is_none_or(|loss| {
                    Some(loss.epoch)
                        != current
                            .current_control_loss_epoch()
                            .map(|epoch| epoch.get())
                        || Some(loss.grace_deadline) != current.current_original_grace_deadline()
                }))
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
        if let Some(proof) = source_exit.as_ref()
            && matches!(reason, ActivityReturnReason::Exit)
        {
            let target = proof.destination();
            if !proof.matches(&from, admitted)
                || (
                    placement.exit_position.cell.x,
                    placement.exit_position.cell.y,
                    i32::from(placement.exit_position.cell.floor),
                ) != (target.x, target.y, target.z)
            {
                return refuse(ActivityRefusal::AuthorityRejected);
            }
        }
        let exit = placement.exit_position;
        if exit.coordinate_frame_digest != to.content_pin().frame_binding_digest()
            || exit.map_revision_digest != to.map_revision_digest()
            || exit.server_artifact_digest != to.content_pin().server_artifact_digest()
        {
            return refuse(ActivityRefusal::ContentChanged);
        }
        let doors = destination.door.lock().await;
        let mut spells = source.states.spell_states.lock().await;
        let mut destination_spells = destination.states.spell_states.lock().await;
        let cell = MovementLocalPosition {
            x: exit.cell.x,
            y: exit.cell.y,
            floor: exit.cell.floor,
        };
        if !super::respawn_cell_admissible(
            destination_view.movement_cells().index(),
            destination_view.movement_cells().scope(),
            destination.scope.world_id(),
            exit.server_artifact_digest,
            doors.blocking_cells(),
            cell,
        ) {
            return refuse(ActivityRefusal::Position);
        }
        let Some((_, _, state)) = spells
            .actors
            .iter()
            .find(|(held, session, _)| *held == actor && *session == admitted.game_session_id)
        else {
            return refuse(ActivityRefusal::AuthorityRejected);
        };
        // Preserve explicit live pools at maxima too. A real dead actor still
        // has no LastVitals and follows the separately proven Death branch.
        let current_vitals = state.vitals();
        let vitals = LastVitals::new(current_vitals.health, current_vitals.mana);
        let death = match reason {
            ActivityReturnReason::Death { occurrence } => {
                let Some(death) = spells.player_death(&from, actor, admitted.game_session_id)
                else {
                    return refuse(ActivityRefusal::AuthorityRejected);
                };
                if death.occurrence.as_bytes() != &occurrence {
                    return refuse(ActivityRefusal::AuthorityRejected);
                }
                true
            }
            _ => {
                if vitals.is_none() {
                    return refuse(ActivityRefusal::AuthorityRejected);
                }
                false
            }
        };
        let rest = Some(state.bed_rest_snapshot(self.owner_now().get()));
        let member = ActivityTransferMember {
            source_actor: actor,
            source_session: admitted.game_session_id,
            source_lease: lease,
            destination_session,
            destination_lease: CharacterLease::new(
                lease.character_id(),
                lease.generation().checked_add(1).ok_or(())?,
            )
            .map_err(|_| ())?,
            destination_cell: cell,
        };
        let request = ActivityReturnRequest {
            return_id,
            handoff_id: placement.handoff_id,
            command_id: command,
            source: fence,
            destination_game_session_id: destination_session,
            destination_transport,
            destination_generation: placement.destination_generation,
            reason,
            current_vitals: vitals,
            current_rest: rest,
        };
        let (mut attack, mut destination_attack) = if source_first {
            let attack = source.states.attack.lock().await;
            let destination_attack = destination.states.attack.lock().await;
            (attack, destination_attack)
        } else {
            let destination_attack = destination.states.attack.lock().await;
            let attack = source.states.attack.lock().await;
            (attack, destination_attack)
        };
        if matches!(
            reason,
            ActivityReturnReason::Logout
                | ActivityReturnReason::IdleLogout
                | ActivityReturnReason::Abandoned { .. }
                | ActivityReturnReason::CapabilityMismatch { .. }
        ) && super::held_step(
            &spells,
            &attack,
            actor,
            admitted.game_session_id,
            self.owner_now(),
            super::HOLD_STEP,
        )
        .is_some()
        {
            return refuse(ActivityRefusal::NotDue);
        }
        let pending = {
            let mut attempts = self.activity_runtimes.returns.lock().map_err(|_| ())?;
            attempts.try_reserve(1).map_err(|_| ())?;
            let mut kill_marks = seal_activity_kills(attack.kills(), &[member])?;
            let held = if death {
                spells.prepare_activity_death_states(&mut destination_spells, &[member])
            } else {
                spells.prepare_activity_states(&mut destination_spells, &[member])
            };
            let mut held = match held {
                Ok(held) => held,
                Err(()) => {
                    finish_activity_kills(attack.kills(), &mut kill_marks, ReleaseEnd::Retryable);
                    return refuse(ActivityRefusal::AuthorityRejected);
                }
            };
            let mut held_attacks =
                match attack.prepare_activity_attacks(&from, &mut destination_attack, &[member]) {
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
            let prepared = if placement.boss_room {
                from.prepare_activity_boss_return(&mut to, &[member])
            } else {
                from.prepare_activity_transfer(&mut to, &[member])
            };
            let physical = match prepared {
                Ok(reserved) => reserved,
                Err(_) => {
                    spells.abort_activity_states(&mut held);
                    attack.abort_activity_attacks(&mut held_attacks);
                    finish_activity_kills(attack.kills(), &mut kill_marks, ReleaseEnd::Retryable);
                    return refuse(ActivityRefusal::NoCapacity);
                }
            };
            let pending = Arc::new(tokio::sync::Mutex::new(NativeActivityReturnAttempt {
                source: source.clone(),
                source_exit,
                exit_effect_start: None,
                destination: destination.clone(),
                admitted,
                trigger_reason,
                placement,
                request,
                physical,
                spells: held,
                attacks: held_attacks,
                kill_marks,
                receipt: None,
                moved: None,
                states_installed: false,
                ancillary_installed: false,
                death_completed: false,
                outcome: None,
            }));
            // From extraction to retention there is no await, so cancellation cannot
            // destroy the sole living actor state or free a possibly committed slot.
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
        drop(slot);
        drop(_second_lane);
        drop(_first_lane);
        self.finish_native_activity_return(pending).await
    }
}

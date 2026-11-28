impl <'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    async fn finish_native_activity_return(
        &self,
        pending: Arc<tokio::sync::Mutex<NativeActivityReturnAttempt<'a>>>,
    ) -> Result<NativeActivityOutcome, ()> {
        let mut attempt = pending.lock().await;
        if let Some(outcome) = &attempt.outcome {
            return Ok(outcome.clone());
        }
        let source = attempt.source.clone();
        let destination = attempt.destination.clone();
        let source_view = self.runtime_view(&source);
        let destination_view = self.runtime_view(&destination);
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
        let _slot = self
            .revision_sequencer
            .acquire(attempt.request.source.character_id)
            .await;
        let budget = AdmissionRetryBudget::for_root(&attempt.admitted.game_session_id, self.root);
        if attempt.receipt.is_none() {
            let account = super::canonical_uuid(&attempt.placement.account_id);
            let owner = ActivityHandoffOwner::new(self.root.clone());
            let result = budget
                .retry_durability("activity_return", || async {
                    use crate::durability::DurabilityError;

                    // Take fresh evidence for every attempt. Its publication
                    // slot has its own hold deadline and must clear before a wait.
                    let evidence = self.evidence
                        .refresh_account_held(self.root, self.holder, &account)
                        .await
                        .map_err(|error| {
                            if error.holder_busy() {
                                DurabilityError::RootUnavailable
                            } else {
                                DurabilityError::Unavailable
                            }
                        })?;
                    let result = if matches!(attempt.request.reason, ActivityReturnReason::Death { .. })
                        && matches!(attempt.trigger_reason,
                            ActivityReturnReason::StaffKick { .. } | ActivityReturnReason::StaffKickLost { .. })
                    {
                        owner.return_forced_death(
                            self.character, self.holder, attempt.request.clone(), attempt.trigger_reason,
                        ).await
                    } else if let Some(proof) = attempt.source_exit.as_ref()
                        && matches!(attempt.request.reason, ActivityReturnReason::Exit)
                    {
                        owner.return_participant_for_native_step(
                            self.character, self.holder, attempt.request.clone(), proof.native_step(),
                        ).await
                    } else {
                        owner.return_participant(self.character, self.holder, attempt.request.clone()).await
                    };
                    evidence.release().await.map_err(|_| DurabilityError::Unavailable)?;
                    result
                })
                .await
                .map_err(|_| ())?;
            match result {
                ActivityReturnOutcome::Returned(receipt)
                | ActivityReturnOutcome::Replayed(receipt) => {
                    if !return_receipt_matches(&attempt.request, &attempt.placement, &receipt) {
                        return Err(());
                    }
                    attempt.receipt = Some(receipt);
                }
                ActivityReturnOutcome::Refused(reason) => {
                    let (mut from, mut to) = if source_first {
                        let from = source.runtime.lock().await;
                        let to = destination.runtime.lock().await;
                        (from, to)
                    } else {
                        let to = destination.runtime.lock().await;
                        let from = source.runtime.lock().await;
                        (from, to)
                    };
                    let mut spells = source.states.spell_states.lock().await;
                    let mut attack = source.states.attack.lock().await;
                    from.abort_activity_transfer(&mut to, &mut attempt.physical)
                        .map_err(|_| ())?;
                    spells.abort_activity_states(&mut attempt.spells);
                    attack.abort_activity_attacks(&mut attempt.attacks);
                    finish_activity_kills(
                        attack.kills(),
                        &mut attempt.kill_marks,
                        ReleaseEnd::Retryable,
                    );
                    let outcome = NativeActivityOutcome::Refused(reason);
                    attempt.outcome = Some(outcome.clone());
                    return Ok(outcome);
                }
            }
        }
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let (current, _) = AdmissionRetryBudget::for_root(&attempt.request.destination_game_session_id, self.root)
            .retry_durability("activity_return", || store.current_session_at(attempt.request.destination_game_session_id))
            .await
            .map_err(|_| ())?;
        let fence = super::item_fence_of(current).ok_or(())?;
        if current.current_transport() != attempt.request.destination_transport
            || fence.runtime_scope != destination.scope
            || fence.scope_ownership_generation.get() != destination.generation
            || fence.character_id != attempt.request.source.character_id
            || Some(fence.character_lease_generation)
                != attempt
                    .request
                    .source
                    .character_lease_generation
                    .checked_add(1)
            || fence.connection_generation.get() != 1
        {
            return Err(());
        }
        let loss = if attempt.request.destination_transport.is_some() {
            if current.session_state() != GameSessionState::Active {
                return Err(());
            }
            None
        } else {
            if current.session_state() != GameSessionState::Reconnectable {
                return Err(());
            }
            Some(ControlLossMark {
                epoch: current.current_control_loss_epoch().ok_or(())?.get(),
                grace_deadline: current.current_original_grace_deadline().ok_or(())?,
            })
        };
        if !attempt.states_installed {
            let (mut from, mut to) = if source_first {
                let from = source.runtime.lock().await;
                let to = destination.runtime.lock().await;
                (from, to)
            } else {
                let to = destination.runtime.lock().await;
                let from = source.runtime.lock().await;
                (from, to)
            };
            let mut spells = destination.states.spell_states.lock().await;
            let (attack, mut destination_attack) = if source_first {
                let attack = source.states.attack.lock().await;
                let destination_attack = destination.states.attack.lock().await;
                (attack, destination_attack)
            } else {
                let destination_attack = destination.states.attack.lock().await;
                let attack = source.states.attack.lock().await;
                (attack, destination_attack)
            };
            if attempt.moved.is_none() {
                let moved = from
                    .commit_activity_transfer(&mut to, &mut attempt.physical)
                    .map_err(|_| ())?;
                if moved.len() != 1 {
                    return Err(());
                }
                attempt.moved = moved.first().copied();
            }
            let moved = attempt.moved.ok_or(())?;
            if let Some(mark) = loss {
                to.record_control_loss(moved.destination_actor, moved.destination_session, mark)
                    .map_err(|_| ())?;
            }
            let moved_states = [moved];
            let NativeActivityReturnAttempt {
                spells: held_spells,
                attacks: held_attacks,
                ..
            } = &mut *attempt;
            let attack_commit =
                destination_attack.prepare_activity_attack_commit(held_attacks, &moved_states)?;
            spells.commit_activity_states(held_spells, &moved_states)?;
            attack_commit.commit();
            attempt.states_installed = true;
            finish_activity_kills(
                attack.kills(),
                &mut attempt.kill_marks,
                ReleaseEnd::Committed,
            );
        }
        let moved = attempt.moved.ok_or(())?;
        let old = attempt.admitted;
        let controller = match attempt.request.destination_transport {
            Some(transport) => Some(ControllerBinding {
                transport,
                account_id: old.controller.ok_or(())?.account_id,
            }),
            None => None,
        };
        let next = AdmittedSession {
            game_session_id: moved.destination_session,
            runtime_actor: Some(moved.destination_actor),
            first_entry: FirstEntryOutcome::Positioned,
            controller,
            item_fence: Some(fence),
            continuity: super::connection::SessionContinuity {
                house_input_pending: matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
                    .then_some(false),
                ..activity_continuity(old.continuity)
            },
            ..old
        };
        if !attempt.ancillary_installed {
            source_view
                .transfer_activity_session_state(&destination_view, old, next)
                .await?;
            attempt.ancillary_installed = true;
        }
        // M18: rebuild the destination death consumer from its transferred quest
        // copy. Repeating this projection after cancellation never replays a move.
        destination_view
            .refresh_grizzly_kill_snapshot(next.game_session_id)
            .await;
        // The destination's existing death owner takes its own lane and revision
        // slot while it places, consumes and refills the transferred dead tuple.
        drop(_slot);
        drop(_second_lane);
        drop(_first_lane);
        if attempt.source_exit.is_some()
            && matches!(attempt.request.reason, ActivityReturnReason::Exit)
            && attempt.exit_effect_start.is_none()
        {
            match destination_view.emit_source_boss_exit_effect(next).await {
                Ok(cursor) => attempt.exit_effect_start = Some(cursor),
                Err(()) => super::operator_event("source_boss_exit_effect_unavailable"),
            }
            // No await separates this best-effort child and outcome publication
            // below. A missing visual cannot strand committed session custody.
        }
        if let ActivityReturnReason::Death { occurrence } = attempt.request.reason
            && !attempt.death_completed
        {
            let occurrence =
                crate::durability::character_death::PlayerDeathOccurrence::from_bytes(occurrence)
                    .map_err(|_| ())?;
            if !destination_view
                .complete_activity_death_return(next, occurrence)
                .await
            {
                return Err(());
            }
            // No await separates the successful child completion and this
            // retained mark, so cancellation cannot cause a second refill.
            attempt.death_completed = true;
        }
        if next.controller.is_none() {
            self.activity_runtimes.remember_offline_return(old, next)?;
        }
        let outcome = NativeActivityOutcome::Committed(NativeActivityCommit {
            handoff_id: attempt.request.return_id,
            deadline_unix_ms: None,
            connections: vec![(old, next)],
            entry_effect_starts: attempt
                .exit_effect_start
                .map(|cursor| vec![(next.game_session_id, cursor)])
                .unwrap_or_default(),
        });
        attempt.outcome = Some(outcome.clone());
        Ok(outcome)
    }
}

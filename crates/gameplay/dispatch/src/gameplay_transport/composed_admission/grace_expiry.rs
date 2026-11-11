use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// CHARM-DESC-FENCE-LEASE: grace expiry fences the slot's damage writes under the runtime
    /// lock (step a), commits the durable release with the lock released (step b), then settles
    /// the slot by the durable outcome (step c). The fence token is the lost epoch, so a retry
    /// joins its own fence and a resume of that epoch lifts it. An unknown expiry keeps its hold
    /// for the next one, so the lifecycle's retries count the release once.
    pub(in crate::gameplay_transport) async fn release_after_grace(
        &self,
        admitted: AdmittedSession,
    ) -> GraceExpiryResult {
        let mut held = self.states.fence_holders.resume(admitted.game_session_id);
        let result = self.release_grace_attempts(admitted, &mut held).await;
        if result == GraceExpiryResult::Unknown
            && let Some(hold) = held
        {
            self.states.fence_holders.keep(hold);
        }
        result
    }

    pub(in crate::gameplay_transport) async fn release_grace_attempts(
        &self,
        admitted: AdmittedSession,
        held: &mut Option<FenceHold>,
    ) -> GraceExpiryResult {
        operator_event("event=control_loss_grace stage=entry");
        // Record only the first pending result at each fixed stage of this call.
        // Successful saves and retries do not consume the diagnostic budget.
        let mut pending_stages = std::collections::BTreeSet::new();
        let mut pending = |stage: &'static str| {
            if pending_stages.insert(stage) {
                operator_event(&format!("event=control_loss_grace stage={stage} kind=pending"));
            }
        };
        let Some(actor) = admitted.runtime_actor else {
            return GraceExpiryResult::NotApplicable;
        };
        let Some(account) = self.activity_runtimes.account_for(admitted) else {
            return GraceExpiryResult::NotApplicable;
        };
        let session = admitted.game_session_id;
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let account_id = canonical_uuid(&account);
        // The deadline is fixed, so waiting converges; store failures back off
        // exponentially, and the bounds only guard an owner that never recovers.
        let mut backoff = RECONCILE_BACKOFF;
        let mut next_backoff = || {
            let pause = backoff;
            backoff = backoff.saturating_mul(2).min(EXPIRY_MAX_BACKOFF);
            pause
        };
        // One hold across the attempts of an epoch, so a retry is counted once.
        let mut settled_death = None;
        // The grace/combat wait does not request preference. Once release work
        // starts, retain it across the existing attempts and their backoff.
        let mut foreground = None;
        let mut attempts = 0;
        while attempts < EXPIRY_ATTEMPTS {
            // Timeout/defeat may return an offline actor while its original
            // grace timer sleeps. Retry through the shared registry, retaining
            // the original deadline and never releasing the terminal source.
            let current = self.activity_runtimes.current_connection(admitted);
            if current.game_session_id != admitted.game_session_id
                || current.item_fence != admitted.item_fence
            {
                if let Some(hold) = held.as_mut() {
                    self.states.fence_holders.release(hold);
                }
                *held = None;
                pending("activity_successor");
                return GraceExpiryResult::Unknown;
            }
            // Poll captured staff decisions while the original grace is held.
            // A forced release owns the same exact epoch; it does not expire it early.
            match Box::pin(self.release_staff_session(admitted)).await {
                GraceExpiryResult::NotApplicable => {}
                GraceExpiryResult::Unknown => {
                    pending("staff_release");
                    return GraceExpiryResult::Unknown;
                }
                result => return result,
            }
            let facts = {
                let runtime = self.runtime.lock().await;
                if runtime.house_actor_state(actor, session).is_ok() {
                    runtime.house_player_control_facts(actor, session)
                } else {
                    runtime.player_control_facts(actor, session)
                }
            };
            let mark = match facts {
                Ok(facts) => facts.control_loss,
                Err(_) => {
                    // Retirement can remove the actor before its offline acknowledgement
                    // succeeds. Retry that exact terminal cleanup, not the release write.
                    // Missing/stale native facts alone never prove terminality or absence
                    // in the current scope; the existing reconciler checks both.
                    return match self.reconcile_terminal_session(admitted).await {
                        GraceExpiryResult::Released => GraceExpiryResult::Released,
                        _ => {
                            pending("terminal_session");
                            GraceExpiryResult::Unknown
                        }
                    };
                }
            };
            // The lost epoch was resumed: nothing is this transition's to release.
            let Some(ControlLossMark {
                epoch,
                grace_deadline: deadline,
            }) = mark
            else {
                if let Some(hold) = held.as_mut() {
                    self.states.fence_holders.release(hold);
                }
                // Every hold kept on the session is of a lost epoch that is over.
                while let Some(mut stale) = self.states.fence_holders.resume(session) {
                    self.states.fence_holders.release(&mut stale);
                }
                // KILL-REWARD-COMP-1 §1.3: no release of the resumed epoch committed, so a kill
                // mark an unknown outcome kept is cleared as retryable.
                self.release_orphaned_kills(session).await;
                return GraceExpiryResult::NotApplicable;
            };
            let token = TransitionFence::GraceExpiry(epoch);
            if let Some(stale) = held.as_mut().filter(|hold| hold.token != token) {
                self.states.fence_holders.release(stale);
            }
            let hold = match &mut *held {
                Some(hold) if hold.token == token => hold,
                slot => slot.insert(FenceHold::new(session, token)),
            };
            // §3 item 6: fence only once the held deadline has passed.
            if let Some(ahead) = unix_seconds()
                .and_then(|now| u64::try_from(deadline.saturating_sub(now)).ok())
                .filter(|ahead| *ahead > 0)
            {
                self.wait_grace(
                    actor,
                    session,
                    Duration::from_secs(ahead)
                        .saturating_add(EXPIRY_SLACK)
                        .min(Duration::from_secs(1)),
                )
                .await;
                continue;
            }
            // Observing the same fixed grace is not a semantic release attempt.
            // Keep the existing bounded retry budget for actual expiry work.
            attempts += 1;
            // ATTACK0-RL-03: a closed client stays in the world until its in-fight deadline ends.
            // Each hold step runs the melee pass; the whole hold counts as one attempt.
            if self
                .hold_while_in_fight_releasing_demand(actor, session, &mut foreground)
                .await
            {
                pending("combat_hold");
                while self.hold_while_in_fight(actor, session).await {}
                continue;
            }
            if foreground.is_none() {
                let Ok(demand) = self.root.foreground_demand() else {
                    pending("foreground_demand");
                    return GraceExpiryResult::Unknown;
                };
                foreground = Some(demand);
            }
            if !self
                .settle_released_death(actor, session, &mut settled_death)
                .await
            {
                pending("death_save");
                tokio::time::sleep(next_backoff()).await;
                continue;
            }
            if matches!(actor.runtime_scope(), RuntimeScopeRefV1::Instance { .. }) {
                // Original deadline, combat hold and death settlement above
                // remain the existing owner. Return before fencing the actor:
                // a frozen release cannot also reserve a native transfer.
                if hold.held {
                    pending("activity_return");
                    return GraceExpiryResult::Unknown;
                }
                match self
                    .return_due_activity(
                        admitted,
                        crate::durability::activity_handoff::ActivityReturnReason::Logout,
                    )
                    .await
                {
                    Ok(super::super::native_activity::NativeActivityOutcome::Committed(commit)) => {
                        if self.publish_activity_commit(&commit).is_err() {
                            pending("activity_publish");
                            return GraceExpiryResult::Unknown;
                        }
                        self.states.fence_holders.release(hold);
                        *held = None;
                        // The outer lifecycle resolves the real fresh origin
                        // owner. It retains the original expired loss deadline.
                        pending("activity_successor");
                        return GraceExpiryResult::Unknown;
                    }
                    Ok(super::super::native_activity::NativeActivityOutcome::Refused(_))
                    | Err(()) => {
                        pending("activity_return");
                        tokio::time::sleep(next_backoff()).await;
                        continue;
                    }
                }
            }
            // The local release fence closes the live lease used to install a successor.
            if !self.reconcile_pending_item_effects(actor, session).await {
                pending("item_effects");
                tokio::time::sleep(next_backoff()).await;
                continue;
            }
            match self.fence_transition(actor, hold, false).await {
                FenceStep::Fenced => {}
                FenceStep::Wait => {
                    pending("release_fence");
                    tokio::time::sleep(next_backoff()).await;
                    continue;
                }
                // The mark moved: the epoch was resumed before the fence.
                FenceStep::Refused(CarrierError::ControlLossConflict) => {
                    self.release_orphaned_kills(session).await;
                    return GraceExpiryResult::NotApplicable;
                }
                FenceStep::Refused(_) => {
                    pending("release_fence");
                    return GraceExpiryResult::Unknown;
                }
            }
            if self.save_familiar_at_actor_end(actor, session, None).await
                == actor_spell::FamiliarLogoutSave::Unknown
            {
                pending("familiar_save");
                tokio::time::sleep(next_backoff()).await;
                continue;
            }
            if self.save_spell_training(actor, session, true).await
                == actor_spell::TrainingSave::Unknown
            {
                pending("training_save");
                tokio::time::sleep(next_backoff()).await;
                continue;
            }
            // SPELL-D8 §8.2 save point 1: the actor's monk values are durable, or fenced out,
            // before the release can end the Character lease.
            if !self.save_wheel_gift(actor, session, true).await {
                pending("wheel_gift");
                tokio::time::sleep(next_backoff()).await;
                continue;
            }
            if self.save_monk_state(&admitted, actor).await == monk_save::MonkSave::Unknown {
                pending("monk_save");
                tokio::time::sleep(next_backoff()).await;
                continue;
            }
            if !super::super::item_use::gourmet::checkpoint_at_actor_end(self, actor, session).await
            {
                pending("gourmet_checkpoint");
                tokio::time::sleep(next_backoff()).await;
                continue;
            }
            if !self.save_conditions_at_actor_end(actor, session).await {
                pending("condition_save");
                tokio::time::sleep(next_backoff()).await;
                continue;
            }
            // KILL-REWARD-COMP-1 §1.3: the session's own kills settle before it ends; a deferred
            // drain sends nothing, and the lifecycle retries the expiry behind its backoff.
            let kills = match self.seal_kill_rewards(session).await {
                KillRelease::Send(mark) => mark,
                KillRelease::Deferred => {
                    pending("kill_rewards");
                    return GraceExpiryResult::Unknown;
                }
            };
            // CHAR-POSITION-0 §3.2 "At logout": the release writes the fenced actor's cell.
            let pause = match self
                .final_position_store(&store, session, actor)
                .await
                .release_expired_loss(session, &account_id)
                .await
            {
                Ok(ExpiredLossReleaseV1::NotApplicable) => {
                    match self.settle_unended(&store, actor, hold).await {
                        UnendedSettle::Lifted => {
                            self.settle_kill_release(kills, ReleaseEnd::Retryable).await;
                            return GraceExpiryResult::NotApplicable;
                        }
                        UnendedSettle::Terminal => {
                            self.settle_kill_release(kills, ReleaseEnd::Committed).await;
                            let result = self.retire_reconciled(account, session, actor).await;
                            self.forget_kill_release(kills).await;
                            if result == GraceExpiryResult::Unknown {
                                pending("retire");
                            }
                            return result;
                        }
                        UnendedSettle::Unknown => {
                            pending("release_not_applicable");
                            self.settle_kill_release(kills, ReleaseEnd::Unknown).await;
                            next_backoff()
                        }
                    }
                }
                Ok(ExpiredLossReleaseV1::NotExpired { deadline, now }) => {
                    pending("release_not_expired");
                    #[cfg(test)]
                    monk_save::postgres_tests::record_not_expired();
                    match self.settle_unended(&store, actor, hold).await {
                        UnendedSettle::Lifted => {
                            self.settle_kill_release(kills, ReleaseEnd::Retryable).await;
                            Duration::from_secs(u64::try_from(deadline - now).unwrap_or(0))
                                .saturating_add(EXPIRY_SLACK)
                        }
                        UnendedSettle::Terminal => {
                            self.settle_kill_release(kills, ReleaseEnd::Committed).await;
                            let result = self.retire_reconciled(account, session, actor).await;
                            self.forget_kill_release(kills).await;
                            if result == GraceExpiryResult::Unknown {
                                pending("retire");
                            }
                            return result;
                        }
                        UnendedSettle::Unknown => {
                            pending("release_settle");
                            self.settle_kill_release(kills, ReleaseEnd::Unknown).await;
                            next_backoff()
                        }
                    }
                }
                Ok(ExpiredLossReleaseV1::Released { .. } | ExpiredLossReleaseV1::Terminal) => {
                    self.settle_kill_release(kills, ReleaseEnd::Committed).await;
                    // PREM-1b: the session is over; its Premium pulls stop.
                    self.release_premium(account, session);
                    let result = self.retire(session, actor).await;
                    self.forget_kill_release(kills).await;
                    if result == GraceExpiryResult::Unknown {
                        pending("retire");
                    }
                    return result;
                }
                // Unknown outcome: keep the fence and the kill mark; the retry reconciles from
                // the durable row.
                Err(_) => {
                    pending("release_commit");
                    self.settle_kill_release(kills, ReleaseEnd::Unknown).await;
                    next_backoff()
                }
            };
            tokio::time::sleep(pause).await;
        }
        GraceExpiryResult::Unknown
    }

    pub(in crate::gameplay_transport) async fn expire_session_control_loss(
        &self,
        admitted: AdmittedSession,
    ) -> GraceExpiryResult {
        let result = self.release_after_grace(admitted).await;
        // Resumed or released: this lost connection can no longer be resumed. An unknown result
        // keeps it; the lifecycle runs the expiry again.
        if result != GraceExpiryResult::Unknown {
            self.forget_lost(
                admitted.game_session_id,
                admitted.continuity.connection_generation,
            );
        }
        result
    }
}

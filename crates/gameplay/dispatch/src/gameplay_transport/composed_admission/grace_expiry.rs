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
        let (Some(actor), Some(controller)) = (admitted.runtime_actor, admitted.controller) else {
            return GraceExpiryResult::NotApplicable;
        };
        let session = admitted.game_session_id;
        let store = FreshAdmissionStore::from_root(self.root.clone());
        let account_id = canonical_uuid(&controller.account_id);
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
        for _ in 0..EXPIRY_ATTEMPTS {
            let mark = match self
                .runtime
                .lock()
                .await
                .player_control_facts(actor, session)
            {
                Ok(facts) => facts.control_loss,
                Err(_) => return GraceExpiryResult::Unknown,
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
                    Duration::from_secs(ahead).saturating_add(EXPIRY_SLACK),
                )
                .await;
                continue;
            }
            // ATTACK0-RL-03: a closed client stays in the world until its in-fight deadline ends.
            // Each hold step runs the melee pass; the whole hold counts as one attempt.
            if self.hold_while_in_fight(actor, session).await {
                while self.hold_while_in_fight(actor, session).await {}
                continue;
            }
            if !self
                .settle_released_death(actor, session, &mut settled_death)
                .await
            {
                tokio::time::sleep(next_backoff()).await;
                continue;
            }
            match self.fence_transition(actor, hold).await {
                FenceStep::Fenced => {}
                FenceStep::Wait => {
                    tokio::time::sleep(next_backoff()).await;
                    continue;
                }
                // The mark moved: the epoch was resumed before the fence.
                FenceStep::Refused(CarrierError::ControlLossConflict) => {
                    self.release_orphaned_kills(session).await;
                    return GraceExpiryResult::NotApplicable;
                }
                FenceStep::Refused(_) => return GraceExpiryResult::Unknown,
            }
            if self.save_familiar_at_actor_end(actor, session, None).await
                == actor_spell::FamiliarLogoutSave::Unknown
                || self.save_spell_training(actor, session, true).await
                    == actor_spell::TrainingSave::Unknown
            {
                tokio::time::sleep(next_backoff()).await;
                continue;
            }
            // SPELL-D8 §8.2 save point 1: the actor's monk values are durable, or fenced out,
            // before the release can end the Character lease.
            if self.save_monk_state(&admitted, actor).await == monk_save::MonkSave::Unknown {
                tokio::time::sleep(next_backoff()).await;
                continue;
            }
            // KILL-REWARD-COMP-1 §1.3: the session's own kills settle before it ends; a deferred
            // drain sends nothing, and the lifecycle retries the expiry behind its backoff.
            let kills = match self.seal_kill_rewards(session).await {
                KillRelease::Send(mark) => mark,
                KillRelease::Deferred => return GraceExpiryResult::Unknown,
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
                            let result = self
                                .retire_reconciled(controller.account_id, session, actor)
                                .await;
                            self.forget_kill_release(kills).await;
                            return result;
                        }
                        UnendedSettle::Unknown => {
                            self.settle_kill_release(kills, ReleaseEnd::Unknown).await;
                            next_backoff()
                        }
                    }
                }
                Ok(ExpiredLossReleaseV1::NotExpired { deadline, now }) => {
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
                            let result = self
                                .retire_reconciled(controller.account_id, session, actor)
                                .await;
                            self.forget_kill_release(kills).await;
                            return result;
                        }
                        UnendedSettle::Unknown => {
                            self.settle_kill_release(kills, ReleaseEnd::Unknown).await;
                            next_backoff()
                        }
                    }
                }
                Ok(ExpiredLossReleaseV1::Released { .. } | ExpiredLossReleaseV1::Terminal) => {
                    self.settle_kill_release(kills, ReleaseEnd::Committed).await;
                    // PREM-1b: the session is over; its Premium pulls stop.
                    self.release_premium(controller.account_id, session);
                    let result = self.retire(session, actor).await;
                    self.forget_kill_release(kills).await;
                    return result;
                }
                // Unknown outcome: keep the fence and the kill mark; the retry reconciles from
                // the durable row.
                Err(_) => {
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

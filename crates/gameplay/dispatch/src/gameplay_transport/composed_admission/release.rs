use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn release_terminal(
        &self,
        admitted: AdmittedSession,
        release: TerminalRelease,
    ) -> GraceExpiryResult {
        let (Some(actor), Some(controller)) = (admitted.runtime_actor, admitted.controller) else {
            return GraceExpiryResult::NotApplicable;
        };
        let session = admitted.game_session_id;
        let store = FreshAdmissionStore::from_root(self.root.clone());
        // ATTACK0-RL-03: a closed client stays in the world until its in-fight deadline ends.
        while self.hold_while_in_fight(actor, session).await {}
        let Ok(token) = release.fence(&mut *self.runtime.lock().await) else {
            return GraceExpiryResult::Unknown;
        };
        let mut hold = FenceHold::new(session, token);
        let mut settled_death = None;
        let mut kills = None;
        let abandoned = matches!(release, TerminalRelease::Abandoned(_));
        let mut backoff = RECONCILE_BACKOFF;
        let mut next_backoff = || {
            let pause = backoff;
            backoff = backoff.saturating_mul(2).min(EXPIRY_MAX_BACKOFF);
            pause
        };
        let mut attempt = 0;
        while retries_release(attempt, abandoned) {
            attempt = attempt.saturating_add(1);
            let step = self
                .release_attempt(
                    &admitted,
                    &store,
                    release,
                    &mut hold,
                    &mut settled_death,
                    &mut kills,
                )
                .await;
            match step {
                ReleaseAttempt::Done(result) => return result,
                ReleaseAttempt::Retry | ReleaseAttempt::Unknown => {
                    tokio::time::sleep(next_backoff()).await;
                }
            }
        }
        if matches!(release, TerminalRelease::CapabilityMismatch(_)) {
            // D449: an unproven mismatch release is reconciled from the durable row. TERMINAL
            // settles as released; a session still holding the lease gives back this release's
            // hold, and the epoch fence is lifted once no other release holds it, so the client's
            // retry can repeat the release.
            // Every attempt gave up its kill hold, so the kill mark is reconciled, not settled.
            return match self.settle_unended(&store, actor, &mut hold).await {
                UnendedSettle::Terminal => {
                    self.reconcile_kill_release(kills, ReleaseEnd::Committed)
                        .await;
                    let result = self
                        .retire_reconciled(controller.account_id, session, actor)
                        .await;
                    self.forget_kill_release(kills).await;
                    result
                }
                UnendedSettle::Lifted => {
                    self.reconcile_kill_release(kills, ReleaseEnd::Retryable)
                        .await;
                    GraceExpiryResult::Unknown
                }
                // The fence stays: the epoch's grace expiry, which joins the same token, takes
                // the hold over instead of counting it again.
                UnendedSettle::Unknown => {
                    self.states.fence_holders.keep(hold);
                    GraceExpiryResult::Unknown
                }
            };
        }
        GraceExpiryResult::Unknown
    }

    /// One attempt of a terminal release (`release_terminal`, `release_logout`): the death
    /// settle, the fence, the actor-end saves, the §1.3 kill seal, the durable transaction and
    /// its settle. `kills` keeps the attempt's kill mark for a later reconcile.
    pub(in crate::gameplay_transport) async fn release_attempt(
        &self,
        admitted: &AdmittedSession,
        store: &FreshAdmissionStore,
        release: TerminalRelease,
        hold: &mut FenceHold,
        settled_death: &mut Option<actor_spell::PlayerDeath>,
        kills: &mut Option<KillReleaseMark>,
    ) -> ReleaseAttempt {
        let (Some(actor), Some(controller)) = (admitted.runtime_actor, admitted.controller) else {
            return ReleaseAttempt::Done(GraceExpiryResult::NotApplicable);
        };
        let session = admitted.game_session_id;
        let account_id = canonical_uuid(&controller.account_id);
        if !self
            .settle_released_death(actor, session, settled_death)
            .await
        {
            return ReleaseAttempt::Retry;
        }
        if !self.stop_timed_actor(actor, session).await {
            return ReleaseAttempt::Retry;
        }
        match self.fence_transition(actor, hold).await {
            FenceStep::Fenced => {}
            FenceStep::Wait => return ReleaseAttempt::Retry,
            FenceStep::Refused(_) => return ReleaseAttempt::Done(GraceExpiryResult::Unknown),
        }
        // A logout's fence is its own minted transition id: one per attempt.
        let logout = match (release, hold.token) {
            (TerminalRelease::Logout(transport), TransitionFence::Transition(attempt)) => {
                Some((transport, attempt))
            }
            (TerminalRelease::Logout(_), TransitionFence::GraceExpiry(_)) => {
                return ReleaseAttempt::Done(GraceExpiryResult::Unknown);
            }
            (TerminalRelease::Abandoned(_) | TerminalRelease::CapabilityMismatch(_), _) => None,
        };
        // SPELL-D8 §8.2 save point 1: the actor's monk values are durable, or fenced out,
        // before the release can end the Character lease.
        if self.save_monk_state(admitted, actor).await == monk_save::MonkSave::Unknown {
            return ReleaseAttempt::Retry;
        }
        if self.save_spell_training(actor, session, true).await
            == actor_spell::TrainingSave::Unknown
        {
            return ReleaseAttempt::Retry;
        }
        // KILL-REWARD-COMP-1 §1.3: the session's own kills settle before it ends.
        *kills = match self.seal_kill_rewards(session).await {
            KillRelease::Send(mark) => mark,
            KillRelease::Deferred => return ReleaseAttempt::Retry,
        };
        let kills = *kills;
        // The familiar's actor-end snapshot is the last save before the durable release, so no
        // retryable step can follow a committed logout snapshot and leave the session running
        // on it (#2127 Codex P1 4279636456).
        if self
            .save_familiar_at_actor_end(actor, session, logout)
            .await
            == actor_spell::FamiliarLogoutSave::Unknown
        {
            self.settle_kill_release(kills, ReleaseEnd::Retryable).await;
            return ReleaseAttempt::Retry;
        }
        // CHAR-POSITION-0 §3.2 "At logout": the release writes the fenced actor's cell.
        let releasing = self.final_position_store(store, session, actor).await;
        let Some(releasing) = self
            .training_release_store(releasing, admitted, logout.is_some())
            .await
        else {
            return ReleaseAttempt::Retry;
        };
        let outcome = match release {
            TerminalRelease::Abandoned(transport) => {
                releasing
                    .release_abandoned_session(session, &account_id, transport)
                    .await
            }
            TerminalRelease::CapabilityMismatch(epoch) => {
                releasing
                    .release_capability_mismatch(session, &account_id, epoch)
                    .await
            }
            TerminalRelease::Logout(transport) => {
                releasing
                    .release_logout_session(session, &account_id, transport)
                    .await
            }
        };
        if ends_session(&outcome) {
            // PREM-1b (D476): the session is over, whether abandoned, mismatched or logged out;
            // its Premium pulls stop, as at grace expiry.
            self.release_premium(controller.account_id, session);
        }
        match outcome {
            Ok(ExpiredLossReleaseV1::NotApplicable | ExpiredLossReleaseV1::NotExpired { .. }) => {
                match self.settle_unended(store, actor, hold).await {
                    UnendedSettle::Lifted => {
                        self.settle_kill_release(kills, ReleaseEnd::Retryable).await;
                        ReleaseAttempt::Done(GraceExpiryResult::NotApplicable)
                    }
                    UnendedSettle::Terminal => {
                        self.settle_kill_release(kills, ReleaseEnd::Committed).await;
                        let result = self
                            .retire_reconciled(controller.account_id, session, actor)
                            .await;
                        self.forget_kill_release(kills).await;
                        ReleaseAttempt::Done(result)
                    }
                    UnendedSettle::Unknown => {
                        self.settle_kill_release(kills, ReleaseEnd::Unknown).await;
                        ReleaseAttempt::Unknown
                    }
                }
            }
            Ok(ExpiredLossReleaseV1::Released { .. } | ExpiredLossReleaseV1::Terminal) => {
                self.settle_kill_release(kills, ReleaseEnd::Committed).await;
                let result = self.retire(session, actor).await;
                self.forget_kill_release(kills).await;
                ReleaseAttempt::Done(result)
            }
            // Unknown outcome: keep the fence and the kill mark; the retry reconciles from
            // the durable row.
            Err(_) => {
                self.settle_kill_release(kills, ReleaseEnd::Unknown).await;
                ReleaseAttempt::Unknown
            }
        }
    }

    /// D449 (ARCH-BATCH-ITEM-EQUIP-PACKETS §1.13): a resume that passed every check but the
    /// capability check ends the lost session in a terminal release with no successor, so the
    /// client's fresh admission finds the character free. Nothing is resumed; the lost entry
    /// is dropped once the release is durable.
    pub(crate) async fn release_capability_mismatch(
        &self,
        lost: AdmittedSession,
        epoch: ControlLossEpochRefV1,
    ) -> GraceExpiryResult {
        let result = self
            .release_terminal(lost, TerminalRelease::CapabilityMismatch(epoch))
            .await;
        if result == GraceExpiryResult::Released {
            self.forget_lost(lost.game_session_id, lost.continuity.connection_generation);
        }
        result
    }

    pub(in crate::gameplay_transport) async fn release_abandoned_session(
        &self,
        admitted: AdmittedSession,
    ) -> GraceExpiryResult {
        let Some(controller) = admitted.controller else {
            return GraceExpiryResult::NotApplicable;
        };
        self.release_terminal(admitted, TerminalRelease::Abandoned(controller.transport))
            .await
    }
}

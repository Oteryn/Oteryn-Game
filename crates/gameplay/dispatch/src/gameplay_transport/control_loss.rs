use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn lose_control_for_cause(
        &self,
        admitted: AdmittedSession,
        wait: Duration,
        reset: Option<&crate::durability::world_reset::RetiringWorldReset>,
    ) -> ControlLossResult {
        let (Some(actor), Some(controller)) = (admitted.runtime_actor, admitted.controller) else {
            return ControlLossResult::NotApplicable;
        };
        tokio::time::sleep(wait).await;
        let result = match reset {
            Some(reset) => {
                self.commit_loss_cause(
                    admitted.game_session_id,
                    actor,
                    controller,
                    reset.loss_cause(),
                )
                .await
            }
            None => {
                self.commit_control_loss(admitted.game_session_id, actor, controller)
                    .await
            }
        };
        if result == ControlLossResult::Recorded {
            // §8.2: no command of the actor is accepted until a recovery initializes it again.
            let runtime = self.runtime.lock().await;
            self.spell_states
                .lock()
                .await
                .detach(&runtime, actor, admitted.game_session_id);
            // ATTACK-0 §3: the lost client's target stops swinging; its in-fight deadline runs on.
            self.attack
                .lock()
                .await
                .clear_target(actor, admitted.game_session_id);
        }
        if result == ControlLossResult::Recorded
            && let Ok(mut lost) = self.lost.lock()
        {
            lost.insert(admitted.game_session_id, admitted);
        }
        result
    }

    pub(super) async fn commit_control_loss(
        &self,
        session: GameSessionId,
        actor: ExactActorRef,
        controller: ControllerBinding,
    ) -> ControlLossResult {
        self.commit_loss_cause(
            session,
            actor,
            controller,
            ControlLossCauseV1::AuthoritativeUnexpectedLoss,
        )
        .await
    }
    async fn commit_loss_cause(
        &self,
        game_session_id: GameSessionId,
        actor: ExactActorRef,
        controller: ControllerBinding,
        cause: ControlLossCauseV1,
    ) -> ControlLossResult {
        let store = FreshAdmissionStore::from_root(self.root.clone());
        // LOGOUT-WIRE-1 (§1.6): a session already TERMINAL ended in a release whose
        // acknowledgement was lost; the lifecycle reconciles it (`reconcile_terminal`).
        match store.current_session(game_session_id).await {
            Ok(current) if current.session_state() == GameSessionState::Terminal => {
                return ControlLossResult::Terminal;
            }
            Ok(_) => {}
            Err(_) => return ControlLossResult::Unknown,
        }
        if !self
            .reconcile_spells_before_control_loss(actor, game_session_id)
            .await
        {
            return ControlLossResult::Unknown;
        }
        // Loss is timed on the durable owner's clock, the same clock that
        // samples the final decision time.
        let Ok((session, now)) = store.current_session_at(game_session_id).await else {
            return ControlLossResult::Unknown;
        };
        // A logout whose commit became visible during the spell reconciliation is the same
        // lost-acknowledgement release.
        if session.session_state() == GameSessionState::Terminal {
            return ControlLossResult::Terminal;
        }
        // Only this controller's still-ACTIVE, never-lost session: a replaced, released or
        // already reconnectable session is not this connection's to lose.
        if session.session_state() != GameSessionState::Active
            || session.current_transport() != Some(controller.transport)
            || session.current_runtime_scope()
                != RuntimeScopeRefV1::channel(self.world_id, self.channel_id)
        {
            return ControlLossResult::NotApplicable;
        }
        // LOGOUT-WIRE-1 (§1.6): the client never received `ACCEPTED`, so an unknown logout of
        // this still-ACTIVE session is an ordinary loss of its transport. The fence the logout
        // kept is lifted and its kill marks are released before the loss is recorded.
        self.lift_kept_logout(game_session_id, actor).await;
        // This controller may have resumed a lost session: its loss opens the next epoch
        // and retains the resumed one's history, read from the durable receipts. A resumed
        // session whose history cannot be proven is released instead (`ResumedHistory`).
        let (history, protection) = match session.current_control_loss_epoch() {
            None => (
                ControlLossHistoryV1::FreshOrigin,
                RecoveryProtectionContinuityV1 {
                    usage: RecoveryProtectionUseV1::Unused {
                        entitlement_generation: 1,
                    },
                    rearm: RecoveryProtectionRearmV1::NotRearmed {
                        generation: 1,
                        stable_control_started_at: None,
                        accepted_deadline: None,
                    },
                },
            ),
            Some(_) => match store.resumed_history(game_session_id).await {
                Ok(Some(
                    history @ ControlLossHistoryV1::Resumed {
                        protection: retained,
                        ..
                    },
                )) => (history, retained),
                _ => return ControlLossResult::ResumedHistory,
            },
        };
        // A refused loss must not strand a resumed session ACTIVE on its dead transport.
        let refused = if matches!(history, ControlLossHistoryV1::Resumed { .. }) {
            ControlLossResult::ResumedHistory
        } else {
            ControlLossResult::Refused
        };
        let next_epoch = match &history {
            ControlLossHistoryV1::FreshOrigin => Some(1),
            ControlLossHistoryV1::Resumed { budget, .. } => budget.epoch().get().checked_add(1),
        };
        let (Some(grace_deadline), Some(next_epoch)) =
            (now.checked_add(SAME_SESSION_GRACE_SECONDS), next_epoch)
        else {
            return ControlLossResult::Unknown;
        };
        let (Ok(epoch), Ok(account_presence)) = (
            ControlLossEpochRefV1::new(next_epoch),
            AccountPresenceClaimV1::new(
                &canonical_uuid(&controller.account_id),
                session.commit().character_id(),
            ),
        ) else {
            return refused;
        };
        let observation = {
            let runtime = self.runtime.lock().await;
            let Ok(facts) = runtime.player_control_facts(actor, game_session_id) else {
                return refused;
            };
            if facts.control_loss.is_some() {
                return ControlLossResult::NotApplicable;
            }
            let source_revision = runtime.binding().source_revision();
            ControlLossObservationV1 {
                source_authority: session.current_runtime_scope(),
                source_revision,
                accepted_source_revision: source_revision,
                decision_identity: epoch,
                accepted_decision_identity: epoch,
                observed_at: now,
                session,
                account_presence,
                placement_identity: facts.placement_identity,
                placement_revision: facts.placement_revision,
                actor_present: true,
                runtime_ready: true,
                cause,
                loss_epoch: epoch,
                loss_origin: now,
                original_grace_deadline: grace_deadline,
                history,
                protection,
            }
        };
        let source = std::sync::Arc::new(ChannelOwnedLossSource(observation));
        let Ok(authorization) =
            ControlLossAuthorizationV1::authorize(source.as_ref(), game_session_id, now)
        else {
            return refused;
        };
        let mut flow = ControlLossFlowV1::begin(authorization);
        let Ok(request) = flow.take_request() else {
            return refused;
        };
        let request = std::sync::Arc::new(request);
        match store
            .commit_fresh_loss(request.clone(), source.clone())
            .await
        {
            Ok(ControlLossOutcomeV1::Committed { .. }) => {}
            Ok(ControlLossOutcomeV1::Rejected) => return refused,
            // The commit may have landed with its acknowledgement lost: reconcile the exact
            // immutable operation; never re-decide with a new observation.
            Ok(ControlLossOutcomeV1::Ambiguous) | Err(_) => {
                let mut proven = None;
                for _ in 0..RECONCILE_ATTEMPTS {
                    match store.reconcile_fresh_loss(request.operation()).await {
                        Ok(FreshLossReconciliation::Committed { .. }) => {
                            proven = Some(true);
                            break;
                        }
                        Ok(FreshLossReconciliation::Absent | FreshLossReconciliation::Conflict) => {
                            proven = Some(false);
                            break;
                        }
                        Err(_) => tokio::time::sleep(RECONCILE_BACKOFF).await,
                    }
                }
                match proven {
                    Some(true) => {}
                    Some(false) => return refused,
                    None => return ControlLossResult::Unknown,
                }
            }
        }
        let mark = ControlLossMark {
            epoch: epoch.get(),
            grace_deadline,
        };
        match self
            .runtime
            .lock()
            .await
            .record_control_loss(actor, game_session_id, mark)
        {
            Ok(()) => ControlLossResult::Recorded,
            Err(_) => ControlLossResult::Unknown,
        }
    }
}

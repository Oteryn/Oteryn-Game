use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn lose_control_for_cause(
        &self,
        admitted: AdmittedSession,
        wait: Duration,
        reset: Option<&crate::durability::world_reset::RetiringWorldReset>,
    ) -> ControlLossResult {
        let Some(actor) = admitted.runtime_actor else {
            return ControlLossResult::NotApplicable;
        };
        let Some(controller) = admitted.controller else {
            return self.recorded_activity_control_loss(admitted).await;
        };
        tokio::time::sleep(wait).await;
        let current = self.activity_runtimes.current_connection(admitted);
        if current.game_session_id != admitted.game_session_id
            || current.item_fence != admitted.item_fence
        {
            return ControlLossResult::Unknown;
        }
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
            let mut states = self.states.spell_states.lock().await;
            if runtime
                .house_actor_state(actor, admitted.game_session_id)
                .is_ok()
            {
                states.detach_house(&runtime, actor, admitted.game_session_id);
            } else {
                states.detach(&runtime, actor, admitted.game_session_id);
            }
            drop(states);
            // ATTACK-0 §3: the lost client's target stops swinging; its in-fight deadline runs on.
            self.states
                .attack
                .lock()
                .await
                .clear_target(actor, admitted.game_session_id);
        }
        if result == ControlLossResult::Recorded
            && let Ok(mut lost) = self.states.lost.lock()
        {
            lost.insert(admitted.game_session_id, admitted);
        }
        result
    }

    /// A server-owned return can carry an already recorded loss to a fresh
    /// origin session. Reuse its exact durable epoch and native mark; this is
    /// not a new loss observation and never invents a live transport.
    async fn recorded_activity_control_loss(&self, admitted: AdmittedSession) -> ControlLossResult {
        let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
            return ControlLossResult::NotApplicable;
        };
        if admitted.controller.is_some() || self.activity_runtimes.account_for(admitted).is_none() {
            return ControlLossResult::NotApplicable;
        }
        let Ok((current, _)) = FreshAdmissionStore::from_root(self.root.clone())
            .current_session_at(admitted.game_session_id)
            .await
        else {
            return ControlLossResult::Unknown;
        };
        if current.session_state() == GameSessionState::Terminal {
            return ControlLossResult::Terminal;
        }
        if item_fence_of(current) != Some(fence)
            || current.session_state() != GameSessionState::Reconnectable
            || current.current_transport().is_some()
        {
            return ControlLossResult::NotApplicable;
        }
        let (Some(epoch), Some(deadline)) = (
            current.current_control_loss_epoch(),
            current.current_original_grace_deadline(),
        ) else {
            return ControlLossResult::Refused;
        };
        let runtime = self.runtime.lock().await;
        if runtime.owner_fence().is_err()
            || runtime.binding().runtime_scope() != fence.runtime_scope
            || runtime.binding().scope_generation() != fence.scope_ownership_generation
        {
            return ControlLossResult::Refused;
        }
        let Ok(facts) = runtime.player_control_facts(actor, admitted.game_session_id) else {
            return ControlLossResult::Unknown;
        };
        if facts.control_loss
            != Some(ControlLossMark {
                epoch: epoch.get(),
                grace_deadline: deadline,
            })
        {
            return ControlLossResult::Unknown;
        }
        drop(runtime);
        let Ok(mut lost) = self.states.lost.lock() else {
            return ControlLossResult::Unknown;
        };
        lost.insert(admitted.game_session_id, admitted);
        ControlLossResult::Recorded
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
        let store = FreshAdmissionStore::from_root(self.root.clone())
            .with_house_lifecycle_owner(self.holder);
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
        let owned_house = {
            let runtime = self.runtime.lock().await;
            runtime
                .house_actor_state(actor, game_session_id)
                .ok()
                .is_some_and(|house| {
                    house.scope == session.current_runtime_scope()
                        && house.generation == session.current_scope_generation()
                        && house.lease == session.current_character_lease()
                })
        };
        if session.session_state() != GameSessionState::Active
            || session.current_transport() != Some(controller.transport)
            || (!owned_house && session.current_runtime_scope() != actor.runtime_scope())
        {
            return ControlLossResult::NotApplicable;
        }
        // LOGOUT-WIRE-1 (§1.6): the client never received `ACCEPTED`, so an unknown logout of
        // this still-ACTIVE session is an ordinary loss of its transport. The fence the logout
        // kept is lifted and its kill marks are released before the loss is recorded.
        self.lift_kept_logout(game_session_id, actor).await;
        // This controller may have resumed a lost session: its loss opens the next epoch
        // and retains the resumed one's history, read from the durable receipts. A resumed
        // session with proven missing or invalid history is released (`ResumedHistory`);
        // an unreadable history stays unknown so the listener retries.
        let (history, protection) = match session.current_control_loss_epoch() {
            None if session.current_game_session_id() != session.commit().game_session_id() => {
                match store.activity_loss_origin(game_session_id).await {
                    Ok(Some(
                        history @ ControlLossHistoryV1::ActivityOrigin {
                            protection: retained,
                            ..
                        },
                    )) => {
                        // The handoff inherits the source entitlement and its
                        // original deadline. Enter/return never rearms it.
                        (history, retained)
                    }
                    Ok(_) | Err(crate::durability::DurabilityError::InvalidStoredState) => {
                        return ControlLossResult::Refused;
                    }
                    Err(_) => return ControlLossResult::Unknown,
                }
            }
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
                Ok(None) => match store.transferred_loss_history(game_session_id).await {
                    Ok(Some(
                        history @ ControlLossHistoryV1::ScopeTransfer {
                            protection: retained,
                            ..
                        },
                    )) => (history, retained),
                    Ok(_) | Err(crate::durability::DurabilityError::InvalidStoredState) => {
                        return ControlLossResult::ResumedHistory;
                    }
                    Err(_) => return ControlLossResult::Unknown,
                },
                Ok(_) | Err(crate::durability::DurabilityError::InvalidStoredState) => {
                    return ControlLossResult::ResumedHistory;
                }
                Err(_) => return ControlLossResult::Unknown,
            },
        };
        // A refused loss must not strand a resumed session ACTIVE on its dead transport.
        let refused = if matches!(history, ControlLossHistoryV1::Resumed { .. }) {
            ControlLossResult::ResumedHistory
        } else {
            ControlLossResult::Refused
        };
        let next_epoch = match &history {
            ControlLossHistoryV1::FreshOrigin | ControlLossHistoryV1::ActivityOrigin { .. } => {
                Some(1)
            }
            ControlLossHistoryV1::ScopeTransfer { entry_epoch, .. } => {
                entry_epoch.get().checked_add(1)
            }
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
            let facts = if owned_house {
                let Ok(house) = runtime.house_actor_state(actor, game_session_id) else {
                    return refused;
                };
                if house.scope != session.current_runtime_scope()
                    || house.generation != session.current_scope_generation()
                    || house.lease != session.current_character_lease()
                {
                    return refused;
                }
                runtime.house_player_control_facts(actor, game_session_id)
            } else {
                runtime.player_control_facts(actor, game_session_id)
            };
            let Ok(facts) = facts else {
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
        let recorded = {
            let mut runtime = self.runtime.lock().await;
            if owned_house {
                runtime.record_house_control_loss(actor, game_session_id, mark)
            } else {
                runtime.record_control_loss(actor, game_session_id, mark)
            }
        };
        match recorded {
            Ok(()) => ControlLossResult::Recorded,
            Err(_) => ControlLossResult::Unknown,
        }
    }
}

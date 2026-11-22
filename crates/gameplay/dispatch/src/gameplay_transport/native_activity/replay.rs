impl <'a, 'f, 's> ComposedFreshAdmission<'a, 'f, 's> {
    pub(super) async fn prune_refused_activity_attempts(
        &self,
        policy: super::activity_runtime::ActivityRefusalPrune,
    ) -> Result<(), ()> {
        let entries: Vec<_> = self
            .activity_runtimes
            .attempts
            .lock()
            .map_err(|_| ())?
            .iter()
            .filter(|(key, _)| {
                policy.includes(key.session, key.connection_generation, Some(key.command))
            })
            .map(|(key, pending)| (*key, Arc::clone(pending)))
            .collect();
        for (key, pending) in entries {
            let held = pending.lock().await;
            if held.receipt.is_some()
                || !matches!(held.outcome, Some(NativeActivityOutcome::Refused(_)))
            {
                continue;
            }
            let mut entries = self.activity_runtimes.attempts.lock().map_err(|_| ())?;
            if entries
                .get(&key)
                .is_some_and(|current| Arc::ptr_eq(current, &pending))
            {
                entries.remove(&key);
            }
        }
        self.prune_refused_activity_returns(policy).await
    }

    pub(super) async fn prune_completed_activity_entries(
        &self,
        scope: RuntimeScopeRefV1,
        generation: u64,
    ) -> Result<(), ()> {
        let entries: Vec<_> = self
            .activity_runtimes
            .attempts
            .lock()
            .map_err(|_| ())?
            .iter()
            .map(|(key, attempt)| (*key, Arc::clone(attempt)))
            .collect();
        for (key, pending) in entries {
            let attempt = pending.lock().await;
            if attempt.destination.scope != scope || attempt.destination.generation != generation {
                continue;
            }
            if attempt.outcome.is_none() {
                return Err(());
            }
            self.activity_runtimes
                .attempts
                .lock()
                .map_err(|_| ())?
                .remove(&key);
        }
        Ok(())
    }

    /// An ended socket can be cancelled after SQL commits and before the wire
    /// barrier is published. Resolve its retained physical attempt before the
    /// ordinary control-loss owner decides which current session actually ended.
    pub(super) async fn settle_native_activity_connection(
        &self,
        admitted: AdmittedSession,
    ) -> Result<AdmittedSession, ()> {
        let admitted = self.activity_runtimes.current_connection(admitted);
        let attempts: Vec<_> = self
            .activity_runtimes
            .attempts
            .lock()
            .map_err(|_| ())?
            .values()
            .cloned()
            .collect();
        for pending in attempts {
            let _transition = self.activity_runtimes.transitions.lock().await;
            let belongs = pending.lock().await.sources.iter().any(|source| {
                source.game_session_id == admitted.game_session_id
                    && source.controller == admitted.controller
                    && source.item_fence == admitted.item_fence
            });
            if !belongs {
                continue;
            }
            if let NativeActivityOutcome::Committed(commit) =
                self.finish_native_activity(pending).await?
            {
                self.publish_activity_commit(&commit)?;
                return Ok(self.activity_runtimes.current_connection(admitted));
            }
            // A prior definite refusal may belong to this same connection.
            // It cannot hide a later retained attempt whose commit is unknown;
            // HashMap iteration order is not a semantic ordering of commands.
        }
        Ok(admitted)
    }

    /// The existing autonomous owner calls this even when the initiating socket
    /// has gone. Only retained attempts are retried; no fresh identities or new
    /// semantic decisions are issued by the retry pass.
    pub(super) async fn drain_native_activity_attempts(&self) -> Result<(), ()> {
        let mut attempts: Vec<_> = self
            .activity_runtimes
            .attempts
            .lock()
            .map_err(|_| ())?
            .iter()
            .map(|(key, pending)| (*key, Arc::clone(pending)))
            .collect();
        attempts.sort_by_key(|(key, _)| {
            (
                *key.session.as_bytes(),
                key.connection_generation,
                key.command,
            )
        });
        let mut available = true;
        let start = self.background_start("activity-entries", attempts.len());
        let count = attempts.len();
        for index in (start..count).chain(0..start) {
            let pending = Arc::clone(&attempts[index].1);
            let _transition = self.activity_runtimes.transitions.lock().await;
            match self
                .background_unit(
                    "activity-entries",
                    index + 1,
                    self.finish_native_activity(pending),
                )
                .await
            {
                Ok(NativeActivityOutcome::Committed(commit)) => {
                    self.publish_activity_commit(&commit)?
                }
                Ok(NativeActivityOutcome::Refused(_)) => {}
                Err(()) => available = false,
            }
        }
        if available { Ok(()) } else { Err(()) }
    }
    /// Publish only a fully installed durable successor. The connection retains
    /// its source sequence until the generic snapshot barrier has been written.
    pub(super) fn publish_activity_commit(&self, commit: &NativeActivityCommit) -> Result<(), ()> {
        use oteryn_protocol_oteryn::activity_scope::{
            ActivityScope, ActivityScopeSnapshot, encode_activity_scope,
        };
        let mut switches = Vec::new();
        switches
            .try_reserve_exact(commit.connections.len())
            .map_err(|_| ())?;
        for &(source, destination) in &commit.connections {
            if destination.controller.is_none() {
                self.activity_runtimes
                    .remember_offline_return(source, destination)?;
                continue;
            }
            let fence = destination.item_fence.ok_or(())?;
            let scope = match fence.runtime_scope {
                RuntimeScopeRefV1::Channel {
                    world_id,
                    channel_id,
                } => ActivityScope::Channel {
                    world: *world_id.as_bytes(),
                    channel: *channel_id.as_bytes(),
                },
                RuntimeScopeRefV1::Instance {
                    world_id,
                    instance_id,
                } => ActivityScope::Instance {
                    world: *world_id.as_bytes(),
                    instance: instance_id,
                },
            };
            let payload = encode_activity_scope(&ActivityScopeSnapshot {
                requires_ack: [source, destination]
                    .iter()
                    .filter_map(|session| session.item_fence)
                    .filter_map(|fence| {
                        self.activity_runtimes
                            .get(fence.runtime_scope, fence.scope_ownership_generation.get())
                    })
                    .any(|owner| {
                        owner.template.as_ref().is_some_and(|template| {
                            template.key == crate::content::soulpit_activity::SOULPIT_TEMPLATE_KEY
                        })
                    }),
                handoff: commit.handoff_id,
                source_session: *source.game_session_id.as_bytes(),
                session: *destination.game_session_id.as_bytes(),
                scope,
                scope_generation: fence.scope_ownership_generation.get(),
                connection_generation: destination.continuity.connection_generation,
                lease_generation: fence.character_lease_generation,
                selection: destination
                    .continuity
                    .selected_capabilities
                    .as_slice()
                    .to_vec(),
            })
            .map_err(|_| ())?;
            switches.push(super::activity_runtime::ActivityConnectionSwitch {
                source,
                destination,
                payload,
                movement_effect_start: commit
                    .entry_effect_starts
                    .iter()
                    .find(|(session, _)| *session == destination.game_session_id)
                    .map(|(_, cursor)| *cursor),
            });
        }
        for switch in switches {
            self.activity_runtimes.stage_switch(switch)?;
        }
        Ok(())
    }
    /// Retry an already retained command before issuing any new server identity.
    /// The command's source connection and content selector remain bound.
    pub(super) async fn retry_native_activity(
        &self,
        admitted: AdmittedSession,
        command: u64,
        activity_key: &str,
    ) -> Result<Option<NativeActivityOutcome>, ()> {
        let key = ActivityAttemptKey {
            session: admitted.game_session_id,
            connection_generation: admitted.continuity.connection_generation,
            command,
        };
        let pending = self
            .activity_runtimes
            .attempts
            .lock()
            .map_err(|_| ())?
            .get(&key)
            .cloned();
        let Some(pending) = pending else {
            return Ok(None);
        };
        let _transition = self.activity_runtimes.transitions.lock().await;
        let attempt = pending.lock().await;
        if attempt.request.activity_key != activity_key
            || !attempt.sources.iter().any(|source| {
                source.game_session_id == admitted.game_session_id
                    && source.controller == admitted.controller
                    && source.item_fence == admitted.item_fence
            })
        {
            return Ok(Some(NativeActivityOutcome::Refused(
                ActivityRefusal::ReplayConflict,
            )));
        }
        drop(attempt);
        self.finish_native_activity(pending).await.map(Some)
    }

    pub(super) async fn retry_native_soulpit_activity(
        &self,
        admitted: AdmittedSession,
        command: u64,
        intent: &[u8],
    ) -> Result<Option<NativeActivityOutcome>, ()> {
        let key = ActivityAttemptKey {
            session: admitted.game_session_id,
            connection_generation: admitted.continuity.connection_generation,
            command,
        };
        let pending = self
            .activity_runtimes
            .attempts
            .lock()
            .map_err(|_| ())?
            .get(&key)
            .cloned();
        if let Some(pending) = pending {
            let attempt = pending.lock().await;
            if attempt
                .soulpit
                .as_ref()
                .is_none_or(|core| core.intent != intent)
            {
                return Ok(Some(NativeActivityOutcome::Refused(
                    ActivityRefusal::ReplayConflict,
                )));
            }
        }
        self.retry_native_activity(
            admitted,
            command,
            crate::durability::soulpit::SOULPIT_ACTIVITY_KEY,
        )
        .await
    }

    /// Boss wire payload remains bound to its original source command when
    /// the two literal onUseExtra callbacks are evaluated under native owners.
    pub(super) async fn enter_native_boss_activity(
        &self,
        request: ActivityEnterRequest,
        wire: [u8; 32],
    ) -> Result<NativeActivityOutcome, ()> {
        self.enter_native_activity_inner(request, true, None, Some(wire))
            .await
    }

    pub(super) async fn enter_native_soulpit(
        &self,
        request: ActivityEnterRequest,
        entry: crate::durability::soulpit::SoulpitEntryRequest,
    ) -> Result<NativeActivityOutcome, ()> {
        self.enter_native_activity_inner(request, false, Some(entry), None)
            .await
    }
    pub(super) async fn retry_native_soulpit(
        &self,
        admitted: AdmittedSession,
        command: u64,
        intent: &[u8],
    ) -> Result<Option<NativeActivityOutcome>, ()> {
        self.retry_native_soulpit_activity(admitted, command, intent)
            .await
    }
}

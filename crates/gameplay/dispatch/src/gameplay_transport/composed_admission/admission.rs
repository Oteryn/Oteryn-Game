use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn admit_fresh_session(
        &self,
        attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        use AdmissionRefusal::{Classified, Rejected, Unavailable};
        use FoundationProtocolError as N8;
        // OPS-SAVE-2: no session begins while the node quiesces for its shutdown save.
        let Some(_permit) = self.states.input_gate.enter().await else {
            return Err(Unavailable);
        };
        let malformed = Classified(N8::AdmissionGrantMalformed);
        let token = std::str::from_utf8(attempt.admission_material).map_err(|_| malformed)?;
        // Untrusted selector only; verification below checks the same token.
        let signing_key_id = fresh_grant_signing_key_id(token).ok_or(malformed)?;
        let character_id = domain::CharacterId::from_bytes(*attempt.character_id.as_bytes())
            .map_err(|_| malformed)?;
        // Every durable pass shares the single, non-blocking durability holder, so a
        // concurrent pass (another attempt, or the node's background work) fails a
        // pass at once with nothing written. Such a busy holder is retried, bounded,
        // with a per-attempt backoff so concurrent attempts do not stay in lockstep.
        let backoff = fresh_admission_backoff(&attempt.game_session_id);
        let retry = move || async move {
            tokio::time::sleep(backoff).await;
        };
        let mut read_round = 0;
        let record = loop {
            read_round += 1;
            match self
                .root
                .read_current_character(self.character, character_id)
                .await
            {
                Ok(record) => break record,
                Err(CharacterAuthorityError::Unavailable(_))
                    if fresh_round_retries(Unavailable, read_round >= FRESH_ADMISSION_ROUNDS) =>
                {
                    retry().await;
                }
                Err(CharacterAuthorityError::Unavailable(_)) => return Err(Unavailable),
                Err(_) => return Err(Rejected),
            }
        };
        // The grant is not authenticated yet: a classified answer here would reveal another
        // world's character, so only 1100/1101 may precede authentication (N8 precedence).
        if record.world_id.as_bytes() != self.world_id.as_bytes() {
            return Err(Rejected);
        }
        let subject = FreshAdmissionSubject {
            account_id: canonical_uuid(record.account_id.as_bytes()),
            character_id: attempt.character_id,
            world_id: self.world_id,
            channel_id: self.channel_id,
            signing_key_id,
        };
        // A concurrent attempt for the same account (for example the same grant on a second
        // socket) may refresh the owner evidence between this attempt's publication,
        // composition and commit, leaving it stale. Such a round is retried from fresh
        // evidence, bounded; the grant replay key still admits at most one GameSession.
        let mut round = 0;
        let (request, reservation) = loop {
            round += 1;
            let last = round >= FRESH_ADMISSION_ROUNDS;
            // D4: the five-second source-age bound requires evidence fetched for
            // this attempt; S2 custody retains it and the composition decides.
            if self
                .evidence
                .refresh_fresh_admission(
                    self.root,
                    self.holder,
                    &subject.account_id,
                    &subject.signing_key_id,
                )
                .await
                .is_err()
            {
                if last {
                    return Err(Unavailable);
                }
                retry().await;
                continue;
            }
            let now = unix_seconds().ok_or(Unavailable)?;
            match self
                .root
                .publish_fresh_admission_sources(self.character, self.holder, &subject, now)
                .await
            {
                Ok(
                    GuardPublicationDisposition::Applied | GuardPublicationDisposition::Existing,
                ) => {}
                Ok(GuardPublicationDisposition::Stale | GuardPublicationDisposition::Conflict)
                | Err(_) => {
                    if last {
                        return Err(Unavailable);
                    }
                    retry().await;
                    continue;
                }
            }
            let Ok(composition) = self
                .root
                .compose_fresh_admission(self.character, self.holder, &subject)
                .await
            else {
                if last {
                    return Err(Unavailable);
                }
                retry().await;
                continue;
            };
            let facts = verify_fresh_grant_durability_v1(
                token,
                now,
                &FreshDurabilityTrustContext::from_owning_source(&composition),
                &FreshDurabilityCurrentAuthorityV1::from_owning_source(&composition),
            )
            .map_err(fresh_grant_refusal)?;
            let authorization = FreshAdmissionCommitAuthorizationV1::new(
                &facts,
                attempt.game_session_id,
                attempt.transport,
                &composition,
                now,
            )
            .map_err(|_| Rejected)?;
            let transition =
                FreshAdmissionClaimTransitionV1::prepare(&composition, &authorization, now)
                    .map_err(|_| Rejected)?;
            // ADMIT0 §4 / FND-04A §7 step 14: only now, after grant authentication and the
            // safe ownership/world classification, may a missing requirement be revealed.
            // The binding is this composition's independently resolved observation; the
            // commit below rechecks it under the deciding transaction's locks. Refusing here
            // precedes capacity reservation and the durable commit: no nonce is consumed.
            let (scope, generation, policy) = composition.runtime_capability_binding();
            if !self
                .required_capabilities
                .matches_current(scope, generation, policy)
            {
                return Err(Rejected);
            }
            let capabilities = self
                .required_capabilities
                .authenticate_support(attempt.supported_capabilities)
                .map_err(|error| match error {
                    world_required_capabilities::SessionCapabilityError::MissingSupport(_) => {
                        Classified(N8::AdmissionCapabilityRequired)
                    }
                    _ => Unavailable,
                })?;
            // LOOP-MAP-1: initialize selection and home-spawn factors before a player is admitted.
            self.initialize_cyclopedia_respawn()
                .await
                .map_err(|_| Unavailable)?;
            let mut flow = FreshAdmissionDurabilityFlowV1::begin(authorization, transition)
                .map_err(|_| Rejected)?;
            let mut prepared = PreparedRequest(None);
            flow.submit(&mut prepared).map_err(|_| Rejected)?;
            let request = prepared.0.ok_or(Unavailable)?;
            // Capacity is reserved only after every semantic/authentication check,
            // but before the durable GameSession mutation. M+1 therefore rejects
            // without creating a playable/committed session.
            // A busy holder on the assignment read reserves nothing: retry the round.
            let reservation = match self.reserve_runtime_player(attempt.game_session_id).await {
                Ok(reservation) => reservation,
                Err(refusal) if fresh_round_retries(refusal, last) => {
                    retry().await;
                    continue;
                }
                Err(refusal) => return Err(refusal),
            };
            match self
                .root
                .commit_composed_fresh_admission(
                    self.character,
                    self.holder,
                    &composition,
                    &request,
                    capabilities.current_check(),
                )
                .await
            {
                Ok(
                    FreshAdmissionDurableOutcomeV1::Committed(_)
                    | FreshAdmissionDurableOutcomeV1::ExistingCommitted(_),
                ) => {}
                // The owner evidence moved after composition: proven noncommit.
                Ok(FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority) if !last => {
                    self.rollback_runtime_player(reservation).await?;
                    retry().await;
                    continue;
                }
                // N8: proven noncommits FND-04A names a row for.
                Ok(
                    outcome @ (FreshAdmissionDurableOutcomeV1::RejectedReplayConflict
                    | FreshAdmissionDurableOutcomeV1::RejectedIncumbent
                    | FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority),
                ) => {
                    self.rollback_runtime_player(reservation).await?;
                    return Err(Classified(match outcome {
                        FreshAdmissionDurableOutcomeV1::RejectedReplayConflict => {
                            N8::AdmissionGrantReplayed
                        }
                        FreshAdmissionDurableOutcomeV1::RejectedIncumbent => {
                            N8::AdmissionIncumbentProtected
                        }
                        _ => N8::AdmissionGrantRuntimeGenerationStale,
                    }));
                }
                // The commit may have landed with its acknowledgement lost: keep
                // the exact operation and reconcile it until the outcome is proven.
                Ok(FreshAdmissionDurableOutcomeV1::AmbiguousOrUnavailable) | Err(_) => {
                    match self
                        .reconcile(&request, attempt.game_session_id, attempt.transport)
                        .await
                    {
                        ReconciliationDisposition::Committed => {}
                        ReconciliationDisposition::DefinitelyNotCurrent(refusal) => {
                            self.rollback_runtime_player(reservation).await?;
                            // No receipt under the admission locks proves noncommit (for
                            // example a commit refused on a busy holder): retry the round;
                            // the grant replay key still admits at most one GameSession.
                            if fresh_round_retries(refusal, last) {
                                retry().await;
                                continue;
                            }
                            return Err(refusal);
                        }
                        ReconciliationDisposition::DurableNotOwned => {
                            // The GameSession is durable but no longer owned by
                            // this socket; that does not prove it terminal. Keep
                            // its slot reserved rather than free durable capacity.
                            return Err(Rejected);
                        }
                        ReconciliationDisposition::Unknown => {
                            // Fail closed. Do not fabricate actor authority and do not
                            // free a slot whose GameSession may already be durable.
                            return Err(Unavailable);
                        }
                    }
                }
                Ok(_) => {
                    self.rollback_runtime_player(reservation).await?;
                    return Err(Rejected);
                }
            }
            break (request, reservation);
        };
        let actor = self
            .runtime
            .lock()
            .await
            .commit_fresh_session(reservation)
            .map_err(|_| Unavailable)?;
        // #935: only after both COMMITs, revalidate current authority and let
        // the Channel owner write the first-entry position. A failure here
        // writes nothing and fabricates no rollback: the committed actor stays
        // unpositioned and is not input-eligible.
        // Every refusal before the Channel-owner write is a read of current authority or data
        // through the single ready-only holder, which a concurrent pass (another session's
        // release or tick) can briefly hold; the attempt wrote no runtime state, so it is
        // repeated from fresh reads, bounded, before the actor is left unpositioned.
        let mut attempt_round = 0;
        let (first_entry, item_fence, decided) = loop {
            attempt_round += 1;
            let result = self
                .initialize_first_entry(&request, attempt.game_session_id, attempt.transport, actor)
                .await;
            if !matches!(
                result.0,
                FirstEntryOutcome::RefusedUnavailable | FirstEntryOutcome::RefusedStaleAuthority
            ) || attempt_round >= RECONCILE_ATTEMPTS
            {
                break result;
            }
            tokio::time::sleep(RECONCILE_BACKOFF).await;
        };
        #[cfg(test)]
        if std::env::var_os("OTERYN_SEAM_SPELL_MANIFEST").is_some() {
            eprintln!("SEAM_EVIDENCE spell_first_entry outcome={first_entry:?}");
        }
        // PREM-1b: pull Premium before any Premium read; admission does not wait on it.
        self.admit_premium(*record.account_id.as_bytes(), attempt.game_session_id);
        self.admit_promotion(attempt.game_session_id).await;
        let mut admitted = AdmittedSession {
            game_session_id: attempt.game_session_id,
            world_id: self.world_id,
            channel_id: self.channel_id,
            runtime_actor: Some(actor),
            first_entry,
            controller: Some(ControllerBinding {
                transport: attempt.transport,
                account_id: *record.account_id.as_bytes(),
            }),
            continuity: SessionContinuity::FRESH,
            item_fence,
        };
        self.states.command_ingress.admit(admitted.game_session_id);
        // ARCH-PROGRESSION-SOURCE-0 §1.5: before the respawn settle and the quest session, so
        // before the session is input-eligible or writes anything that advances the root.
        let positioned = matches!(
            admitted.first_entry,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        );
        if positioned {
            self.admit_progression_session(&mut admitted).await;
        }
        // DEATH-2b: a pending respawn that is not placed and consumed leaves the actor
        // input-ineligible, its pending row intact for the next admission.
        if matches!(
            admitted.first_entry,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        ) {
            let settled = self
                .consume_admitted_newhaven_arrival(
                    admitted.game_session_id,
                    actor,
                    decided.as_ref(),
                )
                .await;
            admitted.first_entry = respawned_first_entry(admitted.first_entry, settled);
        }
        if writes_after_progression(positioned, admitted.first_entry) {
            self.admit_quest_session(&admitted).await;
        }
        Ok(admitted)
    }

    /// Record authoritative unexpected control loss, fresh-origin or after a resume. The owning
    /// source is composed at decision time from the Channel owner (actor present, placement,
    /// assignment revision), the current durable GameSession and the admitted account; the
    /// durable commit revalidates session, claims and runtime guard under its relation locks.
    pub(in crate::gameplay_transport) async fn reserve_runtime_player(
        &self,
        game_session_id: GameSessionId,
    ) -> Result<PlayerActorReservation, AdmissionRefusal> {
        use AdmissionRefusal::{Classified, Unavailable};
        self.reserve_precondition().await?;
        self.runtime
            .lock()
            .await
            .reserve_fresh_session(game_session_id)
            .map_err(|error| match error {
                CarrierError::CapacityExceeded => {
                    Classified(FoundationProtocolError::AdmissionCapacityExceeded)
                }
                _ => Unavailable,
            })
    }

    /// The current assignment still names this node incarnation and matches
    /// the exact committed assignment this runtime was composed from.
    pub(in crate::gameplay_transport) async fn reserve_precondition(
        &self,
    ) -> Result<(), AdmissionRefusal> {
        use AdmissionRefusal::{Rejected, Unavailable};
        let scope = RuntimeScopeRefV1::channel(self.world_id, self.channel_id);
        let assignment = self
            .root
            .read_runtime_scope_assignment(scope)
            .await
            .map_err(|_| Unavailable)?
            .ok_or(Rejected)?;
        let fact = self.holder.fact();
        let binding = self.runtime.lock().await.binding();
        if assignment.state != AssignmentState::Assigned
            || assignment.holder != Some(fact)
            || !binding.matches_committed_assignment(
                self.world_id,
                self.channel_id,
                fact.node_id(),
                fact.registration_revision(),
                assignment.ownership_generation,
                assignment.source_revision,
                &assignment.decision_identity,
            )
        {
            return Err(Rejected);
        }
        Ok(())
    }

    pub(in crate::gameplay_transport) async fn rollback_runtime_player(
        &self,
        reservation: PlayerActorReservation,
    ) -> Result<(), AdmissionRefusal> {
        self.runtime
            .lock()
            .await
            .rollback_definitely_uncommitted(reservation)
            .map_err(|_| AdmissionRefusal::Unavailable)
    }

    /// Committed proves the exact current session belongs to this socket.
    /// Only absent/conflict prove noncommit and allow reservation rollback. A
    /// committed receipt whose session this socket no longer owns is durable,
    /// so its reservation is kept. Repeated read failures remain unknown.
    pub(in crate::gameplay_transport) async fn reconcile(
        &self,
        request: &FreshAdmissionCommitRequestV1,
        game_session_id: GameSessionId,
        transport: AuthenticatedTransportRefV1,
    ) -> ReconciliationDisposition {
        let store = FreshAdmissionStore::from_root(self.root.clone());
        for attempt in 0..RECONCILE_ATTEMPTS {
            if attempt > 0 {
                tokio::time::sleep(RECONCILE_BACKOFF).await;
            }
            match store.reconcile(request.operation()).await {
                Ok(FreshReconciliation::Committed(snapshot)) => {
                    return if owns_fresh_session(
                        snapshot.current_session,
                        game_session_id,
                        transport,
                    ) {
                        ReconciliationDisposition::Committed
                    } else {
                        ReconciliationDisposition::DurableNotOwned
                    };
                }
                Ok(FreshReconciliation::Absent) => {
                    return ReconciliationDisposition::DefinitelyNotCurrent(
                        AdmissionRefusal::Unavailable,
                    );
                }
                Ok(FreshReconciliation::Conflict) => {
                    return ReconciliationDisposition::DefinitelyNotCurrent(
                        AdmissionRefusal::Rejected,
                    );
                }
                Err(_) => {}
            }
        }
        ReconciliationDisposition::Unknown
    }
}

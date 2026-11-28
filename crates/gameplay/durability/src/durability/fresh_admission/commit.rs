impl FreshAdmissionStore {
    /// Require a current stream read at PREPARE and COMMIT, independently of entry checks.
    pub fn with_security_revocation_freshness(mut self, required: bool) -> Self {
        self.security_revocation_freshness_required = required;
        self
    }

    /// One ADMIT0 predicate at both deciding owners. The caller holds the admission
    /// relation locks; read the current guard through its full mirror/history validator.
    /// Neither the prepared operation nor a callback can supply current runtime facts.
    pub async fn check_current_runtime_capabilities(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        scope: RuntimeScopeRefV1,
        generation: u64,
        check: &CurrentCapabilityCheck,
    ) -> Result<bool> {
        let runtime = self
            .guards
            .load_locked(tx, &AdmissionAuthorityGuardKeyV1::Runtime(scope))
            .await?;
        let Some(AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation,
            ready: true,
            world_policy_revision,
            ..
        }) = runtime.as_ref().map(|row| &row.state)
        else {
            return Ok(false);
        };
        Ok(*ownership_generation == generation
            && check(scope, *ownership_generation, world_policy_revision))
    }

    #[cfg(any(test, feature = "test-support"))]
    pub async fn connect_runtime(
        url: &str,
        maximum_operation_bytes: usize,
        maximum_guard_bytes: usize,
    ) -> Result<Self> {
        if maximum_operation_bytes != super::MAX_FRESH_OPERATION_BYTES
            || maximum_guard_bytes != super::MAX_ADMISSION_GUARD_BYTES
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let root = super::DurabilityRoot::connect_test_runtime(url)?;
        if !root.maintain_ready_once().await? {
            return Err(DurabilityError::RootUnavailable);
        }
        Ok(Self::from_root(root))
    }

    /// CHAR-POSITION-0 §3.2 "At logout": this store with `final_position`, which its terminal
    /// release (`release_expired_loss`, `release_abandoned_session`,
    /// `release_capability_mismatch`, `release`) upserts inside the release transaction, before
    /// the session becomes TERMINAL, under the released session's Character lease key. A
    /// generic non-Channel scope, a World other than the record's, a pending respawn or a
    /// later stored key writes nothing; the release itself is unchanged by it. Named House
    /// captures use the separately proven [`Self::with_final_house_position`] path.
    #[must_use]
    pub fn with_final_position(
        &self,
        final_position: super::character_last_position::FinalLastPosition,
    ) -> Self {
        Self {
            final_position: Some(final_position),
            pending_vocation_refill: None,
            final_house: None,
            house_release_node: None,
            ..self.clone()
        }
    }

    /// Save the current cell and rest state while the live vocation refill is pending.
    #[must_use]
    pub fn with_final_position_pending_refill(
        &self,
        final_position: super::character_last_position::FinalLastPosition,
        occurrence: super::character_build::BuildOccurrence,
    ) -> Self {
        Self {
            pending_vocation_refill: Some(occurrence),
            ..self.with_final_position(final_position)
        }
    }

    /// Q16: final named-House capture in the existing terminal release transaction.
    #[must_use]
    pub fn with_final_house_position(
        &self,
        position: super::character_last_position::FinalLastPosition,
        house: super::character_last_position::HouseLastPosition,
        node: &super::runtime_scope_assignment::NodeIncarnationProof,
    ) -> Self {
        Self {
            final_position: Some(position),
            pending_vocation_refill: None,
            final_house: Some((house, node.clone())),
            house_release_node: Some(node.clone()),
            ..self.clone()
        }
    }

    /// The current named-House carrier owner retains its node proof for native
    /// entry-reservation cleanup at terminal release, including a dead actor.
    #[must_use]
    pub fn with_house_release_owner(
        &self,
        node: &super::runtime_scope_assignment::NodeIncarnationProof,
    ) -> Self {
        Self {
            house_release_node: Some(node.clone()),
            ..self.clone()
        }
    }

    /// Complete admitted-World root catalogue for the terminal journal capture.
    #[must_use]
    pub fn with_journal_catalogue(&self, catalogue: super::journal::LogoutCatalogue) -> Self {
        Self {
            journal_catalogue: Some(std::sync::Arc::new(catalogue)),
            ..self.clone()
        }
    }

    /// chsw: the terminal release of a voluntary Channel switch keeps the Party membership.
    #[must_use]
    pub fn with_channel_switch(&self) -> Self {
        Self {
            channel_switch: true,
            ..self.clone()
        }
    }

    /// OFFLINE0 participant: the final build marker commits with the terminal release.
    #[must_use]
    pub fn with_training_logout(
        &self,
        training: impl TerminalReleaseParticipant + 'static,
    ) -> Self {
        Self {
            training_logout: Some(std::sync::Arc::new(training)),
            ..self.clone()
        }
    }

    pub(crate) fn with_offline_activity_mismatch(
        &self,
        proof: super::activity_handoff::OfflineActivityMismatchRelease,
    ) -> Self {
        Self {
            offline_activity_mismatch: Some(std::sync::Arc::new(proof)),
            ..self.clone()
        }
    }

    /// A departed activity's sealed offline return uses this same terminal
    /// writer. The proof is repeated in the deciding transaction; it cannot
    /// terminalize a current native successor or a newly adopted transport.
    pub(crate) fn with_departed_activity_release(
        &self,
        proof: super::activity_handoff::DepartedActivityRelease,
    ) -> Self {
        Self {
            departed_activity: Some(std::sync::Arc::new(proof)),
            ..self.clone()
        }
    }

    #[must_use]
    pub fn from_root(root: super::DurabilityRoot) -> Self {
        Self {
            guards: AdmissionGuardStore::from_root(root),
            maximum_operation_bytes: super::MAX_FRESH_OPERATION_BYTES,
            training_logout: None,
            explicit_logout: false,
            journal_catalogue: None,
            channel_switch: false,
            final_position: None,
            pending_vocation_refill: None,
            final_house: None,
            house_release_node: None,
            retired_holder: None,
            departed_activity: None,
            staff_loss: None,
            security_revocation_freshness_required: false,
            offline_activity_mismatch: None,
        }
    }

    async fn receipt_locked(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        replay: &[u8],
    ) -> Result<Option<FreshAdmissionCommitReceiptV1>> {
        use sqlx::Row;
        let row = sqlx::query("SELECT CASE WHEN octet_length(operation_json) <= $2 AND octet_length(to_jsonb(r)::text) <= $3 THEN operation_json END AS payload, CASE WHEN octet_length(to_jsonb(r)::text) <= $3 THEN to_jsonb(r) - 'operation_json' END AS mirrors FROM game_durability_fresh_admission_receipts r WHERE replay_key = $1")
            .bind(replay).bind(checked(i64::try_from(self.maximum_operation_bytes))?).bind(super::MAX_ADMISSION_ROW_BYTES).fetch_optional(&mut **tx).await?;
        let Some(row) = row else {
            return Ok(None);
        };
        let payload: Option<String> = row.try_get("payload")?;
        let operation = decode_operation(
            &payload.ok_or(DurabilityError::InvalidStoredState)?,
            self.maximum_operation_bytes,
        )?;
        let mirrors: serde_json::Value = row.try_get("mirrors")?;
        if operation.authorization.facts.replay_key().to_bytes().as_slice() != replay {
            return Err(DurabilityError::InvalidStoredState);
        }
        Self::restore_receipt(operation, mirrors).map(Some)
    }

    fn restore_receipt(
        operation: FreshAdmissionOperationV1,
        mirrors: serde_json::Value,
    ) -> Result<FreshAdmissionCommitReceiptV1> {
        let decided_at = mirrors
            .get("authorization_decided_at")
            .and_then(serde_json::Value::as_i64)
            .ok_or(DurabilityError::InvalidStoredState)?;
        let b = &operation.authorization;
        let initial = checked(b.initial_commit())?;
        let expected = serde_json::json!({
            "replay_key": bytea_text(&b.facts.replay_key().to_bytes()),
            "game_session_id": uuid_text(b.candidate_session.as_bytes()),
            "account_id": b.account_id,
            "character_id": uuid_text(initial.character_id().as_bytes()),
            "world_id": uuid_text(initial.world_id().as_bytes()),
            "channel_id": uuid_text(initial.channel_id().as_bytes()),
            "character_lease_generation": initial.character_lease_generation(),
            "scope_ownership_generation": initial.scope_ownership_generation(),
            "connection_generation": 1,
            "transport_ref": bytea_text(&b.transport.to_bytes()),
            "semantic_version": 1,
            "authorization_decided_at": decided_at,
        });
        if mirrors != expected {
            return Err(DurabilityError::InvalidStoredState);
        }
        checked(FreshAdmissionCommitReceiptV1::restore(operation, decided_at))
    }

    /// Fixture-only raw write. Production enters through the composed owner,
    /// which requires current capability revalidation in the deciding transaction.
    #[cfg(any(test, feature = "test-support"))]
    pub async fn commit(
        &self,
        request: &FreshAdmissionCommitRequestV1,
    ) -> Result<FreshAdmissionDurableOutcomeV1> {
        self.commit_revalidated(request, None).await
    }

    /// Commit with an optional owner revalidation that runs inside the commit
    /// transaction, after relation locks and exact-replay reconciliation. A
    /// `false` result rejects the attempt as stale authority.
    pub async fn commit_revalidated(
        &self,
        request: &FreshAdmissionCommitRequestV1,
        revalidate: Option<OwnerRevalidation>,
    ) -> Result<FreshAdmissionDurableOutcomeV1> {
        let issued = self.guards.backend.try_issue_root()?;
        self.commit_revalidated_with_pass(request, revalidate, issued).await
    }

    pub(in crate::durability) async fn commit_revalidated_with_pass(
        &self,
        request: &FreshAdmissionCommitRequestV1,
        revalidate: Option<OwnerRevalidation>,
        issued: Option<super::db::IssuedSemanticPass>,
    ) -> Result<FreshAdmissionDurableOutcomeV1> {
        let store = (*self).clone();
        let request = (*request).clone();
        let backend = store.guards.backend.clone();
        backend
            .run_pass(issued, |holder, deadline| Box::pin(async move {
        let operation = request.operation();
        let b = &operation.authorization;
        let encoded = encode_operation(operation, store.maximum_operation_bytes)?;
        let encoded_successors: Vec<_> = operation
            .transition
            .successors
            .iter()
            .map(|change| encode_guard(change, store.guards.maximum_guard_bytes))
            .collect::<Result<_>>()?;
        let replay = b.facts.replay_key().to_bytes();
        let mut tx = super::admission_journal::begin_pass_transaction(holder, deadline).await?;
        lock_admission_portfolio(&mut tx, &b.account_id).await?;
        if let Some(receipt) = store.receipt_locked(&mut tx, &replay).await? {
            let outcome = receipt.classify_retry(operation);
            super::admission_journal::commit_pass_transaction(tx, deadline).await?;
            return Ok(outcome);
        }
        let service_blocked: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle IN(3,4))")
            .bind(checked(b.initial_commit())?.character_id().as_bytes().as_slice()).fetch_one(&mut *tx).await?;
        if service_blocked { return Ok(FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority); }
        if let Some(revalidate) = revalidate
            && !revalidate(&mut tx).await?
        {
            return Ok(FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority);
        }
        let initial = checked(b.initial_commit())?;
        let use_state = session_use_state(&mut tx, initial.character_id()).await?;
        if !use_state.complete { return Ok(FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority); }
        let used: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_session_use_memberships WHERE game_session_id = encode($1,'hex')::uuid)")
            .bind(b.candidate_session.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
        if used { return Ok(FreshAdmissionDurableOutcomeV1::RejectedCollision(FreshAdmissionCollisionV1::CandidateSession)); }
        if use_state.revision == GAME_SESSION_USE_LEDGER_CAPACITY_V1 { return Err(DurabilityError::AdmissionGameSessionLedgerExhausted); }
        let candidate_exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id = encode($1, 'hex')::uuid)").bind(b.candidate_session.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
        if candidate_exists {
            return Ok(FreshAdmissionDurableOutcomeV1::RejectedCollision(
                FreshAdmissionCollisionV1::CandidateSession,
            ));
        }
        let transport_exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_transport_ref_reservations WHERE transport_ref = $1)").bind(b.transport.to_bytes().as_slice()).fetch_one(&mut *tx).await?;
        if transport_exists {
            return Ok(FreshAdmissionDurableOutcomeV1::RejectedCollision(
                FreshAdmissionCollisionV1::TransportReference,
            ));
        }
        let incumbent: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions WHERE session_state IN (1,2) AND (account_id = $1::text::uuid OR character_id = encode($2, 'hex')::uuid))").bind(&b.account_id).bind(initial.character_id().as_bytes().as_slice()).fetch_one(&mut *tx).await?;
        if incumbent {
            return Ok(FreshAdmissionDurableOutcomeV1::RejectedIncumbent);
        }
        let mut current = Vec::with_capacity(b.expected_guards.len());
        for expected in &b.expected_guards {
            current.push(store.guards.load_locked(&mut tx, &expected.key).await?);
        }
        let previous: Vec<_> = operation
            .transition
            .successors
            .iter()
            .map(|change| {
                current
                    .iter()
                    .flatten()
                    .find(|row| row.key == change.key)
                    .cloned()
            })
            .collect();
        if !store
            .guards
            .successor_history_available(&mut tx, &operation.transition.successors, &previous)
            .await?
        {
            return Ok(FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority);
        }
        let minimum: Option<i64> = sqlx::query_scalar("SELECT native_security_generation FROM game_security_revocation_floors WHERE account_id=$1::text::uuid")
            .bind(&b.account_id).fetch_optional(&mut *tx).await?;
        if minimum.is_some_and(|minimum| u128::from(b.signed_security_generation) < minimum as u128) {
            return Ok(FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority);
        }
        // All relation protection and conflict/history observations precede L.
        // No SQL-generated source revision, decision or source timestamp exists.
        let decided_at: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut *tx)
                .await?;
        let Ok(successors) = request.validate_at_decision(&current, Some(decided_at)) else {
            return Ok(FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority);
        };
        commit_session_use(&mut tx, initial.character_id(), b.candidate_session, b.transport.to_bytes(), use_state.revision).await?;
        let receipt_insert = sqlx::query("INSERT INTO game_durability_fresh_admission_receipts (replay_key, game_session_id, account_id, character_id, world_id, channel_id, character_lease_generation, scope_ownership_generation, connection_generation, transport_ref, semantic_version, operation_json, authorization_decided_at) VALUES ($1, encode($2,'hex')::uuid, $3::text::uuid, encode($4,'hex')::uuid, encode($5,'hex')::uuid, encode($6,'hex')::uuid, $7::text::numeric(20,0), $8::text::numeric(20,0), 1, $9, 1, $10, $11)")
            .bind(replay.as_slice()).bind(b.candidate_session.as_bytes().as_slice()).bind(&b.account_id).bind(initial.character_id().as_bytes().as_slice()).bind(initial.world_id().as_bytes().as_slice()).bind(initial.channel_id().as_bytes().as_slice()).bind(initial.character_lease_generation().to_string()).bind(initial.scope_ownership_generation().to_string()).bind(b.transport.to_bytes().as_slice()).bind(&encoded).bind(decided_at).execute(&mut *tx).await;
        if let Err(error) = receipt_insert {
            if let Some(status) = channel_switch_refusal(&error) {
                return Ok(FreshAdmissionDurableOutcomeV1::RejectedChannelSwitch(status));
            }
            return Err(error.into());
        }
        sqlx::query("INSERT INTO game_durability_reconnect_sessions (game_session_id, account_id, character_id, world_id, runtime_scope_kind, runtime_scope_world_id, runtime_scope_channel_id, character_lease_generation, scope_ownership_generation, current_generation, current_transport_ref, session_state, fresh_replay_key, admission_security_generation) VALUES (encode($1,'hex')::uuid, $2::text::uuid, encode($3,'hex')::uuid, encode($4,'hex')::uuid, 1, encode($4,'hex')::uuid, encode($5,'hex')::uuid, $6::text::numeric(20,0), $7::text::numeric(20,0), 1, $8, 2, $9, $10::text::numeric)")
            .bind(b.candidate_session.as_bytes().as_slice()).bind(&b.account_id).bind(initial.character_id().as_bytes().as_slice()).bind(initial.world_id().as_bytes().as_slice()).bind(initial.channel_id().as_bytes().as_slice()).bind(initial.character_lease_generation().to_string()).bind(initial.scope_ownership_generation().to_string()).bind(b.transport.to_bytes().as_slice()).bind(replay.as_slice()).bind(b.signed_security_generation.to_string()).execute(&mut *tx).await?;
        store.guards
            .persist_locked(&mut tx, successors, &previous, &encoded_successors)
            .await?;
        sqlx::query("INSERT INTO game_durability_transport_ref_reservations (transport_ref, game_session_id, reconnect_attempt_ref, reservation_owner, fresh_replay_key) VALUES ($1, encode($2,'hex')::uuid, NULL, 2, $3)").bind(b.transport.to_bytes().as_slice()).bind(b.candidate_session.as_bytes().as_slice()).bind(replay.as_slice()).execute(&mut *tx).await?;
        let receipt = checked(FreshAdmissionCommitReceiptV1::restore(
            operation.clone(),
            decided_at,
        ))?;
        // An acknowledgement error is uncertain; caller reconciles this original
        // operation rather than assuming rollback or manufacturing another key.
        if super::admission_journal::commit_pass_transaction(tx, deadline).await.is_err() {
            return Ok(FreshAdmissionDurableOutcomeV1::AmbiguousOrUnavailable);
        }
        Ok(FreshAdmissionDurableOutcomeV1::Committed(receipt))

            }))
            .await
    }
}

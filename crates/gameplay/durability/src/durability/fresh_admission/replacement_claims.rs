pub(super) struct PreparedReplacementClaims {
    rows: Vec<Option<AdmissionAuthorityPublicationChangeV1>>,
    encoded_successors: Vec<String>,
    encoded: String,
    pub(super) decided_at: i64,
}
impl FreshAdmissionStore {
    pub(super) fn from_journal(journal: &super::AdmissionReconnectJournal) -> Self {
        Self {
            guards: AdmissionGuardStore {
                backend: journal.backend.clone(),
                maximum_guard_bytes: super::MAX_ADMISSION_GUARD_BYTES,
            },
            maximum_operation_bytes: super::MAX_FRESH_OPERATION_BYTES,
            training_logout: None,
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
    pub(super) async fn replacement_claim_replay(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        transition: &TerminalReplacementClaimTransitionV1,
    ) -> Result<Option<i64>> {
        lifecycle_replay(
            tx,
            transition.evidence(),
            &encode_lifecycle(transition.evidence())?,
        )
        .await
    }
    pub(super) async fn prepare_replacement_claims(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        transition: &TerminalReplacementClaimTransitionV1,
        request: GameSessionUseRequestV1,
        candidate: &ReconnectDurabilityRecordV1,
    ) -> Result<PreparedReplacementClaims> {
        let evidence = transition.evidence();
        let current = self
            .current_session_locked(
                tx,
                evidence
                    .operation
                    .current_session()
                    .current_game_session_id(),
            )
            .await?;
        if request.character_id() != candidate.identity().character_id()
            || request.candidate() != candidate.identity().game_session_id()
            || request.expected_current() != Some(current.current_game_session_id())
            || request.operation_binding() != candidate.connection().transport_ref().to_bytes()
        {
            return Err(DurabilityError::Unavailable);
        }
        let source = load_session_use_source(tx, request).await?;
        match GameSessionUseAuthorityV1::from_owning_source(&source)
            .authorize_terminal_replacement(request)
            .map_err(DurabilityError::SessionUseAuthorization)?
        {
            GameSessionUseDecisionV1::NewSession { .. } => {}
            GameSessionUseDecisionV1::ExactCommittedReplay { .. } => {
                return Err(DurabilityError::InvalidStoredState);
            }
        }
        let mut rows = Vec::with_capacity(2);
        for prior in &evidence.transition.predecessors {
            rows.push(self.guards.load_locked(tx, &prior.key).await?);
        }
        if !self
            .guards
            .successor_history_available(tx, &evidence.transition.successors, &rows)
            .await?
        {
            return Err(DurabilityError::Unavailable);
        }
        let encoded_successors = evidence
            .transition
            .successors
            .iter()
            .map(|row| encode_guard(row, self.guards.maximum_guard_bytes))
            .collect::<Result<Vec<_>>>()?;
        let encoded = encode_lifecycle(evidence)?;
        let decided_at =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut **tx)
                .await?;
        transition
            .validate_locked(&rows, current, candidate, decided_at)
            .map_err(|_| DurabilityError::Unavailable)?;
        Ok(PreparedReplacementClaims {
            rows,
            encoded_successors,
            encoded,
            decided_at,
        })
    }
    pub(super) async fn persist_replacement_claims(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        transition: &TerminalReplacementClaimTransitionV1,
        prepared: &PreparedReplacementClaims,
    ) -> Result<()> {
        let evidence = transition.evidence();
        self.guards
            .persist_locked(
                tx,
                &evidence.transition.successors,
                &prepared.rows,
                &prepared.encoded_successors,
            )
            .await?;
        sqlx::query("INSERT INTO game_durability_admission_lifecycle_receipts (operation_key,operation_json,decided_at) VALUES ($1,$2,$3)")
            .bind(lifecycle_key(evidence)).bind(&prepared.encoded).bind(prepared.decided_at).execute(&mut **tx).await?;
        Ok(())
    }
}

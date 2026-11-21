impl FreshAdmissionStore {
    /// The complete retained recovery budget of one loss epoch, reconstructed from the
    /// immutable prepare/commit receipts and candidate transport reservations under the
    /// caller's relation locks. A prepared attempt no longer named by the session was
    /// aborted or superseded (`Terminal`).
    async fn recovery_budget_locked(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        session: GameSessionId,
        epoch: ControlLossEpochRefV1,
    ) -> Result<RetainedRecoveryBudgetV1> {
        use sqlx::Row;
        let prefix = complete_reconnect_prefix(COMPLETE_PREPARE_KEY, session, epoch.get());
        let prepared: Option<Vec<u8>> = sqlx::query_scalar("SELECT prepared_attempt_ref FROM game_durability_reconnect_sessions WHERE game_session_id = encode($1,'hex')::uuid")
            .bind(session.as_bytes().as_slice()).fetch_optional(&mut **tx).await?
            .ok_or(DurabilityError::InvalidStoredState)?;
        let rows = sqlx::query("SELECT substring(r.operation_key from $2) AS attempt, t.transport_ref, EXISTS (SELECT 1 FROM game_durability_admission_lifecycle_receipts c WHERE c.operation_key = $3 || substring(r.operation_key from $2)) AS committed FROM game_durability_admission_lifecycle_receipts r JOIN game_durability_transport_ref_reservations t ON t.game_session_id = encode($4,'hex')::uuid AND t.reservation_owner = 1 AND t.reconnect_attempt_ref = substring(r.operation_key from $2) WHERE r.operation_key > $1 AND r.operation_key < $1 || '\\xffffffffffffffffff'::bytea AND octet_length(r.operation_key) = $5 + 8 ORDER BY r.decided_at, r.operation_key")
            .bind(&prefix)
            .bind(i32::try_from(prefix.len() + 1).map_err(|_| DurabilityError::InvalidStoredState)?)
            .bind(complete_reconnect_prefix(COMPLETE_COMMIT_KEY, session, epoch.get()))
            .bind(session.as_bytes().as_slice())
            .bind(i32::try_from(prefix.len()).map_err(|_| DurabilityError::InvalidStoredState)?)
            .fetch_all(&mut **tx).await?;
        let mut entries = Vec::with_capacity(rows.len());
        let mut restored = false;
        for row in rows {
            let attempt: Vec<u8> = row.try_get("attempt")?;
            let transport: Vec<u8> = row.try_get("transport_ref")?;
            let committed: bool = row.try_get("committed")?;
            let attempt_bytes: [u8; 8] = attempt
                .as_slice()
                .try_into()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            restored |= committed;
            entries.push(RetainedRecoveryAttemptV1 {
                attempt: checked(ReconnectAttemptRef::new(u64::from_be_bytes(attempt_bytes)))?,
                transport: checked(AuthenticatedTransportRefV1::decode(&transport))?,
                disposition: if committed {
                    RetainedRecoveryAttemptDispositionV1::Committed
                } else if prepared.as_deref() == Some(attempt.as_slice()) {
                    RetainedRecoveryAttemptDispositionV1::Prepared
                } else {
                    RetainedRecoveryAttemptDispositionV1::Terminal
                },
            });
        }
        let state = if restored {
            RecoveryEpochStateV1::Restored
        } else {
            RecoveryEpochStateV1::Open
        };
        checked(RetainedRecoveryBudgetV1::restore(
            epoch, state, true, entries,
        ))
    }

    /// The current retained recovery budget of one loss epoch, for the owning source.
    pub async fn recovery_budget(
        &self,
        session: GameSessionId,
        epoch: ControlLossEpochRefV1,
    ) -> Result<RetainedRecoveryBudgetV1> {
        let store = self.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let budget = store
                        .recovery_budget_locked(&mut tx, session, epoch)
                        .await?;
                    super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                    Ok(budget)
                })
            })
            .await
    }

    /// FND-04B same-session reauthenticated recovery PREPARE or COMMIT of one owning-loss
    /// session (§§11–13, §20). Under the admission relation locks the adapter binds the
    /// operation to the exact durable original loss, the exact current session, current
    /// claim ownership, the ready runtime guard and the reconstructed retained budget, then
    /// samples the single decision time and lets the sealed request validate its source.
    /// PREPARE reserves the candidate transport and names the attempt on the session;
    /// COMMIT consumes the RecoveryGrantNonce and switches the session to the candidate
    /// connection atomically. Receipts are immutable and replay exactly; a lost
    /// acknowledgement is reconciled, never decided twice.
    ///
    /// The recovery credential is revalidated against the registered Recovery V2 floors
    /// fenced in the same transaction (custody, registration, account and trust floors
    /// after the admission relation locks), never against a caller-held observation.
    pub async fn apply_registered_complete_reconnect(
        &self,
        custody: &super::runtime_scope_assignment::NodeIncarnationProof,
        request: std::sync::Arc<CompleteReconnectRequestV1>,
        source: std::sync::Arc<dyn CompleteReconnectSourceV1 + Send + Sync>,
        capabilities: CurrentCapabilityCheck,
    ) -> Result<CompleteReconnectOutcomeV1> {
        self.with_house_lifecycle_owner(custody)
            .apply_complete_reconnect_with(
                Some(custody.clone()),
                request,
                source,
                Some(capabilities),
            )
            .await
    }

    /// Test seam: the same decision against the source's own recovery evidence.
    #[cfg(any(test, feature = "test-support"))]
    pub async fn apply_complete_reconnect(
        &self,
        request: std::sync::Arc<CompleteReconnectRequestV1>,
        source: std::sync::Arc<dyn CompleteReconnectSourceV1 + Send + Sync>,
    ) -> Result<CompleteReconnectOutcomeV1> {
        self.apply_complete_reconnect_with(None, request, source, None)
            .await
    }

    /// Independent signed-fixture qualification of the same locked ADMIT0 gate.
    /// Production always uses registered custody and the mandatory callback above.
    #[cfg(any(test, feature = "test-support"))]
    #[allow(
        dead_code,
        reason = "the focused PostgreSQL target drives this test-only seam"
    )]
    pub async fn apply_complete_reconnect_with_capabilities(
        &self,
        request: std::sync::Arc<CompleteReconnectRequestV1>,
        source: std::sync::Arc<dyn CompleteReconnectSourceV1 + Send + Sync>,
        capabilities: CurrentCapabilityCheck,
    ) -> Result<CompleteReconnectOutcomeV1> {
        self.apply_complete_reconnect_with(None, request, source, Some(capabilities))
            .await
    }
}

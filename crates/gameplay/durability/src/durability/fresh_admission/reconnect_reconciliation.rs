impl FreshAdmissionStore {
    async fn complete_prepare_exists(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        identity: &ReconnectIdentityV1,
        epoch: ControlLossEpochRefV1,
        encoded: &str,
    ) -> Result<bool> {
        let key = complete_reconnect_key(COMPLETE_PREPARE_KEY, identity, epoch.get());
        let stored: Option<Option<String>> = sqlx::query_scalar("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END FROM game_durability_admission_lifecycle_receipts r WHERE operation_key = $1 FOR SHARE")
            .bind(&key).fetch_optional(&mut **tx).await?;
        Ok(stored.flatten().as_deref() == Some(encoded))
    }

    /// The durable original loss of `session` at `epoch` and its decision time, decoded
    /// against the session's canonical fresh receipt. `None` when no owning loss exists.
    pub async fn owning_loss(
        &self,
        session: GameSessionId,
        epoch: ControlLossEpochRefV1,
    ) -> Result<Option<(ControlLossOperationV1, i64)>> {
        let store = self.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let loss = store.owning_loss_locked(&mut tx, session, epoch).await?;
                    super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                    Ok(loss)
                })
            })
            .await
    }

    async fn owning_loss_locked(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        session: GameSessionId,
        epoch: ControlLossEpochRefV1,
    ) -> Result<Option<(ControlLossOperationV1, i64)>> {
        use sqlx::Row;
        let mut key = b"owning-loss-v1".to_vec();
        key.extend_from_slice(session.as_bytes());
        key.extend_from_slice(&epoch.get().to_be_bytes());
        let Some(row) = profile_query(sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json, decided_at FROM game_durability_admission_lifecycle_receipts r WHERE operation_key = $1 FOR SHARE")
            .bind(&key).fetch_optional(&mut **tx)).await? else {
            return Ok(None);
        };
        let stored: Option<String> = row.try_get("operation_json")?;
        let decided_at: i64 = row.try_get("decided_at")?;
        let fresh = self
            .session_fresh_operation_locked(tx, session)
            .await?
            .ok_or(DurabilityError::InvalidStoredState)?;
        let loss = decode_fresh_loss(
            &stored.ok_or(DurabilityError::InvalidStoredState)?,
            checked(fresh.authorization.initial_commit())?,
        )?;
        if loss.observation.session.current_game_session_id() != session
            || loss.observation.loss_epoch != epoch
            || decided_at < loss.authorized_at
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(Some((loss, decided_at)))
    }

    /// The retained history of a resumed session: the restored budget of its current
    /// epoch, that epoch's original grace deadline and the protection its committed
    /// recovery left. `None` unless the session is ACTIVE on the committed recovery's
    /// transport. The owning source reads it to observe a loss after the resume.
    pub async fn resumed_history(
        &self,
        session: GameSessionId,
    ) -> Result<Option<ControlLossHistoryV1>> {
        let store = self.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let current = store.current_session_locked(&mut tx, session).await?;
                    let history = store.resumed_history_locked(&mut tx, current).await?;
                    super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                    Ok(history)
                })
            })
            .await
    }

    pub(super) async fn resumed_history_locked(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    ) -> Result<Option<ControlLossHistoryV1>> {
        let (GameSessionState::Active, Some(epoch), Some(grace), Some(transport)) = (
            current.session_state(),
            current.current_control_loss_epoch(),
            current.current_original_grace_deadline(),
            current.current_transport(),
        ) else {
            return Ok(None);
        };
        let session = current.current_game_session_id();
        let budget = self.recovery_budget_locked(tx, session, epoch).await?;
        let Some(committed) = budget.entries().iter().find(|entry| {
            entry.disposition == RetainedRecoveryAttemptDispositionV1::Committed
                && entry.transport == transport
        }) else {
            return Ok(None);
        };
        let mut key = complete_reconnect_prefix(COMPLETE_COMMIT_KEY, session, epoch.get());
        key.extend_from_slice(&committed.attempt.to_be_bytes());
        let committed_at: i64 = profile_query(sqlx::query_scalar("SELECT decided_at FROM game_durability_admission_lifecycle_receipts WHERE operation_key = $1 FOR SHARE")
            .bind(&key).fetch_optional(&mut **tx)).await?
            .ok_or(DurabilityError::InvalidStoredState)?;
        let (loss, _) = self
            .owning_loss_locked(tx, session, epoch)
            .await?
            .ok_or(DurabilityError::InvalidStoredState)?;
        if budget.state() != RecoveryEpochStateV1::Restored
            || loss.observation.original_grace_deadline != grace
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(Some(ControlLossHistoryV1::Resumed {
            budget,
            original_grace_deadline: grace,
            protection: checked(
                loss.observation
                    .protection
                    .after_complete_reconnect(committed_at),
            )?,
        }))
    }

    /// Withdraw a PREPARED same-session attempt whose COMMIT was refused or could not be
    /// proven, so a later attempt may prepare. The attempt stays in the retained budget as
    /// `Terminal`; its immutable PREPARE receipt and transport reservation remain. A
    /// committed attempt is never withdrawn (the session is no longer RECONNECTABLE).
    pub async fn abort_complete_reconnect(&self, identity: &ReconnectIdentityV1) -> Result<bool> {
        let session = identity.game_session_id();
        let attempt = identity.reconnect_attempt_ref().to_be_bytes();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let cleared = sqlx::query("UPDATE game_durability_reconnect_sessions SET prepared_attempt_ref = NULL WHERE game_session_id = encode($1,'hex')::uuid AND session_state = 1 AND prepared_attempt_ref = $2")
                        .bind(session.as_bytes().as_slice()).bind(attempt.as_slice())
                        .execute(&mut *tx).await?;
                    super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                    Ok(cleared.rows_affected() == 1)
                })
            })
            .await
    }

    /// Recover the outcome of one complete-reconnect operation from its immutable
    /// receipts: `Committed`, else `Prepared`, else absent. Receipts naming a different
    /// operation for the same attempt are a stored-state conflict.
    pub async fn reconcile_complete_reconnect(
        &self,
        operation: &CompleteReconnectDurabilityOperationV1,
    ) -> Result<Option<CompleteReconnectCompletionV1>> {
        let encoded = encode_complete_reconnect(operation)?;
        let operation = operation.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    use sqlx::Row;
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let epoch = operation.recovery.original.loss.observation.loss_epoch.get();
                    let mut outcome = None;
                    for (phase, commit) in [(COMPLETE_COMMIT_KEY, true), (COMPLETE_PREPARE_KEY, false)] {
                        let key = complete_reconnect_key(phase, &operation.recovery.identity, epoch);
                        let Some(row) = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json, decided_at FROM game_durability_admission_lifecycle_receipts r WHERE operation_key = $1 FOR SHARE")
                            .bind(&key).fetch_optional(&mut *tx).await? else { continue };
                        let stored: Option<String> = row.try_get("operation_json")?;
                        if stored.as_deref() != Some(encoded.as_str()) {
                            return Err(DurabilityError::InvalidStoredState);
                        }
                        let decided_at: i64 = row.try_get("decided_at")?;
                        if outcome.is_none() {
                            outcome = Some(if commit {
                                CompleteReconnectOutcomeV1::Committed { decided_at }
                            } else {
                                CompleteReconnectOutcomeV1::Prepared { decided_at }
                            });
                        }
                    }
                    super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                    Ok(outcome.map(|outcome| CompleteReconnectCompletionV1 { operation, outcome }))
                })
            })
            .await
    }
}

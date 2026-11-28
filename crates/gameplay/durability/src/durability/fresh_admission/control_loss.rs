impl FreshAdmissionStore {
    /// Record an owning loss: a fresh-origin loss, or a loss after a same-session resume
    /// (the next epoch, retaining the resumed one). Unsupported continuity shapes remain
    /// unavailable; this API never derives loss authority from reconnect PREPARE.
    pub async fn commit_fresh_loss(
        &self,
        request: std::sync::Arc<ControlLossRequestV1>,
        source: std::sync::Arc<dyn ControlLossSourceV1 + Send + Sync>,
    ) -> Result<ControlLossOutcomeV1> {
        let store = (*self).clone();

        let issued = store.guards.backend.try_issue_root()?;
        let backend = store.guards.backend.clone();
        backend
            .run_pass(issued, |holder, deadline| Box::pin(async move {
        use sqlx::Row;
        let operation = request.operation();
        let encoded = encode_fresh_loss(operation)?;
        let observation = &operation.observation;
        let session_id = observation.session.current_game_session_id();
        let mut key = b"owning-loss-v1".to_vec();
        key.extend_from_slice(session_id.as_bytes());
        key.extend_from_slice(&observation.loss_epoch.get().to_be_bytes());
        let mut tx = super::admission_journal::begin_pass_transaction(holder, deadline).await?;
        super::db::lock_admission_relations(&mut tx).await?;
        if let Some(row) = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json, decided_at FROM game_durability_admission_lifecycle_receipts r WHERE operation_key = $1 FOR SHARE")
            .bind(&key).fetch_optional(&mut *tx).await? {
            let stored: Option<String> = row.try_get("operation_json")?;
            if stored.as_deref() != Some(encoded.as_str()) { return Err(DurabilityError::InvalidStoredState); }
            let decided_at: i64 = row.try_get("decided_at")?;
            if decided_at < operation.authorized_at { return Err(DurabilityError::InvalidStoredState); }
            super::admission_journal::commit_pass_read(tx, deadline).await?;
            return Ok(ControlLossOutcomeV1::Committed { decided_at });
        }
        if !super::world_reset::loss_binding_current(&mut tx, observation.cause).await? {
            return Ok(ControlLossOutcomeV1::Rejected);
        }
        let Some(original) = store.session_fresh_operation_locked(&mut tx, session_id).await? else {
            return Ok(ControlLossOutcomeV1::Rejected);
        };
        let FreshReconciliation::Committed(current) =
            store.reconcile_session_locked(&mut tx, &original, session_id).await?
        else {
            return Ok(ControlLossOutcomeV1::Rejected);
        };
        let expected_claims = &original.transition.successors;
        let mut claims = Vec::with_capacity(2);
        for expected in expected_claims {
            claims.push(store.guards.load_locked(&mut tx, &expected.key).await?);
        }
        if original.authorization.account_id != observation.account_presence.account_id()
            || validate_claim_ownership_v1(
                &original.authorization.account_id,
                observation.session,
                current.current_session,
                expected_claims,
                &claims,
            )
            .is_err()
        {
            return Ok(ControlLossOutcomeV1::Rejected);
        }
        // The sealed observation does not supersede the independently published
        // current runtime owner. Load its complete guard under the same relation
        // fence before taking L, so concurrent ownership/readiness publication
        // cannot authorize loss against a superseded session observation.
        let named_house: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid AND runtime_scope_house_key IS NOT NULL)")
            .bind(session_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
        let runtime_valid = match observation.session.current_runtime_scope() {
            RuntimeScopeRefV1::Instance { .. } if named_house => store
                .house_lifecycle_fence_locked(&mut tx, current.current_session, false).await?.is_some(),
            scope => {
                let runtime = store.guards.load_locked(&mut tx,
                    &AdmissionAuthorityGuardKeyV1::Runtime(scope)).await?;
                matches!(runtime.as_ref().map(|row| &row.state),
                    Some(AdmissionAuthorityGuardStateV1::Runtime { ownership_generation, ready: true, .. })
                    if *ownership_generation == observation.session.current_scope_generation().get())
            }
        };
        if !runtime_valid { return Ok(ControlLossOutcomeV1::Rejected); }
        let activity_origin = store.activity_origin_id_locked(&mut tx, current.current_session).await?;
        let reservation: bool = if activity_origin.is_some() {
            // The activity's initial transport and any lost-ack recovery are
            // retained by its own immutable receipts. Resumed control below is
            // additionally checked against the complete reconnect owner.
            match &observation.history {
                ControlLossHistoryV1::ActivityOrigin { admission_id, protection } if Some(*admission_id) == activity_origin =>
                    store.activity_origin_transport_locked(&mut tx, current.current_session, *admission_id).await?
                        && store.activity_origin_protection_locked(&mut tx, current.current_session).await? == *protection,
                ControlLossHistoryV1::Resumed { .. } => true,
                _ => false,
            }
        } else {
            !matches!(&observation.history, ControlLossHistoryV1::ActivityOrigin { .. }) &&
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_transport_ref_reservations WHERE transport_ref = $1 AND game_session_id = encode($2,'hex')::uuid AND reservation_owner = 2 AND fresh_replay_key = $3 AND reconnect_attempt_ref IS NULL)")
                .bind(observation.session.commit().initial_transport().to_bytes().as_slice()).bind(observation.session.commit().game_session_id().as_bytes().as_slice()).bind(original.authorization.facts.replay_key().to_bytes().as_slice()).fetch_one(&mut *tx).await?
        };
        let epoch_exists: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_control_loss_continuity WHERE character_id = encode($1,'hex')::uuid AND control_loss_epoch = $2::text::numeric(20,0))")
            .bind(observation.session.commit().character_id().as_bytes().as_slice()).bind(observation.loss_epoch.get().to_string()).fetch_one(&mut *tx).await?;
        if !reservation || epoch_exists {
            return Ok(ControlLossOutcomeV1::Rejected);
        }
        // A loss after a same-session resume opens the next epoch. Its retained history is
        // re-derived here from the durable receipts, never taken from the observation.
        let resumed_from = match &observation.history {
            ControlLossHistoryV1::FreshOrigin | ControlLossHistoryV1::ActivityOrigin { .. } => None,
            ControlLossHistoryV1::ScopeTransfer { entry_epoch, .. } => {
                if store.transferred_loss_history_locked(&mut tx, current.current_session).await?.as_ref()
                    != Some(&observation.history)
                    || entry_epoch.get().checked_add(1) != Some(observation.loss_epoch.get()) {
                    return Ok(ControlLossOutcomeV1::Rejected);
                }
                Some(*entry_epoch)
            }
            ControlLossHistoryV1::Resumed { budget, .. } => {
                if store
                    .resumed_history_locked(&mut tx, current.current_session)
                    .await?
                    .as_ref()
                    != Some(&observation.history)
                    || budget.epoch().get().checked_add(1) != Some(observation.loss_epoch.get())
                {
                    return Ok(ControlLossOutcomeV1::Rejected);
                }
                Some(budget.epoch())
            }
        };
        // Strong common relation fencing excludes every sibling semantic writer
        // before this single final decision-time sample.
        let decided_at: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(&mut *tx)
                .await?;
        let Ok(effect) = request.validate_final(source.as_ref(), decided_at) else {
            return Ok(ControlLossOutcomeV1::Rejected);
        };
        if effect.predecessor() != current.current_session {
            return Ok(ControlLossOutcomeV1::Rejected);
        }
        let successor = effect.successor();
        let epoch = successor.current_control_loss_epoch().ok_or(DurabilityError::InvalidStoredState)?.get().to_string();
        let grace = successor.current_original_grace_deadline().ok_or(DurabilityError::InvalidStoredState)?;
        let changed = match resumed_from {
            None => sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state = 1, current_transport_ref = NULL, control_loss_epoch = $2::text::numeric(20,0), original_grace_deadline = $3, predecessor_generation = current_generation WHERE game_session_id = encode($1,'hex')::uuid AND session_state = 2 AND control_loss_epoch IS NULL AND prepared_attempt_ref IS NULL AND attempt_count = 0")
                .bind(session_id.as_bytes().as_slice()).bind(&epoch).bind(grace).execute(&mut *tx).await?,
            // The new epoch starts its own recovery budget.
            Some(resumed) => sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state = 1, current_transport_ref = NULL, control_loss_epoch = $2::text::numeric(20,0), original_grace_deadline = $3, predecessor_generation = current_generation, attempt_count = 0 WHERE game_session_id = encode($1,'hex')::uuid AND session_state = 2 AND control_loss_epoch = $4::text::numeric(20,0) AND prepared_attempt_ref IS NULL")
                .bind(session_id.as_bytes().as_slice()).bind(&epoch).bind(grace).bind(resumed.get().to_string()).execute(&mut *tx).await?,
        };
        if changed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
        // The complete canonical loss/protection operation is retained below.
        // Do not manufacture a legacy protection row: its connection-generation
        // namespace is not this operation's entitlement/rearm namespace. Legacy
        // prepare/replacement fail closed on this receipt until a typed bridge.
        sqlx::query("INSERT INTO game_durability_admission_lifecycle_receipts(operation_key,operation_json,decided_at) VALUES ($1,$2,$3)")
            .bind(&key).bind(&encoded).bind(decided_at).execute(&mut *tx).await?;
        if super::admission_journal::commit_pass_transaction(tx, deadline).await.is_err() {
            return Ok(ControlLossOutcomeV1::Ambiguous);
        }
        Ok(ControlLossOutcomeV1::Committed { decided_at })

            }))
            .await
    }

    /// Bind sealed delivery to an actual validated durable original. Absence or
    /// a conflicting original supplies no definitive completion and no rejection
    /// authority. This does not clear pending custody or reconstruct live inputs.
    pub async fn loss_completion_source(
        &self,
        original: &ControlLossOperationV1,
    ) -> Result<Option<DurableLossCompletionSource>> {
        match self.reconcile_fresh_loss(original).await? {
            FreshLossReconciliation::Committed {
                completion,
                current,
            } => Ok(Some(DurableLossCompletionSource {
                completion: Some(completion),
                current,
            })),
            FreshLossReconciliation::Absent | FreshLossReconciliation::Conflict => Ok(None),
        }
    }

    /// Read the original loss receipt and current canonical session in one fenced
    /// snapshot. Restoring this report never reconstructs a live loss request.
    pub async fn reconcile_fresh_loss(
        &self,
        original: &ControlLossOperationV1,
    ) -> Result<FreshLossReconciliation> {
        let store = (*self).clone();
        let original = (*original).clone();
        let issued = store.guards.backend.try_issue_root()?;
        let backend = store.guards.backend.clone();
        backend
            .run_pass(issued, |holder, deadline| Box::pin(async move {
        use sqlx::Row;
        let encoded = encode_fresh_loss(&original)?;
        let session_id = original.observation.session.current_game_session_id();
        let mut key = b"owning-loss-v1".to_vec();
        key.extend_from_slice(session_id.as_bytes());
        key.extend_from_slice(&original.observation.loss_epoch.get().to_be_bytes());
        let mut tx = super::admission_journal::begin_pass_transaction(holder, deadline).await?;
        super::db::lock_admission_relations(&mut tx).await?;
        let Some(row) = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json, decided_at FROM game_durability_admission_lifecycle_receipts r WHERE operation_key = $1 FOR SHARE")
            .bind(&key).fetch_optional(&mut *tx).await? else {
                super::admission_journal::commit_pass_read(tx, deadline).await?;
                return Ok(FreshLossReconciliation::Absent);
            };
        let stored: Option<String> = row.try_get("operation_json")?;
        let stored = stored.ok_or(DurabilityError::InvalidStoredState)?;
        let decided_at: i64 = row.try_get("decided_at")?;
        // The canonical fresh receipt supplies the initial commit; inventing a
        // replay key merely to deserialize the loss DTO would lose provenance.
        let fresh = store.session_fresh_operation_locked(&mut tx, session_id).await?
            .ok_or(DurabilityError::InvalidStoredState)?;
        let operation = decode_fresh_loss(&stored, checked(fresh.authorization.initial_commit())?)?;
        if operation.observation.session.current_game_session_id() != session_id
            || operation.observation.loss_epoch != original.observation.loss_epoch
            || operation.observation.account_presence.account_id() != fresh.authorization.account_id
            || decided_at < operation.authorized_at
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        if stored != encoded {
            super::admission_journal::commit_pass_read(tx, deadline).await?;
            return Ok(FreshLossReconciliation::Conflict);
        }
        let FreshReconciliation::Committed(current) =
            store.reconcile_session_locked(&mut tx, &fresh, session_id).await?
        else {
            return Err(DurabilityError::InvalidStoredState);
        };
        // The public session snapshot does not retain the SQL predecessor
        // mirror. Read it under the same fence rather than silently dropping its
        // relationship to the immutable original loss.
        let predecessor: Option<String> = sqlx::query_scalar("SELECT predecessor_generation::text FROM game_durability_reconnect_sessions WHERE game_session_id = encode($1,'hex')::uuid FOR SHARE")
            .bind(session_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
        let predecessor: u64 = checked(
            predecessor
                .ok_or(DurabilityError::InvalidStoredState)?
                .parse(),
        )?;
        if current.current_session.current_control_loss_epoch()
            == Some(operation.observation.loss_epoch)
            && predecessor
                != operation
                    .observation
                    .session
                    .current_connection_generation()
                    .get()
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        if current
            .current_session
            .current_connection_generation()
            .get()
            < operation
                .observation
                .session
                .current_connection_generation()
                .get()
            || current
                .current_session
                .current_control_loss_epoch()
                .is_none_or(|epoch| epoch.get() < operation.observation.loss_epoch.get())
            || (current.current_session.current_control_loss_epoch()
                == Some(operation.observation.loss_epoch)
                && current.current_session.current_original_grace_deadline()
                    != Some(operation.observation.original_grace_deadline))
            || (current.current_session.current_connection_generation()
                == operation
                    .observation
                    .session
                    .current_connection_generation()
                && current.current_session.current_transport().is_some())
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        super::admission_journal::commit_pass_read(tx, deadline).await?;
        Ok(FreshLossReconciliation::Committed {
            completion: Box::new(ControlLossCompletionV1 {
                operation,
                outcome: ControlLossOutcomeV1::Committed { decided_at },
            }),
            current,
        })

            }))
            .await
    }
}

impl FreshAdmissionStore {
    pub async fn reconcile(
        &self,
        original: &FreshAdmissionOperationV1,
    ) -> Result<FreshReconciliation> {
        let store = (*self).clone();
        let original = (*original).clone();
        let issued = store.guards.backend.try_issue_root()?;
        let backend = store.guards.backend.clone();
        backend
            .run_pass(issued, |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let result = store.reconcile_locked(&mut tx, &original).await?;
                    super::admission_journal::commit_pass_read(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }

    /// Recheck an already committed admission during first entry. The receipt,
    /// live session and transport reservation come from a single MVCC snapshot.
    /// Ambiguous COMMIT reconciliation still uses the writer exclusion in `reconcile`.
    pub async fn observe_committed_admission(
        &self,
        original: &FreshAdmissionOperationV1,
    ) -> Result<FreshReconciliation> {
        let store = self.clone();
        let original = original.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend.run_pass(issued, move |holder, deadline| {
            Box::pin(async move {
                let mut tx = match deadline {
                    Some(deadline) => super::db::begin_observation_transaction(holder, deadline).await?,
                    None => sqlx::Connection::begin_with(
                        &mut **holder,
                        "BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY",
                    ).await?,
                };
                super::db::bound_periodic_lock_wait(&mut tx).await?;
                let result = store.reconcile_locked(&mut tx, &original).await?;
                super::admission_journal::commit_pass_read(tx, deadline).await?;
                Ok(result)
            })
        }).await
    }

    async fn reconcile_locked(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        original: &FreshAdmissionOperationV1,
    ) -> Result<FreshReconciliation> {
        self.reconcile_session_locked(tx, original, original.authorization.candidate_session)
            .await
    }

    async fn reconcile_session_locked(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        original: &FreshAdmissionOperationV1,
        current_id: GameSessionId,
    ) -> Result<FreshReconciliation> {
        let replay = original.authorization.facts.replay_key().to_bytes();
        let Some(receipt) = self.receipt_locked(tx, &replay).await? else {
            return Ok(FreshReconciliation::Absent);
        };
        if receipt.operation() != original {
            return Ok(FreshReconciliation::Conflict);
        }
        let snapshot = self.read_reconciled_session(tx, receipt, current_id).await?;
        Ok(FreshReconciliation::Committed(Box::new(snapshot)))
    }

    async fn read_reconciled_session(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        receipt: FreshAdmissionCommitReceiptV1,
        current_id: GameSessionId,
    ) -> Result<FreshAdmissionDurableReconciliationSnapshotV1> {
        use sqlx::Row;
        let b = receipt.binding();
        let replay = b.facts.replay_key().to_bytes();
        let row = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(s)::text) <= $2 THEN to_jsonb(s) END AS state FROM game_durability_reconnect_sessions s WHERE game_session_id = encode($1,'hex')::uuid").bind(current_id.as_bytes().as_slice()).bind(super::MAX_ADMISSION_ROW_BYTES).fetch_optional(&mut **tx).await?.ok_or(DurabilityError::InvalidStoredState)?;
        let state: Option<serde_json::Value> = row.try_get("state")?;
        let state = state.ok_or(DurabilityError::InvalidStoredState)?;
        let initial = checked(b.initial_commit())?;
        if json_text(&state, "account_id")? != b.account_id
            || json_text(&state, "character_id")? != uuid_text(initial.character_id().as_bytes())
            || json_text(&state, "world_id")? != uuid_text(initial.world_id().as_bytes())
            || (state["fresh_replay_key"]
                .as_str()
                .or_else(|| state["initial_fresh_replay_key"].as_str())
                != Some(bytea_text(&replay).as_str()))
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let world = checked(WorldId::decode(&json_uuid(
            &state,
            "runtime_scope_world_id",
        )?))?;
        let scope = match json_u64(&state, "runtime_scope_kind")? {
            1 if state["runtime_scope_instance_id"].is_null() => RuntimeScopeRefV1::channel(
                world,
                checked(ChannelId::decode(&json_uuid(
                    &state,
                    "runtime_scope_channel_id",
                )?))?,
            ),
            2 if state["runtime_scope_channel_id"].is_null() => checked(
                RuntimeScopeRefV1::instance(world, json_uuid(&state, "runtime_scope_instance_id")?),
            )?,
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        let session_state = match json_u64(&state, "session_state")? {
            1 => GameSessionState::Reconnectable,
            2 => GameSessionState::Active,
            3 => GameSessionState::Terminal,
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        let current_transport = if state["current_transport_ref"].is_null() {
            None
        } else {
            Some(checked(AuthenticatedTransportRefV1::decode(&json_bytea(
                &state,
                "current_transport_ref",
            )?))?)
        };
        let character = self
            .guards
            .load_locked(
                tx,
                &AdmissionAuthorityGuardKeyV1::Character(initial.character_id()),
            )
            .await?;
        let eligibility = match character.as_ref().map(|row| &row.state) {
            Some(AdmissionAuthorityGuardStateV1::Character {
                world_id,
                eligible: true,
                ..
            }) if *world_id == initial.world_id() => Some(CharacterWorldEligibilityClaimV1::new(
                initial.character_id(),
                *world_id,
            )),
            _ => None,
        };
        let mut current_session =
            checked(GameSessionAuthoritySnapshot::from_persisted_current_facts(
                current_id,
                initial,
                session_state,
                checked(ConnectionGeneration::new(json_u64(
                    &state,
                    "current_generation",
                )?))?,
                current_transport,
                checked(CharacterLease::new(
                    initial.character_id(),
                    json_u64(&state, "character_lease_generation")?,
                ))?,
                eligibility,
                scope,
                checked(ScopeOwnershipGeneration::new(json_u64(
                    &state,
                    "scope_ownership_generation",
                )?))?,
            ))?;
        match (
            state.get("control_loss_epoch"),
            state.get("original_grace_deadline"),
            state.get("predecessor_generation"),
        ) {
            (
                Some(serde_json::Value::Null),
                Some(serde_json::Value::Null),
                Some(serde_json::Value::Null),
            ) if session_state != GameSessionState::Reconnectable
                && state["prepared_attempt_ref"].is_null()
                && json_u64(&state, "attempt_count")? == 0 => {}
            (Some(epoch), Some(grace), Some(predecessor))
                if !epoch.is_null()
                    && !grace.is_null()
                    && predecessor.as_u64().is_some_and(|value| {
                        value > 0 && value <= current_session.current_connection_generation().get()
                    }) =>
            {
                current_session = checked(current_session.with_control_loss_continuity(
                    checked(ControlLossEpochRefV1::new(
                        epoch.as_u64().ok_or(DurabilityError::InvalidStoredState)?,
                    ))?,
                    grace.as_i64().ok_or(DurabilityError::InvalidStoredState)?,
                ))?;
            }
            _ => return Err(DurabilityError::InvalidStoredState),
        }
        let reservation: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_transport_ref_reservations WHERE transport_ref = $1 AND game_session_id = encode($2,'hex')::uuid AND reservation_owner = 2 AND fresh_replay_key = $3 AND reconnect_attempt_ref IS NULL)").bind(b.transport.to_bytes().as_slice()).bind(b.candidate_session.as_bytes().as_slice()).bind(replay.as_slice()).fetch_one(&mut **tx).await?;
        if !reservation {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(FreshAdmissionDurableReconciliationSnapshotV1 {
            receipt,
            current_session,
        })
    }
}

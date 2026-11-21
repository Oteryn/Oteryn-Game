impl FreshAdmissionStore {
    async fn apply_complete_reconnect_with(
        &self,
        custody: Option<super::runtime_scope_assignment::NodeIncarnationProof>,
        request: std::sync::Arc<CompleteReconnectRequestV1>,
        source: std::sync::Arc<dyn CompleteReconnectSourceV1 + Send + Sync>,
        capabilities: Option<CurrentCapabilityCheck>,
    ) -> Result<CompleteReconnectOutcomeV1> {
        let store = (*self).clone();
        let issued = store.guards.backend.try_issue_root()?;
        let backend = store.guards.backend.clone();
        backend
            .run_pass(issued, |holder, deadline| Box::pin(async move {
        use sqlx::Row;
        let commit = match request.kind() {
            CompleteReconnectRequestKindV1::Prepare => false,
            CompleteReconnectRequestKindV1::Commit => true,
            CompleteReconnectRequestKindV1::Reconcile => return Err(DurabilityError::Unavailable),
        };
        let operation = request.operation();
        let encoded = encode_complete_reconnect(operation)?;
        let recovery = &operation.recovery;
        let original = &recovery.original;
        let identity = &recovery.identity;
        let session_id = identity.game_session_id();
        let epoch = original.loss.observation.loss_epoch;
        let key = complete_reconnect_key(
            if commit { COMPLETE_COMMIT_KEY } else { COMPLETE_PREPARE_KEY },
            identity,
            epoch.get(),
        );
        let mut tx = super::admission_journal::begin_pass_transaction(holder, deadline).await?;
        super::db::lock_admission_relations(&mut tx).await?;
        if let Some(row) = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json, decided_at FROM game_durability_admission_lifecycle_receipts r WHERE operation_key = $1 FOR SHARE")
            .bind(&key).fetch_optional(&mut *tx).await? {
            let stored: Option<String> = row.try_get("operation_json")?;
            if stored.as_deref() != Some(encoded.as_str()) { return Err(DurabilityError::InvalidStoredState); }
            let decided_at: i64 = row.try_get("decided_at")?;
            if decided_at < recovery.prepared_at { return Err(DurabilityError::InvalidStoredState); }
            super::admission_journal::commit_pass_transaction(tx, deadline).await?;
            return Ok(if commit {
                CompleteReconnectOutcomeV1::Committed { decided_at }
            } else {
                CompleteReconnectOutcomeV1::Prepared { decided_at }
            });
        }
        // The exact durable original loss this recovery resumes.
        let mut loss_key = b"owning-loss-v1".to_vec();
        loss_key.extend_from_slice(session_id.as_bytes());
        loss_key.extend_from_slice(&epoch.get().to_be_bytes());
        let loss_row = sqlx::query("SELECT CASE WHEN octet_length(to_jsonb(r)::text) <= 131072 THEN operation_json END AS operation_json, decided_at FROM game_durability_admission_lifecycle_receipts r WHERE operation_key = $1 FOR SHARE")
            .bind(&loss_key).fetch_optional(&mut *tx).await?;
        let Some(loss_row) = loss_row else { return Ok(CompleteReconnectOutcomeV1::Rejected) };
        let stored_loss: Option<String> = loss_row.try_get("operation_json")?;
        let loss_decided_at: i64 = loss_row.try_get("decided_at")?;
        if stored_loss.as_deref() != Some(encode_fresh_loss(&original.loss)?.as_str())
            || loss_decided_at != original.loss_decided_at
        {
            return Ok(CompleteReconnectOutcomeV1::Rejected);
        }
        let fresh = store.session_fresh_operation_locked(&mut tx, session_id).await?
            .ok_or(DurabilityError::InvalidStoredState)?;
        if fresh.authorization.account_id != identity.account_id() {
            return Ok(CompleteReconnectOutcomeV1::Rejected);
        }
        // Current session, claim ownership, runtime owner and retained budget.
        let current = store.current_session_locked(&mut tx, session_id).await?;
        // PREPARE leaves the snapshot unchanged, so both phases bind the original.
        if current != original.session {
            return Ok(CompleteReconnectOutcomeV1::Rejected);
        }
        let mut claims = Vec::with_capacity(2);
        for expected in &original.claims {
            claims.push(store.guards.load_locked(&mut tx, &expected.key).await?);
        }
        // Recovery grants control: the locked rows must be exactly the claims the sealed
        // request validated (security allowed and generation floor, eligibility,
        // presence and holder), not merely still owned by this session.
        if claims.len() != original.claims.len()
            || claims.iter().zip(&original.claims).any(|(row, expected)| row.as_ref() != Some(expected))
            || validate_claim_ownership_v1(identity.account_id(), original.session, current, &original.claims, &claims).is_err()
        {
            return Ok(CompleteReconnectOutcomeV1::Rejected);
        }
        let runtime = store.recovery_runtime_locked(&mut tx, current).await?;
        // Channel recovery retains its exact ready owner. A named House was
        // independently proved above under current node/assignment/ACL locks;
        // its origin publication supplies only the existing compatibility policy.
        let revisions = &original.recovery;
        if !matches!(runtime.as_ref().map(|row| &row.state),
            Some(AdmissionAuthorityGuardStateV1::Runtime {
                ownership_generation, ready: true, ruleset_revision, content_revision, map_revision, world_policy_revision, ..
            })
            if *ownership_generation > 0
                && *ruleset_revision == revisions.ruleset_revision
                && *content_revision == revisions.content_revision
                && *map_revision == revisions.map_revision
                && *world_policy_revision == revisions.world_policy_revision)
        {
            return Ok(CompleteReconnectOutcomeV1::Rejected);
        }
        let Some(AdmissionAuthorityPublicationChangeV1 {
            key: AdmissionAuthorityGuardKeyV1::Runtime(policy_scope),
            state: AdmissionAuthorityGuardStateV1::Runtime { ownership_generation: policy_generation, .. }, ..
        }) = runtime.as_ref() else { return Ok(CompleteReconnectOutcomeV1::Rejected); };
        let budget = store.recovery_budget_locked(&mut tx, session_id, epoch).await?;
        let expected_budget = if commit {
            // The prepared attempt is part of the durable budget by now.
            recovery_budget_after_prepare(recovery)?
        } else {
            original.budget.clone()
        };
        if budget != expected_budget {
            return Ok(CompleteReconnectOutcomeV1::Rejected);
        }
        let candidate = original.candidate;
        let attempt = identity.reconnect_attempt_ref().to_be_bytes();
        let transport = candidate.transport_ref().to_bytes();
        let decided_at: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&mut *tx).await?;
        let decision = match &custody {
            Some(custody) => {
                let CompleteReconnectCredentialV1::Recovery(credential) = &recovery.credential else {
                    return Ok(CompleteReconnectOutcomeV1::Rejected);
                };
                let Some(audit) = &credential.v2 else {
                    return Ok(CompleteReconnectOutcomeV1::Rejected);
                };
                let subject = super::recovery_evidence_composition::RecoveryEvidenceSubject::new(
                    identity.account_id(),
                    audit.signing.key_id.clone(),
                )?;
                super::recovery_evidence_composition::decide_with_registered_recovery(
                    &mut tx,
                    custody,
                    &subject,
                    |registered| {
                        request.validate_locked(
                            &RegisteredRecoverySource { inner: source.as_ref(), registered },
                            decided_at,
                        )
                    },
                )
                .await?
            }
            None => request.validate_locked(source.as_ref(), decided_at),
        };
        let Ok(effect) = decision else {
            return Ok(CompleteReconnectOutcomeV1::Rejected);
        };
        // ADMIT0: PREPARE and COMMIT both require the independently locked current
        // runtime binding before any reservation, nonce consumption or session switch.
        // Exact historical receipts above retain their original outcome; replaying one
        // is not a fresh authority grant. Only the test/test-support seam may omit the check.
        match (&custody, &capabilities) {
            (_, Some(check)) if custody.is_some() || cfg!(any(test, feature = "test-support")) => {
                if !store.check_current_runtime_capabilities(&mut tx,
                    *policy_scope, *policy_generation, check).await? {
                    return Ok(CompleteReconnectOutcomeV1::Rejected);
                }
            }
            (None, None) if cfg!(any(test, feature = "test-support")) => {}
            _ => return Ok(CompleteReconnectOutcomeV1::Rejected),
        }
        if store.security_revocation_freshness_required
            && !super::platform_streams::revocation_fresh_in_transaction(&mut tx).await? {
            return Ok(CompleteReconnectOutcomeV1::Rejected);
        }
        if commit {
            if !store.complete_prepare_exists(&mut tx, identity, epoch, &encoded).await? {
                return Ok(CompleteReconnectOutcomeV1::Rejected);
            }
            let CompleteReconnectCredentialV1::Recovery(credential) = &recovery.credential else {
                return Ok(CompleteReconnectOutcomeV1::Rejected);
            };
            let consumed = sqlx::query("INSERT INTO game_durability_recovery_grant_consumptions (recovery_grant_nonce, game_session_id, reconnect_attempt_ref) VALUES ($1, encode($2,'hex')::uuid, $3) ON CONFLICT (recovery_grant_nonce) DO NOTHING")
                .bind(credential.grant_nonce.as_slice()).bind(session_id.as_bytes().as_slice()).bind(attempt.as_slice())
                .execute(&mut *tx).await?;
            if consumed.rows_affected() != 1 {
                return Ok(CompleteReconnectOutcomeV1::Rejected);
            }
            let next = effect.session();
            let switched = sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state = 2, current_generation = $2::text::numeric(20,0), current_transport_ref = $3, prepared_attempt_ref = NULL WHERE game_session_id = encode($1,'hex')::uuid AND session_state = 1 AND current_transport_ref IS NULL AND prepared_attempt_ref = $4 AND current_generation = $5::text::numeric(20,0)")
                .bind(session_id.as_bytes().as_slice())
                .bind(next.current_connection_generation().get().to_string())
                .bind(transport.as_slice())
                .bind(attempt.as_slice())
                .bind(original.session.current_connection_generation().get().to_string())
                .execute(&mut *tx).await?;
            if switched.rows_affected() != 1 {
                return Err(DurabilityError::InvalidStoredState);
            }
            if store.current_session_locked(&mut tx, session_id).await? != next {
                return Err(DurabilityError::InvalidStoredState);
            }
        } else {
            // A grant nonce that was already consumed can never commit; do not let it
            // strand the session with a prepared attempt.
            let CompleteReconnectCredentialV1::Recovery(credential) = &recovery.credential else {
                return Ok(CompleteReconnectOutcomeV1::Rejected);
            };
            let spent: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_recovery_grant_consumptions WHERE recovery_grant_nonce = $1)")
                .bind(credential.grant_nonce.as_slice()).fetch_one(&mut *tx).await?;
            if spent {
                return Ok(CompleteReconnectOutcomeV1::Rejected);
            }
            let reserved = sqlx::query("INSERT INTO game_durability_transport_ref_reservations (transport_ref, game_session_id, reconnect_attempt_ref, reservation_owner) VALUES ($1, encode($2,'hex')::uuid, $3, 1) ON CONFLICT (transport_ref) DO NOTHING")
                .bind(transport.as_slice()).bind(session_id.as_bytes().as_slice()).bind(attempt.as_slice())
                .execute(&mut *tx).await?;
            if reserved.rows_affected() != 1 {
                return Ok(CompleteReconnectOutcomeV1::Rejected);
            }
            let named = sqlx::query("UPDATE game_durability_reconnect_sessions SET prepared_attempt_ref = $2, attempt_count = attempt_count + 1 WHERE game_session_id = encode($1,'hex')::uuid AND session_state = 1 AND current_transport_ref IS NULL AND prepared_attempt_ref IS NULL AND attempt_count < 8")
                .bind(session_id.as_bytes().as_slice()).bind(attempt.as_slice())
                .execute(&mut *tx).await?;
            if named.rows_affected() != 1 {
                return Ok(CompleteReconnectOutcomeV1::Rejected);
            }
        }
        sqlx::query("INSERT INTO game_durability_admission_lifecycle_receipts (operation_key, operation_json, decided_at) VALUES ($1,$2,$3)")
            .bind(&key).bind(&encoded).bind(decided_at).execute(&mut *tx).await?;
        if commit && store.recovery_budget_locked(&mut tx, session_id, epoch).await? != *effect.budget() {
            return Err(DurabilityError::InvalidStoredState);
        }
        if super::admission_journal::commit_pass_transaction(tx, deadline).await.is_err() {
            return Ok(CompleteReconnectOutcomeV1::Ambiguous);
        }
        Ok(if commit {
            CompleteReconnectOutcomeV1::Committed { decided_at }
        } else {
            CompleteReconnectOutcomeV1::Prepared { decided_at }
        })
            }))
            .await
    }
}

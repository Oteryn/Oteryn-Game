//! The single prepared/fenced terminal-release writer, extracted from its owner.
use super::*;

impl FreshAdmissionStore {
    /// Revalidate a prepared terminal release against current claims at the deciding transaction.
    pub async fn release(&self, transition: &TerminalReleaseClaimTransitionV1) -> Result<i64> {
        let encoded = encode_lifecycle(transition.evidence())?;
        let transition = transition.clone();
        let store = self.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend.run_pass(issued, move |holder, deadline| Box::pin(async move {
            let evidence = transition.evidence();
            let mut tx = super::super::admission_journal::begin_pass_transaction(holder, deadline).await?;
            super::super::db::lock_admission_relations(&mut tx).await?;
            if let Some(prior) = lifecycle_replay(&mut tx, evidence, &encoded).await? {
                super::super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                return Ok(prior);
            }
            let session_id = evidence.operation.current_session().current_game_session_id();
            let current = store.current_session_locked(&mut tx, session_id).await?;
            if let Some(proof) = &store.retired_holder
                && !retired_holder_release::validate(&mut tx, proof, current).await? {
                return Err(DurabilityError::Unavailable);
            }
            let mut rows = Vec::with_capacity(2);
            for prior in &evidence.transition.predecessors { rows.push(store.guards.load_locked(&mut tx, &prior.key).await?); }
            if !store.guards.successor_history_available(&mut tx, &evidence.transition.successors, &rows).await? { return Err(DurabilityError::Unavailable); }
            let encoded_successors = evidence.transition.successors.iter().map(|row| encode_guard(row, store.guards.maximum_guard_bytes)).collect::<Result<Vec<_>>>()?;
            let decided_at: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&mut *tx).await?;
            let successors = transition.validate_locked(&rows, current, decided_at).map_err(|_| DurabilityError::Unavailable)?;
            if let Some(training) = &store.training_logout { training.prepare(&mut tx).await?; }
            if let Some(final_position) = store.final_position {
                write_final_position(&mut tx, &current, &final_position).await?;
            }
            let updated = sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state = 3, current_transport_ref = NULL, prepared_attempt_ref = NULL WHERE game_session_id = encode($1,'hex')::uuid AND current_generation = $2::text::numeric(20,0) AND character_lease_generation = $3::text::numeric(20,0) AND scope_ownership_generation = $4::text::numeric(20,0)")
                .bind(session_id.as_bytes().as_slice()).bind(current.current_connection_generation().get().to_string()).bind(current.current_character_lease().generation().to_string()).bind(current.current_scope_generation().get().to_string()).execute(&mut *tx).await?;
            if updated.rows_affected() != 1 { return Err(DurabilityError::Unavailable); }
            sqlx::query("UPDATE game_durability_reconnect_attempts SET state = 4 WHERE game_session_id = encode($1,'hex')::uuid AND state IN (1,5)")
                .bind(session_id.as_bytes().as_slice()).execute(&mut *tx).await?;
            store.guards.persist_locked(&mut tx, successors, &rows, &encoded_successors).await?;
            sqlx::query("INSERT INTO game_durability_admission_lifecycle_receipts (operation_key, operation_json, decided_at) VALUES ($1,$2,$3)")
                .bind(lifecycle_key(evidence)).bind(&encoded).bind(decided_at).execute(&mut *tx).await?;
            super::super::admission_journal::commit_pass_transaction(tx, deadline).await?;
            Ok(decided_at)
        })).await
    }

    pub(super) async fn release_current_claims(
        &self,
        current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
        now: i64,
        account_id: &str,
    ) -> Result<ExpiredLossReleaseV1> {
        let keys = [
            AdmissionAuthorityGuardKeyV1::Account {
                account_id: account_id.into(),
            },
            AdmissionAuthorityGuardKeyV1::Character(current.commit().character_id()),
        ];
        let predecessors = self
            .guards
            .load(&keys)
            .await?
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(DurabilityError::Unavailable)?;
        // Successor source times never precede a predecessor's (a row last
        // published on a faster host clock): until the durable clock catches
        // up, the fenced release refuses and a later attempt retries.
        let prepared_at = predecessors
            .iter()
            .map(|row| row.source.source_observed_at)
            .fold(now, i64::max);
        let mut successors = predecessors.clone();
        for row in &mut successors {
            row.precondition = AdmissionPublicationPreconditionV1::CompareAndSet {
                expected_publication_revision: row.publication_revision,
            };
            row.publication_revision = row
                .publication_revision
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)?;
            row.source.source_revision = row
                .source
                .source_revision
                .checked_add(1)
                .ok_or(DurabilityError::InvalidStoredState)?;
            row.source.decision_identity =
                format!("grace-expiry-release:{}", row.source.source_revision);
            row.source.source_observed_at = prepared_at;
            row.source.clock_uncertainty_seconds = 0;
            match &mut row.state {
                AdmissionAuthorityGuardStateV1::Account { security, presence } => {
                    security.provenance.publication_revision = row.publication_revision;
                    *presence = None;
                }
                AdmissionAuthorityGuardStateV1::Character { holder, .. } => *holder = None,
                _ => return Err(DurabilityError::InvalidStoredState),
            }
        }
        let owner = ExpiredLossReleaseOwner {
            current,
            transition: AdmissionClaimTransitionEvidenceV1 {
                predecessors,
                successors,
                prepared_at,
            },
        };
        let transition = match TerminalReleaseClaimTransitionV1::prepare(
            &owner,
            account_id,
            current,
            prepared_at,
        ) {
            Ok(transition) => transition,
            // Ownership no longer names this session: not this release's to make.
            Err(AdmissionAuthorityPublicationErrorV1::Stale) => {
                return Ok(ExpiredLossReleaseV1::NotApplicable);
            }
            Err(_) => return Err(DurabilityError::Unavailable),
        };
        match self.release(&transition).await {
            Ok(decided_at) => Ok(ExpiredLossReleaseV1::Released { decided_at }),
            Err(error) => match self.reconcile_lifecycle(transition.evidence()).await? {
                Some(decided_at) => Ok(ExpiredLossReleaseV1::Released { decided_at }),
                None => Err(error),
            },
        }
    }
}

//! The single prepared/fenced terminal-release writer, extracted from its owner.
use super::*;

impl FreshAdmissionStore {
    /// Revalidate a prepared terminal release against current claims at the deciding transaction.
    pub async fn release(&self, transition: &TerminalReleaseClaimTransitionV1) -> Result<i64> {
        // Boxed: the bed-aware writer's state machine is large, and reconnect/loss
        // release chains nest it under other large futures (stack overflow otherwise).
        Box::pin(self.release_with_bed(transition, None))
            .await
            .map(|(at, _)| at)
    }

    pub(in crate::durability) async fn release_with_bed(
        &self,
        transition: &TerminalReleaseClaimTransitionV1,
        bed: Option<super::super::bed_sleep::BedSleepPlan>,
    ) -> Result<(i64, Option<super::super::bed_sleep::BedSleepOutcome>)> {
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
                // Another terminal cause can share this claim transition. Only
                // the bed's own immutable receipt proves that sleep committed.
                let outcome = match bed {
                    Some(bed) => Some(bed.apply(&mut tx).await?),
                    None => None,
                };
                super::super::admission_journal::commit_pass_transaction(tx, deadline).await?;
                return Ok((prior, outcome));
            }
            let session_id = evidence.operation.current_session().current_game_session_id();
            let current = store.current_session_locked(&mut tx, session_id).await?;
            if let Some(epoch)=store.staff_loss {
                if current.session_state()!=GameSessionState::Reconnectable || current.current_transport().is_some()
                    || current.current_control_loss_epoch()!=Some(epoch)
                { return Err(DurabilityError::Unavailable); }
                let inherited:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_activity_runtime_returns a JOIN game_durability_reconnect_sessions s ON s.game_session_id=a.source_game_session_id JOIN game_durability_reconnect_sessions d ON d.game_session_id=a.destination_game_session_id WHERE a.destination_game_session_id=encode($1,'hex')::uuid AND $2::text::numeric=1 AND s.control_loss_epoch IS NOT NULL AND s.original_grace_deadline=d.original_grace_deadline AND d.control_loss_epoch=1)")
                    .bind(session_id.as_bytes().as_slice()).bind(epoch.get().to_string()).fetch_one(&mut *tx).await?;
                if !inherited && !has_owning_loss_receipt(&mut tx,session_id.as_bytes().as_slice(),epoch.get()).await? { return Err(DurabilityError::Unavailable); }
                let signed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_staff_disconnects d WHERE d.game_session_id=encode($1,'hex')::uuid OR EXISTS(SELECT 1 FROM game_activity_runtime_returns a WHERE a.destination_game_session_id=encode($1,'hex')::uuid AND a.source_game_session_id=d.game_session_id))")
                    .bind(session_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                if !signed { return Err(DurabilityError::Unavailable); }
            }
            if let Some(proof) = &store.retired_holder
                && !retired_holder_release::validate(&mut tx, proof, current).await? {
                return Err(DurabilityError::Unavailable);
            }
            if let Some(proof) = &store.departed_activity
                && !proof.validate(&mut tx, current).await? {
                return Err(DurabilityError::Unavailable);
            }
            if let Some(proof) = &store.offline_activity_mismatch
                && !proof.validate(&mut tx, current).await? {
                return Err(DurabilityError::Unavailable);
            }
            let mut rows = Vec::with_capacity(2);
            for prior in &evidence.transition.predecessors { rows.push(store.guards.load_locked(&mut tx, &prior.key).await?); }
            if !store.guards.successor_history_available(&mut tx, &evidence.transition.successors, &rows).await? { return Err(DurabilityError::Unavailable); }
            let encoded_successors = evidence.transition.successors.iter().map(|row| encode_guard(row, store.guards.maximum_guard_bytes)).collect::<Result<Vec<_>>>()?;
            let decided_at: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&mut *tx).await?;
            let successors = transition.validate_locked(&rows, current, decided_at).map_err(|_| DurabilityError::Unavailable)?;
            let slept = if let Some(bed) = bed {
                let outcome = bed.apply(&mut tx).await?;
                if outcome != super::super::bed_sleep::BedSleepOutcome::Slept {
                    return Ok((decided_at, Some(outcome)));
                }
                Some(outcome)
            } else { None };
            if let Some(training) = &store.training_logout { training.prepare(&mut tx).await?; }
            if let Some(final_position) = store.final_position {
                write_final_position(&mut tx, &current, &final_position, store.final_house.as_ref()).await?;
            }
            let updated = sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state = 3, current_transport_ref = NULL, prepared_attempt_ref = NULL WHERE game_session_id = encode($1,'hex')::uuid AND current_generation = $2::text::numeric(20,0) AND character_lease_generation = $3::text::numeric(20,0) AND scope_ownership_generation = $4::text::numeric(20,0)")
                .bind(session_id.as_bytes().as_slice()).bind(current.current_connection_generation().get().to_string()).bind(current.current_character_lease().generation().to_string()).bind(current.current_scope_generation().get().to_string()).execute(&mut *tx).await?;
            if updated.rows_affected() != 1 { return Err(DurabilityError::Unavailable); }
            if let Some(node) = &store.house_release_node {
                super::super::character_last_position::release_terminal_house_entry(&mut tx, node,
                    session_id, current.current_character_lease().character_id(), decided_at).await?;
            }
            super::super::world_party::remove_terminal_member_in_transaction(&mut tx, session_id)
                .await.map_err(|_| DurabilityError::Unavailable)?;
            sqlx::query("UPDATE game_durability_reconnect_attempts SET state = 4 WHERE game_session_id = encode($1,'hex')::uuid AND state IN (1,5)")
                .bind(session_id.as_bytes().as_slice()).execute(&mut *tx).await?;
            store.guards.persist_locked(&mut tx, successors, &rows, &encoded_successors).await?;
            sqlx::query("INSERT INTO game_durability_admission_lifecycle_receipts (operation_key, operation_json, decided_at) VALUES ($1,$2,$3)")
                .bind(lifecycle_key(evidence)).bind(&encoded).bind(decided_at).execute(&mut *tx).await?;
            super::super::admission_journal::commit_pass_transaction(tx, deadline).await?;
            Ok((decided_at, slept))
        })).await
    }

    pub(super) async fn release_current_claims(
        &self,
        current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
        now: i64,
        account_id: &str,
    ) -> Result<ExpiredLossReleaseV1> {
        let Some(transition) =
            Box::pin(self.prepare_current_claims_release(current, now, account_id)).await?
        else {
            return Ok(ExpiredLossReleaseV1::NotApplicable);
        };
        match Box::pin(self.release(&transition)).await {
            Ok(decided_at) => Ok(ExpiredLossReleaseV1::Released { decided_at }),
            Err(error) => match self.reconcile_lifecycle(transition.evidence()).await? {
                Some(decided_at) => Ok(ExpiredLossReleaseV1::Released { decided_at }),
                None => Err(error),
            },
        }
    }
    pub(in crate::durability) async fn prepare_current_claims_release(
        &self,
        current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
        now: i64,
        account_id: &str,
    ) -> Result<Option<TerminalReleaseClaimTransitionV1>> {
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
                return Ok(None);
            }
            Err(_) => return Err(DurabilityError::Unavailable),
        };
        Ok(Some(transition))
    }
}

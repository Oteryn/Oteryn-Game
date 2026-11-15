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
        let issued = nb("release_issue_root", backend.try_issue_root())?;
        let passed = backend.run_pass(issued, move |holder, deadline| Box::pin(async move {
            let evidence = transition.evidence();
            let mut tx = nb("release_begin", super::super::admission_journal::begin_pass_transaction(holder, deadline).await)?;
            nb("release_lock", super::super::db::lock_admission_relations(&mut tx).await)?;
            if let Some(prior) = nb("release_lifecycle_replay", lifecycle_replay(&mut tx, evidence, &encoded).await)? {
                // Another terminal cause can share this claim transition. Only
                // the bed's own immutable receipt proves that sleep committed.
                let outcome = match bed {
                    Some(bed) => Some(nb("release_replay_bed", bed.apply(&mut tx).await)?),
                    None => None,
                };
                nb("release_replay_commit", super::super::admission_journal::commit_pass_transaction(tx, deadline).await)?;
                return Ok((prior, outcome));
            }
            let session_id = evidence.operation.current_session().current_game_session_id();
            let current = nb("release_current_session", store.current_session_locked(&mut tx, session_id).await)?;
            if let Some(epoch)=store.staff_loss {
                if current.session_state()!=GameSessionState::Reconnectable || current.current_transport().is_some()
                    || current.current_control_loss_epoch()!=Some(epoch)
                { nbhold_unavailable("release_staff_loss_state", None); return Err(DurabilityError::Unavailable); }
                let inherited:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_activity_runtime_returns a JOIN game_durability_reconnect_sessions s ON s.game_session_id=a.source_game_session_id JOIN game_durability_reconnect_sessions d ON d.game_session_id=a.destination_game_session_id WHERE a.destination_game_session_id=encode($1,'hex')::uuid AND $2::text::numeric=1 AND s.control_loss_epoch IS NOT NULL AND s.original_grace_deadline=d.original_grace_deadline AND d.control_loss_epoch=1)")
                    .bind(session_id.as_bytes().as_slice()).bind(epoch.get().to_string()).fetch_one(&mut *tx).await?;
                if !inherited && !has_owning_loss_receipt(&mut tx,session_id.as_bytes().as_slice(),epoch.get()).await? { nbhold_unavailable("release_staff_loss_receipt", None); return Err(DurabilityError::Unavailable); }
                let signed:bool=sqlx::query_scalar("SELECT game_security_session_revoked(encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_staff_disconnects d WHERE d.game_session_id=encode($1,'hex')::uuid OR EXISTS(SELECT 1 FROM game_activity_runtime_returns a WHERE a.destination_game_session_id=encode($1,'hex')::uuid AND a.source_game_session_id=d.game_session_id))")
                    .bind(session_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                if !signed { nbhold_unavailable("release_staff_unsigned", None); return Err(DurabilityError::Unavailable); }
            }
            if let Some(proof) = &store.retired_holder
                && !nb("release_retired_holder", retired_holder_release::validate(&mut tx, proof, current).await)? {
                nbhold_unavailable("release_retired_holder_refused", None);
                return Err(DurabilityError::Unavailable);
            }
            if let Some(proof) = &store.departed_activity
                && !nb("release_departed_activity", proof.validate(&mut tx, current).await)? {
                nbhold_unavailable("release_departed_activity_refused", None);
                return Err(DurabilityError::Unavailable);
            }
            if let Some(proof) = &store.offline_activity_mismatch
                && !nb("release_offline_activity", proof.validate(&mut tx, current).await)? {
                nbhold_unavailable("release_offline_activity_refused", None);
                return Err(DurabilityError::Unavailable);
            }
            let mut rows = Vec::with_capacity(2);
            for prior in &evidence.transition.predecessors { rows.push(nb("release_load_locked", store.guards.load_locked(&mut tx, &prior.key).await)?); }
            if !nb("release_successor_history_read", store.guards.successor_history_available(&mut tx, &evidence.transition.successors, &rows).await)? { nbhold_unavailable("release_successor_history", None); return Err(DurabilityError::Unavailable); }
            let encoded_successors = evidence.transition.successors.iter().map(|row| encode_guard(row, store.guards.maximum_guard_bytes)).collect::<Result<Vec<_>>>()?;
            let decided_at: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint").fetch_one(&mut *tx).await?;
            let successors = transition.validate_locked(&rows, current, decided_at).map_err(|error| { nbhold_unavailable("release_validate_locked", Some(&format!("{error:?}"))); DurabilityError::Unavailable })?;
            let slept = if let Some(bed) = bed {
                let outcome = nb("release_bed", bed.apply(&mut tx).await)?;
                if outcome != super::super::bed_sleep::BedSleepOutcome::Slept {
                    return Ok((decided_at, Some(outcome)));
                }
                Some(outcome)
            } else { None };
            if let Some(training) = &store.training_logout { nb("release_training", training.prepare(&mut tx).await)?; }
            if let Some(final_position) = store.final_position {
                nb("release_final_position", write_final_position(&mut tx, &current, &final_position, store.final_house.as_ref()).await)?;
            }
            nb("release_journal_capture", Box::pin(super::super::journal::capture::capture(&mut tx, current, store.journal_catalogue.as_deref(), decided_at)).await)?;
            let updated = sqlx::query("UPDATE game_durability_reconnect_sessions SET session_state = 3, current_transport_ref = NULL, prepared_attempt_ref = NULL WHERE game_session_id = encode($1,'hex')::uuid AND current_generation = $2::text::numeric(20,0) AND character_lease_generation = $3::text::numeric(20,0) AND scope_ownership_generation = $4::text::numeric(20,0)")
                .bind(session_id.as_bytes().as_slice()).bind(current.current_connection_generation().get().to_string()).bind(current.current_character_lease().generation().to_string()).bind(current.current_scope_generation().get().to_string()).execute(&mut *tx).await?;
            if updated.rows_affected() != 1 { nbhold_unavailable("release_session_update", None); return Err(DurabilityError::Unavailable); }
            if let Some(node) = &store.house_release_node {
                nb("release_house_entry", super::super::character_last_position::release_terminal_house_entry(&mut tx, node,
                    session_id, current.current_character_lease().character_id(), decided_at).await)?;
            }
            super::super::world_party::remove_terminal_member_in_transaction(&mut tx, session_id)
                .await.map_err(|error| { nbhold_unavailable("release_party_member", Some(&nbhold_party_error(&error))); DurabilityError::Unavailable })?;
            sqlx::query("UPDATE game_durability_reconnect_attempts SET state = 4 WHERE game_session_id = encode($1,'hex')::uuid AND state IN (1,5)")
                .bind(session_id.as_bytes().as_slice()).execute(&mut *tx).await?;
            nb("release_persist_guards", store.guards.persist_locked(&mut tx, successors, &rows, &encoded_successors).await)?;
            sqlx::query("INSERT INTO game_durability_admission_lifecycle_receipts (operation_key, operation_json, decided_at) VALUES ($1,$2,$3)")
                .bind(lifecycle_key(evidence)).bind(&encoded).bind(decided_at).execute(&mut *tx).await?;
            nb("release_commit_pass", super::super::admission_journal::commit_pass_transaction(tx, deadline).await)?;
            Ok((decided_at, slept))
        })).await;
        nb("release_run_pass", passed)
    }

    pub(super) async fn release_current_claims(
        &self,
        current: GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
        now: i64,
        account_id: &str,
    ) -> Result<ExpiredLossReleaseV1> {
        let Some(transition) = nb(
            "claims_prepare",
            Box::pin(self.prepare_current_claims_release(current, now, account_id)).await,
        )?
        else {
            return Ok(ExpiredLossReleaseV1::NotApplicable);
        };
        match Box::pin(self.release(&transition)).await {
            Ok(decided_at) => Ok(ExpiredLossReleaseV1::Released { decided_at }),
            Err(error) => match nb(
                "claims_reconcile",
                self.reconcile_lifecycle(transition.evidence()).await,
            )? {
                Some(decided_at) => Ok(ExpiredLossReleaseV1::Released { decided_at }),
                None => {
                    nb_error("claims_release_unreconciled", &error);
                    Err(error)
                }
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
        let predecessors = nb("prepare_guard_load", self.guards.load(&keys).await)?
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| {
                nbhold_unavailable("prepare_guard_missing", None);
                DurabilityError::Unavailable
            })?;
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
            Err(error) => {
                nbhold_unavailable("prepare_transition", Some(&format!("{error:?}")));
                return Err(DurabilityError::Unavailable);
            }
        };
        Ok(Some(transition))
    }
}

/// [DBG-NBHOLD] Temporary: name which terminal-release check returned `Unavailable`.
fn nbhold_unavailable(site: &str, error: Option<&str>) {
    eprintln!(
        "[DBG-NBHOLD] event=terminal_release_unavailable site={site} error={}",
        error.unwrap_or("-")
    );
}

/// [DBG-NBHOLD] Temporary: name the step that failed, and its error variant.
pub(in crate::durability) fn nb<T>(site: &str, result: Result<T>) -> Result<T> {
    if let Err(error) = &result {
        nb_error(site, error);
    }
    result
}

/// [DBG-NBHOLD] Temporary: `DurabilityError` carries no bound parameters.
pub(in crate::durability) fn nb_error(site: &str, error: &DurabilityError) {
    eprintln!("[DBG-NBHOLD] event=release_step_failed site={site} error={error:?}");
}

/// [DBG-NBHOLD] Variant and database message only; never bound parameters.
fn nbhold_party_error(error: &super::super::spell_item_transaction::SpellItemError) -> String {
    use super::super::spell_item_transaction::SpellItemError;
    match error {
        SpellItemError::Rejected(reason) => format!("rejected:{reason}"),
        SpellItemError::FieldCapabilityRefused => "field_capability_refused".into(),
        SpellItemError::Durability(error) => format!("durability:{error}"),
        SpellItemError::Database(error) => format!("database:{error}"),
        SpellItemError::Training(_) => "training".into(),
    }
}

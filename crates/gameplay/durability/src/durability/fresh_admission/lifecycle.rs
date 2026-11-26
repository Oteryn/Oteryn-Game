fn write_lifecycle(w: &mut Writer, evidence: &AdmissionClaimLifecycleEvidenceV1) -> Result<()> {
    let s = evidence.operation.current_session();
    let c = s.commit();
    w.tag(match &evidence.operation {
        AdmissionClaimLifecycleOperationV1::TerminalRelease { .. } => 1,
        AdmissionClaimLifecycleOperationV1::TerminalReplacement { .. } => 2,
    })?;
    w.text(evidence.operation.account_id())?;
    for id in [
        c.game_session_id().as_bytes(),
        c.character_id().as_bytes(),
        c.world_id().as_bytes(),
        c.channel_id().as_bytes(),
        s.current_game_session_id().as_bytes(),
    ] {
        w.bytes(id)?;
    }
    w.u64(c.character_lease_generation())?;
    w.u64(c.scope_ownership_generation())?;
    w.u64(c.connection_generation().get())?;
    w.bytes(&c.initial_transport().to_bytes())?;
    w.tag(match s.session_state() {
        GameSessionState::Active => 1,
        GameSessionState::Reconnectable => 2,
        GameSessionState::Terminal => 3,
    })?;
    w.u64(s.current_connection_generation().get())?;
    w.boolean(s.current_transport().is_some())?;
    if let Some(t) = s.current_transport() {
        w.bytes(&t.to_bytes())?;
    }
    w.bytes(s.current_character_lease().character_id().as_bytes())?;
    w.u64(s.current_character_lease().generation())?;
    w.boolean(s.current_character_world_eligibility().is_some())?;
    if let Some(e) = s.current_character_world_eligibility() {
        w.bytes(e.character_id().as_bytes())?;
        w.bytes(e.world_id().as_bytes())?;
    }
    write_scope(w, s.current_runtime_scope())?;
    w.u64(s.current_scope_generation().get())?;
    w.boolean(s.current_control_loss_epoch().is_some())?;
    if let Some(epoch) = s.current_control_loss_epoch() {
        w.u64(epoch.get())?;
    }
    w.boolean(s.current_original_grace_deadline().is_some())?;
    if let Some(grace) = s.current_original_grace_deadline() {
        w.i64(grace)?;
    }
    if let AdmissionClaimLifecycleOperationV1::TerminalReplacement { candidate, .. } =
        &evidence.operation
    {
        // Traverse borrowed fields in both passes: no JSON tree or candidate clone
        // may allocate ahead of the complete lifecycle envelope preflight.
        let identity = candidate.identity();
        w.u64(candidate.version().into())?;
        w.bytes(identity.game_session_id().as_bytes())?;
        w.bytes(&identity.reconnect_attempt_ref().to_be_bytes())?;
        w.text(identity.account_id())?;
        w.bytes(identity.character_id().as_bytes())?;
        w.bytes(identity.world_id().as_bytes())?;
        write_scope(w, identity.runtime_scope())?;
        w.u64(candidate.connection().predecessor().get())?;
        w.u64(candidate.connection().candidate().get())?;
        w.bytes(&candidate.connection().transport_ref().to_bytes())?;
        w.u64(candidate.authority().character_lease_generation())?;
        w.u64(candidate.authority().scope_ownership_generation().get())?;
        w.u64(candidate.continuity().control_loss_epoch().get())?;
        w.i64(candidate.continuity().original_grace_deadline())?;
        w.i64(candidate.continuity().prepared_deadline())?;
        match candidate.continuity().protection_entitlement() {
            ProtectionEntitlementV1::Unused => w.tag(1)?,
            ProtectionEntitlementV1::Fenced { generation } => {
                w.tag(2)?;
                w.u64(generation)?;
            }
        }
        match candidate.proof() {
            ReconnectProofV1::FastReconnect {
                reconnect_proof_generation,
            } => {
                w.tag(1)?;
                w.u64(*reconnect_proof_generation)?;
            }
            ReconnectProofV1::ReauthenticatedRecovery {
                recovery_grant_nonce,
            } => {
                w.tag(2)?;
                w.bytes(recovery_grant_nonce)?;
            }
        }
        let fnd02 = candidate.fnd02();
        w.u64(fnd02.next_command_id().get())?;
        w.u64(fnd02.pending().len() as u64)?;
        for pending in fnd02.pending() {
            w.u64(pending.command_id().get())?;
            w.tag(match pending.disposition() {
                PendingCommandDispositionV1::PendingOriginal => 1,
                PendingCommandDispositionV1::TerminalOutcomeRetained => 2,
            })?;
        }
        w.u64(fnd02.server_sequence())?;
        w.u64(fnd02.domain_revisions().len() as u64)?;
        for revision in fnd02.domain_revisions() {
            w.u64(revision.domain_id().into())?;
            w.u64(revision.revision())?;
        }
        let compatibility = candidate.compatibility();
        w.u64(compatibility.protocol_major().into())?;
        w.u64(compatibility.transport_profile().into())?;
        for value in [
            compatibility.ruleset_revision(),
            compatibility.content_revision(),
            compatibility.map_revision(),
            compatibility.world_policy_revision(),
        ] {
            w.text(value)?;
        }
        w.u64(compatibility.account_security_generation())?;
        for fence in [
            compatibility.platform_security_evidence(),
            compatibility.proof_trust_evidence(),
        ] {
            w.text(fence.authority())?;
            w.text(fence.purpose())?;
            w.text(fence.scope())?;
            w.text(fence.source_revision())?;
            w.text(fence.decision_identity())?;
            w.i64(fence.source_observed_at())?;
        }
        w.boolean(compatibility.credential_expiration().is_some())?;
        if let Some(expiration) = compatibility.credential_expiration() {
            w.i64(expiration)?;
        };
    }
    write_changes(w, &evidence.transition.predecessors)?;
    write_changes(w, &evidence.transition.successors)?;
    w.i64(evidence.transition.prepared_at)?;
    Ok(())
}

pub(super) fn encode_lifecycle(evidence: &AdmissionClaimLifecycleEvidenceV1) -> Result<String> {
    let mut size = Writer::counter(super::MAX_FRESH_OPERATION_BYTES);
    write_lifecycle(&mut size, evidence)?;
    envelope_size(size.measured(), super::MAX_FRESH_OPERATION_BYTES)?;
    checked(evidence.validate_historical(evidence.transition.prepared_at))?;
    let mut writer = Writer::new(super::MAX_FRESH_OPERATION_BYTES);
    write_lifecycle(&mut writer, evidence)?;
    encode_envelope(&writer.bytes, super::MAX_FRESH_OPERATION_BYTES)
}

fn lifecycle_key(evidence: &AdmissionClaimLifecycleEvidenceV1) -> Vec<u8> {
    let mut key = b"claim-lifecycle-v1".to_vec();
    key.extend_from_slice(
        evidence
            .operation
            .current_session()
            .current_game_session_id()
            .as_bytes(),
    );
    match &evidence.operation {
        AdmissionClaimLifecycleOperationV1::TerminalRelease { .. } => key.push(1),
        AdmissionClaimLifecycleOperationV1::TerminalReplacement { candidate, .. } => {
            key.push(2);
            key.extend_from_slice(candidate.identity().game_session_id().as_bytes());
        }
    }
    key
}

async fn lifecycle_replay(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    evidence: &AdmissionClaimLifecycleEvidenceV1,
    encoded: &str,
) -> Result<Option<i64>> {
    use sqlx::Row;
    let row = sqlx::query("SELECT CASE WHEN octet_length(operation_json) <= 65536 THEN operation_json END AS operation_json, decided_at FROM game_durability_admission_lifecycle_receipts WHERE operation_key = $1")
        .bind(lifecycle_key(evidence)).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row
        .try_get::<Option<String>, _>("operation_json")?
        .as_deref()
        != Some(encoded)
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let decided_at = row.try_get::<i64, _>("decided_at")?;
    checked(evidence.validate_historical(decided_at))?;
    Ok(Some(decided_at))
}

impl FreshAdmissionStore {
    /// Historical one-hop successor lookup for an authenticated handoff retry.
    /// This never attaches a controller or authorizes a fresh transport. The
    /// connection must apply its existing current-control/reconnect checks to the
    /// returned destination before resending the acknowledgement.
    pub async fn activity_successor_in_transaction(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        character: CharacterId,
        source: GameSessionId,
    ) -> Result<Option<GameSessionId>> {
        if !self.committed_session_use(tx, character, source).await? {
            return Ok(None);
        }
        let rows: Vec<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(destination_game_session_id) FROM (SELECT p.destination_game_session_id FROM game_activity_handoff_participants p JOIN game_activity_handoff_groups g USING(handoff_id) WHERE p.character_id=encode($1,'hex')::uuid AND p.source_game_session_id=encode($2,'hex')::uuid AND g.state=2 UNION ALL SELECT r.destination_game_session_id FROM game_activity_runtime_returns r WHERE r.character_id=encode($1,'hex')::uuid AND r.source_game_session_id=encode($2,'hex')::uuid) destinations LIMIT 2")
            .bind(character.as_bytes().as_slice()).bind(source.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
        let bytes = match rows.as_slice() {
            [] => return Ok(None),
            [one] => one,
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        let destination = checked(GameSessionId::decode(bytes))?;
        if !self
            .committed_activity_origin_in_transaction(tx, character, destination)
            .await?
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        Ok(Some(destination))
    }

    /// Retained activity origin under the caller's admission relation locks.
    /// This proves issuance, lineage and transport history, not current control:
    /// consumers separately compare the current session/lease/scope and holder.
    pub async fn committed_activity_origin_in_transaction(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        character: CharacterId,
        session: GameSessionId,
    ) -> Result<bool> {
        let activity: bool = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.game_session_id=encode($1,'hex')::uuid AND s.character_id=encode($2,'hex')::uuid AND (s.activity_handoff_id IS NOT NULL OR s.activity_return_id IS NOT NULL))")
            .bind(session.as_bytes().as_slice()).bind(character.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        if !activity {
            return Ok(false);
        }
        self.committed_session_use(tx, character, session).await
    }

    /// Resolve current admission inside the caller's actual owner transaction.
    /// The immutable admission receipt alone never supplies current authority.
    pub async fn current_session_in_transaction(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        session: GameSessionId,
    ) -> Result<GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>> {
        self.current_session_locked(tx, session).await
    }

    async fn current_session_locked(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        session: GameSessionId,
    ) -> Result<GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>> {
        let activity_origin: Option<bool> = sqlx::query_scalar("SELECT COALESCE(game_activity_session_origin_valid(s.game_session_id,s.character_id),false) FROM game_durability_reconnect_sessions s WHERE s.game_session_id=encode($1,'hex')::uuid")
            .bind(session.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        if activity_origin != Some(true) {
            return Err(DurabilityError::Unavailable);
        }
        use sqlx::Row;
        // The current and initial keys select the same immutable origin used
        // for replay. Read its bounded mirrors with the payload in this snapshot.
        let row = sqlx::query(
            "SELECT CASE WHEN octet_length(r.operation_json) <= $2 THEN r.operation_json END AS payload, \
                    CASE WHEN octet_length(to_jsonb(r)::text) <= $3 THEN to_jsonb(r) - 'operation_json' END AS mirrors \
             FROM game_durability_reconnect_sessions s \
             JOIN game_durability_fresh_admission_receipts r \
               ON r.replay_key = COALESCE(s.fresh_replay_key,s.initial_fresh_replay_key) \
             WHERE s.game_session_id = encode($1,'hex')::uuid",
        )
        .bind(session.as_bytes().as_slice())
        .bind(checked(i64::try_from(self.maximum_operation_bytes))?)
        .bind(super::MAX_ADMISSION_ROW_BYTES)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(DurabilityError::Unavailable)?;
        let payload: Option<String> = row.try_get("payload")?;
        let original = decode_operation(
            &payload.ok_or(DurabilityError::Unavailable)?,
            self.maximum_operation_bytes,
        )?;
        let mirrors: Option<serde_json::Value> = row.try_get("mirrors")?;
        let receipt =
            Self::restore_receipt(original, mirrors.ok_or(DurabilityError::InvalidStoredState)?)?;
        Ok(self
            .read_reconciled_session(tx, receipt, session)
            .await?
            .current_session)
    }

    pub async fn current_session(
        &self,
        session: GameSessionId,
    ) -> Result<GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>> {
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
                    super::admission_journal::commit_pass_read(tx, deadline).await?;
                    Ok(current)
                })
            })
            .await
    }

    /// The current canonical session together with the database clock sampled
    /// in the same fenced pass. Loss observations are timed on this clock: the
    /// final decision samples it again, so a faster host clock cannot refuse
    /// a loss (`observed_at > decided_at`) or shorten its grace.
    pub async fn current_session_at(
        &self,
        session: GameSessionId,
    ) -> Result<(
        GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
        i64,
    )> {
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
                    let now: i64 = sqlx::query_scalar(
                        "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    super::admission_journal::commit_pass_read(tx, deadline).await?;
                    Ok((current, now))
                })
            })
            .await
    }

    /// CHAR-POSITION-1 (`CHARPOS0-RL-01`): the current session of each of `sessions`, read as
    /// [`Self::current_session_at`] reads one, but all in one root pass, so a periodic
    /// last-position write pass reads a bounded number of passes whatever the number of its
    /// sessions. One entry per session, in order; `None` when that session's read failed. A
    /// storage error of the pass itself fails all of them.
    pub async fn current_sessions_at(
        &self,
        sessions: Vec<GameSessionId>,
    ) -> Result<Vec<Option<GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>>>> {
        if sessions.is_empty() {
            return Ok(Vec::new());
        }
        let store = self.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let mut current = Vec::with_capacity(sessions.len());
                    for session in sessions {
                        current.push(store.current_session_locked(&mut tx, session).await.ok());
                    }
                    super::admission_journal::commit_pass_read(tx, deadline).await?;
                    Ok(current)
                })
            })
            .await
    }

    pub async fn reconcile_lifecycle(
        &self,
        evidence: &AdmissionClaimLifecycleEvidenceV1,
    ) -> Result<Option<i64>> {
        let encoded = encode_lifecycle(evidence)?;
        let evidence = evidence.clone();
        let backend = self.guards.backend.clone();
        let issued = backend.try_issue_root()?;
        backend
            .run_pass(issued, move |holder, deadline| {
                Box::pin(async move {
                    let mut tx =
                        super::admission_journal::begin_pass_transaction(holder, deadline).await?;
                    super::db::lock_admission_relations(&mut tx).await?;
                    let result = lifecycle_replay(&mut tx, &evidence, &encoded).await?;
                    super::admission_journal::commit_pass_read(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }
}

/// CHAR-POSITION-0 §3.2 "At logout": the final last-position upsert of the session being
/// released, in the release transaction (which holds the admission relation locks and has just
/// revalidated the session), before `session_state` becomes 3. A Channel scope of the
/// record's World or a separately proven named-House capture writes; the upsert itself
/// skips a pending respawn and a later stored key.
async fn write_final_position(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    current: &GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
    final_position: &super::character_last_position::FinalLastPosition,
    house: Option<&(
        super::character_last_position::HouseLastPosition,
        super::runtime_scope_assignment::NodeIncarnationProof,
    )>,
    pending_vocation_refill: Option<super::character_build::BuildOccurrence>,
) -> Result<()> {
    if final_position.write_sequence == 0 {
        return Ok(());
    }
    let lease = current.current_character_lease();
    let named_house = match (current.current_runtime_scope(), house) {
        (RuntimeScopeRefV1::Channel { world_id, .. }, None)
            if world_id == final_position.record.world_id =>
        {
            None
        }
        (RuntimeScopeRefV1::Instance { world_id, .. }, Some((saved, node)))
            if world_id == final_position.record.world_id
                && saved.house.runtime_scope()? == current.current_runtime_scope() =>
        {
            let fence = super::item_transfer::CurrentCharacterItemFence {
                character_id: crate::domain::CharacterId::from_bytes(
                    *lease.character_id().as_bytes(),
                )
                .map_err(|_| DurabilityError::InvalidStoredState)?,
                game_session_id: current.current_game_session_id(),
                connection_generation: current.current_connection_generation(),
                character_lease_generation: lease.generation(),
                runtime_scope: current.current_runtime_scope(),
                scope_ownership_generation: current.current_scope_generation(),
            };
            if super::house_hireling::fence::assert_house_custody_fence(
                tx,
                node,
                &fence,
                *saved.origin_channel.as_bytes(),
            )
            .await?
            .as_ref()
                != Some(&saved.house)
            {
                // Terminal release must remain available after ownership,
                // eligibility or node authority is lost. Keep the last valid
                // projection; never retain Account/Character claims for a save.
                return Ok(());
            }
            Some(saved)
        }
        _ => return Ok(()),
    };
    let mut record = final_position.record;
    if let Some(occurrence) = pending_vocation_refill {
        let character = crate::domain::CharacterId::from_bytes(*lease.character_id().as_bytes())
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if super::character_last_position::committed_vocation_refill(tx, character, occurrence).await?
            && let Some(stored) =
            super::character_last_position::load_last_position(tx, character).await?
        {
            if stored.record.cell == record.cell && stored.record.rest == record.rest {
                return Ok(());
            }
            record.vitals = stored.record.vitals;
        }
    }
    super::character_last_position::upsert_scoped_last_position(
        tx,
        lease.character_id().as_bytes(),
        super::character_last_position::LastPositionKey {
            character_lease_generation: lease.generation(),
            write_sequence: final_position.write_sequence,
        },
        &record,
        named_house,
    )
    .await?;
    Ok(())
}

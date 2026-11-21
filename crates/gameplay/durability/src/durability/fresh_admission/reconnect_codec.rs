// ---------------------------------------------------------------------------
// Complete reconnect (FND-04B §§11–13, §20): canonical same-session receipts.
// ---------------------------------------------------------------------------

fn write_session_snapshot(
    w: &mut Writer,
    s: &GameSessionAuthoritySnapshot<AuthenticatedTransportRefV1>,
) -> Result<()> {
    let c = s.commit();
    w.bytes(c.game_session_id().as_bytes())?;
    w.bytes(c.character_id().as_bytes())?;
    w.bytes(c.world_id().as_bytes())?;
    w.bytes(c.channel_id().as_bytes())?;
    w.u64(c.character_lease_generation())?;
    w.u64(c.scope_ownership_generation())?;
    w.u64(c.connection_generation().get())?;
    w.bytes(&c.initial_transport().to_bytes())?;
    w.bytes(s.current_game_session_id().as_bytes())?;
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
    if let Some(e) = s.current_control_loss_epoch() {
        w.u64(e.get())?;
    }
    w.boolean(s.current_original_grace_deadline().is_some())?;
    if let Some(t) = s.current_original_grace_deadline() {
        w.i64(t)?;
    }
    Ok(())
}

fn write_protection(w: &mut Writer, p: RecoveryProtectionContinuityV1) -> Result<()> {
    match p.usage {
        RecoveryProtectionUseV1::NotEntitled => w.tag(0)?,
        RecoveryProtectionUseV1::Unused {
            entitlement_generation,
        } => {
            w.tag(1)?;
            w.u64(entitlement_generation)?;
        }
        RecoveryProtectionUseV1::Activated {
            entitlement_generation,
            activated_at,
            deadline,
        } => {
            w.tag(2)?;
            w.u64(entitlement_generation)?;
            w.i64(activated_at)?;
            w.i64(deadline)?;
        }
    }
    match p.rearm {
        RecoveryProtectionRearmV1::Satisfied {
            generation,
            established_at,
        } => {
            w.tag(1)?;
            w.u64(generation)?;
            w.i64(established_at)?;
        }
        RecoveryProtectionRearmV1::NotRearmed {
            generation,
            stable_control_started_at,
            accepted_deadline,
        } => {
            w.tag(0)?;
            w.u64(generation)?;
            for time in [stable_control_started_at, accepted_deadline] {
                w.boolean(time.is_some())?;
                if let Some(time) = time {
                    w.i64(time)?;
                }
            }
        }
    }
    Ok(())
}

fn write_candidate(w: &mut Writer, c: ReconnectCandidateBindingV1) -> Result<()> {
    w.bytes(c.game_session_id().as_bytes())?;
    w.bytes(&c.reconnect_attempt_ref().to_be_bytes())?;
    w.u64(c.connection_generation().get())?;
    w.bytes(&c.transport_ref().to_bytes())?;
    w.i64(c.prepared_deadline())
}

fn write_budget(w: &mut Writer, b: &RetainedRecoveryBudgetV1) -> Result<()> {
    w.u64(b.epoch().get())?;
    w.tag(match b.state() {
        RecoveryEpochStateV1::Open => 1,
        RecoveryEpochStateV1::Restored => 2,
        RecoveryEpochStateV1::Retired => 3,
    })?;
    w.tag(checked(u8::try_from(b.entries().len()))?)?;
    for e in b.entries() {
        w.bytes(&e.attempt.to_be_bytes())?;
        w.bytes(&e.transport.to_bytes())?;
        w.tag(match e.disposition {
            RetainedRecoveryAttemptDispositionV1::Committed => 1,
            RetainedRecoveryAttemptDispositionV1::Prepared => 2,
            RetainedRecoveryAttemptDispositionV1::TransportCollision => 3,
            RetainedRecoveryAttemptDispositionV1::Terminal => 4,
        })?;
    }
    Ok(())
}

fn write_recovery_revisions(w: &mut Writer, revisions: [&str; 4]) -> Result<()> {
    for revision in revisions {
        w.text(revision)?;
    }
    Ok(())
}

fn write_complete_reconnect(
    w: &mut Writer,
    operation: &CompleteReconnectDurabilityOperationV1,
) -> Result<()> {
    let recovery = &operation.recovery;
    let original = &recovery.original;
    // Only same-session reauthenticated recovery is durable here; a replacement onto a
    // new session and fast-reconnect proof delivery have no owner yet.
    let CompleteReconnectCredentialV1::Recovery(credential) = &recovery.credential else {
        return Err(DurabilityError::Unavailable);
    };
    if recovery.version != 1
        || recovery.mode != CompleteReconnectModeV1::SameSession
        || operation.replacement.is_some()
        || original.replacement_anchor.is_some()
    {
        return Err(DurabilityError::Unavailable);
    }
    w.tag(1)?;
    let identity = &recovery.identity;
    w.bytes(identity.game_session_id().as_bytes())?;
    w.bytes(&identity.reconnect_attempt_ref().to_be_bytes())?;
    w.text(identity.account_id())?;
    w.bytes(identity.character_id().as_bytes())?;
    w.bytes(identity.world_id().as_bytes())?;
    write_scope(w, identity.runtime_scope())?;
    w.i64(recovery.prepared_at)?;
    w.tag(checked(u8::try_from(original.predecessor_attempts.len()))?)?;
    for attempt in &original.predecessor_attempts {
        w.bytes(attempt.session.as_bytes())?;
        w.bytes(&attempt.attempt.to_be_bytes())?;
        w.bytes(&attempt.transport.to_bytes())?;
    }
    write_fresh_loss(w, &original.loss)?;
    w.i64(original.loss_decided_at)?;
    write_scope(w, original.source_authority)?;
    w.u64(original.source_revision)?;
    w.u64(original.accepted_source_revision)?;
    w.i64(original.observed_at)?;
    write_session_snapshot(w, &original.session)?;
    w.text(original.account_presence.account_id())?;
    w.bytes(original.account_presence.character_id().as_bytes())?;
    w.boolean(original.actor_present)?;
    w.boolean(original.runtime_ready)?;
    w.bytes(&original.placement_identity)?;
    w.u64(original.placement_revision)?;
    write_protection(w, original.protection)?;
    write_budget(w, &original.budget)?;
    write_candidate(w, original.candidate)?;
    let proof = &original.proof_transition;
    write_scope(w, proof.owner)?;
    w.u64(proof.revision)?;
    w.u64(proof.accepted_revision)?;
    w.i64(proof.observed_at)?;
    w.bytes(proof.predecessor_session.as_bytes())?;
    w.u64(proof.predecessor_generation)?;
    w.bytes(proof.successor_session.as_bytes())?;
    w.u64(proof.successor_generation)?;
    write_candidate(w, proof.candidate)?;
    let fnd02 = &original.fnd02;
    w.u64(fnd02.next_command_id().get())?;
    w.tag(checked(u8::try_from(fnd02.pending().len()))?)?;
    for pending in fnd02.pending() {
        w.u64(pending.command_id().get())?;
        w.tag(match pending.disposition() {
            PendingCommandDispositionV1::PendingOriginal => 1,
            PendingCommandDispositionV1::TerminalOutcomeRetained => 2,
        })?;
    }
    w.u64(fnd02.server_sequence())?;
    w.tag(checked(u8::try_from(fnd02.domain_revisions().len()))?)?;
    for revision in fnd02.domain_revisions() {
        w.u64(u64::from(revision.domain_id()))?;
        w.u64(revision.revision())?;
    }
    let current = &original.recovery;
    w.text(&current.account_id)?;
    w.bytes(current.character_id.as_bytes())?;
    w.bytes(current.world_id.as_bytes())?;
    write_recovery_revisions(
        w,
        [
            &current.ruleset_revision,
            &current.content_revision,
            &current.map_revision,
            &current.world_policy_revision,
        ],
    )?;
    write_changes(w, &original.claims)?;
    w.tag(1)?;
    w.bytes(&credential.grant_nonce)?;
    w.u64(credential.account_security_generation)?;
    w.u64(credential.protocol_major)?;
    w.u64(credential.transport_profile)?;
    write_recovery_revisions(w, credential.revisions.each_ref().map(String::as_str))?;
    w.i64(credential.expires_at)?;
    w.boolean(credential.v1_trust.is_some())?;
    if let Some(trust) = &credential.v1_trust {
        w.text(&trust.signing_key_id)?;
        w.bytes(&trust.signing_public_key)?;
        w.u64(trust.minimum_generation)?;
    }
    w.boolean(credential.v2.is_some())?;
    if let Some(audit) = &credential.v2 {
        w.bytes(&audit.credential_attempt_ref)?;
        w.bytes(&audit.grant_nonce)?;
        w.text(&audit.account_id)?;
        w.bytes(audit.character_id.as_bytes())?;
        w.bytes(audit.world_id.as_bytes())?;
        w.u64(audit.account_security_generation)?;
        w.u64(audit.protocol_major)?;
        w.u64(audit.transport_profile)?;
        write_recovery_revisions(
            w,
            [
                &audit.ruleset_revision,
                &audit.content_revision,
                &audit.map_revision,
                &audit.world_policy_revision,
            ],
        )?;
        w.i64(audit.issued_at)?;
        w.i64(audit.not_before)?;
        w.i64(audit.expires_at)?;
        w.text(&audit.signing.key_id)?;
        w.bytes(&audit.signing.public_key)?;
        w.boolean(audit.signing.trusted)?;
        write_provenance(w, &audit.signing.provenance)?;
        w.text(&audit.security.account_id)?;
        w.u64(audit.security.minimum_generation)?;
        w.boolean(audit.security.allowed)?;
        write_provenance(w, &audit.security.provenance)?;
        w.i64(audit.verified_at)?;
        w.i64(audit.accepted_deadline)?;
    }
    Ok(())
}

/// Canonical bounded bytes of one same-session complete-reconnect operation.
/// Receipts compare these in full; no digest grants identity.
pub fn encode_complete_reconnect(
    operation: &CompleteReconnectDurabilityOperationV1,
) -> Result<String> {
    checked(operation.validate_historical())?;
    let maximum = super::MAX_FRESH_OPERATION_BYTES;
    let mut counter = Writer::counter(maximum);
    write_complete_reconnect(&mut counter, operation)?;
    envelope_size(counter.measured(), maximum)?;
    let mut writer = Writer::new(maximum);
    write_complete_reconnect(&mut writer, operation)?;
    encode_envelope(&writer.bytes, maximum)
}

const COMPLETE_PREPARE_KEY: &[u8] = b"complete-reconnect-prepare-v1";
const COMPLETE_COMMIT_KEY: &[u8] = b"complete-reconnect-commit-v1";

fn complete_reconnect_prefix(phase: &[u8], session: GameSessionId, epoch: u64) -> Vec<u8> {
    let mut key = phase.to_vec();
    key.extend_from_slice(session.as_bytes());
    key.extend_from_slice(&epoch.to_be_bytes());
    key
}

fn complete_reconnect_key(phase: &[u8], identity: &ReconnectIdentityV1, epoch: u64) -> Vec<u8> {
    let mut key = complete_reconnect_prefix(phase, identity.game_session_id(), epoch);
    key.extend_from_slice(&identity.reconnect_attempt_ref().to_be_bytes());
    key
}

use super::super::MAX_FRESH_OPERATION_BYTES;
use super::*;

// Canonical complete encoding of the supported owning-loss shapes: a fresh-origin loss
// and a loss after a same-session resume, which retains the resumed epoch's history.
// Every omitted alternative is rejected, never silently projected or defaulted.
pub(super) fn write_fresh_loss(w: &mut Writer, operation: &ControlLossOperationV1) -> Result<()> {
    let o = &operation.observation;
    if operation.version != 1 {
        return Err(DurabilityError::Unavailable);
    }
    w.tag(1)?;
    w.i64(operation.authorized_at)?;
    write_scope(w, o.source_authority)?;
    w.u64(o.source_revision)?;
    w.u64(o.accepted_source_revision)?;
    w.u64(o.decision_identity.get())?;
    w.u64(o.accepted_decision_identity.get())?;
    w.i64(o.observed_at)?;
    let s = o.session;
    let c = s.commit();
    w.bytes(c.game_session_id().as_bytes())?;
    w.bytes(c.character_id().as_bytes())?;
    w.bytes(c.world_id().as_bytes())?;
    w.bytes(c.channel_id().as_bytes())?;
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
    if let Some(e) = s.current_control_loss_epoch() {
        w.u64(e.get())?;
    }
    w.boolean(s.current_original_grace_deadline().is_some())?;
    if let Some(t) = s.current_original_grace_deadline() {
        w.i64(t)?;
    }
    w.text(o.account_presence.account_id())?;
    w.bytes(o.account_presence.character_id().as_bytes())?;
    w.bytes(&o.placement_identity)?;
    w.u64(o.placement_revision)?;
    w.boolean(o.actor_present)?;
    w.boolean(o.runtime_ready)?;
    write_loss_cause(w, o.cause)?;
    w.u64(o.loss_epoch.get())?;
    w.i64(o.loss_origin)?;
    w.i64(o.original_grace_deadline)?;
    match &o.history {
        ControlLossHistoryV1::FreshOrigin => w.tag(1)?,
        ControlLossHistoryV1::Resumed {
            budget,
            original_grace_deadline,
            protection,
        } => {
            w.tag(2)?;
            write_budget(w, budget)?;
            w.i64(*original_grace_deadline)?;
            write_protection(w, *protection)?;
        }
    }
    write_protection(w, o.protection)
}
pub fn encode_fresh_loss(operation: &ControlLossOperationV1) -> Result<String> {
    let maximum = MAX_FRESH_OPERATION_BYTES;
    let mut counter = Writer::counter(maximum);
    write_fresh_loss(&mut counter, operation)?;
    envelope_size(counter.measured(), maximum)?;
    checked(ControlLossFlowV1::restore(operation.clone()))?;
    let mut writer = Writer::new(maximum);
    write_fresh_loss(&mut writer, operation)?;
    encode_envelope(&writer.bytes, maximum)
}

pub(super) fn read_protection(r: &mut Reader<'_>) -> Result<RecoveryProtectionContinuityV1> {
    let usage = match r.tag()? {
        0 => RecoveryProtectionUseV1::NotEntitled,
        1 => RecoveryProtectionUseV1::Unused {
            entitlement_generation: r.u64()?,
        },
        2 => RecoveryProtectionUseV1::Activated {
            entitlement_generation: r.u64()?,
            activated_at: r.i64()?,
            deadline: r.i64()?,
        },
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let rearm = match r.tag()? {
        0 => RecoveryProtectionRearmV1::NotRearmed {
            generation: r.u64()?,
            stable_control_started_at: if r.boolean()? { Some(r.i64()?) } else { None },
            accepted_deadline: if r.boolean()? { Some(r.i64()?) } else { None },
        },
        1 => RecoveryProtectionRearmV1::Satisfied {
            generation: r.u64()?,
            established_at: r.i64()?,
        },
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    Ok(RecoveryProtectionContinuityV1 { usage, rearm })
}

pub(super) fn read_budget(r: &mut Reader<'_>) -> Result<RetainedRecoveryBudgetV1> {
    let epoch = checked(ControlLossEpochRefV1::new(r.u64()?))?;
    let state = match r.tag()? {
        1 => RecoveryEpochStateV1::Open,
        2 => RecoveryEpochStateV1::Restored,
        3 => RecoveryEpochStateV1::Retired,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let count = r.tag()?;
    let mut entries = Vec::new();
    for _ in 0..count {
        let attempt = checked(ReconnectAttemptRef::new(u64::from_be_bytes(
            r.bytes::<8>()?,
        )))?;
        let transport = checked(AuthenticatedTransportRefV1::decode(&r.bytes::<16>()?))?;
        let disposition = match r.tag()? {
            1 => RetainedRecoveryAttemptDispositionV1::Committed,
            2 => RetainedRecoveryAttemptDispositionV1::Prepared,
            3 => RetainedRecoveryAttemptDispositionV1::TransportCollision,
            4 => RetainedRecoveryAttemptDispositionV1::Terminal,
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        entries.push(RetainedRecoveryAttemptV1 {
            attempt,
            transport,
            disposition,
        });
    }
    checked(RetainedRecoveryBudgetV1::restore(
        epoch, state, true, entries,
    ))
}

/// Restore bounded historical loss bytes against the actual durable fresh
/// receipt's initial commit. No replay key or live authority is synthesized.
pub fn decode_fresh_loss(
    encoded: &str,
    initial: FreshAdmissionCommit<AuthenticatedTransportRefV1>,
) -> Result<ControlLossOperationV1> {
    let bytes = decode_envelope(encoded, MAX_FRESH_OPERATION_BYTES)?;
    let mut r = Reader::new(&bytes);
    if r.tag()? != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let authorized_at = r.i64()?;
    let source_authority = read_scope(&mut r)?;
    let source_revision = r.u64()?;
    let accepted_source_revision = r.u64()?;
    let decision_identity = checked(ControlLossEpochRefV1::new(r.u64()?))?;
    let accepted_decision_identity = checked(ControlLossEpochRefV1::new(r.u64()?))?;
    let observed_at = r.i64()?;
    if r.bytes::<16>()? != *initial.game_session_id().as_bytes()
        || r.bytes::<16>()? != *initial.character_id().as_bytes()
        || r.bytes::<16>()? != *initial.world_id().as_bytes()
        || r.bytes::<16>()? != *initial.channel_id().as_bytes()
        || r.u64()? != initial.character_lease_generation()
        || r.u64()? != initial.scope_ownership_generation()
        || r.u64()? != initial.connection_generation().get()
        || r.bytes::<16>()? != initial.initial_transport().to_bytes()
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    let state = match r.tag()? {
        1 => GameSessionState::Active,
        2 => GameSessionState::Reconnectable,
        3 => GameSessionState::Terminal,
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let generation = checked(ConnectionGeneration::new(r.u64()?))?;
    let transport = if r.boolean()? {
        Some(checked(AuthenticatedTransportRefV1::decode(
            &r.bytes::<16>()?,
        ))?)
    } else {
        None
    };
    let lease = checked(CharacterLease::new(
        checked(CharacterId::decode(&r.bytes::<16>()?))?,
        r.u64()?,
    ))?;
    let eligibility = if r.boolean()? {
        Some(CharacterWorldEligibilityClaimV1::new(
            checked(CharacterId::decode(&r.bytes::<16>()?))?,
            checked(WorldId::decode(&r.bytes::<16>()?))?,
        ))
    } else {
        None
    };
    let scope = read_scope(&mut r)?;
    let scope_generation = checked(ScopeOwnershipGeneration::new(r.u64()?))?;
    let epoch = if r.boolean()? {
        Some(checked(ControlLossEpochRefV1::new(r.u64()?))?)
    } else {
        None
    };
    let grace = if r.boolean()? { Some(r.i64()?) } else { None };
    let mut session = checked(GameSessionAuthoritySnapshot::from_current_facts(
        initial,
        state,
        generation,
        transport,
        lease,
        eligibility,
        scope,
        scope_generation,
    ))?;
    match (epoch, grace) {
        (Some(epoch), Some(grace)) => {
            session = checked(session.with_control_loss_continuity(epoch, grace))?
        }
        (None, None) => {}
        _ => return Err(DurabilityError::InvalidStoredState),
    }
    let account_presence = checked(AccountPresenceClaimV1::new(
        &r.text()?,
        checked(CharacterId::decode(&r.bytes::<16>()?))?,
    ))?;
    let placement_identity = r.bytes::<16>()?;
    let placement_revision = r.u64()?;
    let actor_present = r.boolean()?;
    let runtime_ready = r.boolean()?;
    let cause = read_loss_cause(&mut r)?;
    let loss_epoch = checked(ControlLossEpochRefV1::new(r.u64()?))?;
    let loss_origin = r.i64()?;
    let original_grace_deadline = r.i64()?;
    let history = match r.tag()? {
        1 => ControlLossHistoryV1::FreshOrigin,
        2 => ControlLossHistoryV1::Resumed {
            budget: read_budget(&mut r)?,
            original_grace_deadline: r.i64()?,
            protection: read_protection(&mut r)?,
        },
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let protection = read_protection(&mut r)?;
    r.finish()?;
    let operation = ControlLossOperationV1 {
        version: 1,
        authorized_at,
        observation: ControlLossObservationV1 {
            source_authority,
            source_revision,
            accepted_source_revision,
            decision_identity,
            accepted_decision_identity,
            observed_at,
            session,
            account_presence,
            placement_identity,
            placement_revision,
            actor_present,
            runtime_ready,
            cause,
            loss_epoch,
            loss_origin,
            original_grace_deadline,
            history,
            protection,
        },
    };
    // Historical Foundation validation cannot yield another live request.
    checked(ControlLossFlowV1::restore(operation.clone()))?;
    Ok(operation)
}

fn write_loss_cause(w: &mut Writer, cause: ControlLossCauseV1) -> Result<()> {
    match cause {
        ControlLossCauseV1::AuthoritativeUnexpectedLoss => w.tag(1),
        ControlLossCauseV1::PlannedWorldReset {
            world_id,
            reset_epoch,
            retirement_binding,
            target_digest,
        } => {
            w.tag(2)?;
            w.bytes(world_id.as_bytes())?;
            w.u64(reset_epoch)?;
            w.bytes(&retirement_binding)?;
            w.bytes(&target_digest)
        }
        _ => Err(DurabilityError::Unavailable),
    }
}

fn read_loss_cause(r: &mut Reader<'_>) -> Result<ControlLossCauseV1> {
    match r.tag()? {
        1 => Ok(ControlLossCauseV1::AuthoritativeUnexpectedLoss),
        2 => Ok(ControlLossCauseV1::PlannedWorldReset {
            world_id: checked(WorldId::decode(&r.bytes::<16>()?))?,
            reset_epoch: r.u64()?,
            retirement_binding: r.bytes::<32>()?,
            target_digest: r.bytes::<32>()?,
        }),
        _ => Err(DurabilityError::InvalidStoredState),
    }
}

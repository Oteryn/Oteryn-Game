//! Own-actor cooldown publication for commands, recipient refreshes and probe corrections.

use super::*;
use oteryn_protocol_oteryn::presentation as wire;

pub(super) async fn refresh_live<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
) -> Result<(), ServeEnd> {
    refresh(
        stream,
        authority,
        &mut session.admitted,
        &mut session.cooldowns,
    )
    .await?;
    session.sequence = session.admitted.continuity.server_sequence;
    Ok(())
}

pub(super) async fn refresh<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    admitted: &mut AdmittedSession,
    publisher: &mut SpellCooldownPublisher,
) -> Result<(), ServeEnd> {
    let continuity = &admitted.continuity;
    if !continuity
        .selected_capabilities
        .contains(wire::CAPABILITY_PRESENTATION_V1)
    {
        return Ok(());
    }
    let payload = match authority
        .observe_spell_cooldowns(*admitted, publisher, false)
        .await
    {
        CooldownObservation::Changed(payload) => payload,
        CooldownObservation::Unchanged | CooldownObservation::Stale => return Ok(()),
        // Keep the last published state. The next visibility turn (House
        // service turn in House scope) retries from the current committed owner.
        CooldownObservation::Unavailable => return Ok(()),
    };
    let base = continuity.cooldown_revision;
    let revision = base.checked_add(1).ok_or(ServeEnd::Disconnected)?;
    let sequence = continuity
        .server_sequence
        .checked_add(1)
        .ok_or(ServeEnd::Disconnected)?;
    let frame = encode_state_delta(
        continuity.connection_generation,
        sequence,
        wire::STATE_DOMAIN_ACTOR_COOLDOWNS,
        base,
        revision,
        wire::DELTA_TYPE_ACTOR_COOLDOWNS_V1,
        &payload,
    )
    .map_err(|_| ServeEnd::Disconnected)?;
    write_frame(stream, &frame)
        .await
        .map_err(|_| ServeEnd::Disconnected)?;
    admitted.continuity.cooldown_revision = revision;
    admitted.continuity.server_sequence = sequence;
    Ok(())
}

pub(super) async fn ack_live<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
    stream: &mut S,
    authority: &A,
    session: &mut ServeState,
    id: u64,
) -> Result<(), ServeEnd> {
    if let Err(error) = session.liveness.ack(id) {
        return Err(close(stream, session.admitted, error).await);
    }
    if session
        .cooldowns
        .live_probe_ack(id, authority.spell_cooldown_now())
    {
        refresh_live(stream, authority, session).await?;
    }
    Ok(())
}

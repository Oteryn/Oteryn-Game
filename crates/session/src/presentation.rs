//! Connection-local presentation revisions; transient cues are never replayed from snapshots.
use super::*;
use oteryn_protocol_oteryn::presentation::*;

pub(super) fn validate_join(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
) -> Result<(), SessionError> {
    if !selected.contains(&CAPABILITY_PRESENTATION_V1) {
        return Ok(());
    }
    for domain in [
        STATE_DOMAIN_WORLD_PRESENTATION,
        STATE_DOMAIN_ACTOR_COOLDOWNS,
    ] {
        let row = snapshots
            .iter()
            .find(|row| row.domain_id == domain)
            .ok_or(SessionError::MissingDomain(domain))?;
        if row.snapshot_type != 1 {
            return Err(SessionError::SessionUnusable);
        }
        if domain == STATE_DOMAIN_WORLD_PRESENTATION {
            decode_presentation_snapshot(&row.payload)
                .map_err(|_| SessionError::SessionUnusable)?;
        } else {
            decode_actor_cooldowns(&row.payload).map_err(|_| SessionError::SessionUnusable)?;
        }
    }
    Ok(())
}

pub(super) fn apply<S: SessionStream>(
    session: &mut Session<S>,
    sequence: u64,
    payload: &[u8],
) -> Result<SessionEvent, SessionError> {
    let delta = decode_state_delta(payload)?;
    if !session.is_selected(CAPABILITY_PRESENTATION_V1) {
        return Err(SessionError::UnselectedDomain {
            domain_id: delta.domain_id,
        });
    }
    let row = session
        .gated_revisions
        .iter_mut()
        .find(|(id, _)| *id == delta.domain_id)
        .ok_or(SessionError::MissingDomain(delta.domain_id))?;
    check_delta(&delta, 1, row.1)?;
    let event = if delta.domain_id == STATE_DOMAIN_WORLD_PRESENTATION {
        SessionEvent::Presentation(AppliedDelta {
            server_sequence: sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            value: decode_presentation_batch(delta.payload)
                .map_err(|_| SessionError::SessionUnusable)?,
        })
    } else {
        SessionEvent::ActorCooldowns(AppliedDelta {
            server_sequence: sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            value: decode_actor_cooldowns(delta.payload)
                .map_err(|_| SessionError::SessionUnusable)?,
        })
    };
    row.1 = delta.new_revision;
    Ok(event)
}

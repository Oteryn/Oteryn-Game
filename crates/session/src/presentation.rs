//! Connection-local presentation revisions; transient cues are never replayed from snapshots.
use super::*;
use oteryn_protocol_oteryn::presentation::*;

pub(super) fn validate_join(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
) -> Result<(), SessionError> {
    if selected.contains(&CAPABILITY_ITEM_EFFECTS_V1) {
        let row = snapshots
            .iter()
            .find(|r| r.domain_id == STATE_DOMAIN_ITEM_EFFECTS)
            .ok_or(SessionError::MissingDomain(STATE_DOMAIN_ITEM_EFFECTS))?;
        if row.snapshot_type != 1 || !row.payload.is_empty() {
            return Err(SessionError::SessionUnusable);
        }
    }
    if selected.contains(&CAPABILITY_AMBIENT_SOUND_V1) {
        let row = snapshots
            .iter()
            .find(|row| row.domain_id == STATE_DOMAIN_AMBIENT_SOUND)
            .ok_or(SessionError::MissingDomain(STATE_DOMAIN_AMBIENT_SOUND))?;
        if row.snapshot_type != 1 || !row.payload.is_empty() {
            return Err(SessionError::SessionUnusable);
        }
    }
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
    let ambient = delta.domain_id == STATE_DOMAIN_AMBIENT_SOUND;
    let item = delta.domain_id == STATE_DOMAIN_ITEM_EFFECTS;
    if ambient && delta.payload.len() > MAX_PRESENTATION_EVENT_BYTES {
        return Err(SessionError::SessionUnusable);
    }
    if !session.is_selected(if item {
        CAPABILITY_ITEM_EFFECTS_V1
    } else if ambient {
        CAPABILITY_AMBIENT_SOUND_V1
    } else {
        CAPABILITY_PRESENTATION_V1
    }) {
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
    let event = if item {
        SessionEvent::ItemEffects(AppliedDelta {
            server_sequence: sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            value: decode_item_effect_batch(delta.payload)
                .map_err(|_| SessionError::SessionUnusable)?,
        })
    } else if ambient || delta.domain_id == STATE_DOMAIN_WORLD_PRESENTATION {
        let value =
            decode_presentation_batch(delta.payload).map_err(|_| SessionError::SessionUnusable)?;
        if ambient
            && (value.len() != 1
                || !matches!(value[0].kind, PresentationEventKind::AmbientSound { .. }))
        {
            return Err(SessionError::SessionUnusable);
        }
        if !ambient
            && value
                .iter()
                .any(|event| matches!(event.kind, PresentationEventKind::AmbientSound { .. }))
        {
            return Err(SessionError::SessionUnusable);
        }
        SessionEvent::Presentation(AppliedDelta {
            server_sequence: sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            value,
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

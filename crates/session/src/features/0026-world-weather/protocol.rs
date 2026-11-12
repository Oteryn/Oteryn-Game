use crate::*;
use world_weather::*;
fn decode(payload: &[u8]) -> Result<WorldWeather, SessionError> {
    decode_world_weather(payload)
        .map_err(|_| SessionError::Protocol(FoundationProtocolError::MalformedFrame))
}
pub(crate) fn validate_join(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
) -> Result<(), SessionError> {
    if selected.contains(&CAPABILITY_WORLD_WEATHER_V1) {
        let row = snapshots
            .iter()
            .find(|row| row.domain_id == STATE_DOMAIN_WORLD_WEATHER)
            .ok_or(SessionError::MissingDomain(STATE_DOMAIN_WORLD_WEATHER))?;
        if row.snapshot_type != SNAPSHOT_TYPE_WORLD_WEATHER_V1 {
            return Err(SessionError::Protocol(
                FoundationProtocolError::MalformedFrame,
            ));
        }
        decode(&row.payload)?;
    }
    Ok(())
}
pub(crate) fn apply<S: SessionStream>(
    session: &mut Session<S>,
    sequence: u64,
    payload: &[u8],
) -> Result<SessionEvent, SessionError> {
    if !session.is_selected(CAPABILITY_WORLD_WEATHER_V1) {
        return Err(SessionError::UnselectedDomain {
            domain_id: STATE_DOMAIN_WORLD_WEATHER,
        });
    }
    let delta = decode_state_delta(payload)?;
    let entry = session
        .gated_revisions
        .iter_mut()
        .find(|(id, _)| *id == STATE_DOMAIN_WORLD_WEATHER)
        .ok_or(SessionError::MissingDomain(STATE_DOMAIN_WORLD_WEATHER))?;
    check_delta(&delta, DELTA_TYPE_WORLD_WEATHER_V1, entry.1)?;
    let value = decode(delta.payload)?;
    entry.1 = delta.new_revision;
    Ok(SessionEvent::WorldWeather(AppliedDelta {
        server_sequence: sequence,
        base_revision: delta.base_revision,
        new_revision: delta.new_revision,
        value,
    }))
}

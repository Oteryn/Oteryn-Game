//! Bundle map admission needed to reach authored depot lockers on real Worlds.
use super::*;
use oteryn_protocol_oteryn::world_map::*;

pub(super) fn validate_join(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
    content: [u8; 32],
) -> Result<Option<MapViewHeader>, SessionError> {
    if !selected.contains(&CAPABILITY_WORLD_MAP_VIEW_V1) {
        return Ok(None);
    }
    let snapshot = snapshots
        .iter()
        .find(|row| row.domain_id == STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or(SessionError::MissingDomain(STATE_DOMAIN_WORLD_MAP_VIEW))?;
    if snapshot.snapshot_type != SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1 {
        return Err(SessionError::UnregisteredSnapshotType {
            domain_id: STATE_DOMAIN_WORLD_MAP_VIEW,
            snapshot_type: snapshot.snapshot_type,
        });
    }
    let value = decode_world_map_snapshot(&snapshot.payload)
        .map_err(|_| SessionError::Protocol(FoundationProtocolError::MalformedEnvelope))?;
    if value.header.content_generation != content {
        return Err(SessionError::ContentGenerationMismatch {
            domain_id: STATE_DOMAIN_WORLD_MAP_VIEW,
        });
    }
    Ok(Some(value.header))
}

pub(super) fn apply<S: SessionStream>(
    session: &mut Session<S>,
    sequence: u64,
    payload: &[u8],
) -> Result<SessionEvent, SessionError> {
    if !session.is_selected(CAPABILITY_WORLD_MAP_VIEW_V1) {
        return Err(SessionError::UnselectedDomain {
            domain_id: STATE_DOMAIN_WORLD_MAP_VIEW,
        });
    }
    let header = session
        .map_header
        .ok_or(SessionError::MissingDomain(STATE_DOMAIN_WORLD_MAP_VIEW))?;
    let delta = decode_state_delta(payload)?;
    let revision = session
        .gated_revisions
        .iter_mut()
        .find(|(id, _)| *id == STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or(SessionError::MissingDomain(STATE_DOMAIN_WORLD_MAP_VIEW))?;
    check_delta(&delta, DELTA_TYPE_WORLD_MAP_VIEW_DELTA_V1, revision.1)?;
    let value = decode_world_map_delta(&header, delta.payload)
        .map_err(|_| SessionError::Protocol(FoundationProtocolError::StateRevisionMismatch))?;
    revision.1 = delta.new_revision;
    session.map_header = Some(value.header);
    Ok(SessionEvent::WorldMap(AppliedDelta {
        server_sequence: sequence,
        base_revision: delta.base_revision,
        new_revision: delta.new_revision,
        value,
    }))
}

impl<S: SessionStream> Session<S> {
    /// The latest validated full map snapshot, including any mid-session replacement.
    pub fn world_map_snapshot(&self) -> Option<WorldMapViewSnapshot> {
        self.gated_snapshot(STATE_DOMAIN_WORLD_MAP_VIEW)
            .and_then(|snapshot| decode_world_map_snapshot(&snapshot.payload).ok())
    }
}

/// Publish a whole replacement rather than letting the native reducer merge old tiles into it.
pub(super) fn replacement_event(
    sequence: u64,
    base_revision: u64,
    revision: u64,
    payload: &[u8],
) -> Result<SessionEvent, SessionError> {
    let value = decode_world_map_snapshot(payload)
        .map_err(|_| FoundationProtocolError::MalformedEnvelope)?;
    Ok(SessionEvent::WorldMapSnapshot(AppliedDelta {
        server_sequence: sequence,
        base_revision,
        new_revision: revision,
        value,
    }))
}

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
    let snapshot = session
        .gated_snapshots
        .iter_mut()
        .find(|snapshot| snapshot.domain_id == STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or(SessionError::MissingDomain(STATE_DOMAIN_WORLD_MAP_VIEW))?;
    let value = apply_map_payload(snapshot, header, delta.payload, delta.new_revision)?;
    revision.1 = delta.new_revision;
    session.map_header = Some(value.header);
    Ok(SessionEvent::WorldMap(AppliedDelta {
        server_sequence: sequence,
        base_revision: delta.base_revision,
        new_revision: delta.new_revision,
        value,
    }))
}

fn apply_map_payload(
    snapshot: &mut GatedSnapshot,
    header: MapViewHeader,
    payload: &[u8],
    revision: u64,
) -> Result<WorldMapViewDelta, SessionError> {
    let value = decode_world_map_delta(&header, payload)
        .map_err(|_| SessionError::Protocol(FoundationProtocolError::StateRevisionMismatch))?;
    let mut next = decode_world_map_snapshot(&snapshot.payload)
        .map_err(|_| FoundationProtocolError::MalformedEnvelope)?;
    next.header = value.header;
    next.tiles.retain(|tile| {
        in_window(value.header.origin, tile.position)
            && !value.cleared.contains(&tile.position)
            && !value
                .tiles
                .iter()
                .any(|replacement| replacement.position == tile.position)
    });
    next.tiles.extend(value.tiles.iter().cloned());
    next.tiles.sort_by_key(|tile| tile_order(&tile.position));
    let payload =
        encode_world_map_snapshot(&next).map_err(|_| FoundationProtocolError::MalformedEnvelope)?;
    snapshot.payload = payload;
    snapshot.revision = revision;
    Ok(value)
}

/// Reapply captured server map envelopes to the recorded live map state.
pub fn replay_world_map_frames(
    mut snapshot: GatedSnapshot,
    generation: u64,
    mut sequence: u64,
    frames: &[Vec<u8>],
) -> Result<WorldMapViewSnapshot, SessionError> {
    let malformed = || SessionError::Protocol(FoundationProtocolError::MalformedEnvelope);
    if snapshot.domain_id != STATE_DOMAIN_WORLD_MAP_VIEW || frames.is_empty() {
        return Err(malformed());
    }
    let mut header = decode_world_map_snapshot(&snapshot.payload)
        .map_err(|_| malformed())?
        .header;
    for frame in frames {
        let envelope = decode_wire_envelope(frame)?;
        envelope.validate(Direction::ServerToClient, true)?;
        let current = envelope.server_sequence();
        if envelope.message_type() != MessageType::StateDelta
            || envelope.connection_generation() != generation
            || current <= sequence
        {
            return Err(malformed());
        }
        let delta = decode_state_delta(envelope.payload())?;
        if delta.domain_id != STATE_DOMAIN_WORLD_MAP_VIEW {
            return Err(malformed());
        }
        check_delta(
            &delta,
            DELTA_TYPE_WORLD_MAP_VIEW_DELTA_V1,
            snapshot.revision,
        )?;
        header =
            apply_map_payload(&mut snapshot, header, delta.payload, delta.new_revision)?.header;
        sequence = current;
    }
    decode_world_map_snapshot(&snapshot.payload).map_err(|_| malformed())
}

impl<S: SessionStream> Session<S> {
    /// Enable bounded recording of incoming map delta envelopes.
    pub fn start_world_map_capture(&mut self) -> u64 {
        self.map_capture = Some(Vec::new());
        self.connection_generation
    }

    /// Stop recording and return the original envelope bytes.
    pub fn finish_world_map_capture(&mut self) -> Vec<Vec<u8>> {
        self.map_capture.take().unwrap_or_default()
    }

    pub(super) fn capture_map_frame(&mut self, frame: &[u8]) -> Result<(), SessionError> {
        let Some(capture) = self.map_capture.as_mut() else {
            return Ok(());
        };
        let envelope = decode_wire_envelope(frame)?;
        if envelope.message_type() == MessageType::StateDelta
            && decode_state_delta(envelope.payload())?.domain_id == STATE_DOMAIN_WORLD_MAP_VIEW
        {
            if capture.len() >= 16 {
                return Err(SessionError::Protocol(
                    FoundationProtocolError::MalformedEnvelope,
                ));
            }
            capture.push(frame.to_vec());
        }
        Ok(())
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_replay_rejects_stale_or_foreign_envelopes() -> Result<(), Box<dyn std::error::Error>> {
        let header = MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 0,
            origin: ActorPosition {
                x: 20,
                y: 20,
                floor: 7,
            },
            first_visible_floor: 7,
        };
        let initial = WorldMapViewSnapshot {
            header,
            tiles: Vec::new(),
        };
        let baseline = GatedSnapshot {
            domain_id: STATE_DOMAIN_WORLD_MAP_VIEW,
            snapshot_type: SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1,
            revision: 3,
            payload: encode_world_map_snapshot(&initial).map_err(|error| format!("{error:?}"))?,
        };
        let mut moved = header;
        moved.origin.x += 1;
        let payload = encode_world_map_delta(
            &header,
            &WorldMapViewDelta {
                header: moved,
                cleared: Vec::new(),
                tiles: Vec::new(),
            },
        )
        .map_err(|error| format!("{error:?}"))?;
        let frame = oteryn_protocol_oteryn::encode_state_delta(
            2,
            11,
            STATE_DOMAIN_WORLD_MAP_VIEW,
            3,
            4,
            DELTA_TYPE_WORLD_MAP_VIEW_DELTA_V1,
            &payload,
        )?;
        let frames = [frame];
        assert_eq!(
            replay_world_map_frames(baseline.clone(), 2, 10, &frames)?.header,
            moved
        );
        assert!(replay_world_map_frames(baseline.clone(), 2, 11, &frames).is_err());
        assert!(replay_world_map_frames(baseline, 3, 10, &frames).is_err());
        Ok(())
    }
}

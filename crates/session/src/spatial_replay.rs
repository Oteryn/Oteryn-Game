//! Opt-in qualification capture from the validated post-admission receive path.
use super::*;

const MAX_CAPTURE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Default)]
pub(super) struct SpatialCapture {
    bytes: Vec<u8>,
    overflow: bool,
}

impl std::fmt::Debug for SpatialCapture {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SpatialCapture")
            .field("length", &self.bytes.len())
            .field("overflow", &self.overflow)
            .finish()
    }
}

/// An in-memory checkpoint for replay of original server frames from one live connection.
pub struct SpatialReplay {
    generation: u64,
    sequence: u64,
    revision: u64,
    spatial: WorldSpatialObservation,
    entities: WorldEntities,
    fields: world_spatial_entities::EntityFields,
}

impl<S: SessionStream> Session<S> {
    /// Capture command results and spatial deltas starting at the current client checkpoint.
    pub fn begin_spatial_capture(&mut self) -> Result<SpatialReplay, SessionError> {
        let entities = self
            .entities
            .clone()
            .ok_or(SessionError::UnselectedDomain {
                domain_id: world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            })?;
        self.spatial_capture = Some(Box::default());
        Ok(SpatialReplay {
            generation: self.connection_generation,
            sequence: self.last_server_sequence,
            revision: self.spatial_revision,
            spatial: self.world_spatial,
            entities,
            fields: world_spatial_entities::EntityFields::selected(self.selected_capabilities()),
        })
    }

    /// Finish the opt-in capture. An overflow invalidates the entire recording.
    pub fn finish_spatial_capture(&mut self) -> Option<Vec<u8>> {
        self.spatial_capture
            .take()
            .filter(|capture| !capture.overflow)
            .map(|capture| capture.bytes)
    }

    pub(super) fn record_spatial_capture(
        &mut self,
        frame: &[u8],
        envelope: &oteryn_protocol_oteryn::WireEnvelopeView<'_>,
    ) {
        let Some(capture) = self.spatial_capture.as_mut() else {
            return;
        };
        let selected = envelope.message_type() == MessageType::CommandResult
            || (envelope.message_type() == MessageType::StateDelta
                && decode_state_delta(envelope.payload()).is_ok_and(|delta| {
                    delta.domain_id == world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY
                }));
        if !selected || capture.overflow {
            return;
        }
        if capture
            .bytes
            .len()
            .saturating_add(frame.len())
            .saturating_add(4)
            > MAX_CAPTURE_BYTES
        {
            capture.overflow = true;
            return;
        }
        let Ok(size) = u32::try_from(frame.len()) else {
            capture.overflow = true;
            return;
        };
        capture.bytes.extend_from_slice(&size.to_be_bytes());
        capture.bytes.extend_from_slice(frame);
    }
}

impl SpatialReplay {
    /// Replay one scoped capture against its live pre-command client state.
    pub fn verify_move(
        mut self,
        mut bytes: &[u8],
        expected: world_spatial::ActorPosition,
        command_id: u64,
        result_sequence: u64,
        move_sequence: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let before = self.spatial.actor_position;
        if before == expected || bytes.len() > MAX_CAPTURE_BYTES {
            return Err("invalid spatial replay pre-state or capture size".into());
        }
        let mut result_seen = false;
        let mut move_seen = false;
        while !bytes.is_empty() {
            let (size, rest) = bytes
                .split_first_chunk::<4>()
                .ok_or("truncated frame size")?;
            let (frame, remaining) = rest
                .split_at_checked(u32::from_be_bytes(*size) as usize)
                .ok_or("truncated captured frame")?;
            bytes = remaining;
            let frame = decode_wire_envelope(frame)?;
            frame.validate(Direction::ServerToClient, true)?;
            let sequence = frame.server_sequence();
            if frame.connection_generation() != self.generation || sequence <= self.sequence {
                return Err("spatial replay session or sequence mismatch".into());
            }
            self.sequence = sequence;
            match frame.message_type() {
                MessageType::CommandResult => {
                    let result = decode_command_result(frame.payload())?;
                    if result_seen
                        || sequence != result_sequence
                        || result.command_id != command_id
                        || result.status != CommandStatus::Accepted
                        || world_object::decode_use_result(result.payload)
                            != Ok(world_object::UseDisposition::Committed)
                    {
                        return Err("spatial replay rope result mismatch".into());
                    }
                    result_seen = true;
                }
                MessageType::StateDelta => {
                    let delta = decode_state_delta(frame.payload())?;
                    check_delta(
                        &delta,
                        world_spatial_entities::DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                        self.revision,
                    )?;
                    if delta.domain_id != world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY {
                        return Err("spatial replay domain mismatch".into());
                    }
                    let (decoded, lights, _) =
                        world_spatial_entities::decode_world_spatial_entities_delta_with_sidecar(
                            delta.payload,
                            self.fields,
                        )
                        .map_err(SessionError::from)?;
                    if decoded.content_generation != self.spatial.content_generation {
                        return Err("spatial replay content generation mismatch".into());
                    }
                    if decoded.actor_position != self.spatial.actor_position {
                        if move_seen
                            || sequence != move_sequence
                            || decoded.actor_position != expected
                        {
                            return Err("spatial replay landing mismatch".into());
                        }
                        move_seen = true;
                    }
                    self.entities = self.entities.after(&decoded, &lights)?;
                    self.spatial.actor_position = decoded.actor_position;
                    self.revision = delta.new_revision;
                }
                _ => return Err("unexpected spatial replay frame".into()),
            }
        }
        if !result_seen || !move_seen {
            return Err("spatial replay missing rope result or movement".into());
        }
        Ok(())
    }
}

//! Mid-session atomic replacements use the same transfer and domain validators as admission.
use super::*;
use snapshot_state::SnapshotState;

impl<S: SessionStream> Session<S> {
    pub(super) async fn replace_snapshot(&mut self, payload: &[u8]) -> Result<(), SessionError> {
        let begin = decode_snapshot_begin(payload)?;
        let expected = self
            .last_server_sequence
            .checked_add(1)
            .ok_or(FoundationProtocolError::ServerSequenceGap)?;
        if begin.target_server_sequence != expected {
            return Err(SessionError::ServerSequenceMismatch {
                expected,
                actual: begin.target_server_sequence,
            });
        }
        let body = snapshot_transfer::read_body(
            &mut self.stream,
            self.deadline,
            self.connection_generation,
            &begin,
        )
        .await?;
        let old = SnapshotState::capture(self);
        let next = SnapshotState::decode(
            &body,
            expected,
            &self.selected_capabilities,
            None,
            Some(&old),
        )?;
        let mut events = next.events.clone();
        for domain in decode_snapshot_body(&body)? {
            let event = match domain.domain_id {
                17 => {
                    let base = old
                        .gated_revisions
                        .iter()
                        .find(|(id, _)| *id == 17)
                        .map_or(0, |(_, revision)| *revision);
                    Some(map_transport::replacement_event(
                        expected,
                        base,
                        domain.revision,
                        domain.payload,
                    )?)
                }
                1 => next.world_spatial_observation.map(|value| {
                    SessionEvent::WorldSpatial(applied(
                        expected,
                        old.spatial_revision,
                        domain.revision,
                        value,
                    ))
                }),
                3 => next.actor_vitals.map(|value| {
                    SessionEvent::ActorVitals(applied(
                        expected,
                        old.vitals_revision,
                        domain.revision,
                        value,
                    ))
                }),
                9 => next.inventory.clone().map(|value| {
                    SessionEvent::Inventory(applied(
                        expected,
                        old.inventory_revision,
                        domain.revision,
                        value,
                    ))
                }),
                10 => next.combat_state.map(|value| {
                    SessionEvent::ActorCombatState(applied(
                        expected,
                        old.combat_revision,
                        domain.revision,
                        value,
                    ))
                }),
                11 => next.open_container.clone().map(|value| {
                    SessionEvent::OpenContainer(applied(
                        expected,
                        old.container_revision,
                        domain.revision,
                        value,
                    ))
                }),
                _ => None,
            };
            if let Some(event) = event {
                events.push(event);
            }
        }
        if self.events.len().saturating_add(events.len()) > MAX_QUEUED_EVENTS {
            return Err(SessionError::EventQueueOverflow {
                limit: MAX_QUEUED_EVENTS,
            });
        }
        next.install(self);
        self.last_server_sequence = expected;
        self.last_received = tokio::time::Instant::now();
        self.events.extend(events);
        Ok(())
    }
}

fn applied<T>(
    server_sequence: u64,
    base_revision: u64,
    new_revision: u64,
    value: T,
) -> AppliedDelta<T> {
    AppliedDelta {
        server_sequence,
        base_revision,
        new_revision,
        value,
    }
}

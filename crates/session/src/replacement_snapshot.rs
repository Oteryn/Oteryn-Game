//! Mid-session atomic replacements use the same transfer and domain validators as admission.
use super::*;
use snapshot_state::SnapshotState;

impl<S: SessionStream> Session<S> {
    pub(super) async fn replace_snapshot(&mut self, payload: &[u8]) -> Result<(), SessionError> {
        self.replace_snapshot_with_scope_change(payload)
            .await
            .map(|_| ())
    }

    pub(super) async fn replace_snapshot_with_scope_change(
        &mut self,
        payload: &[u8],
    ) -> Result<Option<SessionError>, SessionError> {
        let expected = self
            .last_server_sequence
            .checked_add(1)
            .ok_or(FoundationProtocolError::ServerSequenceGap)?;
        self.replace_snapshot_at_with_scope_change(payload, expected)
            .await
    }

    pub(super) async fn replace_snapshot_at(
        &mut self,
        payload: &[u8],
        expected: u64,
    ) -> Result<(), SessionError> {
        self.replace_snapshot_at_with_scope_change(payload, expected)
            .await
            .map(|_| ())
    }

    async fn replace_snapshot_at_with_scope_change(
        &mut self,
        payload: &[u8],
        expected: u64,
    ) -> Result<Option<SessionError>, SessionError> {
        let begin = decode_snapshot_begin(payload)?;
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
        if let Some(changed) = self.adopt_activity_snapshot(&body).await? {
            return Ok(Some(changed));
        }
        if decode_snapshot_body(&body)?
            .iter()
            .any(|domain| domain.domain_id == 99)
        {
            self.replace_activity_scope(&body, expected).await?;
            return Ok(Some(SessionError::ScopeChanged));
        }
        let old = SnapshotState::capture(self);
        let next = SnapshotState::decode(
            &body,
            expected,
            &self.selected_capabilities,
            None,
            Some(&old),
        )?;
        let events = replacement_events(&body, expected, &old, &next)?;
        if self.events.len().saturating_add(events.len()) > MAX_QUEUED_EVENTS {
            return Err(SessionError::EventQueueOverflow {
                limit: MAX_QUEUED_EVENTS,
            });
        }
        next.install(self);
        self.last_server_sequence = expected;
        self.last_received = tokio::time::Instant::now();
        self.events.extend(events);
        Ok(None)
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

pub(super) fn replacement_events(
    body: &[u8],
    expected: u64,
    old: &SnapshotState,
    next: &SnapshotState,
) -> Result<Vec<SessionEvent>, SessionError> {
    let mut events = next.events.clone();
    for domain in decode_snapshot_body(body)? {
        let base = match domain.domain_id {
            1 => old.spatial_revision,
            2 => old.overlay_revision,
            3 => old.vitals_revision,
            9 => old.inventory_revision,
            10 => old.combat_revision,
            11 => old.container_revision,
            12 => old.chat_revision,
            id => old
                .gated_revisions
                .iter()
                .find(|(domain, _)| *domain == id)
                .map_or(0, |(_, revision)| *revision),
        };
        events.push(SessionEvent::GatedSnapshotReplaced(applied(
            expected,
            base,
            domain.revision,
            GatedSnapshot {
                domain_id: domain.domain_id,
                snapshot_type: domain.snapshot_type,
                revision: domain.revision,
                payload: domain.payload.to_vec(),
            },
        )));
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
            2 => next.world_object_overlay.clone().map(|value| {
                SessionEvent::WorldObjectOverlaySnapshot(applied(
                    expected,
                    old.overlay_revision,
                    domain.revision,
                    value,
                ))
            }),
            14 => next.container_views.clone().map(|value| {
                SessionEvent::ContainerViews(applied(expected, base, domain.revision, value))
            }),
            12 => next.chat.clone().map(|value| {
                SessionEvent::ChatSnapshot(applied(
                    expected,
                    old.chat_revision,
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
    Ok(events)
}

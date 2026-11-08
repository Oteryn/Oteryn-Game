//! Adopt a committed activity destination only after its complete fresh snapshot validates.
use super::*;
pub use oteryn_protocol_oteryn::activity_scope::{
    ActivityScope, ActivityScopeAck, ActivityScopeSnapshot, COMMAND_TYPE_ACTIVITY_SCOPE_ACK,
    MAX_ACTIVITY_SCOPE_BYTES, STATE_DOMAIN_ACTIVITY_SCOPE, decode_activity_scope,
    decode_activity_scope_ack, encode_activity_scope, encode_activity_scope_ack,
};

impl<S: SessionStream> Session<S> {
    pub(super) fn encode_current_command(
        &self,
        command: &ClientCommandValue<'_>,
    ) -> Result<Vec<u8>, FoundationProtocolError> {
        if self.activity_scope.is_some() {
            oteryn_protocol_oteryn::encode_client_command_for_session(
                self.connection_generation,
                command,
                self.join_snapshot.game_session_id.as_bytes(),
            )
        } else {
            encode_client_command(self.connection_generation, command)
        }
    }

    pub fn activity_scope(&self) -> Option<&ActivityScopeSnapshot> {
        self.activity_scope.as_ref()
    }

    pub(super) async fn replace_activity_scope(
        &mut self,
        body: &[u8],
        source_sequence: u64,
    ) -> Result<(), SessionError> {
        let malformed = || SessionError::SessionUnusable;
        let domains = decode_snapshot_body(body)?;
        if domains.len() != 1
            || domains[0].domain_id != 99
            || domains[0].snapshot_type != 1
            || domains[0].revision != 1
            || !self.selected_capabilities.contains(&117)
        {
            return Err(malformed());
        }
        let scope = decode_activity_scope(domains[0].payload).map_err(|_| malformed())?;
        let world = match scope.scope {
            ActivityScope::Channel { world, .. } | ActivityScope::Instance { world, .. } => world,
        };
        if scope.source_session != *self.join_snapshot.game_session_id.as_bytes()
            || world != self.world_id
            || scope
                .selection
                .iter()
                .any(|capability| !self.selected_capabilities.contains(capability))
        {
            return Err(malformed());
        }
        let frame = bounded(
            self.deadline,
            "activity destination snapshot",
            read_frame(&mut self.stream),
        )
        .await?;
        let envelope = decode_wire_envelope(&frame)?;
        envelope.validate(Direction::ServerToClient, true)?;
        if envelope.message_type() != MessageType::SnapshotBegin {
            return Err(SessionError::UnexpectedMessage {
                expected: MessageType::SnapshotBegin,
                actual: envelope.message_type(),
            });
        }
        if envelope.connection_generation() != scope.connection_generation {
            return Err(SessionError::ConnectionGenerationMismatch {
                expected: scope.connection_generation,
                actual: envelope.connection_generation(),
            });
        }
        let begin = decode_snapshot_begin(envelope.payload())?;
        self.finish_activity_scope(scope, source_sequence, begin, domains[0].payload)
            .await
    }

    /// Complete the same validated legacy route when the shared reader already
    /// consumed its fresh SnapshotBegin; never read or fabricate that frame twice.
    pub(super) async fn finish_activity_scope(
        &mut self,
        scope: ActivityScopeSnapshot,
        source_sequence: u64,
        begin: oteryn_protocol_oteryn::SnapshotBeginFields,
        source_payload: &[u8],
    ) -> Result<(), SessionError> {
        let malformed = || SessionError::SessionUnusable;
        if decode_activity_scope(source_payload).map_err(|_| malformed())? != scope {
            return Err(malformed());
        }
        // A proven fresh namespace uses the same initial baseline as native
        // cold admission (current producer0; legacy complete fixtures1). This
        // never relaxes sequence checks within an unchanged GameSession.
        if begin.target_server_sequence > 1 {
            return Err(SessionError::ServerSequenceMismatch {
                expected: 0,
                actual: begin.target_server_sequence,
            });
        }
        let destination_sequence = begin.target_server_sequence;
        let destination = snapshot_transfer::read_body(
            &mut self.stream,
            self.deadline,
            scope.connection_generation,
            &begin,
        )
        .await?;
        if current_projection(
            &destination,
            &scope.session,
            &self.world_id,
            scope.connection_generation,
            &scope.selection,
        )?
        .as_ref()
            != Some(&scope)
        {
            return Err(malformed());
        }
        let mut next = snapshot_state::SnapshotState::decode(
            &destination,
            destination_sequence,
            &scope.selection,
            None,
            None,
        )?;
        let spatial = next
            .world_spatial_observation
            .ok_or(SessionError::MissingDomain(1))?;
        let overlay = next
            .world_object_overlay
            .clone()
            .ok_or(SessionError::MissingDomain(2))?;
        let mut events = vec![SessionEvent::ActivityScope(AppliedDelta {
            server_sequence: source_sequence,
            base_revision: 0,
            new_revision: 1,
            value: scope.clone(),
        })];
        events.extend(replacement_snapshot::replacement_events(
            &destination,
            destination_sequence,
            &snapshot_state::SnapshotState::default(),
            &next,
        )?);
        if events.len() > MAX_QUEUED_EVENTS {
            return Err(SessionError::EventQueueOverflow {
                limit: MAX_QUEUED_EVENTS,
            });
        }
        next.gated_snapshots.retain(|domain| domain.domain_id != 99);
        next.gated_revisions.retain(|(domain, _)| *domain != 99);
        next.gated_snapshots.push(GatedSnapshot {
            domain_id: 99,
            snapshot_type: 1,
            revision: 1,
            payload: source_payload.to_vec(),
        });
        next.gated_revisions.push((99, 1));
        let join = JoinSnapshot {
            game_session_id: GameSessionId::decode(&scope.session)?,
            world_spatial: spatial,
            world_spatial_revision: next.spatial_revision,
            world_object_overlay: overlay,
            world_object_overlay_revision: next.overlay_revision,
        };
        next.install(self);
        self.join_snapshot = join;
        self.connection_generation = scope.connection_generation;
        self.selected_capabilities = scope.selection.clone();
        self.activity_scope = Some(scope);
        self.first_command_id = 1;
        self.next_command_id = 1;
        self.last_server_sequence = destination_sequence;
        self.last_probe_id = 0;
        self.duplicates.clear();
        self.last_refusal_detail = None;
        self.vip_presence = None;
        self.last_received = tokio::time::Instant::now();
        self.events = events;
        Ok(())
    }
}

/// Full initial snapshots can carry an idempotent projection of the already admitted scope.
pub(super) fn current_projection(
    body: &[u8],
    session: &[u8; 16],
    world: &[u8; 16],
    generation: u64,
    selection: &[u32],
) -> Result<Option<ActivityScopeSnapshot>, SessionError> {
    let domains = decode_snapshot_body(body)?;
    let Some((index, domain)) = domains
        .iter()
        .enumerate()
        .find(|(_, domain)| domain.domain_id == 99)
    else {
        return Ok(None);
    };
    // Both existing core spatial compositions precede the retained binding:
    // native setup uses domain1; legacy full join also includes its domain2
    // object overlay. Neither admits an intervening gameplay domain. The full
    // bounded body and binding are validated before any store is installed.
    let spatial_prefix = index == 0
        || (index == 1 && domains[0].domain_id == 1)
        || (index == 2 && domains[0].domain_id == 1 && domains[1].domain_id == 2);
    if !spatial_prefix
        || domain.snapshot_type != 1
        || domain.revision != generation
        || !selection.contains(&117)
        || domains
            .iter()
            .filter(|domain| domain.domain_id == 99)
            .count()
            != 1
    {
        return Err(SessionError::SessionUnusable);
    }
    let scope = decode_activity_scope(domain.payload).map_err(|_| SessionError::SessionUnusable)?;
    let scope_world = match scope.scope {
        ActivityScope::Channel { world, .. } | ActivityScope::Instance { world, .. } => world,
    };
    if scope.session != *session
        || scope_world != *world
        || scope.connection_generation != generation
        || scope.selection != selection
    {
        return Err(SessionError::SessionUnusable);
    }
    Ok(Some(scope))
}

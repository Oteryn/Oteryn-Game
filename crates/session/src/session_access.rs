//! Session session access.
use super::*;

impl<S: SessionStream> Session<S> {
    /// Service an idle connection until at least one pushed domain event is applied.
    ///
    /// The deadline bounds the whole wait; individual idle reads are deliberately short so
    /// liveness traffic cannot make an event waiter return before the subscribed domain changes.
    pub async fn wait_for_events(
        &mut self,
        deadline: Duration,
    ) -> Result<Vec<SessionEvent>, SessionError> {
        let until = tokio::time::Instant::now() + deadline;
        loop {
            let events = self.take_events();
            if !events.is_empty() {
                return Ok(events);
            }
            let Some(remaining) = until.checked_duration_since(tokio::time::Instant::now()) else {
                return Err(SessionError::Timeout("pushed domain event"));
            };
            self.service_liveness(remaining.min(Duration::from_millis(250)))
                .await?;
        }
    }

    /// Admits over the established `stream` with `admission` and decodes the join snapshot the
    /// server sends right after `ServerAccepted` (`SnapshotBegin`, `SnapshotChunk`,
    /// `SnapshotCommit` - FND-02 section 16), keeping the admitted connection: the returned
    /// session carries the admitted `connection_generation`, the first `CommandId`
    /// (`ServerAccepted.next_command_id`), the last applied `server_sequence`
    /// (`SnapshotBegin.target_server_sequence`) and both domain revisions the join snapshot
    /// established, which `step`/`use_object` then extend under the same envelope, sequence and
    /// revision discipline.
    ///
    /// Every inbound frame is checked with `WireEnvelopeView::validate` (direction, phase,
    /// sequencing and - pre- vs post-admission - the envelope `connection_generation` presence
    /// rule) before its payload is consumed at all, then correlated to `SnapshotBegin`'s full
    /// declaration and the admitted session: `SnapshotBegin`/`SnapshotChunk`/`SnapshotCommit`
    /// must each carry the admitted `connection_generation`; `SnapshotChunk`'s and
    /// `SnapshotCommit`'s `snapshot_id` must equal `SnapshotBegin`'s; exactly `SnapshotBegin`'s
    /// declared `chunk_count` chunks are read, each with the expected `chunk_index` in order; and
    /// their concatenated `data` bytes must equal `SnapshotBegin`'s declared
    /// `total_encoded_bytes`. The assembled `SnapshotBody` is protobuf-decoded exactly once, only
    /// after every chunk and the matching `SnapshotCommit` have validated - never per chunk, since
    /// a multi-chunk transfer may split a body field at any byte offset. Every frame read is
    /// bounded by `admission.deadline`.
    pub async fn admit(mut stream: S, admission: Admission<'_>) -> Result<Self, SessionError> {
        let bootstrap = encode_client_bootstrap(&ClientBootstrapValue {
            schema_revision: admission.schema_revision,
            character_id: admission.character_id,
            admission_material: admission.admission_material,
            client_build_id: admission.client_build_id,
            supported_capabilities: admission.supported_capabilities,
        })?;
        write_frame(&mut stream, &bootstrap).await?;

        let accepted = bounded(
            admission.deadline,
            "ServerAccepted",
            read_frame(&mut stream),
        )
        .await?;
        let accepted_envelope = decode_wire_envelope(&accepted)?;
        // Pre-admission server traffic: direction, phase (Bootstrap), sequencing
        // (unsequenced) and a
        // zero envelope `connection_generation` (FND-02 §8/§11/§12/§14) are checked before this
        // frame's payload is consumed at all.
        accepted_envelope.validate(Direction::ServerToClient, false)?;
        if accepted_envelope.message_type() == MessageType::ProtocolError
            && let Ok(refusal) = decode_protocol_error(accepted_envelope.payload())
            && ADMISSION_REFUSAL_CODES.contains(&refusal.error_code)
            // N8 fixes the refusal shape; any other disposition or correlation fails closed below.
            && refusal.disposition == ProtocolDisposition::TransportFatal
            && refusal.related_command_id == 0
            && refusal.expected_command_id == 0
            && refusal.expected_server_sequence == 0
        {
            if let Some(queue) = admission_queue::decode_queue(accepted_envelope.payload())? {
                return Err(SessionError::AdmissionQueued {
                    code: refusal.error_code,
                    queue,
                });
            }
            return Err(SessionError::AdmissionRefused {
                code: refusal.error_code,
            });
        }
        if accepted_envelope.message_type() != MessageType::ServerAccepted {
            return Err(SessionError::NotAdmitted(accepted_envelope.message_type()));
        }
        let accepted_fields = decode_server_accepted(accepted_envelope.payload())?;
        let session_generation = accepted_fields.connection_generation;
        // CAP-NEG-1: the server selects only from the advertised set; anything else is a violation.
        if let Some(&capability) = accepted_fields
            .selected_capabilities
            .iter()
            .find(|id| !admission.supported_capabilities.contains(id))
        {
            return Err(SessionError::CapabilityNotRequested(capability));
        }
        let bulletin =
            oteryn_protocol_oteryn::bulletin::document_from_acceptance(accepted_envelope.payload())
                .ok()
                .flatten()
                .map(str::to_owned);
        let mut session = Self::read_join(
            stream,
            admission.deadline,
            admission.appearance_items,
            JoinAcceptance {
                game_session_id: accepted_fields.game_session_id,
                connection_generation: session_generation,
                next_command_id: accepted_fields.next_command_id,
                selected_capabilities: accepted_fields.selected_capabilities,
                recovery_sequence: None,
            },
        )
        .await?;
        session.bulletin = bulletin;
        Ok(session)
    }

    pub fn bulletin(&self) -> Option<&str> {
        self.bulletin.as_deref()
    }

    /// The capabilities the server selected at admission (never widened by a resume).
    pub fn selected_capabilities(&self) -> &[u32] {
        &self.selected_capabilities
    }

    pub fn is_selected(&self, capability: u32) -> bool {
        self.selected_capabilities.contains(&capability)
    }

    /// The latest full snapshot of a selected capability's domain, if the server sent it.
    pub fn gated_snapshot(&self, domain_id: u32) -> Option<&GatedSnapshot> {
        self.gated_snapshots
            .iter()
            .find(|snapshot| snapshot.domain_id == domain_id)
    }

    /// Reads the next `StateDelta` of the selected capability's `domain_id`/`delta_type`, checks
    /// it is based on the applied revision, and advances that revision; the caller decodes the
    /// payload.
    pub async fn read_gated_delta(
        &mut self,
        domain_id: u32,
        delta_type: u32,
    ) -> Result<GatedDelta, SessionError> {
        self.ensure_usable()?;
        let capability = capability_of_domain(domain_id)
            .filter(|capability| self.is_selected(*capability))
            .ok_or(SessionError::CapabilityNotSelected {
                capability: capability_of_domain(domain_id).unwrap_or(0),
            })?;
        debug_assert!(self.is_selected(capability));
        let applied = self
            .gated_revisions
            .iter()
            .find(|(id, _)| *id == domain_id)
            .map_or(0, |(_, revision)| *revision);
        let outcome = self.read_delta(domain_id, delta_type, applied).await;
        let delta = self.poison_on_error(outcome)?;
        if domain_id == spell_catalog::STATE_DOMAIN_ACTOR_SPELL_CATALOG {
            let recorded = self.record_spell_catalog(
                delta.base_revision,
                delta.new_revision,
                delta_type,
                &delta.payload,
            );
            self.poison_on_error(recorded)?;
        }
        match self
            .gated_revisions
            .iter_mut()
            .find(|(id, _)| *id == domain_id)
        {
            Some(entry) => entry.1 = delta.new_revision,
            None => self.gated_revisions.push((domain_id, delta.new_revision)),
        }
        let delta = GatedDelta {
            server_sequence: delta.server_sequence,
            base_revision: delta.base_revision,
            new_revision: delta.new_revision,
            payload: delta.payload,
        };
        if domain_id == player_trade::STATE_DOMAIN_PLAYER_TRADE {
            let recorded = self.record_player_trade_delta(&delta);
            self.poison_on_error(recorded)?;
        }
        Ok(delta)
    }

    pub fn join_snapshot(&self) -> &JoinSnapshot {
        &self.join_snapshot
    }

    pub fn into_join_snapshot(self) -> JoinSnapshot {
        self.join_snapshot
    }

    /// The visible entities after every delta applied so far; `Some` exactly when capability 6 is
    /// selected.
    pub fn world_entities(&self) -> Option<&WorldEntities> {
        self.entities.as_ref()
    }

    /// The open rooms and the last lines after every delta applied so far; `Some` exactly when
    /// capability 7 is selected.
    pub fn chat_log(&self) -> Option<&ChatLog> {
        self.chat.as_ref()
    }

    /// Dismiss the oldest locally retained chat line and its aligned extension data.
    /// This changes no server state, revisions, pending events or liveness bookkeeping.
    pub fn clear_oldest_chat_line(&mut self) -> bool {
        let Some(log) = self.chat.as_mut() else {
            return false;
        };
        if log.lines.pop_front().is_none() {
            return false;
        }
        log.extras.pop_front();
        true
    }

    /// The inventory after every delta applied so far; `Some` exactly when capability 4 is
    /// selected.
    pub fn inventory(&self) -> Option<&CharacterInventory> {
        self.inventory.as_ref()
    }

    /// The open corpse after every delta applied so far (the default means nothing is open);
    /// `Some` exactly when capability 4 is selected.
    pub fn open_container(&self) -> Option<&OpenContainer> {
        self.open_container.as_ref()
    }

    /// The own-actor position after every delta applied so far.
    pub fn world_spatial(&self) -> &WorldSpatialObservation {
        &self.world_spatial
    }

    /// The overlay entries after every delta applied so far.
    pub fn world_object_overlay(&self) -> &[WorldObjectOverlayEntry] {
        &self.world_object_overlay
    }

    /// The own-actor vitals after every delta applied so far, if the server has sent any.
    pub fn actor_vitals(&self) -> Option<&ActorVitals> {
        self.actor_vitals.as_ref()
    }

    /// The own-actor combat state after every delta applied so far; `Some` once a session that
    /// selected capability 17 has received domain 10.
    pub fn combat_state(&self) -> Option<&ActorCombatState> {
        self.combat_state.as_ref()
    }

    /// The `CommandId` the next `step`/`use_object`/`cast_spell` will send.
    pub fn next_command_id(&self) -> u64 {
        self.next_command_id
    }

    /// The `server_sequence` of the last frame applied (initially the snapshot's target).
    pub fn last_server_sequence(&self) -> u64 {
        self.last_server_sequence
    }

    /// Returns (and clears) the duplicate-status results received while reading command results.
    pub fn take_duplicate_outcomes(&mut self) -> Vec<DuplicateOutcome> {
        std::mem::take(&mut self.duplicates)
    }

    /// Returns (and clears) the pushed deltas applied since the last call, in order.
    pub fn take_events(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.events)
    }

    /// Keeps an otherwise idle session alive: for up to `duration`, reads frames, answers each
    /// `LivenessProbe` with a `LivenessAck` (last applied `server_sequence`) and applies each
    /// pushed `StateDelta`, queueing it for `take_events`. Returns `Ok` when
    /// the time elapses. A peer silent for the 20-second idle liveness window times out even
    /// across repeated short calls. Any other frame, a failed validation, a closed connection
    /// or an I/O error fails closed and makes the session unusable. Runs on the caller's task only.
    pub async fn service_liveness(&mut self, duration: Duration) -> Result<(), SessionError> {
        self.ensure_usable()?;
        let until = tokio::time::Instant::now() + duration;
        let outcome = self.serve_liveness_until(until).await;
        self.poison_on_error(outcome)
    }

    pub(super) async fn serve_liveness_until(
        &mut self,
        until: tokio::time::Instant,
    ) -> Result<(), SessionError> {
        loop {
            let silence_until = self.last_received + SERVER_SILENCE_DEADLINE;
            if tokio::time::Instant::now() >= silence_until {
                return Err(SessionError::Timeout("server silence"));
            }
            // Tokio may poll a ready read before an expired timeout. A continuously
            // readable peer must not extend the caller's idle window indefinitely.
            if tokio::time::Instant::now() >= until {
                return Ok(());
            }
            // Waiting for the first byte is cancel-safe (a plain `read` either returns data or
            // consumes nothing), so the idle window can end without losing part of a frame; the
            // rest of a started frame is then read under the ordinary per-frame deadline.
            let mut first = [0_u8; 1];
            match tokio::time::timeout_at(until.min(silence_until), self.stream.read(&mut first))
                .await
            {
                Err(_elapsed) if tokio::time::Instant::now() >= silence_until => {
                    return Err(SessionError::Timeout("server silence"));
                }
                Err(_elapsed) => return Ok(()),
                Ok(Err(error)) => return Err(error.into()),
                Ok(Ok(0)) => return Err(io::Error::from(io::ErrorKind::UnexpectedEof).into()),
                Ok(Ok(_)) => {}
            }
            let frame = bounded(
                self.deadline,
                "idle frame",
                read_frame_after(&mut self.stream, first[0]),
            )
            .await?;
            self.last_received = tokio::time::Instant::now();
            let envelope = decode_wire_envelope(&frame)?;
            envelope.validate(Direction::ServerToClient, true)?;
            self.check_generation(&envelope)?;
            if let Some(error) = self.terminal_protocol_error(&envelope)? {
                return Err(error);
            }
            match envelope.message_type() {
                MessageType::LivenessProbe => self.answer_probe(envelope.payload()).await?,
                MessageType::SnapshotBegin => self.replace_snapshot(envelope.payload()).await?,
                MessageType::StateDelta => {
                    let server_sequence = self.next_sequence(&envelope)?;
                    let event = self.apply_state_delta(server_sequence, envelope.payload())?;
                    self.queue_event(event)?;
                }
                actual => {
                    return Err(SessionError::UnexpectedMessage {
                        expected: MessageType::LivenessProbe,
                        actual,
                    });
                }
            }
        }
    }
}

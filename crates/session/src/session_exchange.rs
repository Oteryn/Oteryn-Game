//! Session session exchange.
use super::*;

impl<S: SessionStream> Session<S> {
    pub(super) fn ensure_usable(&self) -> Result<(), SessionError> {
        if self.unusable {
            Err(SessionError::SessionUnusable)
        } else {
            Ok(())
        }
    }

    pub(super) fn poison_on_error<T>(
        &mut self,
        outcome: Result<T, SessionError>,
    ) -> Result<T, SessionError> {
        if outcome.as_ref().is_err_and(|error| {
            !matches!(
                error,
                SessionError::ScopeChanged | SessionError::ActivityScopeChanged
            )
        }) {
            self.unusable = true;
        }
        outcome
    }

    pub(super) async fn exchange_step(
        &mut self,
        payload: &[u8],
    ) -> Result<StepOutcome, SessionError> {
        let result = self
            .send_and_read_result(world_spatial::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, payload)
            .await?;
        let (payload, detail) = self.item_result_payload(&result.payload)?;
        let disposition = world_spatial::decode_step_result_receipt_with_diagonal(
            payload,
            self.is_selected(CAPABILITY_PACED_MOVEMENT_V1),
            self.is_selected(world_spatial::CAPABILITY_DIAGONAL_MOVEMENT_V1),
        )?
        .disposition;
        check_status_pairing(
            result.command_id,
            result.status,
            matches!(
                disposition,
                StepDisposition::Rejected | StepDisposition::TooEarly
            ),
        )?;
        if detail.is_some() && disposition == StepDisposition::Moved {
            return Err(SessionError::SessionUnusable);
        }
        self.last_refusal_detail = detail;
        let world_spatial_delta =
            if result.status == CommandStatus::Accepted && disposition == StepDisposition::Moved {
                match self
                    .read_claimed_delta(world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY)
                    .await?
                {
                    SessionEvent::WorldSpatial(delta) => Some(delta),
                    SessionEvent::WorldSpatialEntities(delta) => {
                        // The outcome carries only the observation; the entity enter/update/leave
                        // lists reach consumers through the event queue.
                        let observation = AppliedDelta {
                            server_sequence: delta.server_sequence,
                            base_revision: delta.base_revision,
                            new_revision: delta.new_revision,
                            value: WorldSpatialObservation {
                                content_generation: delta.value.content_generation,
                                actor_position: delta.value.actor_position,
                            },
                        };
                        self.queue_event(SessionEvent::WorldSpatialEntities(delta))?;
                        Some(observation)
                    }
                    other => {
                        return Err(SessionError::UnexpectedDomain {
                            expected: world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                            actual: other.domain_id(),
                        });
                    }
                }
            } else {
                None
            };
        Ok(CommandOutcome {
            command_id: result.command_id,
            status: result.status,
            disposition,
            result_server_sequence: result.server_sequence,
            world_spatial_delta,
            world_object_overlay_delta: None,
            gate_message_id: None,
        })
    }

    pub(super) async fn exchange_use(
        &mut self,
        payload: &[u8],
    ) -> Result<UseOutcome, SessionError> {
        let result = self
            .send_and_read_result(world_object::COMMAND_TYPE_USE_INTENT, payload)
            .await?;
        let (payload, detail) = self.item_result_payload(&result.payload)?;
        let (disposition, gate_message_id) = world_object::decode_use_result_with_gate(payload)?;
        // Dispositions 11 to 17 and the gate message id exist only under capability 11, and
        // dispositions 18 and 19 only under capability 22.
        check_use_result_negotiated(
            disposition,
            gate_message_id,
            self.is_selected(world_interaction::CAPABILITY_WORLD_INTERACTION_V1),
            self.is_selected(world_object::CAPABILITY_RUNE_USE_V1),
        )?;
        check_status_pairing(
            result.command_id,
            result.status,
            disposition == UseDisposition::Rejected,
        )?;
        if detail.is_some() && disposition == UseDisposition::Committed {
            return Err(SessionError::SessionUnusable);
        }
        self.last_refusal_detail = detail;
        // A `Committed` use names no domain (it can change 2, 9, 11, several or none), so the
        // exchange returns at its result; its deltas go through the store and the event queue.
        Ok(CommandOutcome {
            command_id: result.command_id,
            status: result.status,
            disposition,
            result_server_sequence: result.server_sequence,
            world_spatial_delta: None,
            world_object_overlay_delta: None,
            gate_message_id,
        })
    }

    pub(super) async fn exchange_cast(
        &mut self,
        payload: &[u8],
    ) -> Result<CastOutcome, SessionError> {
        let result = self
            .send_and_read_result(
                actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
                payload,
            )
            .await?;
        let disposition = actor_spell::decode_spell_cast_result(&result.payload)?;
        check_status_pairing(
            result.command_id,
            result.status,
            disposition == SpellCastDisposition::Rejected,
        )?;
        let actor_vitals_delta = if result.status == CommandStatus::Accepted
            && disposition == SpellCastDisposition::Cast
        {
            match self
                .read_claimed_delta(actor_spell::STATE_DOMAIN_ACTOR_VITALS)
                .await?
            {
                SessionEvent::ActorVitals(delta) => Some(delta),
                other => {
                    return Err(SessionError::UnexpectedDomain {
                        expected: actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                        actual: other.domain_id(),
                    });
                }
            }
        } else {
            None
        };
        Ok(CastOutcome {
            command_id: result.command_id,
            status: result.status,
            disposition,
            result_server_sequence: result.server_sequence,
            actor_vitals_delta,
        })
    }

    /// Sends one command under the next `CommandId` and reads its correlated `CommandResult`.
    pub(super) async fn send_and_read_result(
        &mut self,
        command_type: u32,
        payload: &[u8],
    ) -> Result<ReceivedResult, SessionError> {
        let command_id = self.next_command_id;
        // Consumed before the write: an attempted (possibly half-written) command's id is never
        // reused.
        self.next_command_id = command_id
            .checked_add(1)
            .ok_or(SessionError::CommandIdExhausted)?;
        let frame = self.encode_current_command(&ClientCommandValue {
            command_id,
            command_type,
            payload,
        })?;
        bounded(
            self.deadline,
            "ClientCommand write",
            write_frame(&mut self.stream, &frame),
        )
        .await?;

        loop {
            let (message_type, server_sequence, result_payload) =
                self.read_sequenced_push("CommandResult").await?;
            if message_type == MessageType::StateDelta {
                // Pushed before the result (the session never predicts the next frame's domain).
                let event = self.apply_state_delta(server_sequence, &result_payload)?;
                self.queue_event(event)?;
                continue;
            }
            let result = decode_command_result(&result_payload)?;
            if matches!(
                result.status,
                CommandStatus::DuplicateReplay | CommandStatus::DuplicateOutcomeExpired
            ) {
                // FND-02 §13.2: a duplicate names an already reserved lower CommandId and is
                // never executed again, so it is only ever valid for a CommandId this session
                // sent before the current one; the current command's own result is never a
                // duplicate. It carries no delta and does not end the wait for the real result.
                if result.command_id == command_id {
                    return Err(SessionError::InconsistentCommandResult {
                        command_id,
                        status: result.status,
                    });
                }
                if result.command_id < self.first_command_id || result.command_id > command_id {
                    return Err(SessionError::DuplicateForUnsentCommand {
                        command_id: result.command_id,
                    });
                }
                if self.duplicates.len() >= MAX_RETAINED_DUPLICATES {
                    return Err(FoundationProtocolError::PayloadLimitExceeded.into());
                }
                self.duplicates.push(DuplicateOutcome {
                    command_id: result.command_id,
                    status: result.status,
                    server_sequence,
                    payload: result.payload.to_vec(),
                });
                continue;
            }
            if result.command_id != command_id {
                return Err(SessionError::CommandIdMismatch {
                    expected: command_id,
                    actual: result.command_id,
                });
            }
            return Ok(ReceivedResult {
                command_id,
                status: result.status,
                server_sequence,
                payload: result.payload.to_vec(),
            });
        }
    }

    /// Reads `StateDelta` frames until one names `domain_id` and returns it as the claimed delta:
    /// the first delta of that domain applied after the command's result, by revision alone.
    /// Deltas of other domains apply and queue on the way. `ServerSequence` and revision checks
    /// run for each frame.
    pub(super) async fn read_claimed_delta(
        &mut self,
        domain_id: u32,
    ) -> Result<SessionEvent, SessionError> {
        loop {
            let (server_sequence, payload) = self
                .read_sequenced("StateDelta", MessageType::StateDelta)
                .await?;
            let event = self.apply_state_delta(server_sequence, &payload)?;
            if event.domain_id() == domain_id {
                return Ok(event);
            }
            self.queue_event(event)?;
        }
    }

    pub(super) fn queue_event(&mut self, event: SessionEvent) -> Result<(), SessionError> {
        if self.events.len() >= MAX_QUEUED_EVENTS {
            return Err(SessionError::EventQueueOverflow {
                limit: MAX_QUEUED_EVENTS,
            });
        }
        self.events.push(event);
        Ok(())
    }

    /// Reads one server-sequenced `StateDelta` and checks it names `domain_id`, its registered
    /// `delta_type`, and is based on exactly `applied_revision`. The caller decodes the typed
    /// payload and only then commits the new revision.
    pub(super) async fn read_delta(
        &mut self,
        domain_id: u32,
        delta_type: u32,
        applied_revision: u64,
    ) -> Result<RawDelta, SessionError> {
        loop {
            let (server_sequence, payload) = self
                .read_sequenced("StateDelta", MessageType::StateDelta)
                .await?;
            let delta = decode_state_delta(&payload)?;
            if delta.domain_id != domain_id {
                if !matches!(
                    domain_id,
                    npc_service::STATE_DOMAIN_NPC_CONVERSATION
                        | npc_service::STATE_DOMAIN_NPC_TRADE_WINDOW
                        | player_trade::STATE_DOMAIN_PLAYER_TRADE
                ) {
                    return Err(SessionError::UnexpectedDomain {
                        expected: domain_id,
                        actual: delta.domain_id,
                    });
                }
                let event = self.apply_state_delta(server_sequence, &payload)?;
                self.queue_event(event)?;
                continue;
            }
            let revision = self
                .gated_revisions
                .iter()
                .find(|(id, _)| *id == domain_id)
                .map_or(applied_revision, |(_, revision)| *revision);
            check_delta(&delta, delta_type, revision)?;
            return Ok(RawDelta {
                server_sequence,
                base_revision: delta.base_revision,
                new_revision: delta.new_revision,
                payload: delta.payload.to_vec(),
            });
        }
    }

    /// Reads the next post-admission frame that is not a `LivenessProbe` (each probe is answered
    /// with a `LivenessAck` first). The envelope is validated as post-admission server traffic
    /// and must carry the admitted `connection_generation`.
    pub(super) async fn read_post_admission(
        &mut self,
        label: &'static str,
    ) -> Result<Vec<u8>, SessionError> {
        loop {
            let frame = bounded(self.deadline, label, read_frame(&mut self.stream)).await?;
            self.last_received = tokio::time::Instant::now();
            let envelope = decode_wire_envelope(&frame)?;
            envelope.validate(Direction::ServerToClient, true)?;
            self.check_generation(&envelope)?;
            if envelope.message_type() == MessageType::SnapshotBegin {
                if let Some(changed) = self
                    .replace_snapshot_with_scope_change(envelope.payload())
                    .await?
                {
                    return Err(changed);
                }
                continue;
            }
            if envelope.message_type() != MessageType::LivenessProbe {
                return Ok(frame);
            }
            self.answer_probe(envelope.payload()).await?;
        }
    }
}

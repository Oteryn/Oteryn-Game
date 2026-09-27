//! One gameplay connection. The transport decodes bounded Foundation frames and
//! hands the validated bootstrap to the owning admission authority; it never
//! decides admission itself and fails closed for every message it does not own.

use crate::foundation::{
    AuthenticatedTransportRefV1, ChannelId, CharacterId, ExactActorRef, FoundationProtocolError,
    GameSessionId, MessageType, WorldId, decode_wire_envelope, encode_protocol_error,
    encode_server_accepted,
};
use std::future::Future;
use tokio::io::{AsyncRead, AsyncWrite};

use super::tcp_tls::{read_frame, write_frame};
use super::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, DELTA_TYPE_WORLD_SPATIAL_V1,
    SNAPSHOT_TYPE_WORLD_SPATIAL_V1, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, StepDirection,
    StepDisposition, WorldSpatialObservation, decode_step_intent, encode_step_result,
    encode_world_spatial,
};
use crate::foundation::{
    CommandStatus, DomainSnapshot, encode_command_protocol_error, encode_command_result,
    encode_single_chunk_snapshot, encode_state_delta,
};

/// Foundation schema revision served by this build (FND-02 v1 contract).
pub(crate) const SERVER_SCHEMA_REVISION: u32 = 1;

/// Validated fresh-admission input handed to the owning authority.
#[derive(Debug, Clone, Copy)]
pub(crate) struct FreshAdmissionAttempt<'a> {
    pub(crate) character_id: CharacterId,
    pub(crate) admission_material: &'a [u8],
    pub(crate) game_session_id: GameSessionId,
    pub(crate) transport: AuthenticatedTransportRefV1,
}

/// Authority-committed admission: the only state that lets a transport claim a
/// GameSession.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AdmittedSession {
    pub(crate) game_session_id: GameSessionId,
    pub(crate) world_id: WorldId,
    pub(crate) channel_id: ChannelId,
    /// Present for the composed production fresh-admission path. Transport-only
    /// fixtures may omit it; the transport never invents or mutates actor authority.
    pub(crate) runtime_actor: Option<ExactActorRef>,
    /// #935 first-entry positioning of `runtime_actor`. Only a positioned actor
    /// may later become input-eligible.
    pub(crate) first_entry: FirstEntryOutcome,
}

/// Outcome of #935 first-entry positioning for one admitted session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FirstEntryOutcome {
    /// Transport-only fixture: there is no Channel runtime actor.
    #[cfg_attr(not(test), allow(dead_code))]
    NotApplicable,
    /// The Channel owner wrote the start position.
    Positioned,
    /// The same initialization had already completed; nothing was written.
    Reconciled,
    /// Current authority no longer matches the admission; nothing was written.
    RefusedStaleAuthority,
    /// Current authority could not be read; nothing was written.
    RefusedUnavailable,
    /// The Channel owner refused the write; nothing was written.
    RefusedByChannel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdmissionRefusal {
    /// The material or current owner facts do not admit this attempt.
    Rejected,
    /// The owning authority could not decide; nothing was admitted.
    Unavailable,
}

/// The owning fresh-admission authority. Production composes the real owners;
/// nothing on the transport side can mark a connection admitted.
pub(crate) trait FreshAdmissionAuthority {
    fn admit(
        &self,
        attempt: FreshAdmissionAttempt<'_>,
    ) -> impl Future<Output = Result<AdmittedSession, AdmissionRefusal>>;

    /// The admitted actor's current own-actor observation for the initial snapshot, or `None`
    /// when this authority serves no gameplay (transport-only fixtures).
    fn observe(
        &self,
        _actor: ExactActorRef,
    ) -> impl Future<Output = Option<WorldSpatialObservation>> {
        async { None }
    }

    /// One `WORLD_ACTOR_STEP_INTENT` for the admitted actor, applied by the Channel owner.
    fn step(
        &self,
        _actor: ExactActorRef,
        _direction: StepDirection,
    ) -> impl Future<Output = StepOutcome> {
        async { StepOutcome::rejected() }
    }
}

/// The outcome of one step: its disposition and, only when it moved, the new observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StepOutcome {
    pub(crate) disposition: StepDisposition,
    pub(crate) moved_to: Option<WorldSpatialObservation>,
}

impl StepOutcome {
    pub(crate) const fn rejected() -> Self {
        Self {
            disposition: StepDisposition::Rejected,
            moved_to: None,
        }
    }
}

/// Fresh identifiers for one connection attempt.
pub(crate) trait ConnectionIdentifiers {
    /// `None` when no unpredictable identifier can be produced.
    fn game_session_id(&self) -> Option<GameSessionId>;
    fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectionEnd {
    /// Closed before admission after a Foundation protocol violation.
    ProtocolViolation(FoundationProtocolError),
    /// Admission refused by the owning authority; nothing was admitted.
    AdmissionRefused(AdmissionRefusal),
    /// Resume is not served by this seam yet; nothing was admitted.
    ResumeUnavailable,
    /// Admitted, then closed after unsupported post-admission input.
    AdmittedThenClosed(AdmittedSession, FoundationProtocolError),
    /// Admitted, then the peer closed or the transport failed.
    AdmittedThenDisconnected(AdmittedSession),
    /// The transport failed before admission.
    TransportFailed,
}

/// Admitted connection generation issued with `ServerAccepted`.
const ADMITTED_GENERATION: u64 = 1;

/// Whole connection lifecycle without resource policy; the listener runs the
/// same phases under its budgets.
#[cfg(test)]
pub(crate) async fn serve_connection<S, A, I>(
    stream: &mut S,
    authority: &A,
    identifiers: &I,
) -> ConnectionEnd
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
    I: ConnectionIdentifiers,
{
    let Ok(frame) = read_frame(stream).await else {
        return ConnectionEnd::TransportFailed;
    };
    match admit_frame(stream, &frame, authority, identifiers).await {
        Ok(admitted) => hold_admitted(stream, admitted).await,
        Err(end) => end,
    }
}

/// Decode the entry frame, obtain the owning authority's decision and, only on
/// commit, write `ServerAccepted`.
pub(crate) async fn admit_frame<S, A, I>(
    stream: &mut S,
    frame: &[u8],
    authority: &A,
    identifiers: &I,
) -> Result<AdmittedSession, ConnectionEnd>
where
    S: AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
    I: ConnectionIdentifiers,
{
    let envelope = match decode_wire_envelope(frame) {
        Ok(envelope) => envelope,
        Err(error) => return Err(reject(stream, error, 0).await),
    };
    let bootstrap = match envelope.message_type() {
        MessageType::ClientBootstrap => envelope.client_bootstrap(),
        MessageType::ClientResume => {
            return Err(match envelope.client_resume() {
                Ok(_) => ConnectionEnd::ResumeUnavailable,
                Err(error) => reject(stream, error, 0).await,
            });
        }
        _ => Err(FoundationProtocolError::MalformedEnvelope),
    };
    let bootstrap = match bootstrap {
        Ok(bootstrap) => bootstrap,
        Err(error) => return Err(reject(stream, error, 0).await),
    };
    let (Some(game_session_id), Some(transport)) =
        (identifiers.game_session_id(), identifiers.transport_ref())
    else {
        return Err(ConnectionEnd::AdmissionRefused(
            AdmissionRefusal::Unavailable,
        ));
    };
    let attempt = FreshAdmissionAttempt {
        character_id: bootstrap.character_id,
        admission_material: bootstrap.admission_material,
        game_session_id,
        transport,
    };
    let admitted = authority
        .admit(attempt)
        .await
        .map_err(ConnectionEnd::AdmissionRefused)?;
    let accepted = encode_server_accepted(&crate::foundation::ServerAcceptedValue {
        game_session_id: admitted.game_session_id,
        world_id: admitted.world_id,
        channel_id: admitted.channel_id,
        connection_generation: ADMITTED_GENERATION,
        current_server_sequence: 0,
        next_command_id: 1,
        schema_revision: SERVER_SCHEMA_REVISION,
        selected_capabilities: &[],
    })
    .map_err(|_| ConnectionEnd::AdmittedThenDisconnected(admitted))?;
    write_frame(stream, &accepted)
        .await
        .map_err(|_| ConnectionEnd::AdmittedThenDisconnected(admitted))?;
    Ok(admitted)
}

/// No gameplay command, state or liveness semantics are allocated to the seam:
/// any post-admission input ends the connection without mutation.
pub(crate) async fn hold_admitted<S>(stream: &mut S, admitted: AdmittedSession) -> ConnectionEnd
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    let Ok(frame) = read_frame(stream).await else {
        return ConnectionEnd::AdmittedThenDisconnected(admitted);
    };
    let error = match decode_wire_envelope(&frame) {
        Err(error) => error,
        Ok(envelope) if envelope.connection_generation() != ADMITTED_GENERATION => {
            FoundationProtocolError::StaleConnectionGeneration
        }
        Ok(_) => FoundationProtocolError::UnknownMessageType,
    };
    let _ = send_error(stream, error, ADMITTED_GENERATION).await;
    ConnectionEnd::AdmittedThenClosed(admitted, error)
}

/// FIRST-CONTROL post-admission play (FND-02 §§14-16, #642/#139). A positioned actor gets the
/// initial `WORLD_SPATIAL_VISIBILITY` snapshot (target sequence 0) before any command is accepted;
/// each `ClientCommand` must carry the next CommandId. A step is one Channel-owner work item
/// (`MOVE-RL-02` = 1) answered by a sequenced `CommandResult` and, when it moved, a sequenced
/// `StateDelta`. An actor that is not positioned, or an authority without gameplay, keeps the
/// admission-only behaviour.
pub(crate) async fn serve_admitted<S, A>(
    stream: &mut S,
    admitted: AdmittedSession,
    authority: &A,
) -> ConnectionEnd
where
    S: AsyncRead + AsyncWrite + Unpin,
    A: FreshAdmissionAuthority,
{
    let playable = matches!(
        admitted.first_entry,
        FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
    );
    let Some(actor) = admitted.runtime_actor.filter(|_| playable) else {
        return hold_admitted(stream, admitted).await;
    };
    let Some(baseline) = authority.observe(actor).await else {
        return hold_admitted(stream, admitted).await;
    };
    let mut revision = 1_u64;
    let payload = encode_world_spatial(&baseline);
    let snapshot = encode_single_chunk_snapshot(
        ADMITTED_GENERATION,
        1,
        0,
        &[DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            revision,
            snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
            payload: &payload,
        }],
    );
    let Ok(snapshot) = snapshot else {
        return ConnectionEnd::AdmittedThenDisconnected(admitted);
    };
    for frame in &snapshot {
        if write_frame(stream, frame).await.is_err() {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        }
    }
    let mut sequence = 0_u64;
    let mut next_command = 1_u64;
    loop {
        let Ok(frame) = read_frame(stream).await else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        let command = match decode_wire_envelope(&frame) {
            Err(error) => return close_admitted(stream, admitted, error).await,
            Ok(envelope) if envelope.connection_generation() != ADMITTED_GENERATION => {
                return close_admitted(
                    stream,
                    admitted,
                    FoundationProtocolError::StaleConnectionGeneration,
                )
                .await;
            }
            Ok(envelope) if envelope.message_type() != MessageType::ClientCommand => {
                return close_admitted(
                    stream,
                    admitted,
                    FoundationProtocolError::UnknownMessageType,
                )
                .await;
            }
            Ok(envelope) => match envelope.client_command(ADMITTED_GENERATION) {
                Ok(command) => command,
                Err(error) => return close_admitted(stream, admitted, error).await,
            },
        };
        if command.command_id != next_command {
            // FND-02 §13.2: a lower ID is never re-executed and no outcome is retained; a
            // higher ID is a gap that names the expected ID.
            let (error, expected) = if command.command_id < next_command {
                (FoundationProtocolError::CommandOutcomeExpired, 0)
            } else {
                (FoundationProtocolError::CommandSequenceGap, next_command)
            };
            let _ = match encode_command_protocol_error(
                error,
                ADMITTED_GENERATION,
                command.command_id,
                expected,
            ) {
                Ok(frame) => write_frame(stream, &frame).await,
                Err(_) => Ok(()),
            };
            return ConnectionEnd::AdmittedThenClosed(admitted, error);
        }
        // Unknown command types and malformed step payloads have no effect. The result payload
        // belongs to the command type, so an unregistered type gets none.
        let registered = command.command_type == COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT;
        let outcome = match (registered, decode_step_intent(command.payload)) {
            (true, Ok(direction)) => authority.step(actor, direction).await,
            _ => StepOutcome::rejected(),
        };
        let (Some(result_sequence), Some(following)) =
            (sequence.checked_add(1), next_command.checked_add(1))
        else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        sequence = result_sequence;
        next_command = following;
        let status = if outcome.disposition == StepDisposition::Rejected {
            CommandStatus::Rejected
        } else {
            CommandStatus::Accepted
        };
        let Ok(result) = encode_command_result(
            ADMITTED_GENERATION,
            sequence,
            command.command_id,
            status,
            &if registered {
                encode_step_result(outcome.disposition)
            } else {
                Vec::new()
            },
        ) else {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        };
        if write_frame(stream, &result).await.is_err() {
            return ConnectionEnd::AdmittedThenDisconnected(admitted);
        }
        if let Some(observation) = outcome.moved_to {
            let (Some(delta_sequence), Some(new_revision)) =
                (sequence.checked_add(1), revision.checked_add(1))
            else {
                return ConnectionEnd::AdmittedThenDisconnected(admitted);
            };
            let Ok(delta) = encode_state_delta(
                ADMITTED_GENERATION,
                delta_sequence,
                STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision,
                new_revision,
                DELTA_TYPE_WORLD_SPATIAL_V1,
                &encode_world_spatial(&observation),
            ) else {
                return ConnectionEnd::AdmittedThenDisconnected(admitted);
            };
            sequence = delta_sequence;
            revision = new_revision;
            if write_frame(stream, &delta).await.is_err() {
                return ConnectionEnd::AdmittedThenDisconnected(admitted);
            }
        }
    }
}

async fn close_admitted<S: AsyncWrite + Unpin>(
    stream: &mut S,
    admitted: AdmittedSession,
    error: FoundationProtocolError,
) -> ConnectionEnd {
    let _ = send_error(stream, error, ADMITTED_GENERATION).await;
    ConnectionEnd::AdmittedThenClosed(admitted, error)
}

async fn reject<S: AsyncWrite + Unpin>(
    stream: &mut S,
    error: FoundationProtocolError,
    generation: u64,
) -> ConnectionEnd {
    let _ = send_error(stream, error, generation).await;
    ConnectionEnd::ProtocolViolation(error)
}

async fn send_error<S: AsyncWrite + Unpin>(
    stream: &mut S,
    error: FoundationProtocolError,
    generation: u64,
) -> std::io::Result<()> {
    let frame = encode_protocol_error(error, generation)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    write_frame(stream, &frame).await
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};
    use std::error::Error;
    use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream};

    const CHARACTER: [u8; 16] = uuid_v7(0x11);
    const SESSION: [u8; 16] = uuid_v7(0x22);
    const WORLD: [u8; 16] = uuid_v7(0x33);
    const CHANNEL: [u8; 16] = uuid_v7(0x44);

    const fn uuid_v7(tag: u8) -> [u8; 16] {
        let mut bytes = [tag; 16];
        bytes[6] = 0x70 | (tag & 0x0f);
        bytes[8] = 0x80 | (tag & 0x3f);
        bytes
    }

    struct Authority {
        calls: Cell<usize>,
        outcome: Result<(), AdmissionRefusal>,
        seen: RefCell<Vec<(CharacterId, Vec<u8>)>>,
    }

    impl Authority {
        fn new(outcome: Result<(), AdmissionRefusal>) -> Self {
            Self {
                calls: Cell::new(0),
                outcome,
                seen: RefCell::new(Vec::new()),
            }
        }
    }

    impl FreshAdmissionAuthority for Authority {
        async fn admit(
            &self,
            attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            self.calls.set(self.calls.get() + 1);
            self.seen
                .borrow_mut()
                .push((attempt.character_id, attempt.admission_material.to_vec()));
            self.outcome.map(|()| AdmittedSession {
                game_session_id: attempt.game_session_id,
                world_id: WorldId::decode(&WORLD).expect("world"),
                channel_id: ChannelId::decode(&CHANNEL).expect("channel"),
                runtime_actor: None,
                first_entry: FirstEntryOutcome::NotApplicable,
            })
        }
    }

    struct Identifiers;
    impl ConnectionIdentifiers for Identifiers {
        fn game_session_id(&self) -> Option<GameSessionId> {
            GameSessionId::decode(&SESSION).ok()
        }
        fn transport_ref(&self) -> Option<AuthenticatedTransportRefV1> {
            AuthenticatedTransportRefV1::decode(&[0x5a; 16]).ok()
        }
    }

    fn varint(output: &mut Vec<u8>, mut value: u64) {
        while value >= 0x80 {
            output.push((value as u8 & 0x7f) | 0x80);
            value >>= 7;
        }
        output.push(value as u8);
    }

    fn scalar(output: &mut Vec<u8>, field: u64, value: u64) {
        varint(output, field << 3);
        varint(output, value);
    }

    fn bytes(output: &mut Vec<u8>, field: u64, value: &[u8]) {
        varint(output, (field << 3) | 2);
        varint(output, value.len() as u64);
        output.extend_from_slice(value);
    }

    fn envelope(message_type: u64, generation: u64, payload: &[u8]) -> Vec<u8> {
        let mut output = Vec::new();
        scalar(&mut output, 1, message_type);
        if generation != 0 {
            scalar(&mut output, 2, generation);
        }
        bytes(&mut output, 4, payload);
        output
    }

    fn bootstrap(major: u64, profile: u64, material: &[u8]) -> Vec<u8> {
        let mut payload = Vec::new();
        scalar(&mut payload, 1, major);
        scalar(&mut payload, 2, profile);
        scalar(&mut payload, 3, 1);
        bytes(&mut payload, 5, material);
        bytes(&mut payload, 6, &CHARACTER);
        bytes(&mut payload, 7, b"seam-test");
        envelope(1, 0, &payload)
    }

    fn resume() -> Vec<u8> {
        let mut payload = Vec::new();
        bytes(&mut payload, 1, &SESSION);
        bytes(&mut payload, 2, b"recovery");
        scalar(&mut payload, 4, 1);
        scalar(&mut payload, 5, 1);
        scalar(&mut payload, 6, 1);
        bytes(&mut payload, 8, b"seam-test");
        envelope(3, 0, &payload)
    }

    fn framed(body: &[u8]) -> Vec<u8> {
        let mut output = (body.len() as u32).to_be_bytes().to_vec();
        output.extend_from_slice(body);
        output
    }

    /// Drive one connection with the given client frames; return the end state
    /// and every frame the server wrote.
    async fn drive(
        authority: &Authority,
        client_frames: &[Vec<u8>],
    ) -> Result<(ConnectionEnd, Vec<Vec<u8>>), Box<dyn Error>> {
        let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 21);
        for frame in client_frames {
            client.write_all(&framed(frame)).await?;
        }
        client.shutdown().await?;
        let end = serve_connection(&mut server, authority, &Identifiers).await;
        drop(server);
        let mut output = Vec::new();
        client.read_to_end(&mut output).await?;
        let mut frames = Vec::new();
        let mut cursor = 0;
        while cursor < output.len() {
            let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
            frames.push(output[cursor + 4..cursor + 4 + length].to_vec());
            cursor += 4 + length;
        }
        Ok((end, frames))
    }

    fn run<F: Future<Output = Result<(), Box<dyn Error>>>>(test: F) -> Result<(), Box<dyn Error>> {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(test)
    }

    fn error_frame(error: FoundationProtocolError, generation: u64) -> Vec<u8> {
        encode_protocol_error(error, generation).expect("error frame")
    }

    /// A positioned actor on a one-row room: east of the start is walkable, every other
    /// direction is blocked.
    struct StepAuthority {
        steps: RefCell<Vec<StepDirection>>,
    }

    impl FreshAdmissionAuthority for StepAuthority {
        async fn admit(
            &self,
            _attempt: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }

        async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(at(0))
        }

        async fn step(&self, _actor: ExactActorRef, direction: StepDirection) -> StepOutcome {
            self.steps.borrow_mut().push(direction);
            if direction == StepDirection::East {
                StepOutcome {
                    disposition: StepDisposition::Moved,
                    moved_to: Some(at(1)),
                }
            } else {
                StepOutcome {
                    disposition: StepDisposition::Blocked,
                    moved_to: None,
                }
            }
        }
    }

    fn at(x: i32) -> WorldSpatialObservation {
        WorldSpatialObservation {
            content_generation: [0x5c; 32],
            actor_position: super::super::world_spatial::ActorPosition { x, y: 0, floor: 0 },
        }
    }

    fn command(generation: u64, id: u64, command_type: u64, direction: StepDirection) -> Vec<u8> {
        let mut payload = Vec::new();
        scalar(&mut payload, 1, id);
        scalar(&mut payload, 2, command_type);
        bytes(
            &mut payload,
            4,
            &super::super::world_spatial::encode_step_intent(direction),
        );
        envelope(7, generation, &payload)
    }

    /// Serve one positioned admitted session over the given client frames.
    async fn drive_admitted(
        authority: &StepAuthority,
        client_frames: &[Vec<u8>],
    ) -> Result<(ConnectionEnd, Vec<Vec<u8>>), Box<dyn Error>> {
        let world_id = WorldId::decode(&WORLD)?;
        let channel_id = ChannelId::decode(&CHANNEL)?;
        let admitted = AdmittedSession {
            game_session_id: GameSessionId::decode(&SESSION)?,
            world_id,
            channel_id,
            runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
            first_entry: FirstEntryOutcome::Positioned,
        };
        let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 21);
        for frame in client_frames {
            client.write_all(&framed(frame)).await?;
        }
        client.shutdown().await?;
        let end = serve_admitted(&mut server, admitted, authority).await;
        drop(server);
        let mut output = Vec::new();
        client.read_to_end(&mut output).await?;
        let mut frames = Vec::new();
        let mut cursor = 0;
        while cursor < output.len() {
            let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
            frames.push(output[cursor + 4..cursor + 4 + length].to_vec());
            cursor += 4 + length;
        }
        Ok((end, frames))
    }

    fn baseline() -> Vec<Vec<u8>> {
        encode_single_chunk_snapshot(
            ADMITTED_GENERATION,
            1,
            0,
            &[DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 1,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &encode_world_spatial(&at(0)),
            }],
        )
        .expect("snapshot")
        .into()
    }

    #[test]
    fn admitted_steps_are_sequenced_and_a_replayed_id_expires() -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = StepAuthority {
                steps: RefCell::new(Vec::new()),
            };
            let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
            let (end, frames) = drive_admitted(
                &authority,
                &[
                    command(1, 1, step, StepDirection::East),
                    command(1, 2, step, StepDirection::North),
                    command(1, 3, 0x7fff, StepDirection::East),
                    command(1, 2, step, StepDirection::East),
                ],
            )
            .await?;
            let mut expected = baseline();
            let moved = encode_step_result(StepDisposition::Moved);
            let blocked = encode_step_result(StepDisposition::Blocked);
            expected.extend([
                encode_command_result(1, 1, 1, CommandStatus::Accepted, &moved)?,
                encode_state_delta(
                    1,
                    2,
                    STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    1,
                    2,
                    DELTA_TYPE_WORLD_SPATIAL_V1,
                    &encode_world_spatial(&at(1)),
                )?,
                encode_command_result(1, 3, 2, CommandStatus::Accepted, &blocked)?,
                encode_command_result(1, 4, 3, CommandStatus::Rejected, &[])?,
                encode_command_protocol_error(
                    FoundationProtocolError::CommandOutcomeExpired,
                    1,
                    2,
                    0,
                )?,
            ]);
            assert_eq!(frames, expected);
            assert!(matches!(
                end,
                ConnectionEnd::AdmittedThenClosed(
                    _,
                    FoundationProtocolError::CommandOutcomeExpired
                )
            ));
            // The unregistered type and the replayed ID never reached Movement.
            assert_eq!(
                *authority.steps.borrow(),
                [StepDirection::East, StepDirection::North]
            );
            Ok(())
        })
    }

    #[test]
    fn admitted_session_closes_on_gap_foreign_message_and_stale_generation()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
            for (frame, error, expected) in [
                (
                    command(1, 3, step, StepDirection::East),
                    FoundationProtocolError::CommandSequenceGap,
                    encode_command_protocol_error(
                        FoundationProtocolError::CommandSequenceGap,
                        1,
                        3,
                        1,
                    )?,
                ),
                (
                    envelope(99, 1, &[]),
                    FoundationProtocolError::UnknownMessageType,
                    error_frame(FoundationProtocolError::UnknownMessageType, 1),
                ),
                (
                    command(2, 1, step, StepDirection::East),
                    FoundationProtocolError::StaleConnectionGeneration,
                    error_frame(FoundationProtocolError::StaleConnectionGeneration, 1),
                ),
            ] {
                let authority = StepAuthority {
                    steps: RefCell::new(Vec::new()),
                };
                let (end, frames) = drive_admitted(&authority, &[frame]).await?;
                let mut all = baseline();
                all.push(expected);
                assert_eq!(frames, all);
                assert!(matches!(end, ConnectionEnd::AdmittedThenClosed(_, e) if e == error));
                assert!(authority.steps.borrow().is_empty());
            }
            Ok(())
        })
    }

    #[test]
    fn admission_accepts_only_after_the_owning_authority_commits() -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = Authority::new(Ok(()));
            let (end, frames) = drive(&authority, &[bootstrap(1, 1, b"grant")]).await?;
            let admitted = AdmittedSession {
                game_session_id: GameSessionId::decode(&SESSION)?,
                world_id: WorldId::decode(&WORLD)?,
                channel_id: ChannelId::decode(&CHANNEL)?,
                runtime_actor: None,
                first_entry: FirstEntryOutcome::NotApplicable,
            };
            assert_eq!(end, ConnectionEnd::AdmittedThenDisconnected(admitted));
            assert_eq!(authority.calls.get(), 1);
            assert_eq!(
                authority.seen.borrow().as_slice(),
                &[(CharacterId::decode(&CHARACTER)?, b"grant".to_vec())]
            );
            let accepted = decode_wire_envelope(&frames[0])?;
            assert_eq!(accepted.message_type(), MessageType::ServerAccepted);
            assert_eq!(accepted.connection_generation(), 0);
            assert_eq!(frames.len(), 1);
            Ok(())
        })
    }

    #[test]
    fn admission_refusal_writes_no_acceptance() -> Result<(), Box<dyn Error>> {
        run(async {
            for refusal in [AdmissionRefusal::Rejected, AdmissionRefusal::Unavailable] {
                let authority = Authority::new(Err(refusal));
                let (end, frames) = drive(&authority, &[bootstrap(1, 1, b"grant")]).await?;
                assert_eq!(end, ConnectionEnd::AdmissionRefused(refusal));
                assert_eq!(authority.calls.get(), 1);
                assert!(frames.is_empty());
            }
            Ok(())
        })
    }

    #[test]
    fn admission_version_and_profile_rejected_before_authority() -> Result<(), Box<dyn Error>> {
        run(async {
            for (frame, error) in [
                (
                    bootstrap(2, 1, b"grant"),
                    FoundationProtocolError::ProtocolMajorMismatch,
                ),
                (
                    bootstrap(1, 2, b"grant"),
                    FoundationProtocolError::TransportProfileMismatch,
                ),
            ] {
                let authority = Authority::new(Ok(()));
                let (end, frames) = drive(&authority, &[frame]).await?;
                assert_eq!(end, ConnectionEnd::ProtocolViolation(error));
                assert_eq!(authority.calls.get(), 0);
                assert_eq!(frames, vec![error_frame(error, 0)]);
            }
            Ok(())
        })
    }

    #[test]
    fn admission_phase_direction_and_malformed_input_rejected_before_authority()
    -> Result<(), Box<dyn Error>> {
        run(async {
            let command = envelope(7, 0, &[]);
            let server_direction = envelope(2, 0, &[]);
            let unknown = envelope(99, 0, &[]);
            let generation = {
                let mut payload = Vec::new();
                scalar(&mut payload, 1, 1);
                scalar(&mut payload, 2, 1);
                scalar(&mut payload, 3, 1);
                bytes(&mut payload, 5, b"grant");
                bytes(&mut payload, 6, &CHARACTER);
                bytes(&mut payload, 7, b"seam-test");
                envelope(1, 3, &payload)
            };
            for frame in [command, server_direction, unknown, generation, vec![0xff]] {
                let authority = Authority::new(Ok(()));
                let (end, frames) = drive(&authority, &[frame]).await?;
                assert!(
                    matches!(end, ConnectionEnd::ProtocolViolation(_)),
                    "{end:?}"
                );
                assert_eq!(authority.calls.get(), 0);
                assert_eq!(frames.len(), 1);
                assert_eq!(
                    decode_wire_envelope(&frames[0])?.message_type(),
                    MessageType::ProtocolError
                );
            }
            Ok(())
        })
    }

    #[test]
    fn admission_resume_is_not_served_and_admits_nothing() -> Result<(), Box<dyn Error>> {
        run(async {
            let authority = Authority::new(Ok(()));
            let (end, frames) = drive(&authority, &[resume()]).await?;
            assert_eq!(end, ConnectionEnd::ResumeUnavailable);
            assert_eq!(authority.calls.get(), 0);
            assert!(frames.is_empty());
            Ok(())
        })
    }

    #[test]
    fn admission_post_admission_input_closes_without_gameplay() -> Result<(), Box<dyn Error>> {
        run(async {
            for (frame, error) in [
                (
                    envelope(7, 1, &[]),
                    FoundationProtocolError::UnknownMessageType,
                ),
                (
                    envelope(7, 2, &[]),
                    FoundationProtocolError::StaleConnectionGeneration,
                ),
            ] {
                let authority = Authority::new(Ok(()));
                let (end, frames) = drive(&authority, &[bootstrap(1, 1, b"grant"), frame]).await?;
                let ConnectionEnd::AdmittedThenClosed(_, reported) = end else {
                    return Err(format!("{end:?}").into());
                };
                assert_eq!(reported, error);
                assert_eq!(authority.calls.get(), 1);
                assert_eq!(frames.len(), 2);
                assert_eq!(frames[1], error_frame(error, ADMITTED_GENERATION));
            }
            Ok(())
        })
    }
}

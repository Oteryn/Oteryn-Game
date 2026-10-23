use super::super::super::connection::LogoutReply;
use super::*;
use crate::foundation::CommandStatus;
use oteryn_protocol_oteryn::logout::{
    LogoutResult, decode_logout_result, encode_logout_intent,
};
use oteryn_protocol_oteryn::{decode_command_result, decode_protocol_error};
use std::time::Duration;

struct LogoutAuthority {
    reply: Cell<LogoutReply>,
    /// How long one logout takes; frames read meanwhile are held.
    delay: Duration,
    logouts: Cell<u32>,
    steps: Cell<u32>,
    /// SPEED-1: the pace each step records; zero leaves steps unpaced.
    pace: Duration,
}

impl LogoutAuthority {
    fn new(reply: LogoutReply) -> Self {
        Self {
            reply: Cell::new(reply),
            delay: Duration::ZERO,
            logouts: Cell::new(0),
            steps: Cell::new(0),
            pace: Duration::ZERO,
        }
    }
}

impl FreshAdmissionAuthority for LogoutAuthority {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Unavailable)
    }

    async fn resume(
        &self,
        _attempt: ResumeAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(WorldSpatialObservation {
            content_generation: [0x5c; 32],
            actor_position: ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
        })
    }

    async fn step(&self, _actor: ExactActorRef, _direction: StepDirection) -> StepOutcome {
        self.steps.set(self.steps.get() + 1);
        StepOutcome {
            disposition: StepDisposition::Blocked,
            moved_to: None,
        }
    }

    async fn paced_step(
        &self,
        actor: ExactActorRef,
        _session: GameSessionId,
        direction: StepDirection,
    ) -> (StepOutcome, Option<Duration>) {
        let pace = (!self.pace.is_zero()).then_some(self.pace);
        (self.step(actor, direction).await, pace)
    }

    async fn logout(&self, _admitted: AdmittedSession) -> LogoutReply {
        self.logouts.set(self.logouts.get() + 1);
        if !self.delay.is_zero() {
            tokio::time::sleep(self.delay).await;
        }
        self.reply.get()
    }
}

fn admitted(selected: &[u32]) -> Result<AdmittedSession, Box<dyn Error>> {
    super::session(SessionContinuity {
        selected_capabilities: super::selection(selected),
        ..SessionContinuity::FRESH
    })
}

/// Serves `admitted`; the client writes `frames`, stays open for `linger`, then closes.
async fn serve_open(
    authority: &LogoutAuthority,
    admitted: AdmittedSession,
    frames: &[Vec<u8>],
    linger: Duration,
) -> Result<(ConnectionEnd, Vec<Vec<u8>>), Box<dyn Error>> {
    let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 20);
    for frame in frames {
        let mut framed = (frame.len() as u32).to_be_bytes().to_vec();
        framed.extend_from_slice(frame);
        client.write_all(&framed).await?;
    }
    let closing = async {
        tokio::time::sleep(linger).await;
        client.shutdown().await?;
        let mut output = Vec::new();
        client.read_to_end(&mut output).await?;
        Ok::<_, std::io::Error>(output)
    };
    let serving = async move {
        let end = serve_admitted(&mut server, admitted, authority, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let (mut serving, mut closing) = (std::pin::pin!(serving), std::pin::pin!(closing));
    let (mut end, mut output) = (None, None);
    std::future::poll_fn(|context| {
        if end.is_none()
            && let std::task::Poll::Ready(value) = serving.as_mut().poll(context)
        {
            end = Some(value);
        }
        if output.is_none()
            && let std::task::Poll::Ready(value) = closing.as_mut().poll(context)
        {
            output = Some(value);
        }
        if end.is_some() && output.is_some() {
            std::task::Poll::Ready(())
        } else {
            std::task::Poll::Pending
        }
    })
    .await;
    Ok((end.ok_or("end")?, split(&output.ok_or("output")??)?))
}

/// One written command result: (command id, status, payload).
type Written = (u64, CommandStatus, Vec<u8>);

/// The command results written.
fn results(frames: &[Vec<u8>]) -> Result<Vec<Written>, Box<dyn Error>> {
    let mut results = Vec::new();
    for frame in frames {
        let envelope = decode_wire_envelope(frame)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload())?;
            results.push((result.command_id, result.status, result.payload.to_vec()));
        }
    }
    Ok(results)
}

fn logout_result(
    results: &[Written],
    id: u64,
) -> Result<(CommandStatus, LogoutResult), Box<dyn Error>> {
    let (_, status, payload) = results
        .iter()
        .find(|(command, _, _)| *command == id)
        .ok_or("no logout result")?;
    Ok((
        *status,
        decode_logout_result(payload).map_err(|e| format!("{e:?}"))?,
    ))
}

fn step() -> Vec<u8> {
    oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::North)
}

#[test]
fn without_capability_21_a_logout_is_refused_before_the_authority() -> Result<(), Box<dyn Error>>
{
    run(async {
        let authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::Accepted));
        let frames = [command(1, 23, &encode_logout_intent())];
        let (end, written) =
            serve_open(&authority, admitted(&[])?, &frames, Duration::ZERO).await?;
        assert!(
            matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
            "{end:?}"
        );
        assert_eq!(
            results(&written)?,
            [(1, CommandStatus::Rejected, Vec::new())]
        );
        assert_eq!(authority.logouts.get(), 0);
        Ok(())
    })
}

#[test]
fn a_non_empty_intent_is_rejected_with_an_empty_payload() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::Accepted));
        let frames = [command(1, 23, &[0x08, 0x01])];
        let (_, written) =
            serve_open(&authority, admitted(&[21])?, &frames, Duration::ZERO).await?;
        assert_eq!(
            results(&written)?,
            [(1, CommandStatus::Rejected, Vec::new())]
        );
        assert_eq!(authority.logouts.get(), 0);
        Ok(())
    })
}

#[test]
fn accepted_is_written_then_the_connection_closes_without_reading_further()
-> Result<(), Box<dyn Error>> {
    run(async {
        let authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::Accepted));
        let frames = [
            command(1, 23, &encode_logout_intent()),
            command(2, 1, &step()),
        ];
        let (end, written) = serve_open(
            &authority,
            admitted(&[21])?,
            &frames,
            Duration::from_millis(50),
        )
        .await?;
        assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)));
        let results = results(&written)?;
        assert_eq!(results.len(), 1);
        assert_eq!(
            logout_result(&results, 1)?,
            (CommandStatus::Accepted, LogoutResult::Accepted)
        );
        assert_eq!(authority.steps.get(), 0);
        Ok(())
    })
}

#[test]
fn in_fight_is_rejected_with_its_retry_and_the_session_keeps_playing()
-> Result<(), Box<dyn Error>> {
    run(async {
        let authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::InFight {
            retry_after_ms: 1_500,
        }));
        let frames = [
            command(1, 23, &encode_logout_intent()),
            command(2, 1, &step()),
        ];
        let (end, written) = serve_open(
            &authority,
            admitted(&[21])?,
            &frames,
            Duration::from_millis(50),
        )
        .await?;
        assert!(
            matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
            "{end:?}"
        );
        let results = results(&written)?;
        assert_eq!(
            logout_result(&results, 1)?,
            (
                CommandStatus::Rejected,
                LogoutResult::InFight {
                    retry_after_ms: 1_500
                }
            )
        );
        assert!(results.iter().any(|(id, _, _)| *id == 2));
        assert_eq!(authority.steps.get(), 1);
        Ok(())
    })
}

#[test]
fn busy_is_rejected_and_a_step_held_behind_the_logout_runs_after_it()
-> Result<(), Box<dyn Error>> {
    run(async {
        let mut authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::Busy));
        authority.delay = Duration::from_millis(30);
        let frames = [
            command(1, 23, &encode_logout_intent()),
            command(2, 1, &step()),
        ];
        let (_, written) = serve_open(
            &authority,
            admitted(&[21])?,
            &frames,
            Duration::from_millis(100),
        )
        .await?;
        let results = results(&written)?;
        assert_eq!(
            results.iter().map(|(id, _, _)| *id).collect::<Vec<_>>(),
            [1, 2]
        );
        assert_eq!(
            logout_result(&results, 1)?,
            (CommandStatus::Rejected, LogoutResult::Busy)
        );
        assert_eq!(authority.steps.get(), 1);
        Ok(())
    })
}

#[test]
fn an_unknown_outcome_writes_no_result_and_ends_as_a_disconnect() -> Result<(), Box<dyn Error>>
{
    run(async {
        let authority = LogoutAuthority::new(LogoutReply::Unknown);
        let frames = [
            command(1, 23, &encode_logout_intent()),
            command(2, 1, &step()),
        ];
        let (end, written) = serve_open(
            &authority,
            admitted(&[21])?,
            &frames,
            Duration::from_millis(50),
        )
        .await?;
        assert!(
            matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
            "{end:?}"
        );
        assert!(results(&written)?.is_empty());
        assert_eq!(authority.steps.get(), 0);
        Ok(())
    })
}

#[test]
fn a_second_logout_before_the_first_result_is_a_protocol_error() -> Result<(), Box<dyn Error>> {
    run(async {
        let mut authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::Busy));
        authority.delay = Duration::from_millis(30);
        let frames = [
            command(1, 23, &encode_logout_intent()),
            command(2, 23, &encode_logout_intent()),
        ];
        let (_, written) = serve_open(
            &authority,
            admitted(&[21])?,
            &frames,
            Duration::from_millis(100),
        )
        .await?;
        assert_eq!(authority.logouts.get(), 1);
        let results = results(&written)?;
        assert_eq!(results.len(), 1);
        assert_eq!(
            logout_result(&results, 1)?,
            (CommandStatus::Rejected, LogoutResult::Busy)
        );
        let last = decode_wire_envelope(written.last().ok_or("no frame")?)?;
        assert_eq!(last.message_type(), MessageType::ProtocolError);
        assert_eq!(
            decode_protocol_error(last.payload())?.error_code,
            FoundationProtocolError::TooManyOutstandingCommands.code()
        );
        Ok(())
    })
}

#[test]
fn a_second_logout_before_an_accepted_result_is_a_protocol_error_after_the_logout()
-> Result<(), Box<dyn Error>> {
    run(async {
        let mut authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::Accepted));
        authority.delay = Duration::from_millis(30);
        let frames = [
            command(1, 23, &encode_logout_intent()),
            command(2, 23, &encode_logout_intent()),
        ];
        let (end, written) = serve_open(
            &authority,
            admitted(&[21])?,
            &frames,
            Duration::from_millis(100),
        )
        .await?;
        assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)));
        assert_eq!(authority.logouts.get(), 1);
        let results = results(&written)?;
        assert_eq!(results.len(), 1);
        assert_eq!(
            logout_result(&results, 1)?,
            (CommandStatus::Accepted, LogoutResult::Accepted)
        );
        let last = decode_wire_envelope(written.last().ok_or("no frame")?)?;
        assert_eq!(last.message_type(), MessageType::ProtocolError);
        assert_eq!(
            decode_protocol_error(last.payload())?.error_code,
            FoundationProtocolError::TooManyOutstandingCommands.code()
        );
        Ok(())
    })
}

#[test]
fn a_second_logout_held_behind_a_paced_step_is_a_protocol_error() -> Result<(), Box<dyn Error>>
{
    run(async {
        // Both logouts are read and held while step 2 waits for its pace, so the first one
        // starts with the second already held (#2127 Codex P2 4279788741).
        let mut authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::Busy));
        authority.pace = Duration::from_millis(30);
        let frames = [
            command(1, 1, &step()),
            command(2, 1, &step()),
            command(3, 23, &encode_logout_intent()),
            command(4, 23, &encode_logout_intent()),
        ];
        let (_, written) = serve_open(
            &authority,
            admitted(&[21])?,
            &frames,
            Duration::from_millis(100),
        )
        .await?;
        assert_eq!(authority.steps.get(), 2);
        assert_eq!(authority.logouts.get(), 1);
        let results = results(&written)?;
        assert_eq!(
            results.iter().map(|(id, _, _)| *id).collect::<Vec<_>>(),
            [1, 2, 3]
        );
        assert_eq!(
            logout_result(&results, 3)?,
            (CommandStatus::Rejected, LogoutResult::Busy)
        );
        let last = decode_wire_envelope(written.last().ok_or("no frame")?)?;
        assert_eq!(last.message_type(), MessageType::ProtocolError);
        assert_eq!(
            decode_protocol_error(last.payload())?.error_code,
            FoundationProtocolError::TooManyOutstandingCommands.code()
        );
        Ok(())
    })
}

#[test]
fn a_second_logout_after_the_first_result_is_an_ordinary_command() -> Result<(), Box<dyn Error>>
{
    run(async {
        let authority = LogoutAuthority::new(LogoutReply::Result(LogoutResult::Busy));
        let frames = [
            command(1, 23, &encode_logout_intent()),
            command(2, 23, &encode_logout_intent()),
        ];
        let (_, written) = serve_open(
            &authority,
            admitted(&[21])?,
            &frames,
            Duration::from_millis(50),
        )
        .await?;
        assert_eq!(authority.logouts.get(), 2);
        assert_eq!(results(&written)?.len(), 2);
        Ok(())
    })
}

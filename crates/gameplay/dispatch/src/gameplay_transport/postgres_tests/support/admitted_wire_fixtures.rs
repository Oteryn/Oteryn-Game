//! Drive several commands through one admitted connection, observing each commit in order.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use crate::foundation::MessageType;
use crate::gameplay_transport::{
    connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted},
    tcp_tls::{read_frame, write_frame},
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, decode_wire_envelope,
    encode_client_command,
};
use tokio::io::DuplexStream;

pub(super) struct Peer {
    stream: DuplexStream,
    generation: u64,
    pub(super) next: u64,
}

impl Peer {
    pub(super) async fn send(&mut self, id: u64, kind: u32, payload: &[u8]) -> TestResult<Vec<u8>> {
        let frame = encode_client_command(
            self.generation,
            &ClientCommandValue {
                command_id: id,
                command_type: kind,
                payload,
            },
        )
        .map_err(debug)?;
        write_frame(&mut self.stream, &frame).await?;
        loop {
            let bytes = read_frame(&mut self.stream).await?;
            let frame = decode_wire_envelope(&bytes).map_err(debug)?;
            match frame.message_type() {
                MessageType::CommandResult => {
                    let result = decode_command_result(frame.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, id);
                    if id == self.next {
                        self.next += 1;
                    } else {
                        assert!(id < self.next);
                        assert_eq!(result.status, CommandStatus::DuplicateReplay);
                    }
                    return Ok(result.payload.to_vec());
                }
                MessageType::LivenessProbe => {
                    super::qengine_rewards_fixtures::acknowledge_probe(
                        &mut self.stream,
                        self.generation,
                        frame.payload(),
                    )
                    .await?
                }
                _ => {}
            }
        }
    }
}

pub(super) async fn drive(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    commands: impl AsyncFnOnce(&mut Peer) -> TestResult,
) -> TestResult {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let before = *admitted;
    let (mut server, stream) = tokio::io::duplex(1 << 20);
    let serving = async move { serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await };
    let driving = async move {
        let mut peer = Peer {
            stream,
            generation: before.continuity.connection_generation,
            next: before.continuity.next_command_id,
        };
        commands(&mut peer).await?;
        peer.stream.shutdown().await?;
        let mut buffer = [0; 4096];
        while peer.stream.read(&mut buffer).await? != 0 {}
        Ok::<_, Box<dyn std::error::Error>>(peer.next)
    };
    let (end, next) = tokio::join!(Box::pin(serving), driving);
    let next = next?;
    let ConnectionEnd::AdmittedThenDisconnected(current) = end else {
        return Err(format!("framed commands ended unexpectedly: {end:?}").into());
    };
    assert_eq!(current.continuity.next_command_id, next);
    *admitted = current;
    Ok(())
}

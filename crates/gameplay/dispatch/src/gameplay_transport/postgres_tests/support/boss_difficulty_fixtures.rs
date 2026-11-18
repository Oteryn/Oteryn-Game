use tokio::io::AsyncWriteExt;
use super::fixture::{self, debug};

use super::TestResult;

use crate::foundation::{MessageType, decode_wire_envelope};

use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, IDLE_LIVENESS, serve_admitted,
};

use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};

use crate::gameplay_transport::ComposedFreshAdmission;

use oteryn_protocol_oteryn::boss_difficulty::{self as wire};

use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    exchange_kind(
        owner,
        admitted,
        id,
        payload,
        wire::COMMAND_TYPE_BOSS_DIFFICULTY,
    )
    .await
}

pub(super) async fn exchange_kind(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    id: u64,
    payload: &[u8],
    kind: u32,
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(4096);
    let serving = async move {
        let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        write_frame(
            &mut client,
            &encode_client_command(
                before.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: id,
                    command_type: kind,
                    payload,
                },
            )
            .map_err(debug)?,
        )
        .await?;
        let reply = loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, id);
                assert!(result.payload.len() <= wire::MAX_RESULT_BYTES);
                break (result.status, result.payload.to_vec());
            }
        };
        client.shutdown().await?;
        Ok(reply)
    };
    let (ended, result): (_, TestResult<(CommandStatus, Vec<u8>)>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    )
    .await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &ended
    {
        *admitted = *current;
    }
    result.map_err(|error| format!("difficulty command {id} ended {ended:?}: {error}").into())
}

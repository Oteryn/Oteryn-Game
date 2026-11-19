use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use super::*;

use crate::foundation::{MessageType, decode_wire_envelope};

use crate::gameplay_transport::connection::{
    ConnectionEnd, IDLE_LIVENESS, serve_admitted,
};

use oteryn_protocol_oteryn::prey as wire;

use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    commands: &[(u64, &[u8])],
) -> TestResult<Vec<(CommandStatus, Vec<u8>)>> {
    let mut input = Vec::new();
    for (command_id, payload) in commands {
        let frame = encode_client_command(
            admitted.continuity.connection_generation,
            &ClientCommandValue {
                command_id: *command_id,
                command_type: wire::COMMAND_TYPE_PREY,
                payload,
            },
        )
        .map_err(debug)?;
        input.extend_from_slice(&u32::try_from(frame.len())?.to_be_bytes());
        input.extend_from_slice(&frame);
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.write_all(&input).await?;
    client.shutdown().await?;
    let end = tokio::time::timeout(
        Duration::from_secs(30),
        serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS),
    )
    .await?;
    match end {
        ConnectionEnd::AdmittedThenDisconnected(current)
        | ConnectionEnd::AdmittedThenClosed(current, _) => *admitted = current,
        other => return Err(format!("prey connection ended {other:?}").into()),
    }
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let mut cursor = 0;
    let mut results = Vec::new();
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let envelope =
            decode_wire_envelope(&output[cursor + 4..cursor + 4 + length]).map_err(debug)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload()).map_err(debug)?;
            results.push((result.status, result.payload.to_vec()));
        }
        cursor += 4 + length;
    }
    assert_eq!(
        results.len(),
        commands.len(),
        "Prey command results: {end:?}"
    );
    Ok(results)
}

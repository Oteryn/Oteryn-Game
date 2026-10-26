use super::*;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    payloads: &[Vec<u8>],
) -> TestResult<(Vec<UseDisposition>, AdmittedSession)> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    for (command_id, payload) in (admitted.continuity.next_command_id..).zip(payloads) {
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: 2,
                payload,
            },
        )?;
        client
            .write_all(&u32::try_from(frame.len())?.to_be_bytes())
            .await?;
        client.write_all(&frame).await?;
    }
    client.shutdown().await?;
    let end = crate::gameplay_transport::connection::serve_admitted(
        &mut server,
        admitted,
        &WireOwner(owner),
        crate::gameplay_transport::connection::IDLE_LIVENESS,
    )
    .await;
    let crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(retained) =
        end
    else {
        return Err(format!("{end:?}").into());
    };
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let mut cursor = 0;
    let mut results = Vec::new();
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let frame =
            crate::foundation::decode_wire_envelope(&output[cursor + 4..cursor + 4 + length])?;
        if frame.message_type() == crate::foundation::MessageType::CommandResult {
            let result = oteryn_protocol_oteryn::decode_command_result(frame.payload())?;
            results.push(
                oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                    .map_err(debug)?,
            );
        }
        cursor += 4 + length;
    }
    Ok((results, retained))
}

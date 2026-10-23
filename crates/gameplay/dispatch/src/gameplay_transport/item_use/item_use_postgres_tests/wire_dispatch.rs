//! The existing BAGS-USE wire exchange, shared by its PostgreSQL regression cases.

use super::*;
use oteryn_protocol_oteryn::world_object::COMMAND_TYPE_USE_INTENT;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    payloads: &[Vec<u8>],
) -> TestResult<Vec<u8>> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    for (command_id, payload) in (1..).zip(payloads) {
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: COMMAND_TYPE_USE_INTENT,
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
    assert!(
        matches!(
            end,
            crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(_)
        ),
        "{end:?}"
    );
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    Ok(output)
}

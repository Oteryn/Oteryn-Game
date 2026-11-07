//! The real connection owns sequencing, USE routing, refreshed inventory and status text.
use super::*;
use oteryn_protocol_oteryn::tutorial_notices::{
    STATE_DOMAIN_TUTORIAL_NOTICES, TutorialNotice, decode_tutorial_notice,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(super) struct Exchange {
    pub results: Vec<UseDisposition>,
    pub notices: Vec<TutorialNotice>,
    pub retained: AdmittedSession,
}

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    payloads: &[Vec<u8>],
) -> TestResult<Exchange> {
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
    let mut notices = Vec::new();
    while cursor < output.len() {
        let prefix = output.get(cursor..cursor + 4).ok_or("short frame length")?;
        let length = usize::try_from(u32::from_be_bytes(prefix.try_into()?))?;
        let end = cursor
            .checked_add(4)
            .and_then(|n| n.checked_add(length))
            .ok_or("frame overflow")?;
        let frame = crate::foundation::decode_wire_envelope(
            output.get(cursor + 4..end).ok_or("short frame")?,
        )?;
        match frame.message_type() {
            crate::foundation::MessageType::CommandResult => {
                let result = oteryn_protocol_oteryn::decode_command_result(frame.payload())?;
                results.push(
                    oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                        .map_err(debug)?,
                );
            }
            crate::foundation::MessageType::StateDelta => {
                let delta = oteryn_protocol_oteryn::decode_state_delta(frame.payload())?;
                if delta.domain_id == STATE_DOMAIN_TUTORIAL_NOTICES {
                    notices.push(decode_tutorial_notice(delta.payload).map_err(debug)?);
                }
            }
            _ => {}
        }
        cursor = end;
    }
    Ok(Exchange {
        results,
        notices,
        retained,
    })
}

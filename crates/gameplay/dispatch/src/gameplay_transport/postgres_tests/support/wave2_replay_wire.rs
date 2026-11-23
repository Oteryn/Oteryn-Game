use super::{TestResult, fixture::debug};
use crate::foundation::{CommandStatus, MessageType, decode_wire_envelope};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, ControlLossResult, FreshAdmissionAuthority, IDLE_LIVENESS,
    ResumeAttempt, serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use oteryn_protocol_oteryn::{
    ClientCommandValue, decode_command_result, encode_client_command_for_session,
};
use std::{
    num::{NonZeroU32, NonZeroU64},
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub(super) struct Exchange {
    pub retained: AdmittedSession,
    pub command_result: Option<(CommandStatus, Vec<u8>)>,
    pub handle: Option<NonZeroU64>,
}

/// Take the ordinary loss and authenticated reconnect route before reopening the wire.
pub(super) async fn reconnect(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    lost: AdmittedSession,
    supported: &[u32],
) -> TestResult<AdmittedSession> {
    use super::house_transfer_fixtures::RecoverySource;
    use crate::foundation::AuthenticatedTransportRefV1;
    let nonce = u8::try_from(lost.continuity.connection_generation + 120)?;
    let token = RecoverySource::token(nonce)?;
    assert_eq!(
        owner.lose_control(lost, Duration::ZERO, None).await,
        ControlLossResult::Recorded,
    );
    let resumed = owner.resume(ResumeAttempt {
        game_session_id: lost.game_session_id,
        recovery_material: token.as_bytes(),
        transport: AuthenticatedTransportRefV1::decode(&[nonce; 16]).map_err(debug)?,
        last_applied_server_sequence: lost.continuity.server_sequence,
        supported_capabilities: supported,
    }).await.map_err(debug)?;
    assert_eq!(resumed.continuity.connection_generation, lost.continuity.connection_generation + 1);
    Ok(resumed)
}

pub(super) async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    command_id: u64,
    command_type: u32,
    payload: &[u8],
    carried: Option<NonZeroU32>,
) -> TestResult<Exchange> {
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let driving = async {
        let mut handle = None;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame)?;
            match envelope.message_type() {
                MessageType::SnapshotCommit => break,
                MessageType::SnapshotChunk if carried.is_some() => {
                    let (_, domains) =
                        oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?;
                    for domain in domains {
                        use oteryn_protocol_oteryn::item_view as wire;
                        if domain.domain_id == wire::STATE_DOMAIN_CHARACTER_INVENTORY {
                            let inventory =
                                wire::decode_character_inventory(domain.payload).map_err(debug)?;
                            handle = inventory
                                .entries
                                .iter()
                                .find(|entry| Some(entry.item_definition_ref) == carried)
                                .map(|entry| entry.handle);
                        }
                    }
                }
                _ => {}
            }
        }
        let actual;
        let payload = if carried.is_some() {
            actual = oteryn_protocol_oteryn::world_object::encode_use_item_intent(
                handle.ok_or("carried handle")?,
            );
            &actual
        } else {
            payload
        };
        write_frame(
            &mut client,
            &encode_client_command_for_session(
                admitted.continuity.connection_generation,
                &ClientCommandValue {
                    command_id,
                    command_type,
                    payload,
                },
                admitted.game_session_id.as_bytes(),
            )?,
        )
        .await?;
        client.shutdown().await?;
        let mut frames = Vec::new();
        client.read_to_end(&mut frames).await?;
        let mut frames = frames.as_slice();
        let mut command_result = None;
        while !frames.is_empty() {
            let (prefix, rest) = frames
                .split_first_chunk::<4>()
                .ok_or("short frame length")?;
            let (frame, rest) = rest
                .split_at_checked(usize::try_from(u32::from_be_bytes(*prefix))?)
                .ok_or("short frame")?;
            let envelope = decode_wire_envelope(frame)?;
            if envelope.message_type() == MessageType::CommandResult {
                let reply = decode_command_result(envelope.payload())?;
                assert_eq!(reply.command_id, command_id);
                assert!(
                    command_result
                        .replace((reply.status, reply.payload.to_vec()))
                        .is_none()
                );
            }
            frames = rest;
        }
        Ok::<_, Box<dyn std::error::Error>>((command_result, handle))
    };
    let (end, result) = tokio::time::timeout(Duration::from_secs(30), async {
        tokio::join!(serving, driving)
    })
    .await?;
    let ConnectionEnd::AdmittedThenDisconnected(retained) = end else {
        return Err(format!("unexpected connection end {end:?}").into());
    };
    let (command_result, handle) = result?;
    Ok(Exchange {
        retained,
        command_result,
        handle,
    })
}

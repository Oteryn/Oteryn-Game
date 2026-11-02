use super::*;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::SessionContinuity;
use oteryn_protocol_oteryn::{decode_state_delta, decode_wire_envelope};
use tokio::io::AsyncReadExt;

fn id(seed: u8) -> [u8; 16] {
    let mut bytes = [seed; 16];
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

#[test]
fn raid_warning_and_advance_use_only_selected_existing_server_message_domain()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for selected in [false, true] {
                for message in [
                    ServerMessage::Warning("A raid is coming!".into()),
                    ServerMessage::Advance("The army has arrived!".into()),
                ] {
                    let mut admitted = AdmittedSession {
                        game_session_id: crate::foundation::GameSessionId::decode(&id(50))?,
                        world_id: crate::foundation::WorldId::decode(&id(42))?,
                        channel_id: crate::foundation::ChannelId::decode(&id(43))?,
                        runtime_actor: None,
                        first_entry: FirstEntryOutcome::NotApplicable,
                        controller: None,
                        continuity: SessionContinuity::FRESH,
                        item_fence: None,
                    };
                    if selected {
                        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                        crate::gameplay_transport::capabilities::PRODUCTION_OFFERED_CAPABILITIES,
                        &[21, CAPABILITY_ZONES_NOLOGOUT_V1],
                    ).ok_or("server-message capability")?;
                    }
                    admitted.continuity.server_message_revision = 4;
                    admitted.continuity.server_sequence = 7;
                    let mut sequence = 7;
                    let (mut server, mut client) = tokio::io::duplex(1024);
                    emit_message(&mut server, &mut admitted, &mut sequence, &message)
                        .await
                        .ok_or("message")?;
                    drop(server);
                    let mut bytes = Vec::new();
                    client.read_to_end(&mut bytes).await?;
                    if selected {
                        let header: [u8; 4] = bytes.get(..4).ok_or("frame header")?.try_into()?;
                        assert_eq!(
                            bytes.len(),
                            usize::try_from(u32::from_be_bytes(header))? + 4
                        );
                        let envelope = decode_wire_envelope(&bytes[4..])?;
                        assert_eq!(envelope.server_sequence(), 8);
                        let delta = decode_state_delta(envelope.payload())?;
                        assert_eq!(
                            (delta.domain_id, delta.base_revision, delta.new_revision),
                            (57, 4, 5)
                        );
                        assert_eq!(
                            decode_server_message(delta.payload)
                                .map_err(|error| format!("{error:?}"))?,
                            message
                        );
                        assert_eq!(
                            (sequence, admitted.continuity.server_message_revision),
                            (8, 5)
                        );
                    } else {
                        assert!(bytes.is_empty());
                        assert_eq!(
                            (sequence, admitted.continuity.server_message_revision),
                            (7, 4)
                        );
                    }
                    assert_eq!(admitted.continuity.server_sequence, sequence);
                }
            }
            Ok(())
        })
}

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
fn server_message_framing_requires_the_domain_and_boss_event_capabilities()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for (capabilities, server_messages, boss_defeats, categorized) in [
                (&[][..], false, false, false),
                (&[CAPABILITY_ZONES_NOLOGOUT_V1][..], true, false, false),
                (
                    &[
                        CAPABILITY_ZONES_NOLOGOUT_V1,
                        CAPABILITY_SERVER_MESSAGE_CATEGORY_V1,
                    ][..],
                    true,
                    false,
                    true,
                ),
                (
                    &[CAPABILITY_ZONES_NOLOGOUT_V1, CAPABILITY_BOSS_DEFEATED_V1][..],
                    true,
                    true,
                    false,
                ),
                (
                    &[
                        CAPABILITY_ZONES_NOLOGOUT_V1,
                        CAPABILITY_SERVER_MESSAGE_CATEGORY_V1,
                        CAPABILITY_BOSS_DEFEATED_V1,
                    ][..],
                    true,
                    true,
                    true,
                ),
            ] {
                for message in [
                    ServerMessage::Warning("A raid is coming!".into()),
                    ServerMessage::Advance("The army has arrived!".into()),
                    ServerMessage::BossDefeated("oteryn:creature.ferumbras".into()),
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
                    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                        crate::gameplay_transport::capabilities::PRODUCTION_OFFERED_CAPABILITIES,
                        capabilities,
                    )
                    .ok_or("server-message capability")?;
                    let selected = server_messages
                        && (boss_defeats || !matches!(message, ServerMessage::BossDefeated(_)));
                    admitted.continuity.server_message_revision = 4;
                    admitted.continuity.server_sequence = 7;
                    let mut sequence = 7;
                    let (mut server, mut client) = tokio::io::duplex(1024);
                    let notice = ServerNotification {
                        message: message.clone(),
                        category: ServerMessageCategory::Raid,
                    };
                    emit_notification(&mut server, &mut admitted, &mut sequence, &notice)
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
                        let decoded = decode_server_notification(delta.payload)
                            .map_err(|error| format!("{error:?}"))?;
                        assert_eq!(decoded.message, message);
                        assert_eq!(
                            decoded.category,
                            if categorized {
                                ServerMessageCategory::Raid
                            } else {
                                ServerMessageCategory::Default
                            }
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

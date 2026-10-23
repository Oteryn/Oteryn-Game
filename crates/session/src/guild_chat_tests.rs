use super::tests::{BoxError, admission_with, block_on, join_peer_with, spatial};
use super::*;
use chat::guild_room::{self, GuildChatSnapshot};
use oteryn_protocol_oteryn::{encode_command_result, encode_state_delta};

#[test]
fn guild_chat_client_keeps_login_motd_and_send_rank_and_not_allowed_status() -> Result<(), BoxError>
{
    block_on(async {
        let mut guild_id = [1; 16];
        guild_id[6] = 0x70;
        guild_id[8] = 0x80;
        let guild = GuildRoomState::Member {
            guild_id,
            rank: 20,
            motd: "Welcome".into(),
        };
        let snapshot = guild_room::encode_snapshot(&GuildChatSnapshot {
            rooms: ChatRoomSet::default(),
            guild: guild.clone(),
        })
        .map_err(|e| format!("{e:?}"))?;
        let (client, mut server) = tokio::io::duplex(1 << 16);
        let line = ChatLine::Guild(GuildChatLine {
            guild_id,
            rank: 2,
            speaker_name: "Leader".into(),
            text: "Guild hello".into(),
        });
        let sent = line.clone();
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                &[7, 36],
                &[7, 36],
                &[],
                (1, spatial(0)),
                &[(
                    STATE_DOMAIN_CHAT,
                    3,
                    guild_room::SNAPSHOT_TYPE_GUILD_CHAT_V1,
                    snapshot,
                )],
            )
            .await?;
            let frame = read_frame(&mut server).await?;
            let envelope = decode_wire_envelope(&frame)?;
            let command = envelope.client_command(1)?;
            assert!(
                matches!(chat::decode_chat_intent(command.payload).map_err(|e| format!("{e:?}"))?,ChatIntent::Guild { text } if text=="hello")
            );
            let payload = chat::encode_chat_intent_result(&chat::ChatIntentResult {
                disposition: ChatDisposition::NotAllowed,
                wait_seconds: 0,
            })
            .map_err(|e| format!("{e:?}"))?;
            write_frame(
                &mut server,
                &encode_command_result(
                    1,
                    41,
                    command.command_id,
                    CommandStatus::Accepted,
                    &payload,
                )?,
            )
            .await?;
            let payload = chat::encode_chat_line(&sent).map_err(|e| format!("{e:?}"))?;
            write_frame(
                &mut server,
                &encode_state_delta(
                    1,
                    42,
                    STATE_DOMAIN_CHAT,
                    3,
                    4,
                    chat::DELTA_TYPE_CHAT_LINE_V1,
                    &payload,
                )?,
            )
            .await?;
            let mut rest = Vec::new();
            server.read_to_end(&mut rest).await?;
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, admission_with(&[7, 36])?).await?;
        assert_eq!(session.chat_log().ok_or("chat log")?.guild_room(), &guild);
        let outcome = session
            .chat(&ChatIntent::Guild {
                text: "hello".into(),
            })
            .await?;
        assert_eq!(outcome.status, CommandStatus::Accepted);
        assert_eq!(outcome.disposition, ChatDisposition::NotAllowed);
        session.service_liveness(Duration::from_millis(50)).await?;
        assert_eq!(
            session.chat_log().ok_or("chat log")?.lines().last(),
            Some(&line)
        );
        drop(session);
        peer.await??;
        Ok(())
    })?
}

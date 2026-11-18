//! Secret Library compatibility regression through the native quest journey.
use super::TestResult;

#[test]
fn qcx_the_secret_library_gail_reports_her_daughters_fate() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::questpc8_secret_library::run_library_journey()?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED qcx_the_secret_library_gail_reports_her_daughters_fate duration={:?}",
        started.elapsed()
    );
    Ok(())
}

pub(super) async fn assert_gail_reply(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    gail: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    first: u64,
) -> TestResult {
    use super::fixture::debug;
    use crate::foundation::{MessageType, decode_wire_envelope};
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
    use oteryn_protocol_oteryn::npc_service::{
        COMMAND_TYPE_NPC_TALK_INTENT, NpcIntentDisposition, NpcTalkIntent,
        STATE_DOMAIN_NPC_CONVERSATION, decode_npc_conversation, decode_npc_intent_result,
        encode_npc_talk_intent,
    };
    use oteryn_protocol_oteryn::{
        ClientCommandValue, CommandStatus, decode_command_result, decode_state_delta,
        encode_client_command,
    };
    use tokio::io::AsyncWriteExt;

    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("NPC command window")?;
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
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

        for (command_id, text) in [(first, "hi"), (first + 1, "daughter")] {
            let payload = encode_npc_talk_intent(&NpcTalkIntent {
                npc_actor: gail,
                text: text.into(),
            })
            .map_err(debug)?;
            let frame = encode_client_command(
                before.continuity.connection_generation,
                &ClientCommandValue {
                    command_id,
                    command_type: COMMAND_TYPE_NPC_TALK_INTENT,
                    payload: &payload,
                },
            )
            .map_err(debug)?;
            write_frame(&mut client, &frame).await?;
            let mut accepted = false;
            let mut reply = None;
            while !accepted || reply.is_none() {
                let frame = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                match envelope.message_type() {
                    MessageType::CommandResult => {
                        let result = decode_command_result(envelope.payload()).map_err(debug)?;
                        assert_eq!(result.command_id, command_id);
                        assert_eq!(result.status, CommandStatus::Accepted);
                        assert_eq!(
                            decode_npc_intent_result(result.payload).map_err(debug)?,
                            NpcIntentDisposition::Ok
                        );
                        accepted = true;
                    }
                    MessageType::StateDelta => {
                        let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
                        if delta.domain_id == STATE_DOMAIN_NPC_CONVERSATION {
                            reply = Some(decode_npc_conversation(delta.payload).map_err(debug)?);
                        }
                    }
                    _ => {}
                }
            }
            if text == "daughter" {
                // TibiaWiki Secret Library Quest/Spoiler, curid 85334, oldid 1194739.
                assert_eq!(
                    reply.ok_or("Gail's reply")?.lines,
                    ["I always feared that I lost her. And yet, all those years, I still had a gleam of hope. I'm devastated to learn about her fate - but at least I have certainty now. Thank you for telling me."]
                );
            }
        }
        client.shutdown().await?;
        Ok(())
    };
    let (ended, result): (_, TestResult) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serving, driving),
    )
    .await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = ended
    {
        *admitted = current;
    }
    result
}

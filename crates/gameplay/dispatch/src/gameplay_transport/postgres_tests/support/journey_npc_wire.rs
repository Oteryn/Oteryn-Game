//! Drive successive NPC commands on the same admitted transport.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, debug};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::{
    connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted},
    tcp_tls::{read_frame, write_frame},
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, decode_command_result, encode_client_command,
    npc_service::{
        COMMAND_TYPE_NPC_TALK_INTENT, NpcIntentDisposition, NpcTalkIntent,
        decode_npc_intent_result, encode_npc_talk_intent,
    },
    world_spatial_entities::EntityRef,
};
use tokio::io::{AsyncWriteExt, DuplexStream};

pub(super) struct Conversation {
    client: DuplexStream,
    generation: u64,
    next: u64,
    npc: EntityRef,
}

impl Conversation {
    pub(super) fn next(&self) -> u64 {
        self.next
    }

    pub(super) async fn talk(&mut self, text: &str) -> TestResult<Vec<u8>> {
        let payload = encode_npc_talk_intent(&NpcTalkIntent {
            npc_actor: self.npc,
            text: text.into(),
        })
        .map_err(debug)?;
        let frame = encode_client_command(
            self.generation,
            &ClientCommandValue {
                command_id: self.next,
                command_type: COMMAND_TYPE_NPC_TALK_INTENT,
                payload: &payload,
            },
        )
        .map_err(debug)?;
        write_frame(&mut self.client, &frame).await?;
        loop {
            let frame = read_frame(&mut self.client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            match envelope.message_type() {
                MessageType::LivenessProbe => super::qengine_rewards_fixtures::acknowledge_probe(
                    &mut self.client,
                    self.generation,
                    envelope.payload(),
                )
                .await
                .map_err(std::io::Error::other)?,
                MessageType::CommandResult => {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, self.next, "{text}");
                    self.next += 1;
                    return Ok(result.payload.to_vec());
                }
                _ => {}
            }
        }
    }

    pub(super) async fn expect(
        &mut self,
        text: &str,
        expected: NpcIntentDisposition,
    ) -> TestResult {
        let actual = decode_npc_intent_result(&self.talk(text).await?).map_err(debug)?;
        assert_eq!(actual, expected, "{text}");
        Ok(())
    }
}

pub(super) async fn exchange<T>(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    play: impl AsyncFnOnce(&mut Conversation) -> TestResult<T>,
) -> TestResult<T> {
    let (mut server, client) = tokio::io::duplex(1 << 20);
    let mut conversation = Conversation {
        client,
        generation: admitted.continuity.connection_generation,
        next: admitted.continuity.next_command_id,
        npc,
    };
    let drive = async move {
        let result = play(&mut conversation).await;
        conversation.client.shutdown().await?;
        tokio::io::copy(&mut conversation.client, &mut tokio::io::sink()).await?;
        result
    };
    let initial = *admitted;
    let serve = async move { serve_admitted(&mut server, initial, owner, IDLE_LIVENESS).await };
    let (ended, result) = tokio::join!(serve, drive);
    let ConnectionEnd::AdmittedThenDisconnected(current) = ended else {
        return Err(format!("NPC exchange closed before disconnect: {ended:?}").into());
    };
    *admitted = current;
    result
}

pub(super) async fn say(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    text: &str,
) -> TestResult<NpcIntentDisposition> {
    exchange(owner, admitted, npc, async |conversation| {
        conversation.expect("hi", NpcIntentDisposition::Ok).await?;
        decode_npc_intent_result(&conversation.talk(text).await?).map_err(debug)
    })
    .await
}

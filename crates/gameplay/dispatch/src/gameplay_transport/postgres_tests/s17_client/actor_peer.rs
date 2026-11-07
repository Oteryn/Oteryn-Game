use super::super::{TestResult, fixture::debug};
use crate::gameplay_transport::{
    connection::AdmittedSession,
    tcp_tls::{read_frame, write_frame},
};
use oteryn_dev_client::{client_runtime as oteryn_client_runtime, client_ui as oteryn_client_ui};
use oteryn_protocol_oteryn::{self as wire, cast::Message};
use tokio::io::{AsyncWriteExt, DuplexStream};
pub(super) struct Peer {
    stream: DuplexStream,
    generation: u64,
    next: u64,
    sequence: u64,
    chat_revision: u64,
    pub control: oteryn_client_ui::cast_control::Panel,
    pub chat: oteryn_client_ui::cast_chat::Panel,
    pub gameplay_lines: usize,
}
impl Peer {
    pub async fn join(mut stream: DuplexStream, admitted: AdmittedSession) -> TestResult<Self> {
        loop {
            let bytes = read_frame(&mut stream).await?;
            if wire::decode_wire_envelope(&bytes)
                .map_err(debug)?
                .message_type()
                == wire::MessageType::SnapshotCommit
            {
                break;
            }
        }
        Ok(Self {
            stream,
            generation: admitted.continuity.connection_generation,
            next: admitted.continuity.next_command_id,
            sequence: admitted.continuity.server_sequence,
            chat_revision: 1,
            control: oteryn_client_ui::cast_control::Panel {
                selected: true,
                consent_selected: true,
                ..Default::default()
            },
            chat: oteryn_client_ui::cast_chat::Panel {
                selected: true,
                ..Default::default()
            },
            gameplay_lines: 0,
        })
    }
    async fn read(&mut self) -> TestResult<Option<(wire::CommandStatus, Vec<u8>)>> {
        let bytes = read_frame(&mut self.stream).await?;
        let envelope = wire::decode_wire_envelope(&bytes).map_err(debug)?;
        assert_eq!(envelope.connection_generation(), self.generation);
        if envelope.message_type() == wire::MessageType::LivenessProbe {
            let probe = wire::decode_liveness_probe(envelope.payload()).map_err(debug)?;
            write_frame(
                &mut self.stream,
                &wire::encode_liveness_ack(self.generation, probe, self.sequence).map_err(debug)?,
            )
            .await?;
            return Ok(None);
        }
        if matches!(
            envelope.message_type(),
            wire::MessageType::CommandResult | wire::MessageType::StateDelta
        ) {
            assert_eq!(envelope.server_sequence(), self.sequence + 1);
            self.sequence += 1;
        }
        match envelope.message_type() {
            wire::MessageType::CommandResult => {
                let result = wire::decode_command_result(envelope.payload()).map_err(debug)?;
                Ok(Some((result.status, result.payload.to_vec())))
            }
            wire::MessageType::StateDelta => {
                let delta = wire::decode_state_delta(envelope.payload()).map_err(debug)?;
                if delta.domain_id == 103 {
                    assert_eq!(delta.base_revision, self.chat_revision);
                    assert_eq!(delta.new_revision, self.chat_revision + 1);
                    self.chat_revision = delta.new_revision;
                    match Message::decode(delta.payload).map_err(debug)? {
                        Message::CasterLine { .. } => {
                            assert_eq!(delta.delta_type, 1);
                            let line =
                                oteryn_client_runtime::decode_cast_caster_line(delta.payload)?;
                            oteryn_client_ui::cast_chat::protocol::append(&mut self.chat, line);
                        }
                        Message::CasterRoster { revision, subjects } => {
                            assert_eq!(delta.delta_type, wire::cast::DELTA_TYPE_CAST_ROSTER);
                            oteryn_client_ui::cast_chat::protocol::roster(
                                &mut self.chat,
                                oteryn_client_runtime::CastRoster { revision, subjects },
                            );
                        }
                        _ => return Err("unexpected caster frame".into()),
                    }
                } else if delta.domain_id == 12 {
                    self.gameplay_lines += 1;
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }
    pub async fn exchange(
        &mut self,
        kind: u32,
        payload: &[u8],
    ) -> TestResult<(wire::CommandStatus, Vec<u8>)> {
        let frame = wire::encode_client_command(
            self.generation,
            &wire::ClientCommandValue {
                command_id: self.next,
                command_type: kind,
                payload,
            },
        )
        .map_err(debug)?;
        write_frame(&mut self.stream, &frame).await?;
        self.next += 1;
        loop {
            if let Some(result) = self.read().await? {
                return Ok(result);
            }
        }
    }
    pub async fn cast(&mut self, kind: u32, message: Message) -> TestResult<Message> {
        let (status, bytes) = self
            .exchange(kind, &message.encode().map_err(debug)?)
            .await?;
        assert_eq!(status, wire::CommandStatus::Accepted, "cast command {kind}");
        let reply = Message::decode(&bytes).map_err(debug)?;
        if matches!(
            reply,
            Message::CasterState { .. } | Message::ConsentState { .. } | Message::End
        ) {
            self.control.apply(reply.clone())?;
        }
        Ok(reply)
    }
    pub async fn text(&mut self, text: &str) -> TestResult {
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            while !self.chat.lines.iter().any(|line| line.text.contains(text)) {
                self.read().await?;
            }
            Ok::<(), Box<dyn std::error::Error>>(())
        })
        .await??;
        assert!(oteryn_client_ui::cast_chat::panel_text(&self.chat).contains(text));
        Ok(())
    }
    pub async fn roster(&mut self, old: Option<[u8; 16]>) -> TestResult<[u8; 16]> {
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                if let Some(handle) = self
                    .chat
                    .subjects
                    .iter()
                    .copied()
                    .find(|id| Some(*id) != old)
                {
                    return Ok::<_, Box<dyn std::error::Error>>(handle);
                }
                self.read().await?;
            }
        })
        .await?
    }
    pub async fn close(mut self) -> TestResult {
        self.stream.shutdown().await?;
        Ok(())
    }
}

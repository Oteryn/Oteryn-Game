//! Session session transport.
use super::*;

impl<S: SessionStream> Session<S> {
    /// Begin recording validated sequenced frames for qualification.
    #[cfg(feature = "test-support")]
    pub fn begin_qualification_capture(&mut self) {
        self.qualification_frames = Some(Vec::new());
    }

    /// End recording and return the original sequenced payloads in receive order.
    #[cfg(feature = "test-support")]
    pub fn take_qualification_capture(&mut self) -> Vec<(u64, MessageType, Vec<u8>)> {
        self.qualification_frames.take().unwrap_or_default()
    }

    /// Reads the next non-probe frame, requires it to be `expected`, and requires its
    /// `server_sequence` to be exactly the last applied one plus one. Returns that sequence and
    /// the frame's payload.
    pub(super) async fn read_sequenced(
        &mut self,
        label: &'static str,
        expected: MessageType,
    ) -> Result<(u64, Vec<u8>), SessionError> {
        let (message_type, server_sequence, payload) = self.read_sequenced_push(label).await?;
        if message_type != expected {
            return Err(SessionError::UnexpectedMessage {
                expected,
                actual: message_type,
            });
        }
        Ok((server_sequence, payload))
    }

    /// `read_sequenced` for a frame that is either a `CommandResult` or a pushed `StateDelta`;
    /// any other type is a protocol error naming `CommandResult`.
    pub(super) async fn read_sequenced_push(
        &mut self,
        label: &'static str,
    ) -> Result<(MessageType, u64, Vec<u8>), SessionError> {
        let frame = self.read_post_admission(label).await?;
        let envelope = decode_wire_envelope(&frame)?;
        let message_type = envelope.message_type();
        if !matches!(
            message_type,
            MessageType::CommandResult | MessageType::StateDelta
        ) {
            return Err(SessionError::UnexpectedMessage {
                expected: MessageType::CommandResult,
                actual: message_type,
            });
        }
        let server_sequence = self.next_sequence(&envelope)?;
        Ok((message_type, server_sequence, envelope.payload().to_vec()))
    }

    /// Requires `envelope`'s `server_sequence` to be exactly the last applied one plus one and
    /// records it.
    pub(super) fn next_sequence(
        &mut self,
        envelope: &oteryn_protocol_oteryn::WireEnvelopeView<'_>,
    ) -> Result<u64, SessionError> {
        let expected_sequence = self
            .last_server_sequence
            .checked_add(1)
            .ok_or(FoundationProtocolError::ServerSequenceGap)?;
        if envelope.server_sequence() != expected_sequence {
            return Err(SessionError::ServerSequenceMismatch {
                expected: expected_sequence,
                actual: envelope.server_sequence(),
            });
        }
        #[cfg(feature = "test-support")]
        if let Some(frames) = &mut self.qualification_frames {
            if frames.len() == 64 {
                return Err(FoundationProtocolError::PayloadLimitExceeded.into());
            }
            frames.push((
                expected_sequence,
                envelope.message_type(),
                envelope.payload().to_vec(),
            ));
        }
        self.last_server_sequence = expected_sequence;
        Ok(expected_sequence)
    }
}

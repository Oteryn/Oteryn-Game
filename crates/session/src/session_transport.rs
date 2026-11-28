//! Session session transport.
use super::*;

impl<S: SessionStream> Session<S> {
    /// Waits for the server's clean EOF after an accepted logout without writing to the stream.
    pub async fn observe_logout_close(&mut self) -> Result<(), SessionError> {
        bounded(
            self.deadline,
            "logout transport close",
            expect_clean_eof(&mut self.stream),
        )
        .await
    }

    /// Reads the next non-probe frame, requires it to be `expected`, and requires its
    /// `server_sequence` to be exactly the last applied one plus one. Returns that sequence and
    /// the frame's payload.
    pub(super) async fn read_sequenced(
        &mut self,
        label: &'static str,
        expected: MessageType,
    ) -> Result<(u64, Vec<u8>), SessionError> {
        let (message_type, server_sequence, frame) = self.read_sequenced_push(label).await?;
        if message_type != expected {
            return Err(SessionError::UnexpectedMessage {
                expected,
                actual: message_type,
            });
        }
        Ok((
            server_sequence,
            decode_wire_envelope(&frame)?.payload().to_vec(),
        ))
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
        Ok((message_type, server_sequence, frame))
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
        self.last_server_sequence = expected_sequence;
        Ok(expected_sequence)
    }
}

async fn expect_clean_eof(stream: &mut impl SessionStream) -> Result<(), SessionError> {
    use tokio::io::AsyncReadExt;
    let mut byte = [0];
    if stream.read(&mut byte).await? != 0 {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "server sent data after accepted logout",
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod logout_close_tests {
    use super::*;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn logout_close_rejects_open_stream_and_trailing_data() -> Result<(), SessionError> {
        let (mut client, mut server) = tokio::io::duplex(8);
        assert!(
            bounded(
                Duration::from_millis(10),
                "close",
                expect_clean_eof(&mut client)
            )
            .await
            .is_err()
        );
        server.write_all(&[1]).await?;
        assert!(expect_clean_eof(&mut client).await.is_err());
        server.shutdown().await?;
        expect_clean_eof(&mut client).await
    }
}

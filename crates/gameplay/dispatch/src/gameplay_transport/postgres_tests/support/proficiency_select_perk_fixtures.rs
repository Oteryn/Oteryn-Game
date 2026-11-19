use tokio::io::AsyncWriteExt;
use super::fixture::debug;

use super::{TestResult, item_look_fixtures as item_look};

use crate::gameplay_transport::connection::AdmittedSession;

use oteryn_protocol_oteryn::{
    self as protocol,
};

/// Serve one admitted connection, drive it to its snapshot, run `exercise`, then close it.
pub(super) async fn connection<F>(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    exercise: F,
) -> TestResult<bool>
where
    F: AsyncFnOnce(&mut tokio::io::DuplexStream) -> TestResult,
{
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serve = crate::gameplay_transport::connection::serve_admitted(
        &mut server,
        admitted,
        owner,
        crate::gameplay_transport::connection::IDLE_LIVENESS,
    );
    let drive = async move {
        loop {
            let frame = item_look::read_frame(&mut client).await?;
            let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == protocol::MessageType::SnapshotCommit {
                break;
            }
        }
        exercise(&mut client).await?;
        client.shutdown().await?;
        TestResult::Ok(())
    };
    let mut serve = std::pin::pin!(serve);
    let mut drive = std::pin::pin!(drive);
    let mut driven = None;
    std::future::poll_fn(|cx| {
        use std::task::Poll;
        if driven.is_none()
            && let Poll::Ready(value) = drive.as_mut().poll(cx)
        {
            if let Err(error) = value {
                return Poll::Ready(Err(error));
            }
            driven = Some(());
        }
        if let Poll::Ready(end) = serve.as_mut().poll(cx) {
            return Poll::Ready(if driven.is_some() {
                Ok(true)
            } else {
                Err(format!("connection ended before the exercise: {end:?}").into())
            });
        }
        Poll::Pending
    })
    .await
}

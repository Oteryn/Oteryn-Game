//! Account-only observer transport. No native bootstrap, actor, lease or gameplay
//! command decoder is called on this branch. Writes are immediate under the cast
//! worker's revocation barrier; a partial/pending write terminates the observer.
use super::{
    SecureIdentifiers,
    cast::CastNode,
    tcp_tls::{FrameReader, write_frame},
};
use oteryn_gameplay_social::cast::{
    Content, Error,
    service::{Operation, Reply},
};
use oteryn_protocol_oteryn::cast::{Envelope, Message};
use std::{
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
    time::Duration,
};
use tokio::io::{AsyncRead, AsyncWrite};

pub(super) fn ingress(frame: &[u8]) -> bool {
    frame.starts_with(&[8, 16])
}

pub(super) async fn serve<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
    stream: S,
    frame: &[u8],
    node: CastNode,
    shutdown: &super::CancellationToken,
    admission_unit: tokio::sync::SemaphorePermit<'_>,
) -> Result<(), Error> {
    let first = Envelope::decode(frame, true).map_err(|_| Error::Invalid)?;
    let Message::Join(ticket) = first.message else {
        return Err(Error::Invalid);
    };
    let proof = node
        .access
        .consume(&ticket, &|| node.service.clock())
        .await?;
    let reservation = proof.reservation_id();
    let chat_selected = proof.chat_selected();
    if proof.chat_selected()
        && !super::capabilities::PRODUCTION_OFFERED_CAPABILITIES
            .iter()
            .any(|c| c.id == oteryn_protocol_oteryn::cast::CAPABILITY_CAST_CHAT_V1)
    {
        let _ = node.access.release(reservation).await;
        return Err(Error::Unavailable);
    }
    let id = match node.service.apply(Operation::Join(proof)).await {
        Ok(Reply::Observer(id)) => id,
        _ => {
            let _ = node.access.release(reservation).await;
            return Err(Error::Unavailable);
        }
    };
    // The listener still owns the connection slot; only admission work needs
    // this unit, not the lifetime of the admitted observer.
    drop(admission_unit);
    let result = serve_joined(stream, &node, id, reservation, shutdown, chat_selected).await;
    let _ = node.service.apply(Operation::Leave(id)).await;
    let _ = node.access.release(reservation).await;
    result
}

async fn serve_joined<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
    mut stream: S,
    node: &CastNode,
    id: oteryn_gameplay_social::cast::ObserverId,
    reservation: [u8; 16],
    shutdown: &super::CancellationToken,
    chat_selected: bool,
) -> Result<(), Error> {
    let generation =
        u64::from_be_bytes(SecureIdentifiers::random::<8>().ok_or(Error::Unavailable)?).max(1);
    let Reply::State(state) = node.service.apply(Operation::ObserverState(id)).await? else {
        return Err(Error::Unavailable);
    };
    let clear = Envelope {
        generation,
        message: Message::Ready {
            epoch: state.epoch,
            chat: chat_selected && state.chat_enabled,
            delay: state.delay_seconds,
        },
    }
    .encode()
    .map_err(|_| Error::Invalid)?;
    tokio::time::timeout(Duration::from_secs(1), write_frame(&mut stream, &clear))
        .await
        .map_err(|_| Error::Unavailable)?
        .map_err(|_| Error::Unavailable)?;
    let (mut reader, writer) = tokio::io::split(stream);
    let writer = Arc::new(Mutex::new(writer));
    let mut frames = FrameReader::bounded(16432);
    let mut renew_at = tokio::time::Instant::now() + Duration::from_secs(2);
    let mut sequence = 0u64;
    let mut last_input = tokio::time::Instant::now();
    loop {
        if shutdown.is_cancelled() || last_input.elapsed() >= Duration::from_secs(30) {
            break;
        }
        match tokio::time::timeout(Duration::from_millis(100), frames.next(&mut reader)).await {
            Ok(Ok(frame)) => {
                let envelope = Envelope::decode(&frame, true).map_err(|_| Error::Invalid)?;
                if envelope.generation != generation {
                    return Err(Error::Invalid);
                }
                let op = match envelope.message {
                    Message::Leave => break,
                    Message::Keepalive => Operation::Keepalive(id),
                    Message::Chat(text) => Operation::Chat(id, text),
                    _ => return Err(Error::Invalid),
                };
                let chat = matches!(&op, Operation::Chat(..));
                observer_input_result(node.service.apply(op).await, chat)?;
                last_input = tokio::time::Instant::now();
            }
            Ok(Err(_)) => break,
            Err(_) => {}
        }
        if tokio::time::Instant::now() >= renew_at {
            let proof = node
                .access
                .renew(reservation, &|| node.service.clock())
                .await?;
            node.service.apply(Operation::Renew(id, proof)).await?;
            renew_at = tokio::time::Instant::now() + Duration::from_secs(2);
        }
        let output = writer.clone();
        // Sequence ownership travels with the final synchronous writer. One
        // callback is awaited before installing the next callback.
        let next = Arc::new(Mutex::new(sequence));
        let next_write = next.clone();
        node.service
            .apply(Operation::Flush(
                id,
                Box::new(move |state, content| {
                    let Ok(mut sequence) = next_write.lock() else {
                        return false;
                    };
                    let Some(value) = sequence.checked_add(1) else {
                        return false;
                    };
                    *sequence = value;
                    let message = match content {
                        Content::Metadata {
                            viewers,
                            delay_seconds,
                        } => Message::State {
                            revision: state.revision,
                            epoch: state.epoch,
                            delay: *delay_seconds,
                            count: *viewers,
                        },
                        Content::Clear => Message::Ready {
                            epoch: state.epoch,
                            chat: chat_selected && state.chat_enabled,
                            delay: state.delay_seconds,
                        },
                        Content::Slate => Message::Slate {
                            epoch: state.epoch,
                            sequence: value,
                        },
                        Content::Chat(line) => Message::ChatLine {
                            epoch: state.epoch,
                            sequence: value,
                            handle: line.handle,
                            text: format!("{}: {}", line.name, line.text),
                        },
                        Content::Media(media) => Message::Media {
                            epoch: state.epoch,
                            sequence: value,
                            media: (**media).clone(),
                        },
                        Content::Scene(scene) => Message::Scene {
                            epoch: state.epoch,
                            sequence: value,
                            scene: (**scene).clone(),
                        },
                    };
                    let Ok(body) = (Envelope {
                        generation,
                        message,
                    })
                    .encode() else {
                        return false;
                    };
                    let mut bytes = Vec::with_capacity(body.len() + 4);
                    bytes.extend_from_slice(&(body.len() as u32).to_be_bytes());
                    bytes.extend_from_slice(&body);
                    let Ok(mut writer) = output.lock() else {
                        return false;
                    };
                    write_immediate_frame(&mut *writer, &bytes)
                }),
            ))
            .await?;
        sequence = *next.lock().map_err(|_| Error::Unavailable)?;
    }
    Ok(())
}

fn write_immediate_frame<S: AsyncWrite + Unpin>(writer: &mut S, bytes: &[u8]) -> bool {
    // Tokio may yield for its task budget even when the socket is writable.
    // Only this one synchronous attempt is unconstrained: real backpressure,
    // a partial write or a pending flush still terminates the observer while
    // the final writer remains inside the revocation barrier.
    let mut attempt = std::pin::pin!(tokio::task::unconstrained(std::future::poll_fn(|cx| {
        Poll::Ready(
            matches!(Pin::new(&mut *writer).poll_write(cx, bytes), Poll::Ready(Ok(n)) if n == bytes.len())
                && matches!(Pin::new(&mut *writer).poll_flush(cx), Poll::Ready(Ok(()))),
        )
    })));
    let mut context = Context::from_waker(Waker::noop());
    matches!(
        std::future::Future::poll(attempt.as_mut(), &mut context),
        Poll::Ready(true)
    )
}

fn observer_input_result(result: Result<Reply, Error>, chat: bool) -> Result<(), Error> {
    match result {
        Err(Error::Muted | Error::Full) if chat => Ok(()),
        result => result.map(|_| ()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn exhaust_cooperative_budget() {
        for _ in 0..128 {
            if !tokio::task::coop::has_budget_remaining() {
                return;
            }
            tokio::task::coop::consume_budget().await;
        }
        assert!(!tokio::task::coop::has_budget_remaining());
    }

    #[test]
    fn writable_observer_survives_exhausted_cooperative_budget()
    -> Result<(), Box<dyn std::error::Error>> {
        use tokio::io::AsyncReadExt;
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                tokio::spawn(async {
                    let (mut client, server) = tokio::io::duplex(65536);
                    let (_reader, mut writer) = tokio::io::split(server);
                    let frame = b"\0\0\0\x04cast";
                    exhaust_cooperative_budget().await;
                    assert!(write_immediate_frame(&mut writer, frame));
                    assert!(
                        !tokio::task::coop::has_budget_remaining(),
                        "the bounded write restores its caller's exhausted budget"
                    );
                    let mut received = [0; 8];
                    client.read_exact(&mut received).await?;
                    assert_eq!(&received, frame);
                    Ok::<(), std::io::Error>(())
                })
                .await??;
                Ok::<(), Box<dyn std::error::Error>>(())
            })
    }

    #[test]
    fn backpressured_partial_and_closed_observers_still_fail_closed()
    -> Result<(), Box<dyn std::error::Error>> {
        use tokio::io::AsyncReadExt;
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(async {
                tokio::spawn(async {
                    for (initial, next, expected) in [
                        (b"full".as_slice(), b"x".as_slice(), b"full"),
                        (b"ab".as_slice(), b"xyz".as_slice(), b"abxy"),
                    ] {
                        let (mut client, server) = tokio::io::duplex(4);
                        let (_reader, mut writer) = tokio::io::split(server);
                        assert!(write_immediate_frame(&mut writer, initial));
                        exhaust_cooperative_budget().await;
                        assert!(!write_immediate_frame(&mut writer, next));
                        assert!(!tokio::task::coop::has_budget_remaining());
                        let mut received = [0; 4];
                        client.read_exact(&mut received).await?;
                        assert_eq!(&received, expected);
                    }
                    let (client, server) = tokio::io::duplex(4);
                    let (_reader, mut writer) = tokio::io::split(server);
                    drop(client);
                    exhaust_cooperative_budget().await;
                    assert!(!write_immediate_frame(&mut writer, b"x"));
                    Ok::<(), std::io::Error>(())
                })
                .await??;
                Ok::<(), Box<dyn std::error::Error>>(())
            })
    }

    #[test]
    fn pending_flush_is_not_retried() {
        #[derive(Default)]
        struct PendingFlush {
            writes: usize,
            flushes: usize,
        }
        impl AsyncWrite for PendingFlush {
            fn poll_write(
                mut self: Pin<&mut Self>,
                _: &mut Context<'_>,
                bytes: &[u8],
            ) -> Poll<std::io::Result<usize>> {
                self.writes += 1;
                Poll::Ready(Ok(bytes.len()))
            }
            fn poll_flush(
                mut self: Pin<&mut Self>,
                _: &mut Context<'_>,
            ) -> Poll<std::io::Result<()>> {
                self.flushes += 1;
                Poll::Pending
            }
            fn poll_shutdown(
                self: Pin<&mut Self>,
                _: &mut Context<'_>,
            ) -> Poll<std::io::Result<()>> {
                Poll::Ready(Ok(()))
            }
        }
        let mut writer = PendingFlush::default();
        assert!(!write_immediate_frame(&mut writer, b"frame"));
        assert_eq!((writer.writes, writer.flushes), (1, 1));
    }

    #[test]
    fn muted_or_throttled_chat_preserves_the_observer_session() {
        assert!(observer_input_result(Err(Error::Muted), true).is_ok());
        assert!(observer_input_result(Err(Error::Full), true).is_ok());
        assert!(observer_input_result(Err(Error::Unavailable), true).is_err());
        assert!(observer_input_result(Err(Error::Full), false).is_err());
    }
    #[test]
    fn observer_ingress_has_a_separate_family_and_gameplay_is_not_a_command() {
        assert!(ingress(&[8, 16, 16, 0]));
        assert!(!ingress(&[8, 1]));
        assert!(Envelope::decode(&[8, 1], true).is_err());
        assert!(
            Envelope {
                generation: 1,
                message: Message::Start {
                    revision: 0,
                    access_revision: 1,
                    delay: 0,
                    cap: 50,
                    password: false
                }
            }
            .encode()
            .is_err()
        );
    }
    #[test]
    fn observer_frame_limit_is_checked_before_allocating_the_body()
    -> Result<(), Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(async {
            let prefix = 16433u32.to_be_bytes();
            let mut input = prefix.as_slice();
            let result = FrameReader::bounded(16432).next(&mut input).await;
            assert!(matches!(result,Err(ref e) if e.kind()==std::io::ErrorKind::InvalidData));
        });
        Ok(())
    }
}

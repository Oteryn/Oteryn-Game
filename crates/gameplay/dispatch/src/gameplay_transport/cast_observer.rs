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
        node.service.apply(Operation::Flush(id,Box::new(move |state,content| {
                let Ok(mut sequence)=next_write.lock() else {return false};
                let Some(value)=sequence.checked_add(1) else {return false};
                *sequence=value;
                let message=match content {
                    Content::Metadata{viewers,delay_seconds}=>Message::State{revision:state.revision,epoch:state.epoch,delay:*delay_seconds,count:*viewers},
                    Content::Clear=>Message::Ready{epoch:state.epoch,chat:chat_selected&&state.chat_enabled,delay:state.delay_seconds},
                    Content::Slate=>Message::Slate{epoch:state.epoch,sequence:value},
                    Content::Chat(line)=>Message::ChatLine{epoch:state.epoch,sequence:value,handle:line.handle,text:format!("{}: {}",line.name,line.text)},
                    Content::Media(media)=>Message::Media{epoch:state.epoch,sequence:value,media:(**media).clone()},
                    Content::Scene(scene)=>Message::Scene{epoch:state.epoch,sequence:value,scene:(**scene).clone()},
                };
                let Ok(body)=(Envelope{generation,message}).encode() else {return false};
                let mut bytes=Vec::with_capacity(body.len()+4);
                bytes.extend_from_slice(&(body.len() as u32).to_be_bytes());bytes.extend_from_slice(&body);
                let Ok(mut writer)=output.lock() else {return false};
                let mut context=Context::from_waker(Waker::noop());
                matches!(Pin::new(&mut *writer).poll_write(&mut context,&bytes),Poll::Ready(Ok(n)) if n==bytes.len())
                    && matches!(Pin::new(&mut *writer).poll_flush(&mut context),Poll::Ready(Ok(())))
            }))).await?;
        sequence = *next.lock().map_err(|_| Error::Unavailable)?;
    }
    Ok(())
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

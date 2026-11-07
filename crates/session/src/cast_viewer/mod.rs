//! Account-only S17 observer. Canary livestream.cpp:541: an explicit input allowlist.
//! No CharacterId, player Session, actor, movement or item APIs exist here.
use crate::cast::{Envelope, Message};
use crate::{SessionError, SessionStream};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
fn invalid() -> SessionError {
    SessionError::SessionUnusable
}
/// Cancellation-safe, observer-specific bounded framing. Partial reads stay owned here.
#[derive(Default)]
struct Frames {
    prefix: [u8; 4],
    prefix_len: usize,
    body: Vec<u8>,
    used: usize,
}
impl Frames {
    async fn read<S: SessionStream>(&mut self, stream: &mut S) -> Result<Vec<u8>, SessionError> {
        while self.prefix_len < 4 {
            let n = stream.read(&mut self.prefix[self.prefix_len..]).await?;
            if n == 0 {
                return Err(invalid());
            }
            self.prefix_len += n;
        }
        if self.body.is_empty() {
            let size = u32::from_be_bytes(self.prefix) as usize;
            if size == 0 || size > 16432 {
                return Err(invalid());
            }
            self.body.resize(size, 0);
        }
        while self.used < self.body.len() {
            let n = stream.read(&mut self.body[self.used..]).await?;
            if n == 0 {
                return Err(invalid());
            }
            self.used += n;
        }
        self.prefix_len = 0;
        self.used = 0;
        Ok(std::mem::take(&mut self.body))
    }
}
pub struct CastViewerSession<S> {
    stream: S,
    frames: Frames,
    generation: u64,
    epoch: u64,
    sequence: u64,
    ended: bool,
    chat_allowed: bool,
    delay: u16,
    deadline: Duration,
    next_deadline: tokio::time::Instant,
}
impl<S: SessionStream> CastViewerSession<S> {
    pub async fn join(
        mut stream: S,
        ticket: &[u8],
        deadline: Duration,
    ) -> Result<Self, SessionError> {
        let bytes = Envelope {
            generation: 0,
            message: Message::Join(ticket.to_vec()),
        }
        .encode()
        .map_err(|_| invalid())?;
        tokio::time::timeout(deadline, crate::write_frame(&mut stream, &bytes))
            .await
            .map_err(|_| SessionError::Timeout("cast join"))??;
        let mut frames = Frames::default();
        let bytes = tokio::time::timeout(deadline, frames.read(&mut stream))
            .await
            .map_err(|_| SessionError::Timeout("cast accept"))??;
        let first = Envelope::decode(&bytes, false).map_err(|_| invalid())?;
        let Message::Ready { epoch, chat, delay } = first.message else {
            return Err(invalid());
        };
        if first.generation == 0 || epoch == 0 {
            return Err(invalid());
        }
        Ok(Self {
            stream,
            frames,
            generation: first.generation,
            epoch,
            sequence: 0,
            ended: false,
            chat_allowed: chat,
            delay,
            deadline,
            next_deadline: tokio::time::Instant::now() + Duration::from_secs(310),
        })
    }
    /// The initial clear is applied locally before any content is accepted.
    pub fn chat_allowed(&self) -> bool {
        self.chat_allowed
    }
    pub fn delay(&self) -> u16 {
        self.delay
    }
    pub fn epoch(&self) -> u64 {
        self.epoch
    }
    pub async fn next(&mut self) -> Result<Message, SessionError> {
        if self.ended {
            return Err(invalid());
        }
        let result = self.receive().await;
        if result.is_err() {
            self.ended = true;
        }
        result
    }
    async fn receive(&mut self) -> Result<Message, SessionError> {
        let bytes = tokio::time::timeout_at(self.next_deadline, self.frames.read(&mut self.stream))
            .await
            .map_err(|_| SessionError::Timeout("cast frame"))??;
        self.next_deadline = tokio::time::Instant::now() + Duration::from_secs(310);
        let envelope = Envelope::decode(&bytes, false).map_err(|_| invalid())?;
        if envelope.generation != self.generation {
            return Err(invalid());
        }
        match &envelope.message {
            Message::Ready { epoch, chat, delay } if *epoch >= self.epoch => {
                if *epoch > self.epoch {
                    self.sequence = 0;
                }
                self.epoch = *epoch;
                self.chat_allowed = *chat;
                self.delay = *delay;
            }
            Message::Clear { epoch } if *epoch >= self.epoch => {
                if *epoch > self.epoch {
                    self.sequence = 0;
                }
                self.epoch = *epoch;
            }
            Message::State { epoch, .. } if *epoch == self.epoch => {}
            Message::Media {
                epoch, sequence, ..
            }
            | Message::Scene {
                epoch, sequence, ..
            }
            | Message::Slate { epoch, sequence }
            | Message::ChatLine {
                epoch, sequence, ..
            } if *epoch == self.epoch && *sequence > self.sequence => {
                self.sequence = *sequence;
            }
            Message::End => self.ended = true,
            _ => return Err(invalid()),
        }
        Ok(envelope.message)
    }
    async fn send(&mut self, message: Message) -> Result<(), SessionError> {
        if self.ended {
            return Err(invalid());
        }
        let bytes = Envelope {
            generation: self.generation,
            message,
        }
        .encode()
        .map_err(|_| invalid())?;
        let result =
            match tokio::time::timeout(self.deadline, crate::write_frame(&mut self.stream, &bytes))
                .await
            {
                Ok(result) => result,
                Err(_) => Err(SessionError::Timeout("cast input")),
            };
        if result.is_err() {
            self.ended = true;
        }
        result
    }
    pub async fn keepalive(&mut self) -> Result<(), SessionError> {
        self.send(Message::Keepalive).await
    }
    pub async fn chat(&mut self, text: &str) -> Result<(), SessionError> {
        if !self.chat_allowed {
            return Err(SessionError::CapabilityNotSelected { capability: 291 });
        }
        self.send(Message::Chat(text.to_owned())).await
    }
    pub async fn leave(mut self) -> Result<(), SessionError> {
        let result = self.send(Message::Leave).await;
        let _ = self.stream.shutdown().await;
        result
    }
}

#[cfg(test)]
mod tests;

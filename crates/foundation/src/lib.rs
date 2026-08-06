//! Bounded process-local primitives for the Oteryn client workspace.

mod time;

pub use time::{Deadline, ManualClock, Moment, MonotonicClock, SystemClock, TimeError};

use std::fmt::{self, Display, Formatter};
use std::future::{Future, poll_fn};
use std::pin::Pin;
use std::task::Poll;
use tokio::sync::watch;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessGeneration(u64);

impl ProcessGeneration {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl Display for ProcessGeneration {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientTaskGeneration(u64);

impl ClientTaskGeneration {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl Display for ClientTaskGeneration {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClientSessionEpoch(u64);

impl ClientSessionEpoch {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl Display for ClientSessionEpoch {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.0, formatter)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundError {
    Empty,
    TooLong { max_bytes: usize },
    ControlCharacter,
}

impl Display for BoundError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("value must not be empty"),
            Self::TooLong { max_bytes } => write!(formatter, "value exceeds {max_bytes} bytes"),
            Self::ControlCharacter => formatter.write_str("value contains a control character"),
        }
    }
}

impl std::error::Error for BoundError {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BoundedText(String);

impl BoundedText {
    pub fn new(value: impl Into<String>, max_bytes: usize) -> Result<Self, BoundError> {
        let value = value.into();
        if value.is_empty() {
            return Err(BoundError::Empty);
        }
        if value.len() > max_bytes {
            return Err(BoundError::TooLong { max_bytes });
        }
        if value.chars().any(char::is_control) {
            return Err(BoundError::ControlCharacter);
        }
        Ok(Self(value))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for BoundedText {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone)]
pub struct CancellationToken {
    sender: watch::Sender<bool>,
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}

impl CancellationToken {
    #[must_use]
    pub fn new() -> Self {
        let (sender, _receiver) = watch::channel(false);
        Self { sender }
    }

    pub fn cancel(&self) {
        self.sender.send_replace(true);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        *self.sender.borrow()
    }

    pub async fn cancelled(&self) {
        let mut receiver = self.sender.subscribe();
        if *receiver.borrow() {
            return;
        }
        loop {
            if receiver.changed().await.is_err() || *receiver.borrow() {
                return;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cancellable<T> {
    Completed(T),
    Cancelled,
}

pub async fn cancellable<F>(token: CancellationToken, future: F) -> Cancellable<F::Output>
where
    F: Future,
{
    let mut work = Box::pin(future);
    let mut cancelled = Box::pin(async move { token.cancelled().await });
    poll_fn(move |context| {
        if Pin::as_mut(&mut cancelled).poll(context).is_ready() {
            return Poll::Ready(Cancellable::Cancelled);
        }
        Pin::as_mut(&mut work)
            .poll(context)
            .map(Cancellable::Completed)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounded_text_rejects_controls_and_oversize() {
        assert!(matches!(BoundedText::new("", 4), Err(BoundError::Empty)));
        assert!(matches!(
            BoundedText::new("abcde", 4),
            Err(BoundError::TooLong { max_bytes: 4 })
        ));
        assert!(matches!(
            BoundedText::new("a\nb", 8),
            Err(BoundError::ControlCharacter)
        ));
    }

    #[test]
    fn cancellation_is_observable() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        assert!(token.is_cancelled());
    }
}

use std::error::Error;
use std::fmt::{self, Display, Formatter};

/// `CHAT0-RL-01`: at most 255 Unicode scalar values of text.
pub(crate) const TEXT_MAX_CHARS: usize = 255;
/// `CHAT0-RL-01`: at most 1,020 bytes of UTF-8 text.
pub(crate) const TEXT_MAX_BYTES: usize = 1_020;

/// Why a chat text is refused (CHAT-0 §4). Every refusal answers `REJECTED`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextError {
    Empty,
    TooLong,
    ControlCharacter,
}

impl Display for TextError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Empty => "chat text is empty",
            Self::TooLong => "chat text is over 255 characters or 1,020 bytes",
            Self::ControlCharacter => "chat text contains a control character",
        })
    }
}

impl Error for TextError {}

/// A chat line's text: trimmed, 1 to 255 scalar values, at most 1,020 bytes, no control
/// characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChatText(String);

impl ChatText {
    /// Checks the received text. The byte bound holds for the text as received, so padding cannot
    /// carry an oversized field; the character bound holds after trimming.
    pub(crate) fn parse(received: &str) -> Result<Self, TextError> {
        if received.len() > TEXT_MAX_BYTES {
            return Err(TextError::TooLong);
        }
        if received.chars().any(char::is_control) {
            return Err(TextError::ControlCharacter);
        }
        let trimmed = received.trim();
        if trimmed.is_empty() {
            return Err(TextError::Empty);
        }
        if trimmed.chars().count() > TEXT_MAX_CHARS {
            return Err(TextError::TooLong);
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// The yelled form: each character upper-cased where that keeps it one character, so the
    /// bounds still hold (Canary upper-cases a yell, `game.cpp:7517-7534`).
    pub(crate) fn yelled(&self) -> Self {
        Self(
            self.0
                .chars()
                .map(|c| {
                    let mut upper = c.to_uppercase();
                    match (upper.next(), upper.next()) {
                        (Some(single), None) => single,
                        _ => c,
                    }
                })
                .collect(),
        )
    }
}

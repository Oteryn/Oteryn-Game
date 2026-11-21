//! ITEM_INSCRIPTION_READ_V1: the current inscription of one carried item, so the writing window
//! opens with it (Tibia Global sends the item's text and maximum length with the window).
use serde::{Deserialize, Serialize};

pub const CAPABILITY_ITEM_INSCRIPTION_READ_V1: u32 = 511;
pub const COMMAND_TYPE_ITEM_INSCRIPTION_READ: u32 = 6246;
/// The accepted item text profile bound (`MailTextPolicy`: 1..=4000 characters).
pub const MAX_TEXT_LENGTH: u32 = 4_000;
pub const MAX_REQUEST_BYTES: usize = 64;
/// 4000 characters of up to 4 UTF-8 bytes, newlines escaped, plus the closed envelope.
pub const MAX_REPLY_BYTES: usize = 32_768;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    /// An ITEM_VIEW handle of the session's own carried item.
    pub item_handle: u64,
}

impl Request {
    pub fn encode(&self) -> Option<Vec<u8>> {
        (self.item_handle != 0)
            .then(|| serde_json::to_vec(self).ok())
            .flatten()
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_REQUEST_BYTES {
            return None;
        }
        let request: Self = serde_json::from_slice(bytes).ok()?;
        (request.item_handle != 0).then_some(request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reply {
    /// The stored text; empty when the item carries none.
    pub text: String,
    /// The item's maximum text length in characters.
    pub max_length: u32,
    /// False for a readable item that only shows its text (no rewrite is accepted).
    pub writable: bool,
}

impl Reply {
    pub fn valid(&self) -> bool {
        (1..=MAX_TEXT_LENGTH).contains(&self.max_length)
            && self.text.chars().count() <= self.max_length as usize
            && !self
                .text
                .chars()
                .any(|character| character != '\n' && character.is_control())
    }
    pub fn encode(&self) -> Option<Vec<u8>> {
        if !self.valid() {
            return None;
        }
        let bytes = serde_json::to_vec(self).ok()?;
        (bytes.len() <= MAX_REPLY_BYTES).then_some(bytes)
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() > MAX_REPLY_BYTES {
            return None;
        }
        let reply: Self = serde_json::from_slice(bytes).ok()?;
        reply.valid().then_some(reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_inscription_codec_bounds_handle_text_length_and_fields() -> Result<(), &'static str> {
        assert!(Request { item_handle: 0 }.encode().is_none());
        assert!(Request::decode(br#"{"item_handle":0}"#).is_none());
        assert!(Request::decode(br#"{"item_handle":7,"extra":1}"#).is_none());
        let request = Request { item_handle: 7 };
        assert_eq!(
            Request::decode(&request.encode().ok_or("request")?),
            Some(request)
        );
        let reply = Reply {
            text: "Line one\nżółw 🐢".into(),
            max_length: 15,
            writable: true,
        };
        let bytes = reply.encode().ok_or("reply")?;
        assert_eq!(Reply::decode(&bytes), Some(reply.clone()));
        for end in 0..bytes.len() {
            assert!(Reply::decode(&bytes[..end]).is_none());
        }
        for invalid in [
            Reply {
                max_length: 14,
                ..reply.clone()
            },
            Reply {
                max_length: 0,
                text: String::new(),
                ..reply.clone()
            },
            Reply {
                max_length: MAX_TEXT_LENGTH + 1,
                ..reply.clone()
            },
            Reply {
                text: "a\0b".into(),
                ..reply.clone()
            },
        ] {
            assert!(invalid.encode().is_none(), "{invalid:?}");
        }
        assert!(
            Reply::decode(br#"{"text":"","max_length":1,"writable":true,"extra":1}"#).is_none()
        );
        let longest = Reply {
            text: "🐢".repeat(MAX_TEXT_LENGTH as usize),
            max_length: MAX_TEXT_LENGTH,
            writable: false,
        };
        assert!(
            longest
                .encode()
                .is_some_and(|bytes| bytes.len() <= MAX_REPLY_BYTES)
        );
        Ok(())
    }
}

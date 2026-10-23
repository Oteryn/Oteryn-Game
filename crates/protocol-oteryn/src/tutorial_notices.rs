//! TUTORIAL-1 transient notices. Snapshot is empty state; a delta carries one notice.
pub use crate::charm_wire::CyclopediaWireError as TutorialNoticeError;
use crate::charm_wire::{
    WireResult, push_message_field, push_varint_field, read_bytes, read_varint,
};

pub const CAPABILITY_TUTORIAL_NOTICES_V1: u32 = 31;
pub const STATE_DOMAIN_TUTORIAL_NOTICES: u32 = 23;
pub const TUTORIAL_NOTICES_SNAPSHOT: &[u8] = &[0x08, 1];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TutorialNotice {
    Hint(u8),
    StatusText(String),
}

pub fn encode_tutorial_notice(notice: &TutorialNotice) -> WireResult<Vec<u8>> {
    let mut bytes = Vec::new();
    match notice {
        TutorialNotice::Hint(id) if *id > 0 => push_varint_field(&mut bytes, 1, u64::from(*id)),
        TutorialNotice::StatusText(text) if !text.is_empty() && text.len() <= 255 => {
            push_message_field(&mut bytes, 2, text.as_bytes());
        }
        _ => return Err(TutorialNoticeError::Malformed),
    }
    Ok(bytes)
}

pub fn decode_tutorial_notice(bytes: &[u8]) -> WireResult<TutorialNotice> {
    if bytes.len() > 260 {
        return Err(TutorialNoticeError::LimitExceeded);
    }
    let mut cursor = 0;
    let notice = match read_varint(bytes, &mut cursor)? {
        0x08 => {
            let id = u8::try_from(read_varint(bytes, &mut cursor)?)
                .map_err(|_| TutorialNoticeError::Malformed)?;
            if id == 0 {
                return Err(TutorialNoticeError::Malformed);
            }
            TutorialNotice::Hint(id)
        }
        0x12 => {
            let text = read_bytes(bytes, &mut cursor)?;
            if text.is_empty() || text.len() > 255 {
                return Err(TutorialNoticeError::Malformed);
            }
            TutorialNotice::StatusText(
                std::str::from_utf8(text)
                    .map_err(|_| TutorialNoticeError::Malformed)?
                    .to_owned(),
            )
        }
        _ => return Err(TutorialNoticeError::Malformed),
    };
    if cursor != bytes.len() {
        return Err(TutorialNoticeError::Malformed);
    }
    Ok(notice)
}

#[cfg(test)]
#[path = "tutorial_notices_tests.rs"]
mod tests;

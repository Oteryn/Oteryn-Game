//! MAIL_V1 wire values. Posting and text rewriting share the normal command envelope.

pub use crate::charm_wire::CyclopediaWireError as MailWireError;

pub const CAPABILITY_MAIL_V1: u16 = 115;
pub const COMMAND_TYPE_MAIL: u16 = 124;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MailIntent {
    Post {
        item_handle: u64,
        mailbox_handle: u64,
    },
    WriteText {
        item_handle: u64,
        text: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MailOutcome {
    Posted = 1,
    TextWritten = 2,
    NoAddress = 3,
    UnknownRecipient = 4,
    RecipientCannotReceiveParcels = 5,
    NotMailable = 6,
    RecipientInboxFull = 7,
    Exhausted = 8,
    NotWritable = 9,
    TooLong = 10,
    Rejected = 11,
}

pub fn decode_intent(payload: &[u8]) -> Result<MailIntent, MailWireError> {
    let (&tag, rest) = payload.split_first().ok_or(MailWireError::Malformed)?;
    match tag {
        1 if rest.len() == 16 => Ok(MailIntent::Post {
            item_handle: u64::from_be_bytes(
                rest[..8].try_into().map_err(|_| MailWireError::Malformed)?,
            ),
            mailbox_handle: u64::from_be_bytes(
                rest[8..].try_into().map_err(|_| MailWireError::Malformed)?,
            ),
        }),
        2 if rest.len() >= 10 => {
            let item_handle =
                u64::from_be_bytes(rest[..8].try_into().map_err(|_| MailWireError::Malformed)?);
            let length = usize::from(u16::from_be_bytes(
                rest[8..10]
                    .try_into()
                    .map_err(|_| MailWireError::Malformed)?,
            ));
            if length > 7_996 {
                return Err(MailWireError::LimitExceeded);
            }
            if rest.len() != 10 + length {
                return Err(MailWireError::Malformed);
            }
            Ok(MailIntent::WriteText {
                item_handle,
                text: std::str::from_utf8(&rest[10..])
                    .map_err(|_| MailWireError::Malformed)?
                    .to_owned(),
            })
        }
        _ => Err(MailWireError::Malformed),
    }
}

pub fn encode_intent(intent: &MailIntent) -> Result<Vec<u8>, MailWireError> {
    let mut bytes = Vec::new();
    match intent {
        MailIntent::Post {
            item_handle,
            mailbox_handle,
        } => {
            bytes.push(1);
            bytes.extend(item_handle.to_be_bytes());
            bytes.extend(mailbox_handle.to_be_bytes());
        }
        MailIntent::WriteText { item_handle, text } => {
            if text.len() > 7_996 {
                return Err(MailWireError::LimitExceeded);
            }
            bytes.push(2);
            bytes.extend(item_handle.to_be_bytes());
            bytes.extend(
                u16::try_from(text.len())
                    .map_err(|_| MailWireError::Malformed)?
                    .to_be_bytes(),
            );
            bytes.extend(text.as_bytes());
        }
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mail_registry_keeps_offers_disabled() -> Result<(), serde_json::Error> {
        let registry: serde_json::Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../docs/contracts/protocol-oteryn/v1/registry/capabilities/0115-mail-v1.json"
        )))?;
        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/0115-mail-v1.json"))?;
        assert_eq!(registry["offered"], false);
        assert_eq!(registry["id"], CAPABILITY_MAIL_V1);
        assert_eq!(
            registry["command_types"],
            serde_json::json!([COMMAND_TYPE_MAIL])
        );
        for offer in ["production", "item_view", "bundle_world"] {
            assert_eq!(capability["offers"][offer], false);
        }
        Ok(())
    }

    #[test]
    fn intents_round_trip() -> Result<(), MailWireError> {
        for intent in [
            MailIntent::Post {
                item_handle: 7,
                mailbox_handle: 9,
            },
            MailIntent::WriteText {
                item_handle: 7,
                text: "Alice\nThais".into(),
            },
        ] {
            assert_eq!(decode_intent(&encode_intent(&intent)?), Ok(intent));
        }
        assert!(
            encode_intent(&MailIntent::WriteText {
                item_handle: 1,
                text: "x".repeat(7_997)
            })
            .is_err()
        );
        Ok(())
    }
}

//! Private journal items cursor wire (accepted revision 3).

use crate::{
    SourceError,
    character_commands::{counter, source_authority_valid, uuid_v7},
};
use serde::{Deserialize, Serialize};
pub mod progress;
mod publish;
pub mod publisher;
pub use publish::*;

/// Closed items journal stream families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ItemsFamily {
    #[serde(rename = "ITEMS")]
    Items,
    #[serde(rename = "BANK")]
    Bank,
    #[serde(rename = "MARKET")]
    Market,
    #[serde(rename = "PLAYER_TRADE")]
    PlayerTrade,
}

/// Canonical cursor request. All nullable members must be present on the wire.
/// Encoding validates shape only; the caller must prove the authoritative receipt cut.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadGameJournalCursorV1 {
    pub contract_version: u8,
    pub operation: String,
    pub source_authority: String,
    pub account_id: String,
    pub world_id: String,
    pub character_id: Option<String>,
    pub character_revision: Option<String>,
    pub from_projection_revision: Option<String>,
    pub to_projection_revision: Option<String>,
    pub projection_epoch: Option<String>,
    pub family: ItemsFamily,
    pub recovery_generation: String,
}
impl ReadGameJournalCursorV1 {
    /// Validate and encode in contract member order, without whitespace.
    pub fn encode(&self) -> Result<Vec<u8>, SourceError> {
        let cut = [
            self.character_revision.as_deref(),
            self.from_projection_revision.as_deref(),
            self.to_projection_revision.as_deref(),
            self.projection_epoch.as_deref(),
        ];
        let valid_cut = if self.family == ItemsFamily::Bank {
            self.character_id.is_none() && cut.iter().all(Option::is_none)
        } else {
            self.character_id.as_deref().is_some_and(uuid_v7)
                && cut.iter().all(|v| v.is_some_and(|s| counter(s).is_some()))
        };
        if self.contract_version != 1
            || self.operation != "ReadGameJournalCursorV1"
            || !source_authority_valid(&self.source_authority)
            || !uuid_v7(&self.account_id)
            || !uuid_v7(&self.world_id)
            || counter(&self.recovery_generation).is_none()
            || !valid_cut
        {
            return Err(SourceError::InvalidInput);
        }
        let bytes = serde_json::to_vec(self).map_err(|_| SourceError::InvalidInput)?;
        if bytes.len() > 1024 {
            return Err(SourceError::CapacityExceeded);
        }
        Ok(bytes)
    }
    /// Reject noncanonical ordering, omitted nulls, duplicate/unknown members and escapes.
    pub fn decode(bytes: &[u8]) -> Result<Self, SourceError> {
        if bytes.len() > 1024 {
            return Err(SourceError::CapacityExceeded);
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?;
        if value.encode()? != bytes {
            return Err(SourceError::InvalidInput);
        }
        Ok(value)
    }
}

/// Storage acknowledgement. Only the contract's exact 200 body is accepted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PublishGameJournalRecordV1Response;
impl PublishGameJournalRecordV1Response {
    pub const BYTES: &'static [u8] = br#"{"contract_version":1,"result":"accepted"}"#;
    pub fn decode(bytes: &[u8]) -> Result<Self, SourceError> {
        if bytes != Self::BYTES {
            return Err(SourceError::InvalidInput);
        }
        Ok(Self)
    }
}

/// Contiguous storage watermark; it conveys no ownership authorization.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadGameJournalCursorV1Response {
    pub contract_version: u8,
    pub after_source_revision: String,
    pub last_record_id: Option<String>,
}
impl ReadGameJournalCursorV1Response {
    pub fn encode(&self) -> Result<Vec<u8>, SourceError> {
        let valid = if self.after_source_revision == "0" {
            self.last_record_id.is_none()
        } else {
            counter(&self.after_source_revision).is_some()
                && self.last_record_id.as_deref().is_some_and(uuid_v7)
        };
        if self.contract_version != 1 || !valid {
            return Err(SourceError::InvalidInput);
        }
        serde_json::to_vec(self).map_err(|_| SourceError::InvalidInput)
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, SourceError> {
        if bytes.len() > 1024 {
            return Err(SourceError::CapacityExceeded);
        }
        let value: Self = serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?;
        if value.encode()? != bytes {
            return Err(SourceError::InvalidInput);
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn journal_responses_are_closed() -> Result<(), Box<dyn std::error::Error>> {
        assert!(
            PublishGameJournalRecordV1Response::decode(PublishGameJournalRecordV1Response::BYTES)
                .is_ok()
        );
        assert!(
            PublishGameJournalRecordV1Response::decode(
                br#"{"contract_version":1,"result":"accepted","extra":1}"#
            )
            .is_err()
        );
        for bytes in [
            br#"{"contract_version":1,"after_source_revision":"0","last_record_id":null}"#.as_slice(),
            br#"{"contract_version":1,"after_source_revision":"1","last_record_id":"019a0000-0000-7000-8000-000000000004"}"#.as_slice(),
        ] {
            assert_eq!(ReadGameJournalCursorV1Response::decode(bytes)?.encode()?, bytes);
        }
        for bytes in [
            br#"{"contract_version":1,"after_source_revision":"0"}"#.as_slice(),
            br#"{"contract_version":1,"after_source_revision":"1","last_record_id":null}"#
                .as_slice(),
            br#"{"after_source_revision":"0","contract_version":1,"last_record_id":null}"#
                .as_slice(),
            br#"{"contract_version":1,"after_source_revision":"00","last_record_id":null}"#
                .as_slice(),
        ] {
            assert!(ReadGameJournalCursorV1Response::decode(bytes).is_err());
        }
        Ok(())
    }
    const GOLDEN: &[u8] = br#"{"contract_version":1,"operation":"ReadGameJournalCursorV1","source_authority":"OTERYN_GAME_JOURNAL","account_id":"019a0000-0000-7000-8000-000000000001","world_id":"019a0000-0000-7000-8000-000000000002","character_id":null,"character_revision":null,"from_projection_revision":null,"to_projection_revision":null,"projection_epoch":null,"family":"BANK","recovery_generation":"1"}"#;
    #[test]
    fn journal_cursor_golden_and_closed_encoding() -> Result<(), Box<dyn std::error::Error>> {
        let cursor = ReadGameJournalCursorV1::decode(GOLDEN)?;
        assert_eq!(cursor.encode()?, GOLDEN);
        let text = std::str::from_utf8(GOLDEN)?;
        for invalid in [
            text.replace("\"contract_version\":1,", ""),
            text.replace(
                "\"contract_version\":1,",
                "\"extra\":1,\"contract_version\":1,",
            ),
            text.replace(
                "{\"contract_version\":1,\"operation\":\"ReadGameJournalCursorV1\"",
                "{\"operation\":\"ReadGameJournalCursorV1\",\"contract_version\":1",
            ),
            text.replace(
                "\"recovery_generation\":\"1\"",
                "\"recovery_generation\":\"01\"",
            ),
            text.replace("\"character_id\":null,", ""),
            format!("{text}\n"),
        ] {
            assert!(ReadGameJournalCursorV1::decode(invalid.as_bytes()).is_err());
        }
        assert!(ReadGameJournalCursorV1::decode(&[b' '; 1025]).is_err());
        Ok(())
    }
}

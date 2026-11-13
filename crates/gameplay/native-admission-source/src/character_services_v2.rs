//! Closed character-services v2 wire, Platform proposal revision 2 @ a85b5da.
use super::SourceError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

#[derive(Clone)]
struct Object(Vec<(String, Value)>);
impl<'de> Deserialize<'de> for Object {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = Object;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an ordered object")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut map: M,
            ) -> Result<Object, M::Error> {
                let mut fields = Vec::new();
                while let Some(pair) = map.next_entry::<String, Value>()? {
                    fields.push(pair);
                }
                Ok(Object(fields))
            }
        }
        d.deserialize_map(Visitor)
    }
}
impl Serialize for Object {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = s.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}
#[derive(Clone, Copy)]
enum Kind {
    List,
    Pending,
    Read,
    Intent,
    Receipt,
    State,
    Watermark,
    Ack,
    ReceiptAck,
}
impl Kind {
    fn bound(self) -> usize {
        match self {
            Self::List | Self::Read | Self::Watermark => 512,
            Self::Pending => 8192,
            Self::Intent => 4096,
            Self::Receipt => 1024,
            Self::State => 2048,
            Self::Ack | Self::ReceiptAck => 256,
        }
    }
}
macro_rules! message {
    ($name:ident,$kind:ident) => {
        #[derive(Clone)]
        pub struct $name(Object);
        impl $name {
            pub fn decode(bytes: &[u8]) -> Result<Self, SourceError> {
                decode(Kind::$kind, bytes).map(Self)
            }
            pub fn encode(&self) -> Result<Vec<u8>, SourceError> {
                serde_json::to_vec(&self.0).map_err(|_| SourceError::InvalidInput)
            }
            /// Construct from application facts, applying the same closed wire validation.
            pub fn from_fields(fields: Vec<(String, Value)>) -> Result<Self, SourceError> {
                let object = Object(fields);
                if !valid(Kind::$kind, &object) {
                    return Err(SourceError::InvalidInput);
                }
                let bytes = serde_json::to_vec(&object).map_err(|_| SourceError::InvalidInput)?;
                Self::decode(&bytes)
            }
            pub fn field(&self, name: &str) -> Option<&Value> {
                self.0.0.iter().find(|(k, _)| k == name).map(|(_, v)| v)
            }
        }
    };
}
message!(ListPendingCharacterCommandsV2, List);
message!(PendingCharacterCommandsV2, Pending);
message!(ReadCharacterCommandIntentV2, Read);
message!(CharacterCommandIntentV2, Intent);
message!(PublishCharacterCommandReceiptV2, Receipt);
message!(PublishCharacterServiceStateV2, State);
message!(PublishCharacterServiceWatermarkV2, Watermark);
message!(AcknowledgementV2, Ack);
message!(ReceiptAcknowledgementV2, ReceiptAck);
pub const PENDING_PATH: &str = "/internal/v2/game-auth/character-commands/pending";
pub const READ_PATH: &str = "/internal/v2/game-auth/character-commands/read";
pub const RECEIPT_PATH: &str = "/internal/v2/game-auth/character-commands/receipt";
pub const STATE_PATH: &str = "/internal/v2/game-auth/character-services/state";
pub const WATERMARK_PATH: &str = "/internal/v2/game-auth/character-services/watermark";
impl CharacterCommandIntentV2 {
    /// A missing trusted clock never authorizes execution; expiry is exclusive.
    pub fn valid_at(&self, now: Option<u64>) -> bool {
        now.is_some_and(|now| {
            number(&self.0, "issued_at_source")
                .is_some_and(|issued| issued.saturating_sub(now) <= MAX_FUTURE_SKEW_SECONDS)
                && number(&self.0, "expires_at_source").is_some_and(|expires| now < expires)
        })
    }
}
pub const MAX_INTENT_TTL_SECONDS: u64 = 300;
pub const MAX_FUTURE_SKEW_SECONDS: u64 = 5;
const NIL: &str = "00000000-0000-0000-0000-000000000000";
const ISSUER: &str = "OTERYN_PLATFORM_CHARACTER_AUTHORITY";
const AUDIENCE: &str = "OTERYN_GAME_CHARACTER_AUTHORITY";
const COMMANDS: [(&str, &str); 5] = [
    ("ScheduleCharacterDeletion", "PLATFORM_USER_DELETE"),
    ("CancelCharacterDeletion", "PLATFORM_USER_CANCEL_DELETE"),
    ("RenameCharacter", "PLATFORM_USER_RENAME"),
    ("ChangeCharacterSex", "PLATFORM_USER_SEX_CHANGE"),
    ("TransferCharacterWorld", "PLATFORM_USER_WORLD_TRANSFER"),
];
const PREFIX: &str = "contract_version command variant issuer_authority issuer_decision_id source_revision operation_id character_id account_id expected_character_revision";
const SUFFIX: &str = "issued_at_source expires_at_source audience";
const RECEIPT: &str =
    "contract_version operation source_authority command operation_id outcome result_code";
fn text<'a>(o: &'a Object, k: &str) -> Option<&'a str> {
    o.0.iter().find(|(n, _)| n == k)?.1.as_str()
}
fn number(o: &Object, k: &str) -> Option<u64> {
    super::character_commands::counter(text(o, k)?)
}
fn uuid(o: &Object, k: &str) -> bool {
    text(o, k).is_some_and(super::character_commands::uuid_v7)
}
fn literal(o: &Object, k: &str, v: &str) -> bool {
    text(o, k) == Some(v)
}
fn keys(o: &Object, wanted: &str) -> bool {
    o.0.iter()
        .map(|(k, _)| k.as_str())
        .eq(wanted.split_whitespace())
}
fn head(o: &Object, operation: &str) -> bool {
    literal(o, "operation", operation)
        && text(o, "source_authority")
            .is_some_and(super::character_commands::source_authority_valid)
}
fn command(o: &Object) -> Option<usize> {
    COMMANDS.iter().position(|(c, _)| literal(o, "command", c))
}
fn sex(v: &Value) -> bool {
    matches!(v.as_u64(), Some(1 | 2))
}
fn requested(s: &str) -> bool {
    (2..=29).contains(&s.len())
        && s.split(' ')
            .all(|word| !word.is_empty() && word.bytes().all(|b| b.is_ascii_alphabetic()))
}
fn wire_name(s: &str) -> bool {
    use unicode_normalization::UnicodeNormalization;
    use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};
    (1..=64).contains(&s.len())
        && s.nfc().eq(s.chars())
        && s.chars().all(|c| {
            !matches!(c, '"' | '\\')
                && !matches!(
                    c.general_category(),
                    GeneralCategory::Control | GeneralCategory::Format
                )
        })
}
fn field<'a>(o: &'a Object, k: &str) -> Option<&'a Value> {
    o.0.iter().find(|(n, _)| n == k).map(|(_, v)| v)
}
fn named(o: &Object, k: &str) -> bool {
    text(o, k).is_some_and(wire_name)
}
fn time_pair(o: &Object, a: &str, b: &str) -> bool {
    matches!((number(o,a),number(o,b)),(Some(a),Some(b)) if b>a)
}
fn extra(index: usize) -> &'static str {
    match index {
        0 => "",
        1 => "deletion_operation_id",
        2 => "requested_name",
        3 => "requested_sex",
        _ => "expected_from_world_id target_world_id",
    }
}
fn intent(o: &Object) -> bool {
    let Some(c) = command(o) else { return false };
    keys(o, &format!("{PREFIX} {} {SUFFIX}", extra(c)))
        && literal(o, "variant", COMMANDS[c].1)
        && literal(o, "issuer_authority", ISSUER)
        && literal(o, "audience", AUDIENCE)
        && [
            "issuer_decision_id",
            "operation_id",
            "character_id",
            "account_id",
        ]
        .iter()
        .all(|k| uuid(o, k))
        && [
            "source_revision",
            "expected_character_revision",
            "issued_at_source",
            "expires_at_source",
        ]
        .iter()
        .all(|k| number(o, k).is_some())
        && matches!((number(o,"issued_at_source"),number(o,"expires_at_source")),(Some(a),Some(b)) if b>a&&b-a<=MAX_INTENT_TTL_SECONDS)
        && match c {
            0 => true,
            1 => uuid(o, "deletion_operation_id"),
            2 => text(o, "requested_name").is_some_and(requested),
            3 => field(o, "requested_sex").is_some_and(sex),
            _ => {
                uuid(o, "expected_from_world_id")
                    && uuid(o, "target_world_id")
                    && text(o, "expected_from_world_id") != text(o, "target_world_id")
            }
        }
}
fn rejection(o: &Object, c: usize) -> bool {
    let Some(code) = text(o, "result_code") else {
        return false;
    };
    matches!(
        code,
        "CHAR_CMD_INTENT_INVALID"
            | "CHAR_CMD_INTENT_EXPIRED"
            | "CHAR_CMD_UNSUPPORTED"
            | "CHAR_CMD_OPERATION_CONFLICT"
            | "CHAR_CMD_ACCOUNT_NOT_ELIGIBLE"
            | "CHAR_CMD_CHARACTER_NOT_FOUND"
            | "CHAR_CMD_OWNER_MISMATCH"
            | "CHAR_CMD_CHARACTER_IN_SESSION"
            | "CHAR_CMD_STALE_PRECONDITION"
            | "CHAR_CMD_LIFECYCLE_CONFLICT"
    ) || match c {
        1 => code == "CHAR_CMD_DELETION_WINDOW_CLOSED",
        2 => matches!(code, "CHAR_CMD_NAME_INVALID" | "CHAR_CMD_NAME_UNAVAILABLE"),
        3 => code == "CHAR_CMD_SEX_CHANGE_NOT_ALLOWED",
        4 => matches!(
            code,
            "CHAR_CMD_WORLD_NOT_ACCEPTED"
                | "CHAR_CMD_CHARACTER_NOT_TRANSFERABLE"
                | "CHAR_CMD_HOUSING_NOT_PREPARED"
        ),
        _ => false,
    }
}
fn receipt(o: &Object) -> bool {
    let Some(c) = command(o) else { return false };
    if !head(o, "PublishCharacterCommandReceiptV2")
        || !uuid(o, "operation_id")
        || number(o, "recovery_generation").is_none()
        || number(o, "decided_at").is_none()
    {
        return false;
    }
    if literal(o, "outcome", "REJECTED") {
        return keys(o, &format!("{RECEIPT} recovery_generation decided_at")) && rejection(o, c);
    }
    let more = match c {
        0 => "lifecycle deletion_operation_id deletion_scheduled_at deletion_effective_at",
        1 => "lifecycle deletion_operation_id",
        2 => "previous_name name",
        3 => "previous_sex sex",
        _ => "from_world_id world_id",
    };
    literal(o, "outcome", "COMMITTED")
        && literal(o, "result_code", "CHAR_CMD_OK")
        && keys(
            o,
            &format!(
                "{RECEIPT} character_id account_id character_revision projection_epoch projection_revision {more} recovery_generation decided_at"
            ),
        )
        && ["character_id", "account_id"].iter().all(|k| uuid(o, k))
        && [
            "character_revision",
            "projection_epoch",
            "projection_revision",
        ]
        .iter()
        .all(|k| number(o, k).is_some())
        && match c {
            0 => {
                literal(o, "lifecycle", "DELETION_PENDING")
                    && uuid(o, "deletion_operation_id")
                    && text(o, "deletion_operation_id") == text(o, "operation_id")
                    && time_pair(o, "deletion_scheduled_at", "deletion_effective_at")
            }
            1 => literal(o, "lifecycle", "ACTIVE") && uuid(o, "deletion_operation_id"),
            2 => named(o, "previous_name") && named(o, "name"),
            3 => field(o, "previous_sex").is_some_and(sex) && field(o, "sex").is_some_and(sex),
            _ => {
                uuid(o, "from_world_id")
                    && uuid(o, "world_id")
                    && text(o, "from_world_id") != text(o, "world_id")
            }
        }
}
fn state(o: &Object) -> bool {
    if !keys(
        o,
        "contract_version operation source_authority account_id character_id projection_epoch projection_revision source_observed_at character_revision world_id name sex lifecycle deletion_operation_id deletion_scheduled_at deletion_effective_at",
    ) || !head(o, "PublishCharacterServiceStateV2")
        || !["account_id", "character_id", "world_id"]
            .iter()
            .all(|k| uuid(o, k))
        || ![
            "projection_epoch",
            "projection_revision",
            "source_observed_at",
            "character_revision",
        ]
        .iter()
        .all(|k| number(o, k).is_some())
        || !named(o, "name")
        || !field(o, "sex").is_some_and(|s| s.is_null() || sex(s))
    {
        return false;
    }
    match text(o, "lifecycle") {
        Some("DELETION_PENDING" | "DELETED") => {
            uuid(o, "deletion_operation_id")
                && time_pair(o, "deletion_scheduled_at", "deletion_effective_at")
        }
        Some("ACTIVE" | "REMOVED_FROM_ACCOUNT") => {
            [
                "deletion_operation_id",
                "deletion_scheduled_at",
                "deletion_effective_at",
            ]
            .iter()
            .all(|k| field(o, k).is_some_and(Value::is_null))
                && (!literal(o, "lifecycle", "REMOVED_FROM_ACCOUNT")
                    || field(o, "sex").is_some_and(Value::is_null))
        }
        _ => false,
    }
}
fn valid(kind: Kind, o: &Object) -> bool {
    if field(o, "contract_version").and_then(Value::as_u64) != Some(2) {
        return false;
    }
    match kind {
        Kind::Intent => intent(o),
        Kind::Receipt => receipt(o),
        Kind::State => state(o),
        Kind::ReceiptAck => keys(o, "contract_version result") && literal(o, "result", "accepted"),
        Kind::Ack => {
            keys(o, "contract_version result")
                && matches!(text(o, "result"), Some("accepted" | "superseded"))
        }
        Kind::Read => {
            keys(
                o,
                "contract_version operation source_authority issuer_authority command operation_id",
            ) && head(o, "ReadCharacterCommandIntentV2")
                && literal(o, "issuer_authority", ISSUER)
                && command(o).is_some()
                && uuid(o, "operation_id")
        }
        Kind::Watermark => {
            keys(
                o,
                "contract_version operation source_authority projection_epoch complete_through observed_at",
            ) && head(o, "PublishCharacterServiceWatermarkV2")
                && number(o, "projection_epoch").is_some()
                && matches!((number(o,"complete_through"),number(o,"observed_at")),(Some(a),Some(b)) if a<=b)
        }
        Kind::List => {
            keys(
                o,
                "contract_version operation source_authority after_source_revision after_operation_id max_entries",
            ) && head(o, "ListPendingCharacterCommandsV2")
                && matches!(
                    field(o, "max_entries").and_then(Value::as_u64),
                    Some(1..=32)
                )
                && ((literal(o, "after_source_revision", "0")
                    && literal(o, "after_operation_id", NIL))
                    || (number(o, "after_source_revision").is_some()
                        && uuid(o, "after_operation_id")))
        }
        Kind::Pending => {
            if !keys(o, "contract_version pending") {
                return false;
            }
            let Some(entries) = field(o, "pending")
                .and_then(Value::as_array)
                .filter(|a| a.len() <= 32)
            else {
                return false;
            };
            let mut previous = None;
            for entry in entries {
                let Some(map) = entry.as_object() else {
                    return false;
                };
                if !map.keys().map(String::as_str).eq([
                    "command",
                    "operation_id",
                    "source_revision",
                ]) {
                    return false;
                }
                let Some(c) = map["command"].as_str() else {
                    return false;
                };
                let Some(id) = map["operation_id"]
                    .as_str()
                    .filter(|id| super::character_commands::uuid_v7(id))
                else {
                    return false;
                };
                let Some(rev) = map["source_revision"]
                    .as_str()
                    .and_then(super::character_commands::counter)
                else {
                    return false;
                };
                if !COMMANDS.iter().any(|(name, _)| *name == c)
                    || previous.is_some_and(|old| old >= (rev, id))
                {
                    return false;
                }
                previous = Some((rev, id));
            }
            true
        }
    }
}
fn decode(kind: Kind, bytes: &[u8]) -> Result<Object, SourceError> {
    if bytes.is_empty() || bytes.len() > kind.bound() || bytes.contains(&b'\\') {
        return Err(SourceError::InvalidInput);
    }
    let o: Object = serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidInput)?;
    if !valid(kind, &o) || serde_json::to_vec(&o).map_err(|_| SourceError::InvalidInput)? != bytes {
        return Err(SourceError::InvalidInput);
    }
    Ok(o)
}
#[cfg(test)]
mod tests;

//! Pure embedded Achievement catalogue parsing and admission.
use crate::account_achievement::valid_catalogue_entry;
use serde::Deserialize;
use std::collections::BTreeMap;

/// Every `content/achievements/achievements-*.json` shard, in file-name order. A unit test keeps
/// this list equal to the directory.
pub const SHARDS: [&str; 2] = [
    include_str!("../../../../content/achievements/achievements-00000-00499.json"),
    include_str!("../../../../content/achievements/achievements-00500-00570.json"),
];

const FAMILY: &str = "Achievement";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Shard {
    family: String,
    records: Vec<Record>,
}

/// A catalogue record; the fields neither the grant path nor the display read uses are ignored
/// here.
#[derive(Deserialize)]
struct Record {
    identity: Identity,
    name: String,
    description: String,
    grade: u32,
    points: u32,
    secret: bool,
    #[serde(default)]
    retired: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Identity {
    family: String,
    key: String,
    revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub revision: String,
    pub retired: bool,
    pub name: String,
    pub description: String,
    pub grade: u32,
    pub points: u32,
    pub secret: bool,
}

/// The catalogue could not be loaded; the server refuses readiness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AchievementCatalogueError {
    /// A shard is not a catalogue shard (not JSON, or not the shard shape).
    MalformedShard,
    /// A shard or record identity names another family.
    WrongFamily,
    /// A key or revision outside the catalogue grammar.
    InvalidIdentity,
    /// Two records share a key (contract §2.1).
    DuplicateKey(String),
    /// No record at all.
    Empty,
}

/// Parse every shard as one catalogue, rejecting malformed or duplicate records.
pub fn parse_shards(shards: &[&str]) -> Result<BTreeMap<String, Entry>, AchievementCatalogueError> {
    let mut entries = BTreeMap::new();
    for shard in shards {
        let shard: Shard =
            serde_json::from_str(shard).map_err(|_| AchievementCatalogueError::MalformedShard)?;
        if shard.family != FAMILY {
            return Err(AchievementCatalogueError::WrongFamily);
        }
        for record in shard.records {
            let identity = &record.identity;
            if identity.family != FAMILY {
                return Err(AchievementCatalogueError::WrongFamily);
            }
            if !valid_catalogue_entry(&identity.key, &identity.revision) {
                return Err(AchievementCatalogueError::InvalidIdentity);
            }
            if entries.contains_key(&identity.key) {
                return Err(AchievementCatalogueError::DuplicateKey(
                    identity.key.clone(),
                ));
            }
            let Record {
                identity,
                name,
                description,
                grade,
                points,
                secret,
                retired,
            } = record;
            let entry = Entry {
                revision: identity.revision,
                retired,
                name,
                description,
                grade,
                points,
                secret,
            };
            entries.insert(identity.key, entry);
        }
    }
    if entries.is_empty() {
        return Err(AchievementCatalogueError::Empty);
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn shard(records: &str) -> String {
        format!(r#"{{"family":"Achievement","records":[{records}]}}"#)
    }

    fn record(key: &str, revision: &str, extra: &str) -> String {
        format!(
            r#"{{"identity":{{"family":"Achievement","key":"{key}","revision":"{revision}"}},"name":"N","description":"D","grade":1,"points":1,"secret":false{extra}}}"#
        )
    }

    #[test]
    fn a_malformed_catalogue_fails_closed() {
        use AchievementCatalogueError::{
            DuplicateKey, Empty, InvalidIdentity, MalformedShard, WrongFamily,
        };
        let valid = record("oteryn:achievement/allow_cookies", "1", "");
        let load = |shards: &[&str]| parse_shards(shards);
        for (text, expected) in [
            ("{".to_owned(), MalformedShard),
            (format!("[{valid}]"), MalformedShard),
            (
                format!(r#"{{"family":"Achievement","records":[{valid}],"x":1}}"#),
                MalformedShard,
            ),
            (shard(r#"{"name":"no identity"}"#), MalformedShard),
            (
                shard(&valid.replace(r#""points":1"#, r#""points":1,"retired":"no""#)),
                MalformedShard,
            ),
            (
                format!(r#"{{"family":"Charm","records":[{valid}]}}"#),
                WrongFamily,
            ),
            (
                shard(&valid.replace(r#"family":"Achievement"#, r#"family":"Quest"#)),
                WrongFamily,
            ),
            (shard(&valid.replace("oteryn:", "canary:")), InvalidIdentity),
            (
                shard(&valid.replace("allow_cookies", "Allow_Cookies")),
                InvalidIdentity,
            ),
            (
                shard(&valid.replace(r#""revision":"1""#, r#""revision":"""#)),
                InvalidIdentity,
            ),
            (shard(""), Empty),
        ] {
            assert_eq!(load(&[&text]), Err(expected), "{text}");
        }
        let duplicate = DuplicateKey("oteryn:achievement/allow_cookies".into());
        assert_eq!(load(&[&shard(&valid), &shard(&valid)]), Err(duplicate));
        assert_eq!(load(&[]), Err(Empty));
    }
}

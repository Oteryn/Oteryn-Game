//! ACHIEVEMENT: the runtime Achievement catalogue (`OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1` §2, §3).
//!
//! The shards of `content/achievements/` are embedded at build time, like the V1 spell book, and
//! parsed when the server activates its Content ([`AchievementCatalogue::embedded`]). Any malformed
//! shard, record, key or revision, or a duplicate key, fails the whole catalogue closed; nothing is
//! skipped. A granter resolves a key with [`AchievementCatalogue::lookup`] into the
//! [`AchievementCatalogueLookup`] the grant path takes (§3): `Earnable` at the record's revision,
//! `Retired`, or `Absent`. [`AchievementCatalogue::unbound_reward_claim_achievements`] is the
//! §3.3 Content validation of the reward-claim granters against it.
//!
//! Only the facts the grant path needs are kept (key, revision, retired). The complete record
//! schema is checked offline by `tools/content-schema/achievement-authoring/`.

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::content::{CanonicalReferencePlayableContent, ReferenceDefinitionKind};
use crate::durability::account_achievement::{AchievementCatalogueLookup, valid_catalogue_entry};

/// Every `content/achievements/achievements-*.json` shard, in file-name order. A unit test keeps
/// this list equal to the directory.
const SHARDS: [&str; 2] = [
    include_str!("../../../content/achievements/achievements-00000-00499.json"),
    include_str!("../../../content/achievements/achievements-00500-00570.json"),
];

const FAMILY: &str = "Achievement";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Shard {
    family: String,
    records: Vec<Record>,
}

/// A catalogue record; the fields the grant path does not read are ignored here.
#[derive(Deserialize)]
struct Record {
    identity: Identity,
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
struct Entry {
    revision: String,
    retired: bool,
}

/// The catalogue could not be loaded; the server refuses readiness.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AchievementCatalogueError {
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

/// The world's Achievement catalogue: key to revision and retired state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AchievementCatalogue {
    entries: BTreeMap<String, Entry>,
}

impl AchievementCatalogue {
    /// The catalogue embedded from `content/achievements/`.
    pub(crate) fn embedded() -> Result<Self, AchievementCatalogueError> {
        Self::from_shards(&SHARDS)
    }

    /// All shards together form one catalogue, so keys are unique across them.
    fn from_shards(shards: &[&str]) -> Result<Self, AchievementCatalogueError> {
        let mut entries = BTreeMap::new();
        for shard in shards {
            let shard: Shard = serde_json::from_str(shard)
                .map_err(|_| AchievementCatalogueError::MalformedShard)?;
            if shard.family != FAMILY {
                return Err(AchievementCatalogueError::WrongFamily);
            }
            for Record { identity, retired } in shard.records {
                if identity.family != FAMILY {
                    return Err(AchievementCatalogueError::WrongFamily);
                }
                if !valid_catalogue_entry(&identity.key, &identity.revision) {
                    return Err(AchievementCatalogueError::InvalidIdentity);
                }
                if entries.contains_key(&identity.key) {
                    return Err(AchievementCatalogueError::DuplicateKey(identity.key));
                }
                let revision = identity.revision;
                entries.insert(identity.key, Entry { revision, retired });
            }
        }
        if entries.is_empty() {
            return Err(AchievementCatalogueError::Empty);
        }
        Ok(Self { entries })
    }

    /// The catalogue's entry for `key` (contract §3).
    pub(crate) fn lookup(&self, key: &str) -> AchievementCatalogueLookup {
        match self.entries.get(key) {
            None => AchievementCatalogueLookup::Absent,
            Some(Entry { retired: true, .. }) => AchievementCatalogueLookup::Retired,
            Some(Entry { revision, .. }) => AchievementCatalogueLookup::Earnable {
                revision: revision.clone(),
            },
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Content validation (contract §3.3): the achievement keys that `content`'s RewardClaim
    /// placements name and this catalogue lacks, in Content order. A retired key binds (§3.4);
    /// the server refuses a Content activation with any unbound key.
    pub(crate) fn unbound_reward_claim_achievements<'a>(
        &self,
        content: &'a CanonicalReferencePlayableContent,
    ) -> Vec<&'a str> {
        self.unbound(
            content
                .definitions
                .iter()
                .filter_map(|definition| match &definition.kind {
                    ReferenceDefinitionKind::RewardClaim(claim) => Some(claim),
                    _ => None,
                })
                .flat_map(|claim| &claim.placements)
                .filter_map(|entry| entry.achievement.as_deref()),
        )
    }

    fn unbound<'a>(&self, keys: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
        keys.into_iter()
            .filter(|key| self.lookup(key) == AchievementCatalogueLookup::Absent)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shard(records: &str) -> String {
        format!(r#"{{"family":"Achievement","records":[{records}]}}"#)
    }

    fn record(key: &str, revision: &str, extra: &str) -> String {
        format!(
            r#"{{"identity":{{"family":"Achievement","key":"{key}","revision":"{revision}"}},"name":"N","points":1{extra}}}"#
        )
    }

    fn earnable(revision: &str) -> AchievementCatalogueLookup {
        AchievementCatalogueLookup::Earnable {
            revision: revision.into(),
        }
    }

    type TestResult = Result<(), String>;

    #[test]
    fn earnable_retired_and_absent_keys_resolve() -> TestResult {
        let catalogue = AchievementCatalogue::from_shards(&[
            &shard(&record("oteryn:achievement/allow_cookies", "3", "")),
            &shard(&record(
                "oteryn:achievement/the_more_the_merrier",
                "1",
                r#","retired":true"#,
            )),
        ])
        .map_err(|error| format!("{error:?}"))?;
        assert_eq!(catalogue.len(), 2);
        assert_eq!(
            catalogue.lookup("oteryn:achievement/allow_cookies"),
            earnable("3")
        );
        assert_eq!(
            catalogue.lookup("oteryn:achievement/the_more_the_merrier"),
            AchievementCatalogueLookup::Retired
        );
        assert_eq!(
            catalogue.lookup("oteryn:achievement/not_in_the_catalogue"),
            AchievementCatalogueLookup::Absent
        );
        // A source-derived candidate ref is not a catalogue key.
        assert_eq!(
            catalogue.lookup("canary:achievement/allow_cookies"),
            AchievementCatalogueLookup::Absent
        );
        Ok(())
    }

    #[test]
    fn a_malformed_catalogue_fails_closed() {
        use AchievementCatalogueError::{
            DuplicateKey, Empty, InvalidIdentity, MalformedShard, WrongFamily,
        };
        let valid = record("oteryn:achievement/allow_cookies", "1", "");
        let load = |shards: &[&str]| AchievementCatalogue::from_shards(shards);
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

    #[test]
    fn the_embedded_catalogue_loads() -> TestResult {
        let catalogue = AchievementCatalogue::embedded().map_err(|error| format!("{error:?}"))?;
        assert_eq!(catalogue.len(), 571);
        assert_eq!(
            catalogue.lookup("oteryn:achievement/annihilator"),
            earnable("1")
        );
        assert_eq!(
            catalogue.lookup("oteryn:achievement/the_more_the_merrier"),
            AchievementCatalogueLookup::Retired
        );
        Ok(())
    }

    #[test]
    fn only_a_key_the_catalogue_lacks_is_unbound() -> TestResult {
        let catalogue = AchievementCatalogue::embedded().map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            catalogue.unbound([
                "oteryn:achievement/annihilator",
                "oteryn:achievement/the_more_the_merrier",
                "oteryn:achievement/not_in_the_catalogue",
                "canary:achievement/annihilator",
            ]),
            [
                "oteryn:achievement/not_in_the_catalogue",
                "canary:achievement/annihilator"
            ]
        );
        assert!(catalogue.unbound([]).is_empty());
        Ok(())
    }

    #[test]
    fn the_embedded_shards_are_the_catalogue_directory() {
        let directory =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../content/achievements");
        let mut names: Vec<String> = std::fs::read_dir(&directory)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|entry| entry.file_name().into_string().ok())
            .filter(|name| name.starts_with("achievements-"))
            .collect();
        names.sort();
        let on_disk: Vec<String> = names
            .iter()
            .filter_map(|name| std::fs::read_to_string(directory.join(name)).ok())
            .collect();
        assert_eq!(names.len(), SHARDS.len());
        assert_eq!(on_disk, SHARDS);
    }
}

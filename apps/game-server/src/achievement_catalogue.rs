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
//! [`AchievementCatalogue::account_achievements_page`] is the display read of
//! `OTERYN_ACHIEVEMENT_DISPLAY_CONTRACT_V1` (§2.1, §3; D223-D228): the name, description, grade,
//! points and secret flag of every fact come from this catalogue's record of its key.
//!
//! Only the fields the grant path and the display read need are kept. The complete record
//! schema, including the text and grade bounds, is checked offline by
//! `tools/content-schema/achievement-authoring/`; the wire encoder refuses a row over its bounds.

use std::collections::BTreeMap;

use oteryn_protocol_oteryn::account_achievements::{
    ACCOUNT_ACHIEVEMENTS_PAGE_ROWS, AccountAchievementRow, AccountAchievementsResult,
};
use serde::Deserialize;

use crate::content::{CanonicalReferencePlayableContent, ReferenceDefinitionKind};
use crate::durability::account_achievement::{
    AchievementCatalogueLookup, EarnedAchievement, valid_catalogue_entry,
};

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
struct Entry {
    revision: String,
    retired: bool,
    name: String,
    description: String,
    grade: u32,
    points: u32,
    secret: bool,
}

/// The display read failed closed (display contract §4.3): a fact whose key the catalogue lacks
/// is a server integrity error, never skipped, because a skip would understate the points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AccountAchievementsPageError {
    UnknownKey,
    /// The fact count or the point total does not fit `uint32`.
    Overflow,
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
    pub(crate) fn from_shards(shards: &[&str]) -> Result<Self, AchievementCatalogueError> {
        let mut entries = BTreeMap::new();
        for shard in shards {
            let shard: Shard = serde_json::from_str(shard)
                .map_err(|_| AchievementCatalogueError::MalformedShard)?;
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

    /// Display contract §2.1, §3: page `page` of an account's `facts`, one row per fact and
    /// nothing else. The display fields of a fact come from this catalogue's record of its key; a
    /// retired record shows and counts 0 points. `total_points` sums over all facts and
    /// `fact_count` is their number, the same on every page. Rows are ordered by grade, then name
    /// by Unicode code point (UTF-8 byte order is code point order), then key; a page holds
    /// `min(64, remaining)` rows and a page past the end holds none.
    pub(crate) fn account_achievements_page(
        &self,
        facts: &[EarnedAchievement],
        page: u32,
    ) -> Result<AccountAchievementsResult, AccountAchievementsPageError> {
        let mut rows = Vec::with_capacity(facts.len());
        let mut total_points = 0_u32;
        for fact in facts {
            let entry = self
                .entries
                .get(&fact.achievement_key)
                .ok_or(AccountAchievementsPageError::UnknownKey)?;
            let points = if entry.retired { 0 } else { entry.points };
            total_points = total_points
                .checked_add(points)
                .ok_or(AccountAchievementsPageError::Overflow)?;
            rows.push(AccountAchievementRow {
                key: fact.achievement_key.clone(),
                name: entry.name.clone(),
                description: entry.description.clone(),
                grade: entry.grade,
                points,
                earned_at: fact.earned_at_unix_ms,
                secret: entry.secret,
            });
        }
        let fact_count =
            u32::try_from(rows.len()).map_err(|_| AccountAchievementsPageError::Overflow)?;
        rows.sort_unstable_by(|left, right| {
            (left.grade, &left.name, &left.key).cmp(&(right.grade, &right.name, &right.key))
        });
        let start = usize::try_from(page)
            .ok()
            .and_then(|page| page.checked_mul(ACCOUNT_ACHIEVEMENTS_PAGE_ROWS))
            .unwrap_or(usize::MAX)
            .min(rows.len());
        let end = start
            .saturating_add(ACCOUNT_ACHIEVEMENTS_PAGE_ROWS)
            .min(rows.len());
        let has_more = end < rows.len();
        rows.truncate(end);
        Ok(AccountAchievementsResult {
            total_points,
            fact_count,
            page,
            has_more,
            rows: rows.split_off(start),
        })
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
            r#"{{"identity":{{"family":"Achievement","key":"{key}","revision":"{revision}"}},"name":"N","description":"D","grade":1,"points":1,"secret":false{extra}}}"#
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

    fn fact(slug: &str, earned_at: i64) -> EarnedAchievement {
        EarnedAchievement {
            achievement_key: format!("oteryn:achievement/{slug}"),
            earned_at_unix_ms: earned_at,
        }
    }

    fn display(
        slug: &str,
        name: &str,
        grade: u32,
        points: u32,
        secret: bool,
        extra: &str,
    ) -> String {
        format!(
            r#"{{"identity":{{"family":"Achievement","key":"oteryn:achievement/{slug}","revision":"1"}},"name":"{name}","description":"About {name}","grade":{grade},"points":{points},"secret":{secret}{extra}}}"#
        )
    }

    fn keys(result: &AccountAchievementsResult) -> Vec<&str> {
        result
            .rows
            .iter()
            .map(|row| row.key.trim_start_matches("oteryn:achievement/"))
            .collect()
    }

    /// Display contract §2.1, §3.2, §3.4: exactly the earned facts, ordered by grade, then name
    /// by code point (`É` after `Z`), then key; a retired fact shows with 0 points even when its
    /// record carries others, and a secret record that was never earned is never sent.
    #[test]
    fn a_page_holds_only_the_facts_in_grade_name_key_order() -> TestResult {
        let catalogue = AchievementCatalogue::from_shards(&[&shard(
            &[
                display("beta", "Beta", 2, 3, false, ""),
                display("zeta_a", "Zeta", 1, 1, false, ""),
                display("eclair", "\u{c9}clair", 1, 2, true, ""),
                display("zeta_b", "Zeta", 1, 1, false, ""),
                display("unearned", "Unearned", 3, 5, true, ""),
                display("retired", "Retired", 1, 4, false, r#","retired":true"#),
            ]
            .join(","),
        )])
        .map_err(|error| format!("{error:?}"))?;
        let facts = [
            fact("beta", 10),
            fact("zeta_b", 11),
            fact("eclair", 12),
            fact("zeta_a", 13),
            fact("retired", 14),
        ];
        let page = catalogue
            .account_achievements_page(&facts, 0)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            keys(&page),
            ["retired", "zeta_a", "zeta_b", "eclair", "beta"]
        );
        assert_eq!((page.total_points, page.fact_count), (7, 5));
        assert_eq!((page.page, page.has_more), (0, false));
        assert_eq!(page.rows[0].points, 0);
        assert_eq!(
            page.rows[3],
            AccountAchievementRow {
                key: "oteryn:achievement/eclair".into(),
                name: "\u{c9}clair".into(),
                description: "About \u{c9}clair".into(),
                grade: 1,
                points: 2,
                earned_at: 12,
                secret: true,
            }
        );
        // A zero-point fact moves the watermark and the rows, not the total (§3.3).
        let without_retired = catalogue
            .account_achievements_page(&facts[..4], 0)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            (without_retired.total_points, without_retired.fact_count),
            (7, 4)
        );
        // An empty account.
        assert_eq!(
            catalogue.account_achievements_page(&[], 0),
            Ok(AccountAchievementsResult::default())
        );
        // A fact under a key the catalogue lacks fails closed instead of being skipped (§4.3).
        assert_eq!(
            catalogue.account_achievements_page(&[fact("beta", 1), fact("absent", 2)], 0),
            Err(AccountAchievementsPageError::UnknownKey)
        );
        Ok(())
    }

    /// Display contract §3.3: 64 rows a page, `has_more` until the last, and a page past the end
    /// holds no rows; the totals are the same on every page.
    #[test]
    fn pages_hold_64_rows_and_a_page_past_the_end_holds_none() -> TestResult {
        let records: Vec<String> = (0..130)
            .map(|index| {
                display(
                    &format!("k{index:03}"),
                    &format!("N{index:03}"),
                    1,
                    1,
                    false,
                    "",
                )
            })
            .collect();
        let catalogue = AchievementCatalogue::from_shards(&[&shard(&records.join(","))])
            .map_err(|error| format!("{error:?}"))?;
        let facts: Vec<EarnedAchievement> = (0..130)
            .rev()
            .map(|index| fact(&format!("k{index:03}"), index))
            .collect();
        for (page, rows, first, has_more) in [
            (0, 64, Some("k000"), true),
            (1, 64, Some("k064"), true),
            (2, 2, Some("k128"), false),
            (3, 0, None, false),
            (u32::MAX, 0, None, false),
        ] {
            let result = catalogue
                .account_achievements_page(&facts, page)
                .map_err(|error| format!("{error:?}"))?;
            assert_eq!(result.rows.len(), rows, "page {page}");
            assert_eq!(keys(&result).first().copied(), first, "page {page}");
            assert_eq!(result.has_more, has_more, "page {page}");
            assert_eq!(
                (result.total_points, result.fact_count, result.page),
                (130, 130, page)
            );
        }
        // Exactly one full page: no more.
        let full = catalogue
            .account_achievements_page(&facts[66..], 0)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!((full.rows.len(), full.has_more), (64, false));
        Ok(())
    }

    /// Display contract §3.3: the current catalogue fits the wire. An account holding every
    /// record reads 9 pages that all encode within the registered bounds.
    #[test]
    fn every_embedded_record_fits_the_wire() -> TestResult {
        use oteryn_protocol_oteryn::account_achievements::encode_account_achievements_result;
        let catalogue = AchievementCatalogue::embedded().map_err(|error| format!("{error:?}"))?;
        let facts: Vec<EarnedAchievement> = catalogue
            .entries
            .keys()
            .map(|key| EarnedAchievement {
                achievement_key: key.clone(),
                earned_at_unix_ms: 1,
            })
            .collect();
        let mut seen = 0;
        for page in 0..9 {
            let result = catalogue
                .account_achievements_page(&facts, page)
                .map_err(|error| format!("{error:?}"))?;
            assert_eq!((result.total_points, result.fact_count), (1523, 571));
            assert_eq!(result.has_more, page < 8, "page {page}");
            encode_account_achievements_result(&result)
                .map_err(|error| format!("page {page}: {error:?}"))?;
            seen += result.rows.len();
        }
        assert_eq!(seen, 571);
        Ok(())
    }
}

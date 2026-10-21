//! Pure Achievement grant values and catalogue key validation.
use crate::revision::valid_revision;

const KEY_PREFIX: &str = "oteryn:achievement/";
const MAX_KEY_BYTES: usize = 160;
const MAX_SOURCE_EVENT_BYTES: usize = 64;

/// The world catalogue's entry for the granted key, as the granter resolved it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AchievementCatalogueLookup {
    /// The catalogue has the key at this content revision and it can be earned.
    Earnable { revision: String },
    /// The catalogue has the key and it is retired: nothing is recorded.
    Retired,
    /// The catalogue lacks the key: the granting transaction fails closed.
    Absent,
}

/// The fenced Character event that earned the achievement: the granting
/// domain's kind token and its own identifier of the event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchievementSourceEvent {
    pub kind: String,
    pub event_id: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AchievementGrantRequest {
    pub achievement_key: String,
    pub catalogue: AchievementCatalogueLookup,
    pub source: AchievementSourceEvent,
}

/// One fact of an account as the display read sees it: its key and the
/// earning time of the grant request it was derived from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EarnedAchievement {
    pub achievement_key: String,
    pub earned_at_unix_ms: i64,
}

/// `oteryn:achievement/<slug>`, the catalogue schema's key grammar.
pub fn valid_key(key: &str) -> bool {
    key.len() <= MAX_KEY_BYTES
        && key.strip_prefix(KEY_PREFIX).is_some_and(|slug| {
            slug.split('_').all(|part| {
                !part.is_empty()
                    && part
                        .bytes()
                        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
            })
        })
}

/// A catalogue record's key and revision are ones a grant request can carry
/// (the runtime Achievement catalogue loader).
#[allow(
    dead_code,
    reason = "PG test binaries that recompile durability without the catalogue loader"
)]
pub fn valid_catalogue_entry(key: &str, revision: &str) -> bool {
    valid_key(key) && valid_revision(revision)
}

pub fn valid_request(request: &AchievementGrantRequest) -> bool {
    valid_key(&request.achievement_key)
        && valid_revision(&request.source.kind)
        && (1..=MAX_SOURCE_EVENT_BYTES).contains(&request.source.event_id.len())
        && match &request.catalogue {
            AchievementCatalogueLookup::Earnable { revision } => valid_revision(revision),
            AchievementCatalogueLookup::Retired | AchievementCatalogueLookup::Absent => true,
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn request(key: &str, catalogue: AchievementCatalogueLookup) -> AchievementGrantRequest {
        AchievementGrantRequest {
            achievement_key: key.into(),
            catalogue,
            source: AchievementSourceEvent {
                kind: "oteryn:reward-claim".into(),
                event_id: vec![1; 16],
            },
        }
    }

    #[test]
    fn keys_follow_the_catalogue_grammar() {
        for key in [
            "oteryn:achievement/allow_cookies",
            "oteryn:achievement/the_professor_s_nut",
            "oteryn:achievement/x1",
        ] {
            assert!(valid_key(key), "{key}");
        }
        for key in [
            "",
            "oteryn:achievement/",
            "oteryn:achievement/Allow",
            "oteryn:achievement/a__b",
            "oteryn:achievement/_a",
            "oteryn:achievement/a_",
            "oteryn:achievement/a-b",
            "canary:achievement/a",
            &format!("oteryn:achievement/{}", "a".repeat(142)),
        ] {
            assert!(!valid_key(key), "{key}");
        }
    }

    #[test]
    fn invalid_revision_source_and_event_fail_before_database_work() {
        let earnable = || AchievementCatalogueLookup::Earnable {
            revision: "r1".into(),
        };
        for catalogue in [
            earnable(),
            AchievementCatalogueLookup::Retired,
            AchievementCatalogueLookup::Absent,
        ] {
            assert!(valid_request(&request("oteryn:achievement/a", catalogue)));
        }
        assert!(!valid_request(&request("oteryn:achievement/A", earnable())));
        assert!(!valid_request(&request(
            "oteryn:achievement/a",
            AchievementCatalogueLookup::Earnable {
                revision: "-r1".into()
            }
        )));
        let mut bad_kind = request("oteryn:achievement/a", earnable());
        bad_kind.source.kind = String::new();
        assert!(!valid_request(&bad_kind));
        for length in [0, MAX_SOURCE_EVENT_BYTES + 1] {
            let mut bad_event = request("oteryn:achievement/a", earnable());
            bad_event.source.event_id = vec![1; length];
            assert!(!valid_request(&bad_event));
        }
    }
}

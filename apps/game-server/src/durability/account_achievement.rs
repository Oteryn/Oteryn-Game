//! ACHIEVEMENT step 3: achievement grant requests and the account fact
//! `AccountAchievement` (migration 0021; `OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1`
//! §3, account-progress decision 2026-09-28 §4.4 and §4.6).
//!
//! A granting domain (quest completion, reward claim, interaction child,
//! counter owner) calls [`record_achievement_grant`] inside its own fenced
//! Character transaction, after its complete fence and the `character_root`
//! row lock. In that same transaction the call records the durable grant
//! request and inserts the account's fact if the account lacks the key. A
//! duplicate leaves the first committed fact and its provenance unchanged; the
//! new request stays as provenance of the grant. Neither write advances
//! `CharacterRevision`.
//!
//! The catalogue check is an input: the granter resolves the key against the
//! world's current compatible catalogue and passes the result
//! ([`AchievementCatalogueLookup`]). No runtime catalogue loader exists yet;
//! the step-4 granter provides the lookup from its Content. A key the
//! catalogue lacks fails the granting transaction closed; a retired key is a
//! no-op and the granting transaction continues.
//!
//! No points, no gameplay value, no protocol and no export: points are a read
//! over the facts and a world catalogue that no caller needs yet.

use super::DurabilityError;
use super::character_progression::{uuid_text, valid_revision};
use crate::domain::CharacterId;
use sqlx::Row;

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

/// An account's fact with the provenance of the request it was derived from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountAchievement {
    pub account_id: [u8; 16],
    pub achievement_key: String,
    pub achievement_revision: String,
    pub character_id: CharacterId,
    pub earned_at_unix_ms: i64,
    pub source: AchievementSourceEvent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AchievementGrantOutcome {
    /// This request created the account's fact.
    Granted(AccountAchievement),
    /// The account already held the key; the fact keeps its first provenance.
    AlreadyHeld(AccountAchievement),
    /// The achievement is retired: nothing was recorded.
    Retired,
}

#[derive(Debug)]
pub enum AchievementGrantError {
    InvalidInput,
    AuthorityRejected,
    /// The world's catalogue lacks the key. The caller must not commit.
    UnknownAchievement,
    /// The source event already requested the key for another Character,
    /// account or revision.
    ConflictingSourceEvent,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for AchievementGrantError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl std::fmt::Display for AchievementGrantError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput => formatter.write_str("invalid achievement grant input"),
            Self::AuthorityRejected => formatter.write_str("achievement grant authority rejected"),
            Self::UnknownAchievement => {
                formatter.write_str("achievement key is not in the world catalogue")
            }
            Self::ConflictingSourceEvent => {
                formatter.write_str("achievement source event was reused with different facts")
            }
            Self::Unavailable(error) => {
                write!(formatter, "achievement storage is unavailable: {error:?}")
            }
        }
    }
}

impl std::error::Error for AchievementGrantError {}

/// Evidence that a durability writer holds its complete Character fence,
/// ending with the `character_root` row lock, in the transaction it passes to
/// [`record_achievement_grant`]. Only durability writers can create it.
#[derive(Debug)]
pub struct FencedGrantingCharacter {
    character_id: CharacterId,
}

impl FencedGrantingCharacter {
    /// Call only after the writer's fence succeeded in the same transaction.
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "the first granter (contract §5 step 4) creates it"
        )
    )]
    pub(super) const fn after_fence(character_id: CharacterId) -> Self {
        Self { character_id }
    }
}

type Pass<T> = std::result::Result<std::result::Result<T, AchievementGrantError>, DurabilityError>;

/// Record one grant request and consume it into the account's fact, in the
/// granter's transaction. On `Err(UnknownAchievement)` (or any other error)
/// the granter must drop its transaction uncommitted.
pub async fn record_achievement_grant(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    granter: &FencedGrantingCharacter,
    request: &AchievementGrantRequest,
) -> Pass<AchievementGrantOutcome> {
    if !valid_request(request) {
        return Ok(Err(AchievementGrantError::InvalidInput));
    }
    let revision = match &request.catalogue {
        AchievementCatalogueLookup::Earnable { revision } => revision,
        AchievementCatalogueLookup::Retired => return Ok(Ok(AchievementGrantOutcome::Retired)),
        AchievementCatalogueLookup::Absent => {
            return Ok(Err(AchievementGrantError::UnknownAchievement));
        }
    };
    let character = granter.character_id.as_bytes().as_slice();

    // The granter's own row lock (re-entrant here); the account comes from
    // the locked root, never from the caller.
    let account: Option<String> = sqlx::query_scalar(
        "SELECT account_id::text FROM game_character_roots \
          WHERE character_id = encode($1,'hex')::uuid AND lifecycle = 1 FOR UPDATE",
    )
    .bind(character)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(account) = account else {
        return Ok(Err(AchievementGrantError::AuthorityRejected));
    };

    let inserted = sqlx::query(
        "INSERT INTO game_account_achievement_grant_requests(\
           source_kind, source_event_id, achievement_key, achievement_revision, \
           account_id, character_id, earned_at) \
         VALUES ($1, $2, $3, $4, $5::uuid, encode($6,'hex')::uuid, \
           floor(extract(epoch FROM statement_timestamp())*1000)::bigint) \
         ON CONFLICT (source_kind, source_event_id, achievement_key) DO NOTHING",
    )
    .bind(&request.source.kind)
    .bind(&request.source.event_id)
    .bind(&request.achievement_key)
    .bind(revision)
    .bind(&account)
    .bind(character)
    .execute(&mut **tx)
    .await?;
    if inserted.rows_affected() == 0 {
        // A replay of the same request is idempotent; other facts conflict.
        let same: bool = sqlx::query_scalar(
            "SELECT account_id = $4::uuid AND character_id = encode($5,'hex')::uuid \
                    AND achievement_revision = $6 \
               FROM game_account_achievement_grant_requests \
              WHERE source_kind = $1 AND source_event_id = $2 AND achievement_key = $3",
        )
        .bind(&request.source.kind)
        .bind(&request.source.event_id)
        .bind(&request.achievement_key)
        .bind(&account)
        .bind(character)
        .bind(revision)
        .fetch_one(&mut **tx)
        .await?;
        if !same {
            return Ok(Err(AchievementGrantError::ConflictingSourceEvent));
        }
    }

    let created = sqlx::query(
        "INSERT INTO game_account_achievements(\
           account_id, achievement_key, source_kind, source_event_id) \
         VALUES ($1::uuid, $2, $3, $4) \
         ON CONFLICT (account_id, achievement_key) DO NOTHING",
    )
    .bind(&account)
    .bind(&request.achievement_key)
    .bind(&request.source.kind)
    .bind(&request.source.event_id)
    .execute(&mut **tx)
    .await?
    .rows_affected()
        == 1;
    let fact = load_fact(tx, &account, &request.achievement_key)
        .await?
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(Ok(if created {
        AchievementGrantOutcome::Granted(fact)
    } else {
        AchievementGrantOutcome::AlreadyHeld(fact)
    }))
}

async fn load_fact(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    account: &str,
    key: &str,
) -> std::result::Result<Option<AccountAchievement>, DurabilityError> {
    let row = sqlx::query(
        "SELECT r.account_id::text, r.character_id::text, r.achievement_revision, \
                r.earned_at, r.source_kind, r.source_event_id \
           FROM game_account_achievements f \
           JOIN game_account_achievement_grant_requests r \
             ON r.account_id = f.account_id AND r.achievement_key = f.achievement_key \
            AND r.source_kind = f.source_kind AND r.source_event_id = f.source_event_id \
          WHERE f.account_id = $1::uuid AND f.achievement_key = $2",
    )
    .bind(account)
    .bind(key)
    .fetch_optional(&mut **tx)
    .await?;
    row.map(|row| {
        Ok(AccountAchievement {
            account_id: uuid_text(row.try_get("account_id")?)?,
            achievement_key: key.to_owned(),
            achievement_revision: row.try_get("achievement_revision")?,
            character_id: CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            earned_at_unix_ms: row.try_get("earned_at")?,
            source: AchievementSourceEvent {
                kind: row.try_get("source_kind")?,
                event_id: row.try_get("source_event_id")?,
            },
        })
    })
    .transpose()
}

/// `oteryn:achievement/<slug>`, the catalogue schema's key grammar.
fn valid_key(key: &str) -> bool {
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

fn valid_request(request: &AchievementGrantRequest) -> bool {
    valid_key(&request.achievement_key)
        && valid_revision(&request.source.kind)
        && (1..=MAX_SOURCE_EVENT_BYTES).contains(&request.source.event_id.len())
        && match &request.catalogue {
            AchievementCatalogueLookup::Earnable { revision } => valid_revision(revision),
            AchievementCatalogueLookup::Retired | AchievementCatalogueLookup::Absent => true,
        }
}

/// Test-only granting transaction: the XP writer's complete gameplay fence,
/// then each request in order, committed together or not at all. The first
/// granter (contract §5 step 4) replaces it in production.
#[cfg(test)]
mod granting_harness {
    use super::*;
    use crate::durability::DurabilityRoot;
    use crate::durability::character_authority::{
        ReconciledCharacterAuthority, assert_recovery_fence,
    };
    use crate::durability::character_progression::{
        CurrentCharacterGameplayFence, assert_gameplay_fence,
    };
    use crate::durability::db::{
        begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
    };
    use crate::durability::runtime_scope_assignment::NodeIncarnationProof;

    impl DurabilityRoot {
        pub(crate) async fn commit_test_achievement_grants(
            &self,
            authority: &ReconciledCharacterAuthority<'_, '_>,
            node: &NodeIncarnationProof,
            fence: CurrentCharacterGameplayFence,
            requests: Vec<AchievementGrantRequest>,
        ) -> std::result::Result<Vec<AchievementGrantOutcome>, AchievementGrantError> {
            let recovery = authority
                .record_for(self)
                .map_err(|_| AchievementGrantError::AuthorityRejected)?;
            let node = node.clone();
            self.try_issue_semantic_pass()?
                .run(move |holder, deadline| {
                    Box::pin(async move {
                        let mut tx = begin_semantic_transaction(holder, deadline).await?;
                        assert_recovery_fence(&mut tx, &recovery).await?;
                        lock_admission_relations(&mut tx).await?;
                        if assert_gameplay_fence(&mut tx, &fence, &node)
                            .await?
                            .is_err()
                        {
                            return Ok(Err(AchievementGrantError::AuthorityRejected));
                        }
                        let granter = FencedGrantingCharacter::after_fence(fence.character_id);
                        let mut outcomes = Vec::with_capacity(requests.len());
                        for request in &requests {
                            match record_achievement_grant(&mut tx, &granter, request).await? {
                                Ok(outcome) => outcomes.push(outcome),
                                Err(error) => return Ok(Err(error)),
                            }
                        }
                        commit_semantic_transaction(tx, deadline).await?;
                        Ok(Ok(outcomes))
                    })
                })
                .await?
        }
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

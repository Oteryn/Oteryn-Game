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
//! ([`AchievementCatalogueLookup`]), from the runtime catalogue
//! (`crate::achievement_catalogue`); the reward-claim MINT (step 4) takes the
//! lookup from its caller, the chest `USE`. A key the
//! catalogue lacks fails the granting transaction closed; a retired key is a
//! no-op and the granting transaction continues.
//!
//! No points, no gameplay value and no export are stored: points are a read
//! over the facts and the catalogue. [`DurabilityRoot::read_account_achievements`]
//! is that read's durable half for `ACCOUNT_ACHIEVEMENTS_QUERY`
//! (`OTERYN_ACHIEVEMENT_DISPLAY_CONTRACT_V1` §4): read-only, no Character fence.

use super::DurabilityError;
use super::DurabilityRoot;
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{uuid_text, valid_revision};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::reward_claim_mint::RewardClaimFenceChecked;
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

/// Evidence that the granter holds its complete Character fence, ending with
/// the `character_root` row lock, in the transaction it passes to
/// [`record_achievement_grant`]: the fenced Character and the id of the
/// transaction that checked the fence. Neither `Clone` nor `Copy`; the grant
/// consumes it and rejects it in any other transaction.
#[derive(Debug)]
pub struct FencedGrantingCharacter {
    character_id: CharacterId,
    transaction: String,
}

impl FencedGrantingCharacter {
    /// The only production token: the witness exists only in
    /// `reward_claim_mint::admit`, right after its complete item fence
    /// returned true in `tx`.
    pub(super) async fn after_fence(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        character_id: CharacterId,
        _fenced: RewardClaimFenceChecked,
    ) -> std::result::Result<Self, DurabilityError> {
        Self::in_transaction(tx, character_id).await
    }

    async fn in_transaction(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        character_id: CharacterId,
    ) -> std::result::Result<Self, DurabilityError> {
        let transaction: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
            .fetch_one(&mut **tx)
            .await?;
        Ok(Self {
            character_id,
            transaction,
        })
    }
}

type Pass<T> = std::result::Result<std::result::Result<T, AchievementGrantError>, DurabilityError>;

/// Record one request through the same bounded batch owner. The caller must
/// drop its transaction uncommitted on any error.
pub async fn record_achievement_grant(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    granter: FencedGrantingCharacter,
    request: &AchievementGrantRequest,
) -> Pass<AchievementGrantOutcome> {
    let outcomes =
        match record_achievement_grants(tx, granter, std::slice::from_ref(request)).await? {
            Ok(outcomes) => outcomes,
            Err(error) => return Ok(Err(error)),
        };
    let outcome = outcomes
        .into_iter()
        .next()
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(Ok(outcome))
}

/// ACHIEVEMENT0-RL-01: at most 64 requests in one earning transaction.
pub const MAX_ACHIEVEMENT_GRANTS: usize = 64;

/// Record every grant belonging to one fenced Character earning commit.
/// Consumes and verifies the witness once, including an all-retired batch.
/// Writes follow ascending (source event id, achievement key) order; equal
/// pairs use source kind as a stable tie-break. Outcomes retain input order.
/// On any error the granter must drop its entire transaction uncommitted;
/// this function neither commits nor opens an independent transaction.
pub async fn record_achievement_grants(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    granter: FencedGrantingCharacter,
    requests: &[AchievementGrantRequest],
) -> Pass<Vec<AchievementGrantOutcome>> {
    let order = match ordered_batch(requests) {
        Ok(order) => order,
        Err(error) => return Ok(Err(error)),
    };
    let character = granter.character_id.as_bytes().as_slice();

    // The granter's own row lock (re-entrant here), only in the transaction
    // that checked the complete fence; the account never comes from input.
    let account: Option<String> = sqlx::query_scalar(
        "SELECT account_id::text FROM game_character_roots \
          WHERE character_id = encode($1,'hex')::uuid AND lifecycle = 1 \
            AND pg_current_xact_id()::text = $2 FOR UPDATE",
    )
    .bind(character)
    .bind(&granter.transaction)
    .fetch_optional(&mut **tx)
    .await?;
    let Some(account) = account else {
        return Ok(Err(AchievementGrantError::AuthorityRejected));
    };

    let mut outcomes = Vec::with_capacity(requests.len());
    for index in order {
        match record_validated_grant(tx, character, &account, &requests[index]).await? {
            Ok(outcome) => outcomes.push((index, outcome)),
            Err(error) => return Ok(Err(error)),
        }
    }
    outcomes.sort_unstable_by_key(|(index, _)| *index);
    Ok(Ok(outcomes
        .into_iter()
        .map(|(_, outcome)| outcome)
        .collect()))
}

/// Validate the complete input before any storage work. A source includes
/// both its kind and event id, so distinct kinds may legitimately share bytes.
fn ordered_batch(
    requests: &[AchievementGrantRequest],
) -> std::result::Result<Vec<usize>, AchievementGrantError> {
    if !(1..=MAX_ACHIEVEMENT_GRANTS).contains(&requests.len())
        || requests.iter().any(|request| !valid_request(request))
    {
        return Err(AchievementGrantError::InvalidInput);
    }
    let mut seen = std::collections::BTreeSet::new();
    for request in requests {
        if !seen.insert((
            &request.source.kind,
            &request.source.event_id,
            &request.achievement_key,
        )) {
            return Err(AchievementGrantError::InvalidInput);
        }
    }
    if requests
        .iter()
        .any(|request| request.catalogue == AchievementCatalogueLookup::Absent)
    {
        return Err(AchievementGrantError::UnknownAchievement);
    }
    let mut order: Vec<_> = (0..requests.len()).collect();
    order.sort_unstable_by(|left, right| {
        let left = &requests[*left];
        let right = &requests[*right];
        (
            &left.source.event_id,
            &left.achievement_key,
            &left.source.kind,
        )
            .cmp(&(
                &right.source.event_id,
                &right.achievement_key,
                &right.source.kind,
            ))
    });
    Ok(order)
}

/// The one owner of request/fact SQL, called only after batch validation and
/// its one witness check. No witness can escape or be copied between requests.
async fn record_validated_grant(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: &[u8],
    account: &str,
    request: &AchievementGrantRequest,
) -> Pass<AchievementGrantOutcome> {
    let revision = match &request.catalogue {
        AchievementCatalogueLookup::Earnable { revision } => revision,
        AchievementCatalogueLookup::Retired => return Ok(Ok(AchievementGrantOutcome::Retired)),
        AchievementCatalogueLookup::Absent => {
            return Ok(Err(AchievementGrantError::UnknownAchievement));
        }
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
    .bind(account)
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
        .bind(account)
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
    .bind(account)
    .bind(&request.achievement_key)
    .bind(&request.source.kind)
    .bind(&request.source.event_id)
    .execute(&mut **tx)
    .await?
    .rows_affected()
        == 1;
    let fact = load_fact(tx, account, &request.achievement_key)
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

/// One fact of an account as the display read sees it: its key and the
/// earning time of the grant request it was derived from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EarnedAchievement {
    pub achievement_key: String,
    pub earned_at_unix_ms: i64,
}

#[derive(Debug)]
pub enum AccountAchievementReadError {
    AuthorityRejected,
    Unavailable(DurabilityError),
}

impl From<DurabilityError> for AccountAchievementReadError {
    fn from(error: DurabilityError) -> Self {
        Self::Unavailable(error)
    }
}

impl std::fmt::Display for AccountAchievementReadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AuthorityRejected => formatter.write_str("achievement read authority rejected"),
            Self::Unavailable(error) => {
                write!(formatter, "achievement storage is unavailable: {error:?}")
            }
        }
    }
}

impl std::error::Error for AccountAchievementReadError {}

impl DurabilityRoot {
    /// Display contract §4: the facts of `account_id`, ordered by key, at most
    /// `limit` of them, each with the `earned_at` of the request it was
    /// derived from (the fact's own composite foreign key, so a later request
    /// for the same key never shows). Reads only `game_account_achievements`
    /// and its requests, which are append-only and immutable (migration
    /// 0021), under the runtime role's `SELECT`; it takes no Character fence
    /// and writes nothing (§4.1). The caller passes the session's own account.
    pub async fn read_account_achievements(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        account_id: [u8; 16],
        limit: usize,
    ) -> std::result::Result<Vec<EarnedAchievement>, AccountAchievementReadError> {
        let limit = i64::try_from(limit).map_err(|_| DurabilityError::InvalidStoredState)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| AccountAchievementReadError::AuthorityRejected)?;
        Ok(self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let rows = sqlx::query(
                        "SELECT f.achievement_key, r.earned_at \
                           FROM game_account_achievements f \
                           JOIN game_account_achievement_grant_requests r \
                             ON r.account_id = f.account_id \
                            AND r.achievement_key = f.achievement_key \
                            AND r.source_kind = f.source_kind \
                            AND r.source_event_id = f.source_event_id \
                          WHERE f.account_id = encode($1,'hex')::uuid \
                          ORDER BY f.achievement_key LIMIT $2",
                    )
                    .bind(account_id.as_slice())
                    .bind(limit)
                    .fetch_all(&mut *tx)
                    .await?;
                    let facts = rows
                        .iter()
                        .map(|row| {
                            Ok(EarnedAchievement {
                                achievement_key: row.try_get("achievement_key")?,
                                earned_at_unix_ms: row.try_get("earned_at")?,
                            })
                        })
                        .collect::<std::result::Result<Vec<_>, DurabilityError>>()?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(facts)
                })
            })
            .await?)
    }
}

/// `oteryn:achievement/<slug>`, the catalogue schema's key grammar.
pub(super) fn valid_key(key: &str) -> bool {
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
/// (the runtime catalogue loader, `crate::achievement_catalogue`).
#[allow(
    dead_code,
    reason = "PG test binaries that recompile durability without the catalogue loader"
)]
pub(crate) fn valid_catalogue_entry(key: &str, revision: &str) -> bool {
    valid_key(key) && valid_revision(revision)
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
/// then one batch call, committed together or not at all. In
/// production the reward-claim MINT is the granter (contract §5 step 4).
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
                        let granter =
                            FencedGrantingCharacter::in_transaction(&mut tx, fence.character_id)
                                .await?;
                        let outcomes =
                            match record_achievement_grants(&mut tx, granter, &requests).await? {
                                Ok(outcomes) => outcomes,
                                Err(error) => return Ok(Err(error)),
                            };
                        commit_semantic_transaction(tx, deadline).await?;
                        Ok(Ok(outcomes))
                    })
                })
                .await?
        }

        /// Negative only: a token minted after the complete fence in one
        /// committed transaction, presented in a second transaction under the
        /// same complete fence, which commits whatever the grant wrote.
        pub(crate) async fn commit_test_achievement_grant_with_foreign_token(
            &self,
            authority: &ReconciledCharacterAuthority<'_, '_>,
            node: &NodeIncarnationProof,
            fence: CurrentCharacterGameplayFence,
            request: AchievementGrantRequest,
        ) -> std::result::Result<AchievementGrantOutcome, AchievementGrantError> {
            let recovery = authority
                .record_for(self)
                .map_err(|_| AchievementGrantError::AuthorityRejected)?;
            let (first_recovery, first_node) = (recovery.clone(), node.clone());
            let token = self
                .try_issue_semantic_pass()?
                .run(move |holder, deadline| {
                    Box::pin(async move {
                        let mut tx = begin_semantic_transaction(holder, deadline).await?;
                        assert_recovery_fence(&mut tx, &first_recovery).await?;
                        lock_admission_relations(&mut tx).await?;
                        if assert_gameplay_fence(&mut tx, &fence, &first_node)
                            .await?
                            .is_err()
                        {
                            return Ok(Err(AchievementGrantError::AuthorityRejected));
                        }
                        let token =
                            FencedGrantingCharacter::in_transaction(&mut tx, fence.character_id)
                                .await?;
                        commit_semantic_transaction(tx, deadline).await?;
                        Ok(Ok(token))
                    })
                })
                .await??;
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
                        let outcome = record_achievement_grant(&mut tx, token, &request).await?;
                        commit_semantic_transaction(tx, deadline).await?;
                        Ok(outcome)
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

    /// The PostgreSQL targets drive the negative harness.
    #[test]
    fn the_foreign_token_harness_is_linked() {
        let _ = crate::durability::DurabilityRoot::commit_test_achievement_grant_with_foreign_token;
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

    /// The PostgreSQL targets drive the display read; the binaries that
    /// recompile durability without it still link it.
    #[test]
    fn the_display_read_is_linked() {
        let _ = crate::durability::DurabilityRoot::read_account_achievements;
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

    #[test]
    fn achievement_batch_rejects_empty_and_oversized_input() {
        assert!(matches!(
            ordered_batch(&[]),
            Err(AchievementGrantError::InvalidInput)
        ));
        let request = request("oteryn:achievement/a", AchievementCatalogueLookup::Retired);
        let requests: Vec<_> = (0..=MAX_ACHIEVEMENT_GRANTS)
            .map(|index| {
                let mut request = request.clone();
                request.source.event_id = index.to_be_bytes().to_vec();
                request
            })
            .collect();
        assert!(ordered_batch(&requests[..MAX_ACHIEVEMENT_GRANTS]).is_ok());
        assert!(matches!(
            ordered_batch(&requests),
            Err(AchievementGrantError::InvalidInput)
        ));
    }

    #[test]
    fn achievement_batch_duplicate_identity_includes_kind_but_not_revision() {
        let first = request("oteryn:achievement/a", AchievementCatalogueLookup::Retired);
        let mut other = first.clone();
        other.catalogue = AchievementCatalogueLookup::Earnable {
            revision: "r2".into(),
        };
        assert!(matches!(
            ordered_batch(&[first.clone(), other.clone()]),
            Err(AchievementGrantError::InvalidInput)
        ));
        other.source.kind = "oteryn:quest-transition".into();
        assert!(ordered_batch(&[first, other]).is_ok());
    }

    #[test]
    fn achievement_batch_order_uses_event_then_key_then_kind() {
        let make = |event, key, kind: &str| {
            let mut request = request(key, AchievementCatalogueLookup::Retired);
            request.source.event_id = vec![event];
            request.source.kind = kind.into();
            request
        };
        let requests = [
            make(2, "oteryn:achievement/a", "oteryn:counter-threshold"),
            make(1, "oteryn:achievement/b", "oteryn:quest-transition"),
            make(1, "oteryn:achievement/a", "oteryn:quest-transition"),
            make(1, "oteryn:achievement/a", "oteryn:counter-threshold"),
        ];
        assert_eq!(ordered_batch(&requests).ok(), Some(vec![3, 2, 1, 0]));
    }

    #[test]
    fn achievement_batch_checks_every_request_and_unknown_catalogue_entry() {
        let first = request("oteryn:achievement/a", AchievementCatalogueLookup::Retired);
        let mut other = request("oteryn:achievement/b", AchievementCatalogueLookup::Absent);
        assert!(matches!(
            ordered_batch(&[first.clone(), other.clone()]),
            Err(AchievementGrantError::UnknownAchievement)
        ));
        other.achievement_key = "invalid".into();
        assert!(matches!(
            ordered_batch(&[first, other]),
            Err(AchievementGrantError::InvalidInput)
        ));
    }
}

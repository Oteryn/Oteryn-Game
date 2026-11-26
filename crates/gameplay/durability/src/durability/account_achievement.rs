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
//! ([`AchievementCatalogueLookup`]), from the runtime Achievement catalogue;
//! the reward-claim MINT (step 4) takes the
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
use super::character_progression::{CharacterProgressionError, assert_item_only_fence, uuid_text};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::item_transfer::CurrentCharacterItemFence;
use super::reward_claim_mint::RewardClaimFenceChecked;
use crate::domain::CharacterId;
use crate::foundation::{
    ConnectionGeneration, CreatureDeathOccurrenceKey, GameSessionId, RuntimeScopeRefV1,
    ScopeOwnershipGeneration,
};
use sha2::{Digest, Sha256};
use sqlx::Row;

pub use oteryn_gameplay_progression::account_achievement::*;

/// Canonical catalogue key used by Crystal's snow movement script.
pub const SNOWBUNNY_KEY: &str = "oteryn:achievement/snowbunny";
/// Crystal's number of footprints required for Snowbunny.
pub const SNOWBUNNY_TOTAL: u32 = 10_000;

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

    // Also called by item_use::mount_taming immediately after the complete item fence.
    pub(super) async fn in_transaction(
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

/// Record one grant request and consume it into the account's fact, in the
/// granter's transaction. On `Err(UnknownAchievement)` (or any other error)
/// the granter must drop its transaction uncommitted.
pub async fn record_achievement_grant(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    granter: FencedGrantingCharacter,
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

    // The granter's own row lock (re-entrant here), only in the transaction
    // that checked the fence; the account comes from the locked root, never
    // from the caller.
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

    write_achievement_grant(tx, character, &account, request, revision).await
}

async fn write_achievement_grant(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character: &[u8],
    account: &str,
    request: &AchievementGrantRequest,
    revision: &str,
) -> Pass<AchievementGrantOutcome> {
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

/// The embedded catalogue's entry for `key` (`content/achievements`), parsed once.
fn embedded_lookup(key: &str) -> std::result::Result<AchievementCatalogueLookup, DurabilityError> {
    use oteryn_gameplay_progression::achievement_catalogue::{Entry, SHARDS, parse_shards};
    static CATALOGUE: std::sync::LazyLock<Option<std::collections::BTreeMap<String, Entry>>> =
        std::sync::LazyLock::new(|| parse_shards(&SHARDS).ok());
    let catalogue = CATALOGUE
        .as_ref()
        .ok_or(DurabilityError::InvalidStoredState)?;
    Ok(match catalogue.get(key) {
        None => AchievementCatalogueLookup::Absent,
        Some(entry) if entry.retired => AchievementCatalogueLookup::Retired,
        Some(entry) => AchievementCatalogueLookup::Earnable {
            revision: entry.revision.clone(),
        },
    })
}

/// Grant each of `keys` that the Character's account lacks, from one source event, in the
/// granter's transaction after its complete fence and `character_root` lock. A held key writes
/// nothing, so a counter that stays past its threshold does not grow the request log.
pub(super) async fn grant_missing(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character_id: CharacterId,
    keys: &[&str],
    source_kind: &str,
    event_id: &[u8],
) -> std::result::Result<(), DurabilityError> {
    if keys.is_empty() {
        return Ok(());
    }
    let held: Vec<String> = sqlx::query_scalar(
        "SELECT f.achievement_key FROM game_account_achievements f \
           JOIN game_character_roots r ON r.account_id = f.account_id \
          WHERE r.character_id = encode($1,'hex')::uuid AND f.achievement_key = ANY($2)",
    )
    .bind(character_id.as_bytes().as_slice())
    .bind(keys)
    .fetch_all(&mut **tx)
    .await?;
    for key in keys
        .iter()
        .filter(|key| !held.iter().any(|held| held == *key))
    {
        let request = AchievementGrantRequest {
            achievement_key: (*key).to_owned(),
            catalogue: embedded_lookup(key)?,
            source: AchievementSourceEvent {
                kind: source_kind.to_owned(),
                event_id: event_id.to_vec(),
            },
        };
        let granter = FencedGrantingCharacter::in_transaction(tx, character_id).await?;
        record_achievement_grant(tx, granter, &request)
            .await?
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    Ok(())
}

/// The exact damaging Character lease observed by the lethal owner.
#[derive(Debug, Clone, Copy)]
pub struct BossDeathAchievementRecipient {
    pub character_id: CharacterId,
    pub session: GameSessionId,
    pub lease_generation: u64,
}

/// The achievement producer selected by the native wild creature death.
#[derive(Debug, Clone, Copy)]
#[repr(i16)]
pub enum BossDeathAchievements {
    Oberon = 1,
    Scarlett = 2,
    MitmahVanguard = 3,
}

impl BossDeathAchievements {
    pub fn for_creature(creature: &str) -> Option<Self> {
        match creature {
            "oteryn:creature.grand_master_oberon" => Some(Self::Oberon),
            "oteryn:creature.scarlett_etzel" => Some(Self::Scarlett),
            "oteryn:creature.mitmah_vanguard" => Some(Self::MitmahVanguard),
            _ => None,
        }
    }

    fn awards(self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Oberon => ("oteryn:oberon-death", &[
                "oteryn:achievement/master_debater",
                "oteryn:achievement/millennial_falcon",
            ]),
            Self::Scarlett => ("oteryn:boss-death", &["oteryn:achievement/a_study_in_scarlett"]),
            Self::MitmahVanguard => ("oteryn:boss-death", &["oteryn:achievement/museum_goer"]),
        }
    }
}

/// The roster captured by the lethal owner and handed to the last reward writer.
#[derive(Debug, Clone)]
pub struct BossDeathAchievementWork {
    pub boss: BossDeathAchievements,
    pub recipients: Vec<BossDeathAchievementRecipient>,
}

impl BossDeathAchievementWork {
    /// Call inside the completing reward transaction, after proving the source scope.
    pub async fn retain(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        death: CreatureDeathOccurrenceKey,
    ) -> Result<(), DurabilityError> {
        let mut recipients = Vec::with_capacity(self.recipients.len());
        for recipient in &self.recipients {
            let character = sqlx::types::Uuid::from_bytes(*recipient.character_id.as_bytes());
            let session = sqlx::types::Uuid::from_bytes(*recipient.session.as_bytes());
            if let Some(account) = boss_earning_account(tx, *recipient).await? {
                recipients.push((
                    character.to_string(),
                    session.to_string(),
                    recipient.lease_generation,
                    account,
                ));
            }
        }
        sqlx::query(
            "INSERT INTO game_boss_death_achievement_pending(scope_key, death, generation, boss, recipients)
             VALUES($1, $2, $3::text::numeric, $4, $5) ON CONFLICT (death) DO NOTHING",
        )
        .bind(super::runtime_scope_assignment::assignment_scope_key(death.runtime_scope()).as_slice())
        .bind(death.native_bytes())
        .bind(death.scope_ownership_generation().get().to_string())
        .bind(self.boss as i16)
        .bind(sqlx::types::Json(recipients))
        .execute(&mut **tx)
        .await?;
        Ok(())
    }
}

async fn boss_earning_account(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    recipient: BossDeathAchievementRecipient,
) -> Result<Option<String>, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT account_id::text FROM game_durability_reconnect_sessions
         WHERE character_id=encode($1,'hex')::uuid AND game_session_id=encode($2,'hex')::uuid
           AND character_lease_generation=$3::text::numeric",
    )
    .bind(recipient.character_id.as_bytes().as_slice())
    .bind(recipient.session.as_bytes().as_slice())
    .bind(recipient.lease_generation.to_string())
    .fetch_optional(&mut **tx)
    .await?)
}

async fn qualify_boss_recipient(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &super::runtime_scope_assignment::NodeIncarnationProof,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
    recipient: BossDeathAchievementRecipient,
) -> Result<bool, DurabilityError> {
    // Reconnect may advance the connection generation while retaining the lease.
    let connection: Option<String> = sqlx::query_scalar(
        "SELECT s.current_generation::text FROM game_durability_reconnect_sessions s
         JOIN game_character_creation_receipts c USING (character_id)
         WHERE s.game_session_id = encode($1,'hex')::uuid AND s.session_state IN (1,2)
           AND NOT game_account_closure_detached(s.character_id) FOR SHARE OF s",
    )
    .bind(recipient.session.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    let Some(connection) = connection else { return Ok(false); };
    let connection = connection.parse().map_err(|_| DurabilityError::InvalidStoredState)?;
    let fence = CurrentCharacterItemFence {
        character_id: recipient.character_id,
        game_session_id: recipient.session,
        connection_generation: ConnectionGeneration::new(connection)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        character_lease_generation: recipient.lease_generation,
        runtime_scope: scope,
        scope_ownership_generation: generation,
    };
    match assert_item_only_fence(tx, &fence, node).await? {
        Ok(_) => Ok(true),
        Err(CharacterProgressionError::AuthorityRejected | CharacterProgressionError::ProgressionContextMismatch) => Ok(false),
        Err(CharacterProgressionError::Unavailable(error)) => Err(error),
        Err(_) => Err(DurabilityError::CommitRejected),
    }
}

type StoredBossRoster = sqlx::types::Json<Vec<(String, String, u64, String)>>;

struct RetainedBossRecipient {
    lease: BossDeathAchievementRecipient,
    account: String,
}

fn decode_boss_roster(row: &sqlx::postgres::PgRow) -> Result<Vec<RetainedBossRecipient>, DurabilityError> {
    row.try_get::<StoredBossRoster, _>("recipients")?.0.into_iter()
        .map(|(character, session, lease_generation, account)| {
            let character = sqlx::types::Uuid::parse_str(&character)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let session = sqlx::types::Uuid::parse_str(&session)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            Ok(RetainedBossRecipient {
                lease: BossDeathAchievementRecipient {
                    character_id: CharacterId::from_bytes(*character.as_bytes())
                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                    session: GameSessionId::decode(session.as_bytes())
                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                    lease_generation,
                },
                account,
            })
        })
        .collect()
}

async fn qualify_retained_boss_recipient(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    scope: RuntimeScopeRefV1,
    recipient: &RetainedBossRecipient,
) -> Result<bool, DurabilityError> {
    let lease = recipient.lease;
    // An owner replacement can retire the old lease. Voluntary logout is
    // independently recorded by its terminal transaction.
    let session: Option<i32> = sqlx::query_scalar(
        "SELECT 1 FROM game_durability_reconnect_sessions
         WHERE game_session_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid
           AND character_lease_generation=$3::text::numeric AND account_id=$4::uuid
           AND NOT boss_achievement_explicit_logout FOR SHARE",
    )
    .bind(lease.session.as_bytes().as_slice())
    .bind(lease.character_id.as_bytes().as_slice())
    .bind(lease.lease_generation.to_string())
    .bind(&recipient.account)
    .fetch_optional(&mut **tx)
    .await?;
    if session.is_none() {
        return Ok(false);
    }
    Ok(sqlx::query_scalar::<_, i32>(
        "SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid
         AND world_id=encode($2,'hex')::uuid AND lifecycle=1
         AND NOT game_account_closure_detached(character_id) FOR UPDATE",
    )
    .bind(lease.character_id.as_bytes().as_slice())
    .bind(scope.world_id().as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?
    .is_some())
}

async fn held_boss_achievements(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    account: &str,
    awards: &[&str],
) -> Result<Vec<String>, DurabilityError> {
    Ok(sqlx::query_scalar(
        "SELECT achievement_key FROM game_account_achievements
         WHERE account_id=$1::uuid AND achievement_key=ANY($2)",
    )
    .bind(account)
    .bind(awards)
    .fetch_all(&mut **tx)
    .await?)
}

async fn grant_retained_boss_rewards(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    recipient: RetainedBossRecipient,
    awards: &[&str],
    source: &str,
    death: &[u8],
) -> Result<(), DurabilityError> {
    let character = recipient.lease.character_id;
    let occurrence = Sha256::new().chain_update(character.as_bytes()).chain_update(death).finalize();
    let held = held_boss_achievements(tx, &recipient.account, awards).await?;
    for key in awards.iter().filter(|key| !held.iter().any(|held| held == *key)) {
        let catalogue = embedded_lookup(key)?;
        let AchievementCatalogueLookup::Earnable { revision } = &catalogue else {
            if catalogue == AchievementCatalogueLookup::Retired {
                continue;
            }
            return Err(DurabilityError::InvalidStoredState);
        };
        let request = AchievementGrantRequest {
            achievement_key: (*key).into(),
            catalogue: catalogue.clone(),
            source: AchievementSourceEvent {
                kind: source.into(),
                event_id: occurrence.to_vec(),
            },
        };
        write_achievement_grant(tx, character.as_bytes(), &recipient.account, &request, revision)
            .await?
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    }
    Ok(())
}

async fn finish_boss_achievement(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &super::runtime_scope_assignment::NodeIncarnationProof,
    scope: RuntimeScopeRefV1,
    live_generation: Option<ScopeOwnershipGeneration>,
    death: &[u8],
    boss: BossDeathAchievements,
    mut recipients: Vec<RetainedBossRecipient>,
) -> Result<(), DurabilityError> {
    let (source, awards) = boss.awards();
    recipients.sort_unstable_by_key(|recipient| *recipient.lease.character_id.as_bytes());
    let mut eligible = Vec::with_capacity(recipients.len());
    for recipient in recipients {
        let qualified = if let Some(generation) = live_generation {
            qualify_boss_recipient(tx, node, scope, generation, recipient.lease).await?
        } else {
            qualify_retained_boss_recipient(tx, scope, &recipient).await?
        };
        if qualified {
            eligible.push(recipient);
        }
    }
    for recipient in eligible {
        grant_retained_boss_rewards(tx, recipient, awards, source, death).await?;
    }
    // The earning-account guard must see the immutable roster before it is consumed.
    sqlx::query("SET CONSTRAINTS game_account_achievement_request_consumed IMMEDIATE")
        .execute(&mut **tx).await?;
    sqlx::query("DELETE FROM game_boss_death_achievement_pending WHERE death=$1")
        .bind(death).execute(&mut **tx).await?;
    Ok(())
}

impl DurabilityRoot {
    /// Grant the eligible death roster together and consume its durable work.
    pub async fn grant_boss_death_achievements(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &super::runtime_scope_assignment::NodeIncarnationProof,
        death: CreatureDeathOccurrenceKey,
        mut work: BossDeathAchievementWork,
    ) -> Result<(), DurabilityError> {
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            super::db::lock_admission_relations(&mut tx).await?;
            let scope = death.runtime_scope();
            let generation = death.scope_ownership_generation();
            if !super::character_progression::assert_runtime_scope_fence(
                &mut tx, scope, generation, &node,
            ).await? {
                return Err(DurabilityError::CommitRejected);
            }
            let row = sqlx::query(
                "SELECT recipients FROM game_boss_death_achievement_pending
                 WHERE death=$1 AND scope_key=$2 AND generation=$3::text::numeric AND boss=$4
                 FOR UPDATE",
            )
            .bind(death.native_bytes())
            .bind(super::runtime_scope_assignment::assignment_scope_key(scope).as_slice())
            .bind(generation.get().to_string())
            .bind(work.boss as i16)
            .fetch_optional(&mut *tx).await?;
            if let Some(row) = row {
                finish_boss_achievement(&mut tx, &node, scope, Some(generation),
                    &death.native_bytes(), work.boss, decode_boss_roster(&row)?).await?;
            } else {
                // A lost commit reply can leave no pending row. Every still-eligible
                // recipient must already hold the awards on the original earning account.
                let (_, awards) = work.boss.awards();
                work.recipients.sort_unstable_by_key(|recipient| *recipient.character_id.as_bytes());
                for recipient in work.recipients {
                    if !qualify_boss_recipient(&mut tx, &node, scope, generation, recipient).await? {
                        continue;
                    }
                    let account = boss_earning_account(&mut tx, recipient).await?
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    let held = held_boss_achievements(&mut tx, &account, awards).await?;
                    for key in awards.iter().filter(|key| !held.iter().any(|held| held == *key)) {
                        if embedded_lookup(key)? != AchievementCatalogueLookup::Retired {
                            return Err(DurabilityError::CommitRejected);
                        }
                    }
                }
            }
            commit_semantic_transaction(tx, deadline).await
        })).await
    }

    /// Reconcile one pending death before accepting new work on an empty runtime.
    /// True means the queue was empty. False leaves the caller's bounded drain active.
    pub async fn reconcile_boss_death_achievement(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &super::runtime_scope_assignment::NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
    ) -> Result<bool, DurabilityError> {
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let scope_key = super::runtime_scope_assignment::assignment_scope_key(scope);
            if !sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM game_boss_death_achievement_pending WHERE scope_key=$1)"
            ).bind(scope_key.as_slice()).fetch_one(&mut *tx).await? {
                tx.rollback().await?;
                return Ok(true);
            }
            assert_recovery_fence(&mut tx, &recovery).await?;
            super::db::lock_admission_relations(&mut tx).await?;
            if !super::character_progression::assert_runtime_scope_fence(&mut tx, scope, generation, &node).await? {
                return Err(DurabilityError::CommitRejected);
            }
            let row = sqlx::query(
                "SELECT death, generation::text, boss, recipients FROM game_boss_death_achievement_pending
                 WHERE scope_key=$1 AND generation <= $2::text::numeric ORDER BY death LIMIT 1 FOR UPDATE",
            ).bind(scope_key.as_slice()).bind(generation.get().to_string()).fetch_optional(&mut *tx).await?;
            let Some(row) = row else {
                tx.rollback().await?;
                return Ok(true);
            };
            let death: Vec<u8> = row.try_get("death")?;
            let prior_generation: u64 = row.try_get::<String,_>("generation")?.parse()
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            let boss = match row.try_get::<i16, _>("boss")? {
                1 => BossDeathAchievements::Oberon,
                2 => BossDeathAchievements::Scarlett,
                3 => BossDeathAchievements::MitmahVanguard,
                _ => return Err(DurabilityError::InvalidStoredState),
            };
            finish_boss_achievement(&mut tx, &node, scope, (prior_generation == generation.get()).then_some(generation),
                &death, boss, decode_boss_roster(&row)?).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(false)
        })).await
    }
}

/// Count one event toward `key` for the Character and grant it when the count reaches `total`.
/// A count at its total stays there and writes nothing (Crystal `register_achievements.lua:696`).
pub(super) async fn add_progress(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character_id: CharacterId,
    key: &str,
    total: u32,
    source_kind: &str,
    event_id: &[u8],
) -> std::result::Result<(), DurabilityError> {
    let progress: Option<i64> = sqlx::query_scalar(
        "INSERT INTO game_character_achievement_progress AS p \
                (character_id, achievement_key, progress) \
         VALUES (encode($1,'hex')::uuid, $2, 1) \
         ON CONFLICT (character_id, achievement_key) \
         DO UPDATE SET progress = p.progress + 1 WHERE p.progress < $3 \
         RETURNING progress",
    )
    .bind(character_id.as_bytes().as_slice())
    .bind(key)
    .bind(i64::from(total))
    .fetch_optional(&mut **tx)
    .await?;
    if progress == Some(i64::from(total)) {
        grant_missing(tx, character_id, &[key], source_kind, event_id).await?;
    }
    Ok(())
}

/// Crystal `data/scripts/movements/snow.lua:18-20`: one committed footprint counts toward
/// Snowbunny, whose donor threshold is 10,000.
pub async fn add_snowbunny_progress(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    character_id: CharacterId,
    event_id: &[u8],
) -> std::result::Result<(), DurabilityError> {
    add_progress(
        tx,
        character_id,
        SNOWBUNNY_KEY,
        SNOWBUNNY_TOTAL,
        "oteryn:snow-footprint",
        event_id,
    )
    .await
}

/// Count and commit one already-authorized Snowbunny footprint transaction.
///
/// The caller passes the transaction which already contains the durable step authority check;
/// keeping the progress write here, before the sole commit, is intentional.  A failure of either
/// half therefore rolls the entire footprint back rather than leaving a moved step with a
/// separately committed counter.
pub async fn commit_snowbunny_progress(
    mut tx: sqlx::Transaction<'_, sqlx::Postgres>,
    character_id: CharacterId,
    event_id: &[u8],
    deadline: tokio::time::Instant,
) -> std::result::Result<(), DurabilityError> {
    add_snowbunny_progress(&mut tx, character_id, event_id).await?;
    commit_semantic_transaction(tx, deadline).await
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

/// Test-only granting transaction: the XP writer's complete gameplay fence,
/// then each request in order, committed together or not at all. In
/// production the reward-claim MINT is the granter (contract §5 step 4).
#[cfg(any(test, feature = "test-support"))]
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
        pub async fn commit_test_achievement_grants(
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
                        let mut outcomes = Vec::with_capacity(requests.len());
                        for request in &requests {
                            let granter = FencedGrantingCharacter::in_transaction(
                                &mut tx,
                                fence.character_id,
                            )
                            .await?;
                            match record_achievement_grant(&mut tx, granter, request).await? {
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

        /// Negative only: a token minted after the complete fence in one
        /// committed transaction, presented in a second transaction under the
        /// same complete fence, which commits whatever the grant wrote.
        pub async fn commit_test_achievement_grant_with_foreign_token(
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
    #[test]
    fn snowbunny_uses_crystals_snow_footprint_threshold() {
        // Crystal data/scripts/movements/snow.lua:18-20.
        assert_eq!(super::SNOWBUNNY_KEY, "oteryn:achievement/snowbunny");
        assert_eq!(super::SNOWBUNNY_TOTAL, 10_000);
    }
    /// The PostgreSQL targets drive the negative harness.
    #[test]
    fn the_foreign_token_harness_is_linked() {
        let _ = crate::durability::DurabilityRoot::commit_test_achievement_grant_with_foreign_token;
    }

    /// The PostgreSQL targets drive the display read; the binaries that
    /// recompile durability without it still link it.
    #[test]
    fn the_display_read_is_linked() {
        let _ = crate::durability::DurabilityRoot::read_account_achievements;
    }
}

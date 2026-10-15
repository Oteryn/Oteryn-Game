//! Native GUILD0 membership/invitation owner preparation, UNREGISTERED and UNINSTALLED.
//!
//! Found uses the genuine native Premium gate; activation/jobs, disband, Guild Bank/Hall, wire
//! and runtime installation remain outside this private component.
//! The private schema and event-data proposal require CP admission. Native recovery/session/
//! node guards remain the actual current-authority source; retained operations are outcomes.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{assert_session_fence, numeric_u64, uuid_text};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::guild_audit::GuildAuditProposal;
use super::item_mint_audit::check_uuid_v7;
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId;
use crate::domain::guild::{
    DEFAULT_RANKS, FORMATION_LIFETIME_MS, GUILD_INVITATIONS_MAX, GuildId, GuildName, GuildRank,
    GuildState, INVITATION_LIFETIME_MS, MEMBERS_MAX, TARGET_INVITATIONS_MAX,
};
use crate::domain::premium::PremiumStatus;
use crate::foundation::CommandRef;
use crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator;
use sha2::{Digest, Sha256};
use sqlx::{Acquire, Postgres, Row, Transaction};

type Result<T> = std::result::Result<T, GuildError>;

#[derive(Debug)]
pub enum GuildError {
    InvalidInput,
    NotCurrent,
    ConflictingOccurrence,
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for GuildError {
    fn from(e: DurabilityError) -> Self {
        Self::Unavailable(e)
    }
}
impl From<sqlx::Error> for GuildError {
    fn from(e: sqlx::Error) -> Self {
        Self::Unavailable(e.into())
    }
}
impl std::fmt::Display for GuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "guild: {self:?}")
    }
}
impl std::error::Error for GuildError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildCommand {
    Invite(CharacterId),
    Revoke(CharacterId),
    Accept,
    Leave,
    Exclude(CharacterId),
    /// Only commit_guild_found may execute this creation action.
    Found,
}
impl GuildCommand {
    fn code(self) -> i16 {
        match self {
            Self::Invite(_) => 1,
            Self::Revoke(_) => 2,
            Self::Accept => 3,
            Self::Leave => 4,
            Self::Exclude(_) => 5,
            Self::Found => 6,
        }
    }
    fn target(self, actor: CharacterId) -> CharacterId {
        match self {
            Self::Invite(c) | Self::Revoke(c) | Self::Exclude(c) => c,
            Self::Accept | Self::Leave | Self::Found => actor,
        }
    }
}

/// Fixed once before the first attempt; a retry changes neither IDs nor command bytes.
#[derive(Debug, Clone)]
pub struct GuildOperationIdentity {
    pub occurrence: [u8; 16],
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
}
#[derive(Debug, Clone)]
pub struct GuildRequest {
    pub identity: GuildOperationIdentity,
    pub command: CommandRef,
    pub guild_id: GuildId,
    pub operation: GuildCommand,
}

/// Game-issued identity and a name already checked by the native 0022 grammar. Neither these
/// immutable request bytes nor a persisted operation establish Premium or current authority.
#[derive(Debug, Clone)]
pub struct GuildFoundRequest {
    pub identity: GuildOperationIdentity,
    pub command: CommandRef,
    pub guild_id: GuildId,
    pub name: GuildName,
}

impl GuildFoundRequest {
    fn operation(&self) -> GuildRequest {
        GuildRequest {
            identity: self.identity.clone(),
            command: self.command,
            guild_id: self.guild_id,
            operation: GuildCommand::Found,
        }
    }
}

/// Policy outcomes under actual locked facts. No caller revision is required for these five
/// operations: GUILD0 §12 reserves expected-revision preconditions for edits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildResult {
    Ok,
    NotAllowed,
    AlreadyInGuild,
    GuildFull,
    NotInvited,
    InvitationLimit,
    GuildDisbanding,
    GuildDeadlinePassed,
    NameTaken,
    AccountHasPosition,
    NotPremium,
    Junior,
}
impl GuildResult {
    fn code(self) -> i16 {
        match self {
            Self::Ok => 0,
            Self::NotAllowed => 1,
            Self::AlreadyInGuild => 2,
            Self::GuildFull => 3,
            Self::NotInvited => 4,
            Self::InvitationLimit => 5,
            Self::GuildDisbanding => 6,
            Self::GuildDeadlinePassed => 7,
            Self::NameTaken => 8,
            Self::AccountHasPosition => 9,
            Self::NotPremium => 10,
            Self::Junior => 11,
        }
    }
    fn from_code(code: i16) -> Option<Self> {
        Some(match code {
            0 => Self::Ok,
            1 => Self::NotAllowed,
            2 => Self::AlreadyInGuild,
            3 => Self::GuildFull,
            4 => Self::NotInvited,
            5 => Self::InvitationLimit,
            6 => Self::GuildDisbanding,
            7 => Self::GuildDeadlinePassed,
            8 => Self::NameTaken,
            9 => Self::AccountHasPosition,
            10 => Self::NotPremium,
            11 => Self::Junior,
            _ => return None,
        })
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuildReceipt {
    pub result: GuildResult,
    pub previous_revision: Option<u64>,
    pub revision: Option<u64>,
    pub transaction_id: [u8; 16],
    pub event_id: Option<[u8; 16]>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuildOutcome {
    Committed(GuildReceipt),
    Replayed(GuildReceipt),
}

fn invalid() -> GuildError {
    DurabilityError::InvalidStoredState.into()
}

fn binding(fence: &CurrentCharacterItemFence, request: &GuildRequest) -> Result<Vec<u8>> {
    let ids = [
        &request.identity.occurrence,
        &request.identity.transaction_id,
        &request.identity.event_id,
    ];
    for (n, id) in ids.iter().enumerate() {
        check_uuid_v7(id.as_slice()).map_err(|_| GuildError::InvalidInput)?;
        if ids[..n].contains(id) {
            return Err(GuildError::InvalidInput);
        }
    }
    if request.identity.occurred_at_unix_ms <= 0
        || request.command.game_session_id() != fence.game_session_id
    {
        return Err(GuildError::InvalidInput);
    }
    let mut h = Sha256::new();
    h.update(b"oteryn:guild-membership-proposal:v1");
    h.update(fence.character_id.as_bytes());
    h.update(fence.runtime_scope.world_id().as_bytes());
    h.update(request.command.game_session_id().as_bytes());
    h.update(request.command.command_id().get().to_be_bytes());
    h.update(request.guild_id.as_bytes());
    h.update(request.operation.code().to_be_bytes());
    h.update(request.operation.target(fence.character_id).as_bytes());
    for id in ids {
        h.update(id);
    }
    h.update(request.identity.occurred_at_unix_ms.to_be_bytes());
    Ok(h.finalize().to_vec())
}

impl DurabilityRoot {
    /// Actual Found entry. The Account selector is read under the native session fence, never
    /// accepted from the caller. A terminal receipt is read before asking the Premium owner.
    /// NotActivated writes in the gate's physical transaction; it never takes a second holder.
    pub(crate) async fn commit_guild_found(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: GuildFoundRequest,
        premium: &SpellPremiumCoordinator,
    ) -> Result<GuildOutcome> {
        let common = request.operation();
        let mut digest = Sha256::new();
        digest.update(b"oteryn:guild-found-proposal:v1");
        digest.update(binding(&fence, &common)?);
        digest.update(request.name.as_str().as_bytes());
        let binding = digest.finalize().to_vec();
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let pre_recovery = recovery.clone();
        let pre_node = node.clone();
        let pre_request = common.clone();
        let pre_binding = binding.clone();
        let preflight = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &pre_recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let result = async {
                        lock_operation(&mut tx, &pre_request).await?;
                        if let Some(outcome) = replay(&mut tx, &pre_request, &pre_binding).await? {
                            return Ok(FoundPreflight::Replayed(outcome));
                        }
                        let account = assert_session_fence(&mut tx, &fence, &pre_node)
                            .await?
                            .map_err(|_| GuildError::NotCurrent)?;
                        Ok(FoundPreflight::Fresh(uuid_text(&account)?))
                    }
                    .await;
                    match result {
                        Ok(result) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(result))
                        }
                        Err(GuildError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await??;
        let account = match preflight {
            FoundPreflight::Replayed(outcome) => return Ok(outcome),
            FoundPreflight::Fresh(account) => account,
        };
        let root = self.clone();
        let write = FoundWrite {
            node: node.clone(),
            fence,
            common,
            name: request.name,
            binding,
            account,
        };
        premium
            .with_premium_gate(account, move |status, tx| {
                Box::pin(async move {
                    match tx {
                        Some(tx) => {
                            if !matches!(&status, PremiumStatus::NotActivated(_)) {
                                return Err(invalid());
                            }
                            // The gate commits whatever the callback returns. The savepoint contains
                            // every Guild write, so a late rejection cannot leak partial creation.
                            let mut savepoint = tx.begin().await?;
                            let result = async {
                                assert_recovery_fence(&mut savepoint, &recovery).await?;
                                lock_admission_relations(&mut savepoint).await?;
                                write.execute(&mut savepoint, &status).await
                            }
                            .await;
                            match result {
                                Ok(outcome) => {
                                    savepoint.commit().await?;
                                    Ok(outcome)
                                }
                                Err(error) => {
                                    savepoint.rollback().await?;
                                    Err(error)
                                }
                            }
                        }
                        None => {
                            // Only the real gate chooses these delivered statuses. Never let a
                            // borrowed pre-delivery witness escape into a second transaction.
                            let status: PremiumStatus<'static> = match status {
                                PremiumStatus::Current => PremiumStatus::Current,
                                PremiumStatus::NotCurrent => PremiumStatus::NotCurrent,
                                PremiumStatus::NotActivated(_) => return Err(invalid()),
                            };
                            root.try_issue_semantic_pass()?
                                .run(move |holder, deadline| {
                                    Box::pin(async move {
                                        let mut tx =
                                            begin_semantic_transaction(holder, deadline).await?;
                                        assert_recovery_fence(&mut tx, &recovery).await?;
                                        lock_admission_relations(&mut tx).await?;
                                        let result = write.execute(&mut tx, &status).await;
                                        match result {
                                            Ok(outcome) => {
                                                commit_semantic_transaction(tx, deadline).await?;
                                                Ok(Ok(outcome))
                                            }
                                            Err(GuildError::Unavailable(error)) => Err(error),
                                            Err(error) => Ok(Err(error)),
                                        }
                                    })
                                })
                                .await?
                        }
                    }
                })
            })
            .await?
    }

    /// One bounded native pass; success/refusal receipt and a successful mutation's one candidate
    /// event commit together. A historical receipt is replayed first, never current authority.
    pub async fn commit_guild_membership(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: GuildRequest,
    ) -> Result<GuildOutcome> {
        if request.operation == GuildCommand::Found {
            return Err(GuildError::InvalidInput);
        }
        let binding = binding(&fence, &request)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            // Native relation locks precede GUILD0-LO-01. No Guild row is locked first.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:guild-command:' || encode($1,'hex') || ':' || $2,0))")
                .bind(request.command.game_session_id().as_bytes().as_slice())
                .bind(request.command.command_id().get().to_string()).execute(&mut *tx).await?;
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:guild-operation:' || encode($1,'hex'),0))")
                .bind(request.identity.occurrence.as_slice()).execute(&mut *tx).await?;
            let result = execute(&mut tx, &node, &fence, &request, &binding).await;
            match result {
                Ok(result) => { commit_semantic_transaction(tx, deadline).await?; Ok(Ok(result)) },
                Err(GuildError::Unavailable(e)) => Err(e),
                Err(error) => Ok(Err(error)), // Dropping the transaction rolls back every effect.
            }
        })).await?
    }
}

enum FoundPreflight {
    Replayed(GuildOutcome),
    Fresh([u8; 16]),
}

struct FoundWrite {
    node: NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    common: GuildRequest,
    name: GuildName,
    binding: Vec<u8>,
    account: [u8; 16],
}

impl FoundWrite {
    async fn execute(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        premium: &PremiumStatus<'_>,
    ) -> Result<GuildOutcome> {
        lock_operation(tx, &self.common).await?;
        if let Some(outcome) = replay(tx, &self.common, &self.binding).await? {
            return Ok(outcome);
        }
        let account = assert_session_fence(tx, &self.fence, &self.node)
            .await?
            .map_err(|_| GuildError::NotCurrent)?;
        if uuid_text(&account)? != self.account {
            return Err(GuildError::NotCurrent);
        }
        let actor = self.fence.character_id;
        let root = sqlx::query("SELECT account_id::text,world_id::text,lifecycle FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(actor.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        let Some(root) = root else {
            return Err(GuildError::NotCurrent);
        };
        if root.try_get::<i16, _>("lifecycle")? != 1
            || uuid_text(&root.try_get::<String, _>("account_id")?)? != self.account
            || uuid_text(&root.try_get::<String, _>("world_id")?)?
                != *self.fence.runtime_scope.world_id().as_bytes()
        {
            return Err(GuildError::NotCurrent);
        }
        // Guild namespace reservation is at the Guild class, after the actor root. The native
        // unique index is still the final invariant; no Character-name reservation is taken.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:guild-name:' || encode($1,'hex') || ':' || $2,0))")
            .bind(self.fence.runtime_scope.world_id().as_bytes().as_slice()).bind(self.name.comparison_key()).execute(&mut **tx).await?;
        let existing: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid)",
        )
        .bind(self.common.guild_id.as_bytes().as_slice())
        .fetch_one(&mut **tx)
        .await?;
        if existing {
            return Err(GuildError::ConflictingOccurrence);
        }
        let name_taken:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guilds WHERE world_id=encode($1,'hex')::uuid AND name_key=$2 AND state<>4)")
            .bind(self.fence.runtime_scope.world_id().as_bytes().as_slice()).bind(self.name.comparison_key()).fetch_one(&mut **tx).await?;
        let member:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid)")
            .bind(actor.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        // Lock an existing native Account guard to serialize the absent global leadership row.
        // This is not a new Account authority or a Guild-local substitute for that source.
        let guard:Option<i32>=sqlx::query_scalar("SELECT 1 FROM game_character_account_guards WHERE account_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(self.account.as_slice()).fetch_optional(&mut **tx).await?;
        if guard.is_none() {
            return Err(GuildError::NotCurrent);
        }
        let position:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_account_guild_leadership WHERE account_id=encode($1,'hex')::uuid)")
            .bind(self.account.as_slice()).fetch_one(&mut **tx).await?;
        let result = if member {
            GuildResult::AlreadyInGuild
        } else if super::bank::fee_payer_is_junior(actor) {
            GuildResult::Junior
        } else if position {
            GuildResult::AccountHasPosition
        } else if name_taken {
            GuildResult::NameTaken
        } else if matches!(premium, PremiumStatus::NotCurrent) {
            GuildResult::NotPremium
        } else {
            GuildResult::Ok
        };
        if result != GuildResult::Ok {
            return store_operation(
                tx,
                &self.fence,
                &self.common,
                &self.binding,
                result,
                OperationPayload {
                    revisions: (None, None),
                    audit: None,
                    found_name: Some(self.name.as_str()),
                },
            )
            .await;
        }
        let now: i64 = sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM transaction_timestamp())*1000)::bigint",
        )
        .fetch_one(&mut **tx)
        .await?;
        let audit = GuildAuditProposal {
            version: 1,
            occurrence: self.common.identity.occurrence,
            transaction_id: self.common.identity.transaction_id,
            event_id: self.common.identity.event_id,
            guild_id: *self.common.guild_id.as_bytes(),
            world_id: *self.fence.runtime_scope.world_id().as_bytes(),
            actor: *actor.as_bytes(),
            target: *actor.as_bytes(),
            action: 6,
            previous_revision: 0,
            revision: 1,
            rank_before: None,
            rank_after: Some(1),
            removed_invitations: 0,
            occurred_at_unix_ms: self.common.identity.occurred_at_unix_ms,
        }
        .checked_bytes()
        .ok_or_else(invalid)?;
        let outcome = store_operation(
            tx,
            &self.fence,
            &self.common,
            &self.binding,
            GuildResult::Ok,
            OperationPayload {
                revisions: (Some(0), Some(1)),
                audit: Some(&audit),
                found_name: Some(self.name.as_str()),
            },
        )
        .await?;
        sqlx::query("INSERT INTO game_guilds(guild_id,world_id,name,state,formation_deadline,revision,created_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,1,$4,1,$5)")
            .bind(self.common.guild_id.as_bytes().as_slice()).bind(self.fence.runtime_scope.world_id().as_bytes().as_slice())
            .bind(self.name.as_str()).bind(now.checked_add(FORMATION_LIFETIME_MS).ok_or_else(invalid)?).bind(now).execute(&mut **tx).await?;
        for (level, name) in DEFAULT_RANKS {
            sqlx::query("INSERT INTO game_guild_ranks(guild_id,level,name) VALUES(encode($1,'hex')::uuid,$2,$3)")
                .bind(self.common.guild_id.as_bytes().as_slice()).bind(i16::from(level)).bind(name).execute(&mut **tx).await?;
        }
        sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,title,joined_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,'',$3)")
            .bind(actor.as_bytes().as_slice()).bind(self.common.guild_id.as_bytes().as_slice()).bind(now).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_account_guild_leadership(account_id,guild_id,character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
            .bind(self.account.as_slice()).bind(self.common.guild_id.as_bytes().as_slice()).bind(actor.as_bytes().as_slice()).execute(&mut **tx).await?;
        Ok(outcome)
    }
}

async fn lock_operation(tx: &mut Transaction<'_, Postgres>, request: &GuildRequest) -> Result<()> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:guild-command:' || encode($1,'hex') || ':' || $2,0))")
        .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).execute(&mut **tx).await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:guild-operation:' || encode($1,'hex'),0))")
        .bind(request.identity.occurrence.as_slice()).execute(&mut **tx).await?;
    Ok(())
}

async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    request: &GuildRequest,
    binding: &[u8],
) -> Result<Option<GuildOutcome>> {
    if let Some(row)=sqlx::query("SELECT request_binding,result,previous_revision::text,revision::text,transaction_id::text,event_id::text FROM game_guild_operations WHERE occurrence=encode($1,'hex')::uuid")
        .bind(request.identity.occurrence.as_slice()).fetch_optional(&mut **tx).await? {
        if row.try_get::<Vec<u8>,_>("request_binding")? != binding { return Err(GuildError::ConflictingOccurrence); }
        return Ok(Some(GuildOutcome::Replayed(receipt(&row)?)));
    }
    let used:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guild_operations WHERE (game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric) OR transaction_id=encode($3,'hex')::uuid OR requested_event_id=encode($4,'hex')::uuid)")
        .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .bind(request.identity.transaction_id.as_slice()).bind(request.identity.event_id.as_slice()).fetch_one(&mut **tx).await?;
    if used {
        return Err(GuildError::ConflictingOccurrence);
    }
    Ok(None)
}

#[derive(Clone)]
struct Member {
    guild: [u8; 16],
    rank: u8,
}

fn member_in_guild<'a>(member: &'a Option<Member>, guild: &GuildId) -> Option<&'a Member> {
    member.as_ref().filter(|m| m.guild == *guild.as_bytes())
}

async fn execute(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    request: &GuildRequest,
    binding: &[u8],
) -> Result<GuildOutcome> {
    if let Some(row) = sqlx::query("SELECT request_binding,result,previous_revision::text,revision::text,transaction_id::text,event_id::text FROM game_guild_operations WHERE occurrence=encode($1,'hex')::uuid")
        .bind(request.identity.occurrence.as_slice()).fetch_optional(&mut **tx).await? {
        if row.try_get::<Vec<u8>,_>("request_binding")? != binding { return Err(GuildError::ConflictingOccurrence); }
        return Ok(GuildOutcome::Replayed(receipt(&row)?));
    }
    let used: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guild_operations WHERE (game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric) OR transaction_id=encode($3,'hex')::uuid OR requested_event_id=encode($4,'hex')::uuid)")
        .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .bind(request.identity.transaction_id.as_slice()).bind(request.identity.event_id.as_slice()).fetch_one(&mut **tx).await?;
    if used {
        return Err(GuildError::ConflictingOccurrence);
    }
    let account = match assert_session_fence(tx, fence, node).await? {
        Ok(account) => account,
        Err(_) => return Err(GuildError::NotCurrent),
    };
    let actor = fence.character_id;
    let target = request.operation.target(actor);
    let mut roots = vec![actor];
    if matches!(request.operation, GuildCommand::Invite(_)) && target != actor {
        roots.push(target);
    }
    roots.sort();
    let mut target_exists = true;
    for character in roots {
        let root = sqlx::query("SELECT account_id::text,world_id::text,lifecycle FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(character.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        let valid = root.as_ref().is_some_and(|r| {
            r.try_get::<i16, _>("lifecycle").ok() == Some(1)
                && r.try_get::<String, _>("world_id")
                    .ok()
                    .and_then(|s| uuid_text(&s).ok())
                    == Some(*fence.runtime_scope.world_id().as_bytes())
        });
        if character == actor {
            if !valid
                || root
                    .as_ref()
                    .and_then(|r| r.try_get::<String, _>("account_id").ok())
                    .as_ref()
                    != Some(&account)
            {
                return Err(GuildError::NotCurrent);
            }
        } else {
            target_exists = valid;
        }
    }
    let guild = sqlx::query("SELECT world_id::text,state,revision::text,formation_deadline,vice_deficit_since FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(request.guild_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(guild) = guild else {
        return store(
            tx,
            fence,
            request,
            binding,
            GuildResult::NotAllowed,
            (None, None),
            None,
        )
        .await;
    };
    let previous = numeric_u64(&guild, "revision")?;
    let state = match guild.try_get::<i16, _>("state")? {
        1 => GuildState::Forming,
        2 => GuildState::Active,
        3 => GuildState::Disbanding,
        4 => GuildState::Disbanded,
        _ => return Err(invalid()),
    };
    if uuid_text(&guild.try_get::<String, _>("world_id")?)?
        != *fence.runtime_scope.world_id().as_bytes()
    {
        return store(
            tx,
            fence,
            request,
            binding,
            GuildResult::NotAllowed,
            (Some(previous), None),
            None,
        )
        .await;
    }
    if state.ensure_command_allowed().is_err() {
        return store(
            tx,
            fence,
            request,
            binding,
            GuildResult::GuildDisbanding,
            (Some(previous), None),
            None,
        )
        .await;
    }
    // Other-Guild membership never authorizes this operation. Accept reads that exclusion
    // predicate under the actor root/admission fence; only this locked Guild's rows are locked.
    let already_member = if request.operation == GuildCommand::Accept {
        sqlx::query_scalar::<_,bool>("SELECT EXISTS(SELECT 1 FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid)")
            .bind(actor.as_bytes().as_slice()).fetch_one(&mut **tx).await?
    } else {
        false
    };
    let member_target = if matches!(request.operation, GuildCommand::Revoke(_)) {
        actor
    } else {
        target
    };
    let members = sqlx::query("SELECT character_id::text,guild_id::text,rank_level FROM game_guild_members WHERE guild_id=encode($3,'hex')::uuid AND character_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid) ORDER BY character_id FOR UPDATE")
        .bind(actor.as_bytes().as_slice()).bind(member_target.as_bytes().as_slice()).bind(request.guild_id.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
    let member = |id: CharacterId| -> Result<Option<Member>> {
        for row in &members {
            if uuid_text(&row.try_get::<String, _>("character_id")?)? == *id.as_bytes() {
                let rank =
                    u8::try_from(row.try_get::<i16, _>("rank_level")?).map_err(|_| invalid())?;
                GuildRank::new(rank).map_err(|_| invalid())?;
                return Ok(Some(Member {
                    guild: uuid_text(&row.try_get::<String, _>("guild_id")?)?,
                    rank,
                }));
            }
        }
        Ok(None)
    };
    let acting = member(actor)?;
    let subject = member(target)?;
    let can_invite = member_in_guild(&acting, &request.guild_id).is_some_and(|m| m.rank <= 2);
    let now: i64 = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM transaction_timestamp())*1000)::bigint",
    )
    .fetch_one(&mut **tx)
    .await?;
    let invitations = if matches!(
        request.operation,
        GuildCommand::Invite(_) | GuildCommand::Accept
    ) {
        sqlx::query("SELECT guild_id::text,expires_at FROM game_guild_invitations WHERE character_id=encode($1,'hex')::uuid ORDER BY guild_id LIMIT 51 FOR UPDATE")
            .bind(target.as_bytes().as_slice()).fetch_all(&mut **tx).await?
    } else if matches!(request.operation, GuildCommand::Revoke(_)) {
        sqlx::query("SELECT guild_id::text,expires_at FROM game_guild_invitations WHERE guild_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid FOR UPDATE")
            .bind(request.guild_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).fetch_all(&mut **tx).await?
    } else {
        Vec::new()
    };
    if invitations.len() > TARGET_INVITATIONS_MAX {
        return Err(invalid());
    }
    let mut expired = Vec::new();
    let mut own_invitation = None;
    for row in &invitations {
        let id = uuid_text(&row.try_get::<String, _>("guild_id")?)?;
        let expiry: i64 = row.try_get("expires_at")?;
        if expiry <= now {
            expired.push(id);
        }
        if id == *request.guild_id.as_bytes() {
            own_invitation = Some(expiry);
        }
    }
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_guild_members WHERE guild_id=encode($1,'hex')::uuid",
    )
    .bind(request.guild_id.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    let vices: i64 = sqlx::query_scalar("SELECT count(*) FROM game_guild_members WHERE guild_id=encode($1,'hex')::uuid AND rank_level=2")
        .bind(request.guild_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let mut rank_before = None;
    let mut rank_after = None;
    let mut removes = Vec::new();
    let result = match request.operation {
        GuildCommand::Invite(_) => {
            if !can_invite
                || !target_exists
                || member_in_guild(&subject, &request.guild_id).is_some()
                || own_invitation.is_some_and(|e| e > now)
            {
                GuildResult::NotAllowed
            } else {
                let guild_count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_guild_invitations WHERE guild_id=encode($1,'hex')::uuid")
                    .bind(request.guild_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                if guild_count - i64::from(own_invitation.is_some()) >= GUILD_INVITATIONS_MAX as i64
                    || invitations.len() - expired.len() >= TARGET_INVITATIONS_MAX
                {
                    GuildResult::InvitationLimit
                } else {
                    removes = expired;
                    GuildResult::Ok
                }
            }
        }
        GuildCommand::Revoke(_) => {
            if !can_invite {
                GuildResult::NotAllowed
            } else if own_invitation.is_none() {
                GuildResult::NotInvited
            } else {
                removes.push(*request.guild_id.as_bytes());
                GuildResult::Ok
            }
        }
        GuildCommand::Accept => {
            if already_member {
                GuildResult::AlreadyInGuild
            } else if own_invitation.is_none_or(|e| e <= now) {
                GuildResult::NotInvited
            } else if count >= MEMBERS_MAX as i64 {
                GuildResult::GuildFull
            } else {
                let rank: i16 = sqlx::query_scalar(
                    "SELECT max(level) FROM game_guild_ranks WHERE guild_id=encode($1,'hex')::uuid",
                )
                .bind(request.guild_id.as_bytes().as_slice())
                .fetch_one(&mut **tx)
                .await?;
                let rank = u8::try_from(rank).map_err(|_| invalid())?;
                if !(3..=20).contains(&rank) {
                    return Err(invalid());
                }
                rank_after = Some(rank);
                for row in &invitations {
                    removes.push(uuid_text(&row.try_get::<String, _>("guild_id")?)?);
                }
                GuildResult::Ok
            }
        }
        GuildCommand::Found => return Err(GuildError::InvalidInput),
        GuildCommand::Leave => match member_in_guild(&acting, &request.guild_id) {
            Some(m) if m.rank > 1 => {
                rank_before = Some(m.rank);
                GuildResult::Ok
            }
            _ => GuildResult::NotAllowed,
        },
        GuildCommand::Exclude(_) => match (
            member_in_guild(&acting, &request.guild_id),
            member_in_guild(&subject, &request.guild_id),
        ) {
            (Some(a), Some(t))
                if GuildRank::new(a.rank)
                    .map(|r| r.may_exclude(GuildRank::new(t.rank).unwrap_or(GuildRank::LEADER)))
                    .unwrap_or(false) =>
            {
                rank_before = Some(t.rank);
                GuildResult::Ok
            }
            _ => GuildResult::NotAllowed,
        },
    };
    if result != GuildResult::Ok {
        return store(
            tx,
            fence,
            request,
            binding,
            result,
            (Some(previous), None),
            None,
        )
        .await;
    }
    let proposed_vices = vices - i64::from(rank_before == Some(2));
    if matches!(
        request.operation,
        GuildCommand::Accept | GuildCommand::Leave | GuildCommand::Exclude(_)
    ) && state
        .ensure_vice_count_change_allowed(
            usize::try_from(proposed_vices).map_err(|_| invalid())?,
            guild.try_get("formation_deadline")?,
            guild.try_get("vice_deficit_since")?,
            now,
        )
        .is_err()
    {
        return store(
            tx,
            fence,
            request,
            binding,
            GuildResult::GuildDeadlinePassed,
            (Some(previous), None),
            None,
        )
        .await;
    }
    let next = previous.checked_add(1).ok_or_else(invalid)?;
    let audit = GuildAuditProposal {
        version: 1,
        occurrence: request.identity.occurrence,
        transaction_id: request.identity.transaction_id,
        event_id: request.identity.event_id,
        guild_id: *request.guild_id.as_bytes(),
        world_id: *fence.runtime_scope.world_id().as_bytes(),
        actor: *actor.as_bytes(),
        target: *target.as_bytes(),
        action: request.operation.code(),
        previous_revision: previous,
        revision: next,
        rank_before,
        rank_after,
        removed_invitations: u8::try_from(removes.len()).map_err(|_| invalid())?,
        occurred_at_unix_ms: request.identity.occurred_at_unix_ms,
    };
    let audit_bytes = audit.checked_bytes().ok_or_else(invalid)?;
    // Insert the immutable operation before its closed row transitions; deferred guards prove
    // both postconditions and one exact event when the transaction actually commits.
    let outcome = store(
        tx,
        fence,
        request,
        binding,
        GuildResult::Ok,
        (Some(previous), Some(next)),
        Some(&audit_bytes),
    )
    .await?;
    // Accept's INSERT guard observes the still-live invitation, then all invitations disappear.
    // Invite removes its expired target rows before inserting the new invitation.
    if request.operation != GuildCommand::Accept {
        remove_invitations(tx, target, &removes).await?;
    }
    match request.operation {
        GuildCommand::Invite(_) => {
            sqlx::query("INSERT INTO game_guild_invitations(guild_id,character_id,inviter,expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4)")
                .bind(request.guild_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).bind(actor.as_bytes().as_slice())
                .bind(now.checked_add(INVITATION_LIFETIME_MS).ok_or_else(invalid)?).execute(&mut **tx).await?;
        }
        GuildCommand::Accept => {
            sqlx::query("INSERT INTO game_guild_members(character_id,guild_id,rank_level,title,joined_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,'',$4)")
                .bind(actor.as_bytes().as_slice()).bind(request.guild_id.as_bytes().as_slice()).bind(i16::from(rank_after.ok_or_else(invalid)?)).bind(now).execute(&mut **tx).await?;
        }
        GuildCommand::Leave | GuildCommand::Exclude(_) => {
            sqlx::query("DELETE FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid AND guild_id=encode($2,'hex')::uuid")
                .bind(target.as_bytes().as_slice()).bind(request.guild_id.as_bytes().as_slice()).execute(&mut **tx).await?;
            if rank_before == Some(2) {
                let target_account:String=sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                    .bind(target.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                let moved=sqlx::query("DELETE FROM game_account_guild_leadership WHERE account_id=$1::uuid AND guild_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid")
                    .bind(target_account).bind(request.guild_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).execute(&mut **tx).await?;
                if moved.rows_affected() != 1 {
                    return Err(invalid());
                }
            }
        }
        GuildCommand::Revoke(_) => {}
        GuildCommand::Found => return Err(GuildError::InvalidInput),
    }
    if request.operation == GuildCommand::Accept {
        remove_invitations(tx, target, &removes).await?;
    }
    let deficit = if state == GuildState::Active && proposed_vices < 4 {
        Some(
            guild
                .try_get::<Option<i64>, _>("vice_deficit_since")?
                .unwrap_or(now),
        )
    } else {
        guild.try_get::<Option<i64>, _>("vice_deficit_since")?
    };
    sqlx::query("UPDATE game_guilds SET revision=$2::text::numeric,vice_deficit_since=$3 WHERE guild_id=encode($1,'hex')::uuid")
        .bind(request.guild_id.as_bytes().as_slice()).bind(next.to_string()).bind(deficit).execute(&mut **tx).await?;
    Ok(outcome)
}

async fn remove_invitations(
    tx: &mut Transaction<'_, Postgres>,
    target: CharacterId,
    ids: &[[u8; 16]],
) -> Result<()> {
    for id in ids {
        sqlx::query("DELETE FROM game_guild_invitations WHERE guild_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid")
            .bind(id.as_slice()).bind(target.as_bytes().as_slice()).execute(&mut **tx).await?;
    }
    Ok(())
}

async fn store(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    request: &GuildRequest,
    binding: &[u8],
    result: GuildResult,
    revisions: (Option<u64>, Option<u64>),
    audit: Option<&[u8]>,
) -> Result<GuildOutcome> {
    store_operation(
        tx,
        fence,
        request,
        binding,
        result,
        OperationPayload {
            revisions,
            audit,
            found_name: None,
        },
    )
    .await
}

struct OperationPayload<'a> {
    revisions: (Option<u64>, Option<u64>),
    audit: Option<&'a [u8]>,
    found_name: Option<&'a str>,
}

async fn store_operation(
    tx: &mut Transaction<'_, Postgres>,
    fence: &CurrentCharacterItemFence,
    request: &GuildRequest,
    binding: &[u8],
    result: GuildResult,
    payload: OperationPayload<'_>,
) -> Result<GuildOutcome> {
    let OperationPayload {
        revisions: (previous, revision),
        audit,
        found_name,
    } = payload;
    let event = audit.map(|_| request.identity.event_id);
    sqlx::query("INSERT INTO game_guild_operations(occurrence,transaction_id,requested_event_id,game_session_id,command_id,actor,world_id,guild_id,target,action,request_binding,result,previous_revision,revision,event_id,audit_sha256,found_name) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid,encode($7,'hex')::uuid,encode($8,'hex')::uuid,encode($9,'hex')::uuid,$10,$11,$12,$13::text::numeric,$14::text::numeric,encode($15,'hex')::uuid,sha256($16),$17)")
        .bind(request.identity.occurrence.as_slice()).bind(request.identity.transaction_id.as_slice()).bind(request.identity.event_id.as_slice())
        .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(request.guild_id.as_bytes().as_slice())
        .bind(request.operation.target(fence.character_id).as_bytes().as_slice()).bind(request.operation.code()).bind(binding).bind(result.code())
        .bind(previous.map(|x|x.to_string())).bind(revision.or(previous).map(|x|x.to_string())).bind(event.map(|x|x.to_vec())).bind(audit).bind(found_name)
        .execute(&mut **tx).await?;
    if let Some(audit) = audit {
        sqlx::query("INSERT INTO game_guild_event_outbox(event_id,occurrence,guild_id,occurred_at,payload,payload_sha256) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,$5,sha256($5))")
            .bind(request.identity.event_id.as_slice()).bind(request.identity.occurrence.as_slice()).bind(request.guild_id.as_bytes().as_slice())
            .bind(request.identity.occurred_at_unix_ms).bind(audit).execute(&mut **tx).await?;
    }
    Ok(GuildOutcome::Committed(GuildReceipt {
        result,
        previous_revision: previous,
        revision: revision.or(previous),
        transaction_id: request.identity.transaction_id,
        event_id: event,
    }))
}

fn receipt(row: &sqlx::postgres::PgRow) -> Result<GuildReceipt> {
    let number = |name| -> Result<Option<u64>> {
        row.try_get::<Option<String>, _>(name)?
            .map(|s| s.parse().map_err(|_| invalid()))
            .transpose()
    };
    Ok(GuildReceipt {
        result: GuildResult::from_code(row.try_get("result")?).ok_or_else(invalid)?,
        previous_revision: number("previous_revision")?,
        revision: number("revision")?,
        transaction_id: uuid_text(&row.try_get::<String, _>("transaction_id")?)?,
        event_id: row
            .try_get::<Option<String>, _>("event_id")?
            .map(|s| uuid_text(&s))
            .transpose()?,
    })
}

/// Complete owner table shape for this private qualification. NOT a numbered migration,
/// production grant or promise that sibling GuildBank/Hall proposals exist. Found now creates
/// its initial Guild under the closed native command guards; old five-command fixture seeds
/// remain historical setup only.
/// The fixture seeds a valid initial Guild snapshot BEFORE installing GUARDS; it never disables
/// native Character/admission guards. Production must install tables+guards atomically and use
/// its separately admitted formation/bootstrap owner instead of that privileged fixture seed.
pub const GUILD_TABLES_PROPOSAL: &str = r#"
CREATE TABLE game_guilds(
 guild_id UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(guild_id)),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 name TEXT NOT NULL CHECK(game_character_is_name(name)),
 name_key TEXT GENERATED ALWAYS AS(game_character_name_key(name)) STORED,
 state SMALLINT NOT NULL CHECK(state BETWEEN 1 AND 4),
 formation_deadline BIGINT NOT NULL,vice_deficit_since BIGINT NULL,premium_deficit_since BIGINT NULL,
 motd TEXT NOT NULL DEFAULT '' CHECK(char_length(motd)<=255),
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 created_at BIGINT NOT NULL);
CREATE UNIQUE INDEX game_guild_live_name ON game_guilds(world_id,name_key) WHERE state<>4;
CREATE TABLE game_guild_ranks(
 guild_id UUID NOT NULL REFERENCES game_guilds,level SMALLINT NOT NULL CHECK(level BETWEEN 1 AND 20),
 name TEXT NOT NULL CHECK(name<>''),name_key TEXT GENERATED ALWAYS AS(game_character_name_key(name)) STORED,
 PRIMARY KEY(guild_id,level),UNIQUE(guild_id,name_key));
CREATE TABLE game_guild_members(
 character_id UUID PRIMARY KEY REFERENCES game_character_roots,guild_id UUID NOT NULL REFERENCES game_guilds,
 rank_level SMALLINT NOT NULL,title TEXT NOT NULL DEFAULT '' CHECK(char_length(title)<=29),joined_at BIGINT NOT NULL,
 FOREIGN KEY(guild_id,rank_level) REFERENCES game_guild_ranks);
CREATE UNIQUE INDEX game_guild_one_leader ON game_guild_members(guild_id) WHERE rank_level=1;
CREATE TABLE game_guild_invitations(
 guild_id UUID NOT NULL REFERENCES game_guilds,character_id UUID NOT NULL REFERENCES game_character_roots,
 inviter UUID NOT NULL REFERENCES game_character_roots,expires_at BIGINT NOT NULL,PRIMARY KEY(guild_id,character_id));
CREATE INDEX game_guild_target_invitations ON game_guild_invitations(character_id,guild_id);
CREATE TABLE game_account_guild_leadership(
 account_id UUID PRIMARY KEY REFERENCES game_character_account_guards,guild_id UUID NOT NULL REFERENCES game_guilds,
 character_id UUID NOT NULL UNIQUE REFERENCES game_character_roots);
CREATE TABLE game_guild_operations(
 occurrence UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(occurrence)),
 transaction_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),
 requested_event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(requested_event_id)),
 game_session_id UUID NOT NULL CHECK(game_character_is_uuid_v7(game_session_id)),
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 actor UUID NOT NULL REFERENCES game_character_roots,world_id UUID NOT NULL,guild_id UUID NOT NULL,
 target UUID NOT NULL CHECK(game_character_is_uuid_v7(target)),action SMALLINT NOT NULL CHECK(action BETWEEN 1 AND 6),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),result SMALLINT NOT NULL CHECK(result BETWEEN 0 AND 11),
 previous_revision NUMERIC(20,0),revision NUMERIC(20,0),event_id UUID UNIQUE,audit_sha256 BYTEA,
 found_name TEXT CHECK((action=6 AND found_name IS NOT NULL AND game_character_is_name(found_name)) OR (action<>6 AND found_name IS NULL)),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(game_session_id,command_id),UNIQUE(created_xact_id),
 CHECK(CASE WHEN result=0 THEN previous_revision IS NOT NULL AND revision IS NOT NULL
 AND event_id IS NOT NULL AND audit_sha256 IS NOT NULL
 AND ((action=6 AND previous_revision=0 AND revision=1) OR (action<>6 AND previous_revision>=1 AND revision=previous_revision+1)) AND event_id=requested_event_id AND octet_length(audit_sha256)=32
 ELSE previous_revision IS NOT DISTINCT FROM revision AND event_id IS NULL AND audit_sha256 IS NULL END));
CREATE TABLE game_guild_event_outbox(
 event_id UUID PRIMARY KEY,occurrence UUID NOT NULL UNIQUE REFERENCES game_guild_operations,
 guild_id UUID NOT NULL REFERENCES game_guilds DEFERRABLE INITIALLY DEFERRED,occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 payload BYTEA NOT NULL CHECK(octet_length(payload) BETWEEN 1 AND 4096),
 payload_sha256 BYTEA NOT NULL CHECK(payload_sha256=sha256(payload)),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id());
"#;

/// Private integrity guards for the five commands; no runtime/grant/SECURITY DEFINER shortcut.
/// Other accepted Guild writers (rank/activation/jobs/disband), normative event publication and
/// GUILD_ACTIVITY_RETENTION_V1 expiry require CP composition before production installation.
/// In particular this component cannot enter DISBANDED or assert external Bank/Hall closure.
pub const GUILD_GUARDS_PROPOSAL: &str = r#"
CREATE FUNCTION game_guild_rows_immutable() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN RAISE EXCEPTION 'retained Guild operation/event cannot be changed' USING ERRCODE='23514';END $$;
CREATE TRIGGER game_guild_operation_immutable BEFORE UPDATE OR DELETE ON game_guild_operations FOR EACH ROW EXECUTE FUNCTION game_guild_rows_immutable();
CREATE TRIGGER game_guild_event_immutable BEFORE UPDATE OR DELETE ON game_guild_event_outbox FOR EACH ROW EXECUTE FUNCTION game_guild_rows_immutable();
CREATE TRIGGER game_guild_operation_stamp BEFORE INSERT ON game_guild_operations FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_guild_event_stamp BEFORE INSERT ON game_guild_event_outbox FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
DO $$ DECLARE tab TEXT;BEGIN
 FOREACH tab IN ARRAY ARRAY['game_guilds','game_guild_ranks','game_guild_members','game_guild_invitations','game_account_guild_leadership','game_guild_operations','game_guild_event_outbox'] LOOP
  EXECUTE format('CREATE TRIGGER %I BEFORE TRUNCATE ON %I FOR EACH STATEMENT EXECUTE FUNCTION game_guild_rows_immutable()',tab||'_no_truncate',tab);
 END LOOP;
END $$;

CREATE FUNCTION game_guild_command_mutation() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE o game_guild_operations%ROWTYPE;r SMALLINT;w UUID;n BIGINT;
BEGIN
 SELECT * INTO o FROM game_guild_operations WHERE created_xact_id=pg_current_xact_id() AND result=0;
 IF NOT FOUND THEN RAISE EXCEPTION 'Guild mutation requires its same-transaction operation' USING ERRCODE='23514';END IF;
 IF TG_TABLE_NAME='game_guilds' THEN
  IF TG_OP='INSERT' THEN
   n=floor(extract(epoch FROM transaction_timestamp())*1000)::bigint;
   IF o.action<>6 OR NEW.guild_id<>o.guild_id OR NEW.world_id<>o.world_id OR NEW.name<>o.found_name
      OR NEW.state<>1 OR NEW.revision<>1 OR o.previous_revision<>0 OR o.revision<>1
      OR NEW.created_at<>n OR NEW.formation_deadline<>n+259200000
      OR NEW.vice_deficit_since IS NOT NULL OR NEW.premium_deficit_since IS NOT NULL OR NEW.motd<>'' THEN
    RAISE EXCEPTION 'Found must create exactly its bound initial Guild' USING ERRCODE='23514';END IF;
   RETURN NEW;
  END IF;
  -- Stored generated columns are NULL in NEW until AFTER BEFORE triggers. Its immutable
  -- source name stays in the comparison; the generated expression and unique index stay active.
  IF TG_OP<>'UPDATE' OR OLD.guild_id<>o.guild_id OR OLD.state NOT IN(1,2)
    OR OLD.revision<>o.previous_revision OR NEW.revision<>o.revision
    OR (to_jsonb(OLD)-'revision'-'vice_deficit_since'-'name_key')<>(to_jsonb(NEW)-'revision'-'vice_deficit_since'-'name_key') THEN
   RAISE EXCEPTION 'Guild mutation changes only the locked revision/vice deficit' USING ERRCODE='23514';END IF;
  SELECT count(*) INTO n FROM game_guild_members WHERE guild_id=o.guild_id AND rank_level=2;
  IF NEW.vice_deficit_since IS DISTINCT FROM (CASE WHEN NEW.state=2 AND n<4 THEN coalesce(OLD.vice_deficit_since,floor(extract(epoch FROM transaction_timestamp())*1000)::bigint) ELSE OLD.vice_deficit_since END) THEN
   RAISE EXCEPTION 'vice deficit must retain its first time' USING ERRCODE='23514';END IF;
  RETURN NEW;
 ELSIF TG_TABLE_NAME='game_guild_members' THEN
  IF TG_OP='INSERT' THEN
   SELECT world_id INTO w FROM game_character_roots WHERE character_id=NEW.character_id AND lifecycle=1;
   IF o.action=6 THEN
    IF NEW.character_id<>o.actor OR NEW.guild_id<>o.guild_id OR NEW.rank_level<>1 OR NEW.title<>''
       OR NEW.joined_at<>floor(extract(epoch FROM transaction_timestamp())*1000)::bigint OR w IS DISTINCT FROM o.world_id THEN
     RAISE EXCEPTION 'Found must create its actual same-world founder' USING ERRCODE='23514';END IF;
    RETURN NEW;
   END IF;
   IF o.action<>3 OR NEW.character_id<>o.actor OR NEW.guild_id<>o.guild_id
      OR NEW.rank_level<>(SELECT max(level) FROM game_guild_ranks WHERE guild_id=o.guild_id)
      OR NEW.rank_level<3 OR w IS DISTINCT FROM o.world_id
      OR NOT EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=o.guild_id AND character_id=o.actor AND expires_at>floor(extract(epoch FROM transaction_timestamp())*1000)::bigint) THEN
    RAISE EXCEPTION 'join must consume the actual live invitation into the lowest rank' USING ERRCODE='23514';END IF;
   RETURN NEW;
  ELSIF TG_OP='DELETE' THEN
   SELECT rank_level INTO r FROM game_guild_members WHERE character_id=o.actor AND guild_id=o.guild_id;
   IF OLD.guild_id<>o.guild_id OR OLD.character_id<>o.target OR OLD.rank_level=1
      OR NOT coalesce((o.action=4 AND o.actor=OLD.character_id) OR (o.action=5 AND r<=2 AND r<OLD.rank_level),false) THEN
    RAISE EXCEPTION 'leave/exclude requires the actual lower-ranked member' USING ERRCODE='23514';END IF;
   RETURN OLD;
  END IF;
 ELSIF TG_TABLE_NAME='game_guild_invitations' THEN
  SELECT rank_level INTO r FROM game_guild_members WHERE character_id=o.actor AND guild_id=o.guild_id;
  IF TG_OP='INSERT' THEN
   SELECT world_id INTO w FROM game_character_roots WHERE character_id=NEW.character_id AND lifecycle=1;
   IF o.action<>1 OR NEW.guild_id<>o.guild_id OR NEW.character_id<>o.target OR NEW.inviter<>o.actor
      OR r IS NULL OR r>2 OR w IS DISTINCT FROM o.world_id
      OR NEW.expires_at<>floor(extract(epoch FROM transaction_timestamp())*1000)::bigint+2592000000
      OR EXISTS(SELECT 1 FROM game_guild_members WHERE character_id=NEW.character_id AND guild_id=NEW.guild_id) THEN
    RAISE EXCEPTION 'invitation needs current same-world target and leader/vice' USING ERRCODE='23514';END IF;
   RETURN NEW;
  ELSIF TG_OP='DELETE' AND OLD.character_id=o.target AND (
      (o.action=1 AND OLD.expires_at<=floor(extract(epoch FROM transaction_timestamp())*1000)::bigint)
      OR (o.action=2 AND OLD.guild_id=o.guild_id AND r<=2)
      OR (o.action=3 AND o.actor=o.target)) THEN RETURN OLD;
  END IF;
 ELSIF TG_TABLE_NAME='game_guild_ranks' AND TG_OP='INSERT' THEN
  IF o.action<>6 OR NEW.guild_id<>o.guild_id OR NEW.name IS DISTINCT FROM (CASE NEW.level WHEN 1 THEN 'Leader' WHEN 2 THEN 'Vice Leader' WHEN 3 THEN 'Member' END) THEN
   RAISE EXCEPTION 'Found must create its three bound default ranks' USING ERRCODE='23514';END IF;
  RETURN NEW;
 ELSIF TG_TABLE_NAME='game_account_guild_leadership' AND TG_OP='INSERT' THEN
  IF o.action=6 AND NEW.guild_id=o.guild_id AND NEW.character_id=o.actor
     AND NEW.account_id=(SELECT account_id FROM game_character_roots WHERE character_id=o.actor) THEN RETURN NEW;END IF;
 ELSIF TG_TABLE_NAME='game_account_guild_leadership' AND TG_OP='DELETE' THEN
  IF o.action IN(4,5) AND OLD.guild_id=o.guild_id AND OLD.character_id=o.target
     AND OLD.account_id=(SELECT account_id FROM game_character_roots WHERE character_id=o.target) THEN RETURN OLD;END IF;
 END IF;
 RAISE EXCEPTION 'this Guild operation does not own that row transition' USING ERRCODE='23514';
END $$;
CREATE TRIGGER game_guild_row_mutation BEFORE INSERT OR UPDATE OR DELETE ON game_guilds FOR EACH ROW EXECUTE FUNCTION game_guild_command_mutation();
CREATE TRIGGER game_guild_member_mutation BEFORE INSERT OR UPDATE OR DELETE ON game_guild_members FOR EACH ROW EXECUTE FUNCTION game_guild_command_mutation();
CREATE TRIGGER game_guild_invitation_mutation BEFORE INSERT OR UPDATE OR DELETE ON game_guild_invitations FOR EACH ROW EXECUTE FUNCTION game_guild_command_mutation();
CREATE TRIGGER game_guild_leadership_mutation BEFORE INSERT OR UPDATE OR DELETE ON game_account_guild_leadership FOR EACH ROW EXECUTE FUNCTION game_guild_command_mutation();
CREATE TRIGGER game_guild_rank_not_owned BEFORE INSERT OR UPDATE OR DELETE ON game_guild_ranks FOR EACH ROW EXECUTE FUNCTION game_guild_command_mutation();

CREATE FUNCTION game_guild_snapshot_valid() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE g UUID;s SMALLINT;members BIGINT;leaders BIGINT;ranks BIGINT;
BEGIN
 IF TG_OP='DELETE' THEN g=OLD.guild_id;ELSE g=NEW.guild_id;END IF;
 SELECT state INTO s FROM game_guilds WHERE guild_id=g;
 SELECT count(*),count(*) FILTER(WHERE rank_level=1) INTO members,leaders FROM game_guild_members WHERE guild_id=g;
 SELECT count(*) INTO ranks FROM game_guild_ranks WHERE guild_id=g;
 IF members>2000 OR ranks NOT BETWEEN 3 AND 20 OR (s<>4 AND leaders<>1)
    OR (s=4 AND (members<>0 OR EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=g)))
    OR EXISTS(SELECT 1 FROM game_guild_members m JOIN game_character_roots c USING(character_id) JOIN game_guilds x USING(guild_id) WHERE m.guild_id=g AND (c.world_id<>x.world_id OR c.lifecycle<>1))
    OR EXISTS(SELECT 1 FROM game_guild_members m JOIN game_character_roots c USING(character_id) WHERE m.guild_id=g AND m.rank_level<=2 AND NOT EXISTS(SELECT 1 FROM game_account_guild_leadership l WHERE l.guild_id=m.guild_id AND l.character_id=m.character_id AND l.account_id=c.account_id))
    OR EXISTS(SELECT 1 FROM game_account_guild_leadership l JOIN game_character_roots c USING(character_id) WHERE l.guild_id=g AND (l.account_id<>c.account_id OR NOT EXISTS(SELECT 1 FROM game_guild_members m WHERE m.character_id=l.character_id AND m.guild_id=g AND m.rank_level<=2))) THEN
  RAISE EXCEPTION 'Guild snapshot violates membership/rank/leadership invariants' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
DO $$ DECLARE tab TEXT;BEGIN
 FOREACH tab IN ARRAY ARRAY['game_guilds','game_guild_ranks','game_guild_members','game_account_guild_leadership'] LOOP
  EXECUTE format('CREATE CONSTRAINT TRIGGER %I AFTER INSERT OR UPDATE OR DELETE ON %I DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_snapshot_valid()',tab||'_snapshot',tab);
 END LOOP;
END $$;
CREATE FUNCTION game_guild_invitation_caps() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF (SELECT count(*) FROM game_guild_invitations WHERE guild_id=NEW.guild_id)>500
 OR (SELECT count(*) FROM game_guild_invitations WHERE character_id=NEW.character_id)>50 THEN
  RAISE EXCEPTION 'Guild invitation row cap exceeded' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guild_invitation_caps AFTER INSERT ON game_guild_invitations DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_invitation_caps();

CREATE FUNCTION game_guild_operation_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.result=0 THEN
  IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(SELECT 1 FROM game_guilds g WHERE g.guild_id=NEW.guild_id AND g.world_id=NEW.world_id AND g.state IN(1,2) AND g.revision=NEW.revision)
     OR NOT EXISTS(SELECT 1 FROM game_guild_event_outbox e WHERE e.occurrence=NEW.occurrence AND e.event_id=NEW.event_id AND e.guild_id=NEW.guild_id AND e.payload_sha256=NEW.audit_sha256 AND e.created_xact_id=pg_current_xact_id())
     OR (NEW.action=1 AND NOT EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=NEW.guild_id AND character_id=NEW.target AND inviter=NEW.actor))
     OR (NEW.action=2 AND EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=NEW.guild_id AND character_id=NEW.target))
     OR (NEW.action=3 AND (NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND rank_level>=3) OR EXISTS(SELECT 1 FROM game_guild_invitations WHERE character_id=NEW.actor)))
     OR (NEW.action IN(4,5) AND EXISTS(SELECT 1 FROM game_guild_members WHERE character_id=NEW.target))
     OR (NEW.action=6 AND (
       NEW.actor<>NEW.target OR NEW.previous_revision<>0 OR NEW.revision<>1
       OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=NEW.guild_id AND name=NEW.found_name AND state=1 AND vice_deficit_since IS NULL AND premium_deficit_since IS NULL AND motd='')
       OR (SELECT count(*) FROM game_guild_members WHERE guild_id=NEW.guild_id)<>1
       OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND rank_level=1 AND title='')
       OR (SELECT count(*) FROM game_guild_ranks WHERE guild_id=NEW.guild_id)<>3
       OR NOT EXISTS(SELECT 1 FROM game_guild_ranks WHERE guild_id=NEW.guild_id AND level=1 AND name='Leader')
       OR NOT EXISTS(SELECT 1 FROM game_guild_ranks WHERE guild_id=NEW.guild_id AND level=2 AND name='Vice Leader')
       OR NOT EXISTS(SELECT 1 FROM game_guild_ranks WHERE guild_id=NEW.guild_id AND level=3 AND name='Member')
       OR EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=NEW.guild_id)
       OR NOT EXISTS(SELECT 1 FROM game_account_guild_leadership l JOIN game_character_roots c ON c.character_id=l.character_id WHERE l.guild_id=NEW.guild_id AND l.character_id=NEW.actor AND l.account_id=c.account_id))) THEN
   RAISE EXCEPTION 'Guild operation must commit its transition and one exact event' USING ERRCODE='23514';END IF;
 ELSIF EXISTS(SELECT 1 FROM game_guild_event_outbox WHERE occurrence=NEW.occurrence) THEN
  RAISE EXCEPTION 'refusal must not manufacture a Guild mutation event' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guild_operation_proven AFTER INSERT ON game_guild_operations DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_operation_proven();
CREATE FUNCTION game_guild_event_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(SELECT 1 FROM game_guild_operations o WHERE o.occurrence=NEW.occurrence AND o.result=0 AND o.guild_id=NEW.guild_id AND o.event_id=NEW.event_id AND o.audit_sha256=NEW.payload_sha256 AND o.created_xact_id=pg_current_xact_id()) THEN
  RAISE EXCEPTION 'Guild event has no matching operation' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guild_event_proven AFTER INSERT ON game_guild_event_outbox DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_event_proven();
"#;

#[cfg(test)]
#[path = "guild_tests.rs"]
mod tests;

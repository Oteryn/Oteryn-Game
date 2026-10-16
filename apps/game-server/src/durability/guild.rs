//! Native GUILD0 membership/invitation owner preparation, UNREGISTERED and UNINSTALLED.
//!
//! Found uses the genuine native Premium gate. Deadline jobs durably begin disband; final
//! Bank/Hall closure, wire and runtime installation remain outside this private component.
//! The private schema and event-data proposal require CP admission. Native recovery/session/
//! node guards remain the actual current-authority source; retained operations are outcomes.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::{assert_session_fence, numeric_u64, uuid_text};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::guild_audit::{
    GuildAuditProposal, GuildDeclineAuditProposal, GuildEditRanksAuditProposal,
    GuildRankAuditProposal, GuildResignAuditProposal, GuildTextAuditProposal,
};
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
    /// Private proposed persistence kinds, not allocated protocol IDs.
    SetTitle(CharacterId),
    SetMessage,
    SetRank(CharacterId),
    Resign(CharacterId),
    EditRanks,
    /// Invited actor deletes only its selected Guild invitation (GUILDS_V1§5).
    Decline,
    Disband,
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
            Self::SetTitle(_) => 7,
            Self::SetMessage => 8,
            Self::SetRank(_) => 9,
            Self::Resign(_) => 10,
            Self::EditRanks => 11,
            Self::Decline => 12,
            Self::Disband => 13,
        }
    }
    fn target(self, actor: CharacterId) -> CharacterId {
        match self {
            Self::Invite(c)
            | Self::Revoke(c)
            | Self::Exclude(c)
            | Self::SetTitle(c)
            | Self::SetRank(c)
            | Self::Resign(c) => c,
            Self::Accept
            | Self::Leave
            | Self::Found
            | Self::SetMessage
            | Self::EditRanks
            | Self::Decline
            | Self::Disband => actor,
        }
    }
}

/// Accepted Disband confirmation DATA. It never supplies role or current authority.
#[derive(Debug, Clone)]
pub(crate) struct GuildDisbandRequest {
    common: GuildRequest,
    confirmation: String,
}
impl GuildDisbandRequest {
    pub(crate) fn new(common: GuildRequest, confirmation: &str) -> Result<Self> {
        if common.operation != GuildCommand::Disband
            || confirmation.chars().count() > 29
            || confirmation.contains('\0')
        {
            return Err(GuildError::InvalidInput);
        }
        Ok(Self {
            common,
            confirmation: confirmation.to_owned(),
        })
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

/// Bounded edit DATA. Private fields prevent the legacy membership entry from silently
/// interpreting a text-less new arm. The expected revision is a CAS operand, never authority.
#[derive(Debug, Clone)]
pub(crate) struct GuildTextRequest {
    common: GuildRequest,
    expected_revision: u64,
    text: String,
}
impl GuildTextRequest {
    pub(crate) fn new(
        common: GuildRequest,
        expected_revision: u64,
        received: &str,
    ) -> Result<Self> {
        if expected_revision == 0 {
            return Err(GuildError::InvalidInput);
        }
        let text = match common.operation {
            // GUILD0 specifies only the 29-scalar title bound, not the 0022 name grammar.
            // Empty title is bounded DATA; no empty-message/clear exception is introduced.
            GuildCommand::SetTitle(_)
                if received.chars().count() <= 29 && !received.contains('\0') =>
            {
                received.to_owned()
            }
            GuildCommand::SetMessage => crate::chat::ChatText::parse(received)
                .map_err(|_| GuildError::InvalidInput)?
                .as_str()
                .to_owned(),
            _ => return Err(GuildError::InvalidInput),
        };
        Ok(Self {
            common,
            expected_revision,
            text,
        })
    }
}

/// Rank edit DATA only. Current ranks, Account position and Premium are resolved by the writer.
#[derive(Debug, Clone)]
pub(crate) struct GuildRankRequest {
    common: GuildRequest,
    expected_revision: u64,
    level: GuildRank,
}
impl GuildRankRequest {
    pub(crate) fn new(common: GuildRequest, expected_revision: u64, level: u8) -> Result<Self> {
        if expected_revision == 0 || !matches!(common.operation, GuildCommand::SetRank(_)) {
            return Err(GuildError::InvalidInput);
        }
        Ok(Self {
            common,
            expected_revision,
            level: GuildRank::new(level).map_err(|_| GuildError::InvalidInput)?,
        })
    }
}

fn rank_binding(fence: &CurrentCharacterItemFence, rank: &GuildRankRequest) -> Result<Vec<u8>> {
    let mut digest = Sha256::new();
    digest.update(b"oteryn:guild-rank-proposal:v1");
    digest.update(binding(fence, &rank.common)?);
    digest.update(rank.expected_revision.to_be_bytes());
    digest.update([rank.level.level()]);
    Ok(digest.finalize().to_vec())
}

/// Resign DATA only. Expected revision is a private proposed precondition, not a new wire ID.
#[derive(Debug, Clone)]
pub(crate) struct GuildResignRequest {
    common: GuildRequest,
    expected_revision: u64,
}
impl GuildResignRequest {
    pub(crate) fn new(common: GuildRequest, expected_revision: u64) -> Result<Self> {
        if expected_revision == 0 || !matches!(common.operation, GuildCommand::Resign(_)) {
            return Err(GuildError::InvalidInput);
        }
        Ok(Self {
            common,
            expected_revision,
        })
    }
}
fn resign_binding(
    fence: &CurrentCharacterItemFence,
    resign: &GuildResignRequest,
) -> Result<Vec<u8>> {
    let mut digest = Sha256::new();
    digest.update(b"oteryn:guild-resign-proposal:v1");
    digest.update(binding(fence, &resign.common)?);
    digest.update(resign.expected_revision.to_be_bytes());
    Ok(digest.finalize().to_vec())
}

/// Ordered rank-name DATA only. Rank-specific lexical/length policy is not invented here.
/// Empty/NUL checks reflect the existing nonempty SQL column and PostgreSQL text encoding.
#[derive(Debug, Clone)]
pub(crate) struct GuildEditRanksRequest {
    common: GuildRequest,
    expected_revision: u64,
    names: Vec<String>,
}
impl GuildEditRanksRequest {
    pub(crate) fn new(
        common: GuildRequest,
        expected_revision: u64,
        names: Vec<String>,
    ) -> Result<Self> {
        if expected_revision == 0
            || common.operation != GuildCommand::EditRanks
            || crate::domain::guild::validate_rank_count(names.len()).is_err()
            || names
                .iter()
                .any(|name| name.is_empty() || name.contains('\0'))
        {
            return Err(GuildError::InvalidInput);
        }
        Ok(Self {
            common,
            expected_revision,
            names,
        })
    }
}
fn edit_ranks_binding(
    fence: &CurrentCharacterItemFence,
    edit: &GuildEditRanksRequest,
) -> Result<Vec<u8>> {
    let mut digest = Sha256::new();
    digest.update(b"oteryn:guild-edit-ranks-proposal:v1");
    digest.update(binding(fence, &edit.common)?);
    digest.update(edit.expected_revision.to_be_bytes());
    digest.update([u8::try_from(edit.names.len()).map_err(|_| GuildError::InvalidInput)?]);
    for name in &edit.names {
        digest.update(
            u64::try_from(name.len())
                .map_err(|_| GuildError::InvalidInput)?
                .to_be_bytes(),
        );
        digest.update(name.as_bytes());
    }
    Ok(digest.finalize().to_vec())
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
    StaleRevision,
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
            Self::StaleRevision => 12,
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
            12 => Self::StaleRevision,
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
    /// GUILD0 §3.2 leader-confirmed start. No Premium/Bank/Hall authority is inferred.
    pub(crate) async fn commit_guild_disband(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        disband: GuildDisbandRequest,
    ) -> Result<GuildOutcome> {
        let mut digest = Sha256::new();
        digest.update(b"oteryn:guild-disband-proposal:v1");
        digest.update(binding(&fence, &disband.common)?);
        digest.update(disband.confirmation.as_bytes());
        let binding = digest.finalize().to_vec();
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = async {
                        lock_fenced_guild_operation(&mut tx, &recovery, &disband.common).await?;
                        let edit = MemberEdit::Disband(&disband);
                        execute(
                            &mut tx,
                            &node,
                            &fence,
                            &disband.common,
                            &binding,
                            None,
                            Some(&edit),
                        )
                        .await
                    }
                    .await;
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
        if matches!(
            request.operation,
            GuildCommand::Found
                | GuildCommand::SetTitle(_)
                | GuildCommand::SetMessage
                | GuildCommand::SetRank(_)
                | GuildCommand::Resign(_)
                | GuildCommand::EditRanks
                | GuildCommand::Disband
        ) {
            return Err(GuildError::InvalidInput);
        }
        let binding = binding(&fence, &request)?;
        self.commit_guild_operation(authority, node, fence, request, binding, None)
            .await
    }

    /// One accepted title/message edit, through the same native owner transaction as the
    /// existing commands. Text/revision carry no role, session or membership authority.
    pub(crate) async fn commit_guild_text(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        text: GuildTextRequest,
    ) -> Result<GuildOutcome> {
        let mut digest = Sha256::new();
        digest.update(b"oteryn:guild-text-proposal:v1");
        digest.update(binding(&fence, &text.common)?);
        digest.update(text.expected_revision.to_be_bytes());
        digest.update(text.text.as_bytes());
        let binding = digest.finalize().to_vec();
        self.commit_guild_operation(
            authority,
            node,
            fence,
            text.common.clone(),
            binding,
            Some(text),
        )
        .await
    }

    /// GUILD0 §3.2/§3.3: mutate a real member's rank and its synchronous vice-count effects.
    /// Only a new move into level2 consumes the actual TARGET Account's Premium decision.
    pub(crate) async fn commit_guild_rank(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        rank: GuildRankRequest,
        premium: &SpellPremiumCoordinator,
    ) -> Result<GuildOutcome> {
        let binding = rank_binding(&fence, &rank)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let account = match self
            .preflight_guild_target(
                recovery.clone(),
                node.clone(),
                fence,
                rank.common.clone(),
                binding.clone(),
            )
            .await?
        {
            GuildTargetPreflight::Replayed(value) => return Ok(value),
            GuildTargetPreflight::Fresh(_, account) => account,
        };
        let write = RankWrite {
            node: node.clone(),
            fence,
            rank,
            binding,
            account,
        };
        let Some(account) = account else {
            // Unknown/inactive/other-World target cannot acquire a Premium decision.
            // The real writer rechecks all current facts and can only refuse without one.
            return self.run_guild_rank_write(recovery, write, None).await;
        };
        let root = self.clone();
        premium
            .with_premium_gate(account, move |status, tx| {
                Box::pin(async move {
                    match tx {
                        Some(tx) => {
                            if !matches!(&status, PremiumStatus::NotActivated(_)) {
                                return Err(invalid());
                            }
                            // The native gate commits its physical transaction even on callback error.
                            let mut savepoint = tx.begin().await?;
                            let result = write
                                .execute(&mut savepoint, &recovery, Some(&status))
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
                            let status: PremiumStatus<'static> = match status {
                                PremiumStatus::Current => PremiumStatus::Current,
                                PremiumStatus::NotCurrent => PremiumStatus::NotCurrent,
                                PremiumStatus::NotActivated(_) => return Err(invalid()),
                            };
                            root.run_guild_rank_write(recovery, write, Some(status))
                                .await
                        }
                    }
                })
            })
            .await?
    }

    async fn preflight_guild_target(
        &self,
        recovery: crate::character_recovery_fence::CharacterRecoveryFenceV1,
        node: NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: GuildRequest,
        binding: Vec<u8>,
    ) -> Result<GuildTargetPreflight> {
        let pre_recovery = recovery;
        let pre_node = node;
        let pre_request = request;
        let pre_binding = binding;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                let result = async {
                    lock_fenced_guild_operation(&mut tx, &pre_recovery, &pre_request).await?;
                    if let Some(outcome) = replay(&mut tx, &pre_request, &pre_binding).await? {
                        return Ok(GuildTargetPreflight::Replayed(outcome));
                    }
                    let actor = assert_session_fence(&mut tx, &fence, &pre_node).await?
                        .map_err(|_| GuildError::NotCurrent)?;
                    // A selector read, never caller Account data or target session authority.
                    // GUILD0-LO-01 explicitly takes no target root lock for this command.
                    let target = pre_request.operation.target(fence.character_id);
                    let row = sqlx::query("SELECT account_id::text,world_id::text,lifecycle FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                        .bind(target.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                    let account = match row {
                        Some(row) if row.try_get::<i16,_>("lifecycle")? == 1
                            && uuid_text(&row.try_get::<String,_>("world_id")?)? == *fence.runtime_scope.world_id().as_bytes() =>
                            Some(uuid_text(&row.try_get::<String,_>("account_id")?)?),
                        _ => None,
                    };
                    Ok(GuildTargetPreflight::Fresh(uuid_text(&actor)?,account))
                }.await;
                match result {
                    Ok(value) => { commit_semantic_transaction(tx, deadline).await?; Ok(Ok(value)) }
                    Err(GuildError::Unavailable(error)) => Err(error),
                    Err(error) => Ok(Err(error)),
                }
            })
        }).await?
    }

    /// GUILD0 §3.2: actual leader and same-Guild vice swap levels immediately.
    /// Both moves into a NEW level1/2 consume their actual Account's native Premium decision.
    pub(crate) async fn commit_guild_resign(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        resign: GuildResignRequest,
        premium: &SpellPremiumCoordinator,
    ) -> Result<GuildOutcome> {
        let binding = resign_binding(&fence, &resign)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let accounts = match self
            .preflight_guild_target(
                recovery.clone(),
                node.clone(),
                fence,
                resign.common.clone(),
                binding.clone(),
            )
            .await?
        {
            GuildTargetPreflight::Replayed(value) => return Ok(value),
            GuildTargetPreflight::Fresh(actor, target) => target.map(|target| [actor, target]),
        };
        let write = ResignWrite {
            node: node.clone(),
            fence,
            resign,
            binding,
            accounts,
        };
        let Some(accounts) = accounts else {
            return self.run_guild_resign_write(recovery, write, None).await;
        };
        let root = self.clone();
        premium
            .with_premium_batch_gate(&accounts, move |statuses, tx| {
                Box::pin(async move {
                    match tx {
                        Some(tx) => {
                            if statuses.len() != 2
                                || !statuses
                                    .iter()
                                    .all(|s| matches!(s, PremiumStatus::NotActivated(_)))
                            {
                                return Err(invalid());
                            }
                            // The native gate commits even on callback error: roll back all local writes.
                            let mut savepoint = tx.begin().await?;
                            let result = write
                                .execute(&mut savepoint, &recovery, Some(&statuses))
                                .await;
                            match result {
                                Ok(value) => {
                                    savepoint.commit().await?;
                                    Ok(value)
                                }
                                Err(error) => {
                                    savepoint.rollback().await?;
                                    Err(error)
                                }
                            }
                        }
                        None => {
                            let mut owned = Vec::with_capacity(statuses.len());
                            for status in statuses {
                                owned.push(match status {
                                    PremiumStatus::Current => PremiumStatus::Current,
                                    PremiumStatus::NotCurrent => PremiumStatus::NotCurrent,
                                    PremiumStatus::NotActivated(_) => return Err(invalid()),
                                });
                            }
                            root.run_guild_resign_write(recovery, write, Some(owned))
                                .await
                        }
                    }
                })
            })
            .await?
    }
    async fn run_guild_resign_write(
        &self,
        recovery: crate::character_recovery_fence::CharacterRecoveryFenceV1,
        write: ResignWrite,
        premium: Option<Vec<PremiumStatus<'static>>>,
    ) -> Result<GuildOutcome> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = write.execute(&mut tx, &recovery, premium.as_deref()).await;
                    match result {
                        Ok(value) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(GuildError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }

    async fn run_guild_rank_write(
        &self,
        recovery: crate::character_recovery_fence::CharacterRecoveryFenceV1,
        write: RankWrite,
        premium: Option<PremiumStatus<'static>>,
    ) -> Result<GuildOutcome> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = write.execute(&mut tx, &recovery, premium.as_ref()).await;
                    match result {
                        Ok(value) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(GuildError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }

    /// Accepted leader-only rank definition replacement in the existing physical transaction.
    /// No Premium read is needed: N>=3 preserves both leadership levels and all positions.
    pub(crate) async fn commit_guild_edit_ranks(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        edit_ranks: GuildEditRanksRequest,
    ) -> Result<GuildOutcome> {
        let binding = edit_ranks_binding(&fence, &edit_ranks)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let write = EditRanksWrite {
            node: node.clone(),
            fence,
            edit_ranks,
            binding,
        };
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = write.execute(&mut tx, &recovery).await;
                    match result {
                        Ok(value) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(GuildError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }

    async fn commit_guild_operation(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: GuildRequest,
        binding: Vec<u8>,
        text: Option<GuildTextRequest>,
    ) -> Result<GuildOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = execute_fenced_operation(
                        &mut tx,
                        &recovery,
                        &node,
                        &fence,
                        &request,
                        &binding,
                        text.as_ref(),
                    )
                    .await;
                    match result {
                        Ok(result) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(result))
                        }
                        Err(GuildError::Unavailable(e)) => Err(e),
                        Err(error) => Ok(Err(error)), // Dropping the transaction rolls back every effect.
                    }
                })
            })
            .await?
    }
}

enum GuildTargetPreflight {
    Replayed(GuildOutcome),
    Fresh([u8; 16], Option<[u8; 16]>),
}
struct RankWrite {
    node: NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    rank: GuildRankRequest,
    binding: Vec<u8>,
    account: Option<[u8; 16]>,
}
struct RankEdit<'a, 'premium> {
    request: &'a GuildRankRequest,
    account: Option<[u8; 16]>,
    premium: Option<&'a PremiumStatus<'premium>>,
}
impl RankWrite {
    async fn execute(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
        premium: Option<&PremiumStatus<'_>>,
    ) -> Result<GuildOutcome> {
        lock_fenced_guild_operation(tx, recovery, &self.rank.common).await?;
        let edit = MemberEdit::Rank(RankEdit {
            request: &self.rank,
            account: self.account,
            premium,
        });
        execute(
            tx,
            &self.node,
            &self.fence,
            &self.rank.common,
            &self.binding,
            None,
            Some(&edit),
        )
        .await
    }
}

struct ResignWrite {
    node: NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    resign: GuildResignRequest,
    binding: Vec<u8>,
    accounts: Option<[[u8; 16]; 2]>,
}
struct ResignEdit<'a, 'premium> {
    request: &'a GuildResignRequest,
    accounts: Option<[[u8; 16]; 2]>,
    premium: Option<&'a [PremiumStatus<'premium>]>,
}
enum MemberEdit<'a, 'premium> {
    Rank(RankEdit<'a, 'premium>),
    Resign(ResignEdit<'a, 'premium>),
    EditRanks(&'a GuildEditRanksRequest),
    Disband(&'a GuildDisbandRequest),
}
impl ResignWrite {
    async fn execute(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
        premium: Option<&[PremiumStatus<'_>]>,
    ) -> Result<GuildOutcome> {
        lock_fenced_guild_operation(tx, recovery, &self.resign.common).await?;
        let edit = MemberEdit::Resign(ResignEdit {
            request: &self.resign,
            accounts: self.accounts,
            premium,
        });
        execute(
            tx,
            &self.node,
            &self.fence,
            &self.resign.common,
            &self.binding,
            None,
            Some(&edit),
        )
        .await
    }
}

struct EditRanksWrite {
    node: NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    edit_ranks: GuildEditRanksRequest,
    binding: Vec<u8>,
}
impl EditRanksWrite {
    async fn execute(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    ) -> Result<GuildOutcome> {
        lock_fenced_guild_operation(tx, recovery, &self.edit_ranks.common).await?;
        let edit = MemberEdit::EditRanks(&self.edit_ranks);
        execute(
            tx,
            &self.node,
            &self.fence,
            &self.edit_ranks.common,
            &self.binding,
            None,
            Some(&edit),
        )
        .await
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
                    text: None,
                    rank: None,
                    resign: None,
                    edit_ranks: None,
                    disband: None,
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
                text: None,
                rank: None,
                resign: None,
                edit_ranks: None,
                disband: None,
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

/// The single native transaction prelude: recovery remains before historical receipt or
/// current actor checks. Factoring it also permits negative DATA against the real predicate;
/// no test constructs or weakens the sealed Root capability.
async fn execute_fenced_operation(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    request: &GuildRequest,
    binding: &[u8],
    text: Option<&GuildTextRequest>,
) -> Result<GuildOutcome> {
    lock_fenced_guild_operation(tx, recovery, request).await?;
    execute(tx, node, fence, request, binding, text, None).await
}

async fn lock_fenced_guild_operation(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &crate::character_recovery_fence::CharacterRecoveryFenceV1,
    request: &GuildRequest,
) -> Result<()> {
    assert_recovery_fence(tx, recovery).await?;
    lock_admission_relations(tx).await?;
    lock_operation(tx, request).await
}

async fn execute(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: &CurrentCharacterItemFence,
    request: &GuildRequest,
    binding: &[u8],
    text: Option<&GuildTextRequest>,
    edit: Option<&MemberEdit<'_, '_>>,
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
    let guild = sqlx::query("SELECT name,world_id::text,state,revision::text,formation_deadline,vice_deficit_since FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE")
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
    let members = sqlx::query("SELECT character_id::text,guild_id::text,rank_level FROM game_guild_members WHERE guild_id=encode($3,'hex')::uuid AND ($4 OR character_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid)) ORDER BY character_id FOR UPDATE")
        .bind(actor.as_bytes().as_slice()).bind(member_target.as_bytes().as_slice()).bind(request.guild_id.as_bytes().as_slice()).bind(request.operation == GuildCommand::EditRanks).fetch_all(&mut **tx).await?;
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
    if let Some(MemberEdit::Disband(disband)) = edit {
        if acting.as_ref().is_none_or(|member| member.rank != 1)
            || disband.confirmation != guild.try_get::<String, _>("name")?
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
        let next = previous.checked_add(1).ok_or_else(invalid)?;
        let audit = super::guild_audit::GuildDisbandAuditProposal {
            operation: GuildAuditProposal {
                version: 1,
                occurrence: request.identity.occurrence,
                transaction_id: request.identity.transaction_id,
                event_id: request.identity.event_id,
                guild_id: *request.guild_id.as_bytes(),
                world_id: *fence.runtime_scope.world_id().as_bytes(),
                actor: *actor.as_bytes(),
                target: *actor.as_bytes(),
                action: 13,
                previous_revision: previous,
                revision: next,
                rank_before: None,
                rank_after: None,
                removed_invitations: 0,
                occurred_at_unix_ms: request.identity.occurred_at_unix_ms,
            },
            confirmation: disband.confirmation.clone(),
        }
        .checked_bytes()
        .ok_or_else(invalid)?;
        let outcome = store_operation(
            tx,
            fence,
            request,
            binding,
            GuildResult::Ok,
            OperationPayload {
                revisions: (Some(previous), Some(next)),
                audit: Some(&audit),
                found_name: None,
                text: None,
                rank: None,
                resign: None,
                edit_ranks: None,
                disband: Some(disband),
            },
        )
        .await?;
        sqlx::query("INSERT INTO game_guild_job_checkpoints(guild_id,step,world_id,reason,previous_revision,revision,occurred_at) VALUES(encode($1,'hex')::uuid,1,encode($2,'hex')::uuid,3,$3::text::numeric,$4::text::numeric,floor(extract(epoch FROM transaction_timestamp())*1000)::bigint)")
            .bind(request.guild_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice())
            .bind(previous.to_string()).bind(next.to_string()).execute(&mut **tx).await?;
        sqlx::query("UPDATE game_guilds SET state=3,revision=$2::text::numeric WHERE guild_id=encode($1,'hex')::uuid")
            .bind(request.guild_id.as_bytes().as_slice()).bind(next.to_string()).execute(&mut **tx).await?;
        return Ok(outcome);
    }
    if let Some(MemberEdit::EditRanks(edit)) = edit {
        // The one sorted member lock query above locks the complete roster for this action.
        // Do not lock the actor first and then acquire smaller CharacterIds out of order.
        let refusal = if acting.as_ref().is_none_or(|m| m.rank != 1) {
            Some(GuildResult::NotAllowed)
        } else if edit.expected_revision != previous {
            Some(GuildResult::StaleRevision)
        } else {
            None
        };
        if let Some(result) = refusal {
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
        let unique: i64 = sqlx::query_scalar("SELECT count(DISTINCT game_character_name_key(name)) FROM unnest($1::text[]) AS proposed(name)")
            .bind(&edit.names).fetch_one(&mut **tx).await?;
        if usize::try_from(unique).map_err(|_| invalid())? != edit.names.len() {
            return store(
                tx,
                fence,
                request,
                binding,
                GuildResult::NameTaken,
                (Some(previous), None),
                None,
            )
            .await;
        }
        let next = previous.checked_add(1).ok_or_else(invalid)?;
        let audit = GuildEditRanksAuditProposal {
            operation: GuildAuditProposal {
                version: 1,
                occurrence: request.identity.occurrence,
                transaction_id: request.identity.transaction_id,
                event_id: request.identity.event_id,
                guild_id: *request.guild_id.as_bytes(),
                world_id: *fence.runtime_scope.world_id().as_bytes(),
                actor: *actor.as_bytes(),
                target: *actor.as_bytes(),
                action: 11,
                previous_revision: previous,
                revision: next,
                rank_before: None,
                rank_after: None,
                removed_invitations: 0,
                occurred_at_unix_ms: request.identity.occurred_at_unix_ms,
            },
            expected_revision: edit.expected_revision,
            names: edit.names.clone(),
        }
        .checked_bytes()
        .ok_or(GuildError::InvalidInput)?;
        let outcome = store_operation(
            tx,
            fence,
            request,
            binding,
            GuildResult::Ok,
            OperationPayload {
                revisions: (Some(previous), Some(next)),
                audit: Some(&audit),
                found_name: None,
                text: None,
                rank: None,
                resign: None,
                edit_ranks: Some(edit),
                disband: None,
            },
        )
        .await?;
        // Only this action defers the named rank-name constraint; old actions keep immediate checks.
        sqlx::query("SET CONSTRAINTS game_guild_rank_name_unique DEFERRED")
            .execute(&mut **tx)
            .await?;
        for (index, name) in edit.names.iter().enumerate() {
            let level = i16::try_from(index + 1).map_err(|_| invalid())?;
            sqlx::query("INSERT INTO game_guild_ranks(guild_id,level,name) VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT(guild_id,level) DO UPDATE SET name=EXCLUDED.name")
                .bind(request.guild_id.as_bytes().as_slice()).bind(level).bind(name).execute(&mut **tx).await?;
        }
        let count = i16::try_from(edit.names.len()).map_err(|_| invalid())?;
        // The immediate membership FK remains enabled: move all removed-tail members first.
        sqlx::query("UPDATE game_guild_members SET rank_level=$2 WHERE guild_id=encode($1,'hex')::uuid AND rank_level>$2")
            .bind(request.guild_id.as_bytes().as_slice()).bind(count).execute(&mut **tx).await?;
        sqlx::query(
            "DELETE FROM game_guild_ranks WHERE guild_id=encode($1,'hex')::uuid AND level>$2",
        )
        .bind(request.guild_id.as_bytes().as_slice())
        .bind(count)
        .execute(&mut **tx)
        .await?;
        sqlx::query("UPDATE game_guilds SET revision=$2::text::numeric WHERE guild_id=encode($1,'hex')::uuid")
            .bind(request.guild_id.as_bytes().as_slice()).bind(next.to_string()).execute(&mut **tx).await?;
        // Rank definitions and lower-member moves cannot alter positions, vice count or deadlines.
        return Ok(outcome);
    }
    if let Some(MemberEdit::Resign(edit)) = edit {
        let target_root=sqlx::query("SELECT account_id::text,world_id::text,lifecycle FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(target.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        let target_account = match target_root {
            Some(row)
                if row.try_get::<i16, _>("lifecycle")? == 1
                    && uuid_text(&row.try_get::<String, _>("world_id")?)?
                        == *fence.runtime_scope.world_id().as_bytes() =>
            {
                Some(uuid_text(&row.try_get::<String, _>("account_id")?)?)
            }
            _ => None,
        };
        let current_accounts = target_account
            .map(|target| Ok::<_, GuildError>([uuid_text(&account)?, target]))
            .transpose()?;
        if current_accounts != edit.accounts {
            return Err(GuildError::NotCurrent);
        }
        let allowed = actor != target
            && target_account.is_some()
            && acting.as_ref().is_some_and(|a| a.rank == 1)
            && subject.as_ref().is_some_and(|t| t.rank == 2);
        let refusal = if !allowed {
            Some(GuildResult::NotAllowed)
        } else if edit.request.expected_revision != previous {
            Some(GuildResult::StaleRevision)
        } else if super::bank::fee_payer_is_junior(target) {
            Some(GuildResult::Junior)
        } else {
            None
        };
        if let Some(result) = refusal {
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
        let accounts = current_accounts.ok_or_else(invalid)?;
        if accounts[0] == accounts[1] {
            return Err(invalid());
        }
        let mut ordered = accounts;
        ordered.sort();
        // Both positions already exist. Lock/validate them in AccountId order and retain every byte.
        for account_id in ordered {
            let guard:Option<i32>=sqlx::query_scalar("SELECT 1 FROM game_character_account_guards WHERE account_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(account_id.as_slice()).fetch_optional(&mut **tx).await?;
            if guard.is_none() {
                return Err(GuildError::NotCurrent);
            }
        }
        let positions=sqlx::query("SELECT account_id::text,guild_id::text,character_id::text FROM game_account_guild_leadership WHERE account_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid) ORDER BY account_id FOR UPDATE")
            .bind(accounts[0].as_slice()).bind(accounts[1].as_slice()).fetch_all(&mut **tx).await?;
        if positions.len() != 2 {
            return Err(invalid());
        }
        for row in &positions {
            let account_id = uuid_text(&row.try_get::<String, _>("account_id")?)?;
            let expected = if account_id == accounts[0] {
                actor
            } else if account_id == accounts[1] {
                target
            } else {
                return Err(invalid());
            };
            if uuid_text(&row.try_get::<String, _>("guild_id")?)? != *request.guild_id.as_bytes()
                || uuid_text(&row.try_get::<String, _>("character_id")?)? != *expected.as_bytes()
            {
                return Err(invalid());
            }
        }
        if !edit.premium.is_some_and(|s| {
            s.len() == 2
                && s.iter()
                    .all(|s| matches!(s, PremiumStatus::Current | PremiumStatus::NotActivated(_)))
        }) {
            return store(
                tx,
                fence,
                request,
                binding,
                GuildResult::NotPremium,
                (Some(previous), None),
                None,
            )
            .await;
        }
        let next = previous.checked_add(1).ok_or_else(invalid)?;
        let audit = GuildResignAuditProposal {
            operation: GuildAuditProposal {
                version: 1,
                occurrence: request.identity.occurrence,
                transaction_id: request.identity.transaction_id,
                event_id: request.identity.event_id,
                guild_id: *request.guild_id.as_bytes(),
                world_id: *fence.runtime_scope.world_id().as_bytes(),
                actor: *actor.as_bytes(),
                target: *target.as_bytes(),
                action: 10,
                previous_revision: previous,
                revision: next,
                rank_before: Some(2),
                rank_after: Some(1),
                removed_invitations: 0,
                occurred_at_unix_ms: request.identity.occurred_at_unix_ms,
            },
            expected_revision: edit.request.expected_revision,
        }
        .checked_bytes()
        .ok_or_else(invalid)?;
        let outcome = store_operation(
            tx,
            fence,
            request,
            binding,
            GuildResult::Ok,
            OperationPayload {
                revisions: (Some(previous), Some(next)),
                audit: Some(&audit),
                found_name: None,
                text: None,
                rank: None,
                resign: Some(edit.request),
                edit_ranks: None,
                disband: None,
            },
        )
        .await?;
        // Preserve the immediate unique leader index: never create two leaders, even transiently.
        for (character, before, after) in [(actor, 1_i16, 2_i16), (target, 2, 1)] {
            let changed=sqlx::query("UPDATE game_guild_members SET rank_level=$4 WHERE guild_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND rank_level=$3")
                .bind(request.guild_id.as_bytes().as_slice()).bind(character.as_bytes().as_slice()).bind(before).bind(after).execute(&mut **tx).await?;
            if changed.rows_affected() != 1 {
                return Err(invalid());
            }
        }
        // Vice count is unchanged; no lifecycle, deadline or deficit timestamp can be cured here.
        sqlx::query("UPDATE game_guilds SET revision=$2::text::numeric WHERE guild_id=encode($1,'hex')::uuid")
            .bind(request.guild_id.as_bytes().as_slice()).bind(next.to_string()).execute(&mut **tx).await?;
        return Ok(outcome);
    }
    if let Some(MemberEdit::Rank(edit)) = edit {
        let desired = edit.request.level.level();
        let target_root = sqlx::query("SELECT account_id::text,world_id::text,lifecycle FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(target.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        let target_account = match target_root {
            Some(row)
                if row.try_get::<i16, _>("lifecycle")? == 1
                    && uuid_text(&row.try_get::<String, _>("world_id")?)?
                        == *fence.runtime_scope.world_id().as_bytes() =>
            {
                Some(uuid_text(&row.try_get::<String, _>("account_id")?)?)
            }
            _ => None,
        };
        if target_account != edit.account {
            return Err(GuildError::NotCurrent);
        }
        let rank_exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guild_ranks WHERE guild_id=encode($1,'hex')::uuid AND level=$2)")
            .bind(request.guild_id.as_bytes().as_slice()).bind(i16::from(desired)).fetch_one(&mut **tx).await?;
        let allowed = match (&acting, &subject) {
            (Some(a), Some(t)) if target_account.is_some() && rank_exists => {
                GuildRank::new(a.rank).map_err(|_| invalid())?.may_set_rank(
                    GuildRank::new(t.rank).map_err(|_| invalid())?,
                    edit.request.level,
                )
            }
            _ => false,
        };
        let refusal = if !allowed {
            Some(GuildResult::NotAllowed)
        } else if edit.request.expected_revision != previous {
            Some(GuildResult::StaleRevision)
        } else {
            None
        };
        if let Some(result) = refusal {
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
        let before = subject.as_ref().ok_or_else(invalid)?.rank;
        let account = target_account.ok_or_else(invalid)?;
        let guard: Option<i32> = sqlx::query_scalar("SELECT 1 FROM game_character_account_guards WHERE account_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(account.as_slice()).fetch_optional(&mut **tx).await?;
        if guard.is_none() {
            return Err(GuildError::NotCurrent);
        }
        let position = sqlx::query("SELECT guild_id::text,character_id::text FROM game_account_guild_leadership WHERE account_id=encode($1,'hex')::uuid FOR UPDATE")
            .bind(account.as_slice()).fetch_optional(&mut **tx).await?;
        if before == 2 {
            let Some(row) = &position else {
                return Err(invalid());
            };
            if uuid_text(&row.try_get::<String, _>("guild_id")?)? != *request.guild_id.as_bytes()
                || uuid_text(&row.try_get::<String, _>("character_id")?)? != *target.as_bytes()
            {
                return Err(invalid());
            }
        }
        let promoting = before != 2 && desired == 2;
        let refusal = if promoting && position.is_some() {
            Some(GuildResult::AccountHasPosition)
        } else if promoting
            && !matches!(
                edit.premium,
                Some(PremiumStatus::Current | PremiumStatus::NotActivated(_))
            )
        {
            Some(GuildResult::NotPremium)
        } else {
            None
        };
        if let Some(result) = refusal {
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
        let vices: i64 = sqlx::query_scalar("SELECT count(*) FROM game_guild_members WHERE guild_id=encode($1,'hex')::uuid AND rank_level=2")
            .bind(request.guild_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
        let after_vices = vices - i64::from(before == 2) + i64::from(desired == 2);
        let now: i64 = sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM transaction_timestamp())*1000)::bigint",
        )
        .fetch_one(&mut **tx)
        .await?;
        if state
            .ensure_vice_count_change_allowed(
                usize::try_from(after_vices).map_err(|_| invalid())?,
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
        // GUILD0 §3.3 counts actual vice memberships, including retained ranks after Premium lapse.
        // Premium leadership counts belong only to the daily job; never write its deficit here.
        let activates = state == GuildState::Forming && after_vices >= 4;
        let next_state = if activates {
            2_i16
        } else {
            guild.try_get::<i16, _>("state")?
        };
        let old_deficit: Option<i64> = guild.try_get("vice_deficit_since")?;
        let deficit = if next_state == 2 {
            if after_vices >= 4 {
                None
            } else {
                old_deficit.or(Some(now))
            }
        } else {
            old_deficit
        };
        let next = previous.checked_add(1).ok_or_else(invalid)?;
        let audit = GuildRankAuditProposal {
            operation: GuildAuditProposal {
                version: 1,
                occurrence: request.identity.occurrence,
                transaction_id: request.identity.transaction_id,
                event_id: request.identity.event_id,
                guild_id: *request.guild_id.as_bytes(),
                world_id: *fence.runtime_scope.world_id().as_bytes(),
                actor: *actor.as_bytes(),
                target: *target.as_bytes(),
                action: 9,
                previous_revision: previous,
                revision: next,
                rank_before: Some(before),
                rank_after: Some(desired),
                removed_invitations: 0,
                occurred_at_unix_ms: request.identity.occurred_at_unix_ms,
            },
            expected_revision: edit.request.expected_revision,
        }
        .checked_bytes()
        .ok_or_else(invalid)?;
        let outcome = store_operation(
            tx,
            fence,
            request,
            binding,
            GuildResult::Ok,
            OperationPayload {
                revisions: (Some(previous), Some(next)),
                audit: Some(&audit),
                found_name: None,
                text: None,
                rank: Some((edit.request, before)),
                resign: None,
                edit_ranks: None,
                disband: None,
            },
        )
        .await?;
        sqlx::query("UPDATE game_guild_members SET rank_level=$3 WHERE guild_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid")
            .bind(request.guild_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).bind(i16::from(desired)).execute(&mut **tx).await?;
        if promoting {
            sqlx::query("INSERT INTO game_account_guild_leadership(account_id,guild_id,character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid)")
                .bind(account.as_slice()).bind(request.guild_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).execute(&mut **tx).await?;
        } else if before == 2 && desired != 2 {
            let changed = sqlx::query("DELETE FROM game_account_guild_leadership WHERE account_id=encode($1,'hex')::uuid AND guild_id=encode($2,'hex')::uuid AND character_id=encode($3,'hex')::uuid")
                .bind(account.as_slice()).bind(request.guild_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).execute(&mut **tx).await?;
            if changed.rows_affected() != 1 {
                return Err(invalid());
            }
        }
        sqlx::query("UPDATE game_guilds SET revision=$2::text::numeric,state=$3,vice_deficit_since=$4 WHERE guild_id=encode($1,'hex')::uuid")
            .bind(request.guild_id.as_bytes().as_slice()).bind(next.to_string()).bind(next_state).bind(deficit).execute(&mut **tx).await?;
        return Ok(outcome);
    }
    if let Some(text) = text {
        let allowed = match request.operation {
            GuildCommand::SetTitle(_) => {
                acting.as_ref().is_some_and(|m| m.rank == 1) && subject.is_some()
            }
            GuildCommand::SetMessage => acting.as_ref().is_some_and(|m| m.rank <= 2),
            _ => return Err(GuildError::InvalidInput),
        };
        let refusal = if !allowed {
            Some(GuildResult::NotAllowed)
        } else if text.expected_revision != previous {
            Some(GuildResult::StaleRevision)
        } else {
            None
        };
        if let Some(result) = refusal {
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
        let next = previous.checked_add(1).ok_or_else(invalid)?;
        let audit = GuildTextAuditProposal {
            operation: GuildAuditProposal {
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
                rank_before: None,
                rank_after: None,
                removed_invitations: 0,
                occurred_at_unix_ms: request.identity.occurred_at_unix_ms,
            },
            expected_revision: text.expected_revision,
            text: text.text.clone(),
        }
        .checked_bytes()
        .ok_or_else(invalid)?;
        let outcome = store_operation(
            tx,
            fence,
            request,
            binding,
            GuildResult::Ok,
            OperationPayload {
                revisions: (Some(previous), Some(next)),
                audit: Some(&audit),
                found_name: None,
                text: Some(text),
                rank: None,
                resign: None,
                edit_ranks: None,
                disband: None,
            },
        )
        .await?;
        if matches!(request.operation, GuildCommand::SetTitle(_)) {
            let changed = sqlx::query("UPDATE game_guild_members SET title=$3 WHERE guild_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid")
                .bind(request.guild_id.as_bytes().as_slice()).bind(target.as_bytes().as_slice()).bind(&text.text).execute(&mut **tx).await?.rows_affected();
            if changed != 1 {
                return Err(invalid());
            }
            sqlx::query("UPDATE game_guilds SET revision=$2::text::numeric WHERE guild_id=encode($1,'hex')::uuid")
                .bind(request.guild_id.as_bytes().as_slice()).bind(next.to_string()).execute(&mut **tx).await?;
        } else {
            sqlx::query("UPDATE game_guilds SET motd=$3,revision=$2::text::numeric WHERE guild_id=encode($1,'hex')::uuid")
                .bind(request.guild_id.as_bytes().as_slice()).bind(next.to_string()).bind(&text.text).execute(&mut **tx).await?;
        }
        return Ok(outcome);
    }
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
    } else if matches!(
        request.operation,
        GuildCommand::Revoke(_) | GuildCommand::Decline
    ) {
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
        GuildCommand::Decline => {
            // Accepted decline requires an existing row; it does not require unexpired time,
            // membership/leader rights, or a mutation of the actor's other invitations.
            if own_invitation.is_none() {
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
        GuildCommand::Found
        | GuildCommand::SetTitle(_)
        | GuildCommand::SetMessage
        | GuildCommand::SetRank(_)
        | GuildCommand::Resign(_)
        | GuildCommand::EditRanks
        | GuildCommand::Disband => {
            return Err(GuildError::InvalidInput);
        }
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
    let audit_bytes = if request.operation == GuildCommand::Decline {
        GuildDeclineAuditProposal { operation: audit }.checked_bytes()
    } else {
        audit.checked_bytes()
    }
    .ok_or_else(invalid)?;
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
    if request.operation == GuildCommand::Decline {
        let deleted = sqlx::query("DELETE FROM game_guild_invitations WHERE guild_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid")
            .bind(request.guild_id.as_bytes().as_slice()).bind(actor.as_bytes().as_slice()).execute(&mut **tx).await?.rows_affected();
        if deleted != 1 {
            return Err(invalid());
        }
    } else if request.operation != GuildCommand::Accept {
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
        GuildCommand::Revoke(_) | GuildCommand::Decline => {}
        GuildCommand::Found
        | GuildCommand::SetTitle(_)
        | GuildCommand::SetMessage
        | GuildCommand::SetRank(_)
        | GuildCommand::Resign(_)
        | GuildCommand::EditRanks
        | GuildCommand::Disband => {
            return Err(GuildError::InvalidInput);
        }
    }
    if request.operation == GuildCommand::Accept {
        remove_invitations(tx, target, &removes).await?;
    }
    if request.operation == GuildCommand::Decline {
        // Declining changes no vice/lifecycle/Premium facts, including an existing first deficit.
        let changed = sqlx::query("UPDATE game_guilds SET revision=$2::text::numeric WHERE guild_id=encode($1,'hex')::uuid")
            .bind(request.guild_id.as_bytes().as_slice()).bind(next.to_string()).execute(&mut **tx).await?.rows_affected();
        if changed != 1 {
            return Err(invalid());
        }
        return Ok(outcome);
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
            text: None,
            rank: None,
            resign: None,
            edit_ranks: None,
            disband: None,
        },
    )
    .await
}

struct OperationPayload<'a> {
    revisions: (Option<u64>, Option<u64>),
    audit: Option<&'a [u8]>,
    found_name: Option<&'a str>,
    text: Option<&'a GuildTextRequest>,
    rank: Option<(&'a GuildRankRequest, u8)>,
    resign: Option<&'a GuildResignRequest>,
    edit_ranks: Option<&'a GuildEditRanksRequest>,
    disband: Option<&'a GuildDisbandRequest>,
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
        text,
        rank,
        resign,
        edit_ranks,
        disband,
    } = payload;
    let event = audit.map(|_| request.identity.event_id);
    sqlx::query("INSERT INTO game_guild_operations(occurrence,transaction_id,requested_event_id,game_session_id,command_id,actor,world_id,guild_id,target,action,request_binding,result,previous_revision,revision,event_id,audit_sha256,found_name,expected_revision,edit_text,previous_rank,edit_rank,edit_rank_names,disband_confirmation) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid,encode($7,'hex')::uuid,encode($8,'hex')::uuid,encode($9,'hex')::uuid,$10,$11,$12,$13::text::numeric,$14::text::numeric,encode($15,'hex')::uuid,sha256($16),$17,$18::text::numeric,$19,$20,$21,$22,$23)")
        .bind(request.identity.occurrence.as_slice()).bind(request.identity.transaction_id.as_slice()).bind(request.identity.event_id.as_slice())
        .bind(request.command.game_session_id().as_bytes().as_slice()).bind(request.command.command_id().get().to_string())
        .bind(fence.character_id.as_bytes().as_slice()).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(request.guild_id.as_bytes().as_slice())
        .bind(request.operation.target(fence.character_id).as_bytes().as_slice()).bind(request.operation.code()).bind(binding).bind(result.code())
        .bind(previous.map(|x|x.to_string())).bind(revision.or(previous).map(|x|x.to_string())).bind(event.map(|x|x.to_vec())).bind(audit).bind(found_name).bind(text.map(|t| t.expected_revision).or_else(||rank.map(|(r,_)|r.expected_revision)).or_else(||resign.map(|r|r.expected_revision)).or_else(||edit_ranks.map(|r|r.expected_revision)).map(|r|r.to_string())).bind(text.map(|t| t.text.as_str())).bind(rank.map(|(_,before)|i16::from(before)).or_else(||resign.map(|_|2_i16))).bind(rank.map(|(r,_)|i16::from(r.level.level())).or_else(||resign.map(|_|1_i16))).bind(edit_ranks.map(|r|r.names.as_slice())).bind(disband.map(|r|r.confirmation.as_str()))
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

/// GUILD0-RL-09: one World deadline scan admits at most 100 Guild candidates.
pub(crate) const GUILD_JOB_PASS_MAX: i64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuildDeadlineOutcome {
    Started,
    AlreadyStarted,
    NoLongerDue,
}

impl DurabilityRoot {
    /// Candidate discovery takes no Guild locks. The worker must recheck under the Guild lock.
    pub(crate) async fn guild_deadline_candidates(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: crate::foundation::WorldId,
    ) -> Result<Vec<GuildId>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        Ok(self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if !super::runtime_scope_assignment::prove_current_incarnation(&mut tx, &node).await? {
                    return Err(DurabilityError::Unavailable);
                }
                let rows: Vec<String> = sqlx::query_scalar("SELECT guild_id::text FROM game_guilds WHERE world_id=encode($1,'hex')::uuid AND ((state=1 AND formation_deadline<floor(extract(epoch FROM transaction_timestamp())*1000)::bigint) OR (state=2 AND vice_deficit_since IS NOT NULL AND floor(extract(epoch FROM transaction_timestamp())*1000)::numeric-vice_deficit_since::numeric>1209600000)) ORDER BY guild_id LIMIT $2")
                    .bind(world.as_bytes().as_slice()).bind(GUILD_JOB_PASS_MAX).fetch_all(&mut *tx).await?;
                let ids = rows.into_iter().map(|id| {
                    GuildId::from_bytes(uuid_text(&id)?).map_err(|_| DurabilityError::InvalidStoredState)
                }).collect::<std::result::Result<Vec<_>, _>>()?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(ids)
            })
        }).await?)
    }

    /// Only accepted GUILD0 §3.4 start: freeze commands and retain the pending step durably.
    /// Bank payout and Hall disposition must finish before any future terminal step is admitted.
    /// World jobs use current recovery/admission, never a Character session or retained receipt
    /// as authority (GUILDS_V1 §3; GUILD0 §4.1). The node proof attests only current process.
    pub(crate) async fn start_guild_deadline_disband(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: crate::foundation::WorldId,
        guild: GuildId,
    ) -> Result<GuildDeadlineOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !super::runtime_scope_assignment::prove_current_incarnation(&mut tx, &node)
                        .await?
                    {
                        return Err(DurabilityError::Unavailable);
                    }
                    let outcome = start_deadline_disband(&mut tx, world, guild).await;
                    match outcome {
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

    /// Reload pending starts after a process/session change. This is a bounded delivery query,
    /// not a completion API: no Bank/Hall closure or external work is inferred from a checkpoint.
    pub(crate) async fn pending_guild_disbands(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: crate::foundation::WorldId,
    ) -> Result<Vec<GuildId>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        Ok(self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if !super::runtime_scope_assignment::prove_current_incarnation(&mut tx, &node).await? {
                    return Err(DurabilityError::Unavailable);
                }
                let rows: Vec<String> = sqlx::query_scalar("SELECT j.guild_id::text FROM game_guild_job_checkpoints j JOIN game_guilds g USING(guild_id) WHERE j.world_id=encode($1,'hex')::uuid AND g.world_id=j.world_id AND g.state=3 ORDER BY j.guild_id LIMIT $2")
                    .bind(world.as_bytes().as_slice()).bind(GUILD_JOB_PASS_MAX).fetch_all(&mut *tx).await?;
                let ids = rows.into_iter().map(|id| {
                    GuildId::from_bytes(uuid_text(&id)?).map_err(|_| DurabilityError::InvalidStoredState)
                }).collect::<std::result::Result<Vec<_>, _>>()?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(ids)
            })
        }).await?)
    }
}

async fn start_deadline_disband(
    tx: &mut Transaction<'_, Postgres>,
    world: crate::foundation::WorldId,
    guild: GuildId,
) -> Result<GuildDeadlineOutcome> {
    // Occurrence first, Guild second. Separate candidate transactions preserve the full order.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('guild-disband-start:'||encode($1,'hex'),0))")
        .bind(guild.as_bytes().as_slice()).execute(&mut **tx).await?;
    let Some(row) = sqlx::query("SELECT world_id::text,state,revision::text,formation_deadline,vice_deficit_since,floor(extract(epoch FROM transaction_timestamp())*1000)::bigint AS now FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(guild.as_bytes().as_slice()).fetch_optional(&mut **tx).await? else {
        return Ok(GuildDeadlineOutcome::NoLongerDue);
    };
    if uuid_text(&row.try_get::<String, _>("world_id")?)? != *world.as_bytes() {
        return Ok(GuildDeadlineOutcome::NoLongerDue);
    }
    let state = match row.try_get::<i16, _>("state")? {
        1 => GuildState::Forming,
        2 => GuildState::Active,
        3 => {
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guild_job_checkpoints WHERE guild_id=encode($1,'hex')::uuid AND step=1)")
                .bind(guild.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            return Ok(if exists {
                GuildDeadlineOutcome::AlreadyStarted
            } else {
                GuildDeadlineOutcome::NoLongerDue
            });
        }
        4 => return Ok(GuildDeadlineOutcome::NoLongerDue),
        _ => return Err(invalid()),
    };
    let job = if state == GuildState::Forming {
        crate::domain::guild::GuildDeadlineJob::Formation
    } else {
        crate::domain::guild::GuildDeadlineJob::ViceDeficit
    };
    let now: i64 = row.try_get("now")?;
    if !job.due(
        state,
        row.try_get("formation_deadline")?,
        row.try_get("vice_deficit_since")?,
        now,
    ) {
        return Ok(GuildDeadlineOutcome::NoLongerDue);
    }
    let previous = numeric_u64(&row, "revision")?;
    let revision = previous.checked_add(1).ok_or_else(invalid)?;
    let audit = super::guild_audit::GuildJobAuditProposal {
        version: 1,
        guild_id: *guild.as_bytes(),
        world_id: *world.as_bytes(),
        step: 1,
        reason: job.code(),
        previous_revision: previous,
        revision,
        occurred_at_unix_ms: now,
    }
    .checked_bytes()
    .ok_or_else(invalid)?;
    sqlx::query("INSERT INTO game_guild_job_checkpoints(guild_id,step,world_id,reason,previous_revision,revision,occurred_at) VALUES(encode($1,'hex')::uuid,1,encode($2,'hex')::uuid,$3,$4::text::numeric,$5::text::numeric,$6)")
        .bind(guild.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(job.code())
        .bind(previous.to_string()).bind(revision.to_string()).bind(now).execute(&mut **tx).await?;
    sqlx::query("UPDATE game_guilds SET state=3,revision=$2::text::numeric WHERE guild_id=encode($1,'hex')::uuid")
        .bind(guild.as_bytes().as_slice()).bind(revision.to_string()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_guild_job_event_outbox(guild_id,step,payload,payload_sha256) VALUES(encode($1,'hex')::uuid,1,$2,sha256($2))")
        .bind(guild.as_bytes().as_slice()).bind(&audit).execute(&mut **tx).await?;
    Ok(GuildDeadlineOutcome::Started)
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
 PRIMARY KEY(guild_id,level),CONSTRAINT game_guild_rank_name_unique UNIQUE(guild_id,name_key) DEFERRABLE INITIALLY IMMEDIATE);
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
CREATE FUNCTION game_guild_rank_names_valid(names TEXT[]) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE AS $$
 SELECT coalesce(array_ndims(names)=1 AND array_lower(names,1)=1 AND cardinality(names) BETWEEN 3 AND 20
   AND NOT EXISTS(SELECT 1 FROM unnest(names) AS n(value) WHERE value IS NULL OR value='')
   AND (SELECT count(DISTINCT game_character_name_key(value)) FROM unnest(names) AS n(value))=cardinality(names),false)
$$;
CREATE TABLE game_guild_operations(
 occurrence UUID PRIMARY KEY CHECK(game_character_is_uuid_v7(occurrence)),
 transaction_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(transaction_id)),
 requested_event_id UUID NOT NULL UNIQUE CHECK(game_character_is_uuid_v7(requested_event_id)),
 game_session_id UUID NOT NULL CHECK(game_character_is_uuid_v7(game_session_id)),
 command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 actor UUID NOT NULL REFERENCES game_character_roots,world_id UUID NOT NULL,guild_id UUID NOT NULL,
 target UUID NOT NULL CHECK(game_character_is_uuid_v7(target)),action SMALLINT NOT NULL CHECK(action BETWEEN 1 AND 13),
 request_binding BYTEA NOT NULL CHECK(octet_length(request_binding)=32),result SMALLINT NOT NULL CHECK(result BETWEEN 0 AND 12),
 previous_revision NUMERIC(20,0),revision NUMERIC(20,0),event_id UUID UNIQUE,audit_sha256 BYTEA,
 found_name TEXT CHECK((action=6 AND found_name IS NOT NULL AND game_character_is_name(found_name)) OR (action<>6 AND found_name IS NULL)),
 expected_revision NUMERIC(20,0),edit_text TEXT,previous_rank SMALLINT,edit_rank SMALLINT,edit_rank_names TEXT[],disband_confirmation TEXT,
 CHECK(CASE WHEN action=13 AND result=0 THEN disband_confirmation IS NOT NULL AND char_length(disband_confirmation)<=29 ELSE disband_confirmation IS NULL END),
 CHECK(CASE WHEN action=11 AND result=0 THEN game_guild_rank_names_valid(edit_rank_names) ELSE edit_rank_names IS NULL END),
 CHECK(CASE WHEN action IN(7,8) AND result=0 THEN expected_revision IS NOT NULL AND expected_revision=previous_revision AND expected_revision>=1 AND edit_text IS NOT NULL
   AND ((action=7 AND char_length(edit_text)<=29) OR (action=8 AND char_length(edit_text) BETWEEN 1 AND 255 AND octet_length(edit_text)<=1020))
   WHEN action IN(9,10,11) AND result=0 THEN expected_revision IS NOT NULL AND expected_revision=previous_revision AND expected_revision>=1 AND edit_text IS NULL
   ELSE expected_revision IS NULL AND edit_text IS NULL END),
 CHECK(CASE WHEN action=9 AND result=0 THEN previous_rank BETWEEN 2 AND 20 AND previous_rank IS NOT NULL AND edit_rank BETWEEN 2 AND 20 AND edit_rank IS NOT NULL WHEN action=10 AND result=0 THEN previous_rank=2 AND previous_rank IS NOT NULL AND edit_rank=1 AND edit_rank IS NOT NULL ELSE previous_rank IS NULL AND edit_rank IS NULL END),
 CHECK(result<>12 OR action IN(7,8,9,10,11)),
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
-- Pending start is retained even while external Hall/Bank composition is unavailable.
-- There is deliberately no terminalizer, fake closure port, or public event allocation.
CREATE TABLE game_guild_job_checkpoints(
 guild_id UUID NOT NULL REFERENCES game_guilds,step SMALLINT NOT NULL CHECK(step=1),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),
 reason SMALLINT NOT NULL CHECK(reason IN(1,2,3)),
 previous_revision NUMERIC(20,0) NOT NULL CHECK(previous_revision BETWEEN 1 AND 18446744073709551614),
 revision NUMERIC(20,0) NOT NULL CHECK(revision=previous_revision+1),
 occurred_at BIGINT NOT NULL CHECK(occurred_at>0),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),PRIMARY KEY(guild_id,step));
CREATE TABLE game_guild_job_event_outbox(
 guild_id UUID NOT NULL,step SMALLINT NOT NULL,
 payload BYTEA NOT NULL CHECK(octet_length(payload) BETWEEN 1 AND 4096),
 payload_sha256 BYTEA NOT NULL CHECK(payload_sha256=sha256(payload)),
 created_xact_id xid8 NOT NULL DEFAULT pg_current_xact_id(),PRIMARY KEY(guild_id,step),
 FOREIGN KEY(guild_id,step) REFERENCES game_guild_job_checkpoints);
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
DECLARE o game_guild_operations%ROWTYPE;r SMALLINT;w UUID;n BIGINT;now_millis BIGINT;rank_state SMALLINT;rank_deficit BIGINT;j game_guild_job_checkpoints%ROWTYPE;
BEGIN
 IF TG_TABLE_NAME='game_guilds' AND TG_OP='UPDATE' THEN
  SELECT * INTO j FROM game_guild_job_checkpoints WHERE guild_id=OLD.guild_id AND step=1 AND created_xact_id=pg_current_xact_id();
  IF FOUND THEN
   IF OLD.state NOT IN(1,2) OR NEW.state<>3 OR OLD.world_id<>j.world_id
      OR OLD.revision<>j.previous_revision OR NEW.revision<>j.revision
      OR (to_jsonb(OLD)-'state'-'revision'-'name_key') IS DISTINCT FROM (to_jsonb(NEW)-'state'-'revision'-'name_key') THEN
    RAISE EXCEPTION 'Guild job only freezes its due Guild at the exact next revision' USING ERRCODE='23514';END IF;
   RETURN NEW;
  END IF;
 END IF;
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
  IF o.action=12 THEN
   IF TG_OP<>'UPDATE' OR OLD.guild_id<>o.guild_id OR OLD.world_id<>o.world_id OR OLD.state NOT IN(1,2)
      OR o.actor<>o.target OR OLD.revision<>o.previous_revision OR NEW.revision<>o.revision
      OR (to_jsonb(OLD)-'revision'-'name_key') IS DISTINCT FROM (to_jsonb(NEW)-'revision'-'name_key') THEN
    RAISE EXCEPTION 'Decline changes only its locked Guild revision' USING ERRCODE='23514';END IF;
   RETURN NEW;
  END IF;
  IF o.action=11 THEN
   IF TG_OP<>'UPDATE' OR OLD.guild_id<>o.guild_id OR OLD.world_id<>o.world_id OR OLD.state NOT IN(1,2)
      OR OLD.revision IS DISTINCT FROM o.expected_revision OR OLD.revision IS DISTINCT FROM o.previous_revision
      OR NEW.revision IS DISTINCT FROM o.revision OR o.target<>o.actor
      OR (to_jsonb(OLD)-'revision'-'name_key') IS DISTINCT FROM (to_jsonb(NEW)-'revision'-'name_key')
      OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor AND rank_level=1) THEN
    RAISE EXCEPTION 'EditRanks changes only its current leader Guild revision' USING ERRCODE='23514';END IF;
   RETURN NEW;
  END IF;
  IF o.action=10 THEN
   IF TG_OP<>'UPDATE' OR OLD.guild_id<>o.guild_id OR OLD.world_id<>o.world_id OR OLD.state NOT IN(1,2)
      OR OLD.revision<>o.expected_revision OR OLD.revision<>o.previous_revision OR NEW.revision<>o.revision
      OR (to_jsonb(OLD)-'revision'-'name_key')<>(to_jsonb(NEW)-'revision'-'name_key')
      OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor AND rank_level=2)
      OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.target AND rank_level=1) THEN
    RAISE EXCEPTION 'Resign changes only revision after its exact fixed role swap' USING ERRCODE='23514';END IF;
   RETURN NEW;
  END IF;
  IF o.action=9 THEN
   SELECT rank_level INTO r FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor;
   SELECT count(*) INTO n FROM game_guild_members WHERE guild_id=o.guild_id AND rank_level=2;
   now_millis=floor(extract(epoch FROM transaction_timestamp())*1000)::bigint;
   rank_state=CASE WHEN OLD.state=1 AND n>=4 THEN 2 ELSE OLD.state END;
   rank_deficit=CASE WHEN rank_state=2 AND n>=4 THEN NULL WHEN rank_state=2 THEN coalesce(OLD.vice_deficit_since,now_millis) ELSE OLD.vice_deficit_since END;
   IF TG_OP<>'UPDATE' OR OLD.guild_id<>o.guild_id OR OLD.world_id<>o.world_id OR OLD.state NOT IN(1,2)
      OR OLD.revision<>o.expected_revision OR OLD.revision<>o.previous_revision OR NEW.revision<>o.revision
      OR NOT coalesce(r<o.previous_rank AND r<o.edit_rank,false)
      OR NEW.state<>rank_state OR NEW.vice_deficit_since IS DISTINCT FROM rank_deficit
      OR (to_jsonb(OLD)-'revision'-'state'-'vice_deficit_since'-'name_key')<>(to_jsonb(NEW)-'revision'-'state'-'vice_deficit_since'-'name_key')
      OR (n>=4 AND ((OLD.state=1 AND now_millis>OLD.formation_deadline)
          OR (OLD.state=2 AND OLD.vice_deficit_since IS NOT NULL AND now_millis::numeric-OLD.vice_deficit_since::numeric>1209600000))) THEN
    RAISE EXCEPTION 'rank edit requires actual roles/revision and exact synchronous lifecycle' USING ERRCODE='23514';END IF;
   RETURN NEW;
  END IF;
  IF o.action IN(7,8) THEN
   SELECT rank_level INTO r FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor;
   IF TG_OP<>'UPDATE' OR OLD.guild_id<>o.guild_id OR OLD.world_id<>o.world_id OR OLD.state NOT IN(1,2)
      OR OLD.revision<>o.expected_revision OR OLD.revision<>o.previous_revision OR NEW.revision<>o.revision
      OR NOT coalesce((o.action=7 AND r=1 AND EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.target))
         OR (o.action=8 AND r<=2 AND o.target=o.actor),false) THEN
    RAISE EXCEPTION 'text edit requires its actual current Guild/role/revision' USING ERRCODE='23514';END IF;
   IF o.action=7 THEN
    IF (to_jsonb(OLD)-'revision'-'name_key')<>(to_jsonb(NEW)-'revision'-'name_key') THEN
     RAISE EXCEPTION 'title edit changes no other Guild field' USING ERRCODE='23514';END IF;
   ELSE
    IF NEW.motd IS DISTINCT FROM o.edit_text OR (to_jsonb(OLD)-'revision'-'name_key'-'motd')<>(to_jsonb(NEW)-'revision'-'name_key'-'motd') THEN
     RAISE EXCEPTION 'message edit changes only its exact bound text' USING ERRCODE='23514';END IF;
   END IF;
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
  ELSIF TG_OP='UPDATE' AND o.action=11 THEN
   IF OLD.guild_id<>o.guild_id OR OLD.rank_level<=cardinality(o.edit_rank_names)
      OR NEW.rank_level IS DISTINCT FROM cardinality(o.edit_rank_names)
      OR (to_jsonb(OLD)-'rank_level') IS DISTINCT FROM (to_jsonb(NEW)-'rank_level')
      OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=o.guild_id AND world_id=o.world_id AND state IN(1,2) AND revision=o.expected_revision)
      OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor AND rank_level=1) THEN
    RAISE EXCEPTION 'EditRanks moves only removed-tail members to exact new lowest' USING ERRCODE='23514';END IF;
   RETURN NEW;
  ELSIF TG_OP='UPDATE' AND o.action=10 THEN
   IF OLD.guild_id<>o.guild_id OR (to_jsonb(OLD)-'rank_level')<>(to_jsonb(NEW)-'rank_level')
      OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=o.guild_id AND world_id=o.world_id AND state IN(1,2) AND revision=o.expected_revision)
      OR NOT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=OLD.character_id AND world_id=o.world_id AND lifecycle=1)
      OR NOT ((OLD.character_id=o.actor AND OLD.rank_level=1 AND NEW.rank_level=2
                AND EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.target AND rank_level=2))
          OR (OLD.character_id=o.target AND OLD.rank_level=2 AND NEW.rank_level=1
                AND EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor AND rank_level=2)
                AND NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND rank_level=1))) THEN
    RAISE EXCEPTION 'Resign requires exact ordered current leader/vice swap' USING ERRCODE='23514';END IF;
   RETURN NEW;
  ELSIF TG_OP='UPDATE' AND o.action=9 THEN
   SELECT rank_level INTO r FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor;
   IF OLD.guild_id<>o.guild_id OR OLD.character_id<>o.target OR OLD.rank_level<>o.previous_rank OR NEW.rank_level<>o.edit_rank
      OR NOT coalesce(r<OLD.rank_level AND r<NEW.rank_level,false)
      OR (to_jsonb(OLD)-'rank_level')<>(to_jsonb(NEW)-'rank_level')
      OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=o.guild_id AND world_id=o.world_id AND state IN(1,2) AND revision=o.expected_revision)
      OR NOT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=o.target AND world_id=o.world_id AND lifecycle=1) THEN
    RAISE EXCEPTION 'rank edit requires its exact current member/role/bound levels' USING ERRCODE='23514';END IF;
   RETURN NEW;
  ELSIF TG_OP='UPDATE' AND o.action=7 THEN
   SELECT rank_level INTO r FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor;
   IF r IS DISTINCT FROM 1 OR OLD.guild_id<>o.guild_id OR OLD.character_id<>o.target OR NEW.title IS DISTINCT FROM o.edit_text
      OR (to_jsonb(OLD)-'title')<>(to_jsonb(NEW)-'title')
      OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=o.guild_id AND world_id=o.world_id AND state IN(1,2) AND revision=o.expected_revision) THEN
    RAISE EXCEPTION 'title requires the current leader and exact member/text' USING ERRCODE='23514';END IF;
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
      OR (o.action=12 AND o.actor=o.target AND OLD.guild_id=o.guild_id)
      OR (o.action=3 AND o.actor=o.target)) THEN RETURN OLD;
  END IF;
 ELSIF TG_TABLE_NAME='game_guild_ranks' AND o.action=11 THEN
  IF NOT game_guild_rank_names_valid(o.edit_rank_names) OR o.target<>o.actor
     OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=o.guild_id AND world_id=o.world_id AND state IN(1,2) AND revision=o.expected_revision)
     OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.actor AND rank_level=1) THEN
   RAISE EXCEPTION 'EditRanks definitions require their current leader and bound array' USING ERRCODE='23514';END IF;
  IF TG_OP='DELETE' THEN
   IF OLD.guild_id<>o.guild_id OR OLD.level<=cardinality(o.edit_rank_names) THEN
    RAISE EXCEPTION 'EditRanks deletes only removed-tail definitions' USING ERRCODE='23514';END IF;
   RETURN OLD;
  END IF;
  IF NEW.guild_id<>o.guild_id OR NEW.level>cardinality(o.edit_rank_names)
     OR NEW.name IS DISTINCT FROM o.edit_rank_names[NEW.level] THEN
   RAISE EXCEPTION 'EditRanks definition must equal exact ordered name' USING ERRCODE='23514';END IF;
  IF TG_OP='UPDATE' AND (to_jsonb(OLD)-'name'-'name_key') IS DISTINCT FROM (to_jsonb(NEW)-'name'-'name_key') THEN
   RAISE EXCEPTION 'EditRanks retains level and Guild identity' USING ERRCODE='23514';END IF;
  RETURN NEW;
 ELSIF TG_TABLE_NAME='game_guild_ranks' AND TG_OP='INSERT' THEN
  IF o.action<>6 OR NEW.guild_id<>o.guild_id OR NEW.name IS DISTINCT FROM (CASE NEW.level WHEN 1 THEN 'Leader' WHEN 2 THEN 'Vice Leader' WHEN 3 THEN 'Member' END) THEN
   RAISE EXCEPTION 'Found must create its three bound default ranks' USING ERRCODE='23514';END IF;
  RETURN NEW;
 ELSIF TG_TABLE_NAME='game_account_guild_leadership' AND TG_OP='INSERT' THEN
  IF o.action=9 AND o.previous_rank<>2 AND o.edit_rank=2 AND NEW.guild_id=o.guild_id AND NEW.character_id=o.target
     AND NEW.account_id=(SELECT account_id FROM game_character_roots WHERE character_id=o.target)
     AND EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.target AND rank_level=2) THEN RETURN NEW;END IF;
  IF o.action=6 AND NEW.guild_id=o.guild_id AND NEW.character_id=o.actor
     AND NEW.account_id=(SELECT account_id FROM game_character_roots WHERE character_id=o.actor) THEN RETURN NEW;END IF;
 ELSIF TG_TABLE_NAME='game_account_guild_leadership' AND TG_OP='DELETE' THEN
  IF o.action=9 AND o.previous_rank=2 AND o.edit_rank<>2 AND OLD.guild_id=o.guild_id AND OLD.character_id=o.target
     AND OLD.account_id=(SELECT account_id FROM game_character_roots WHERE character_id=o.target)
     AND EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=o.guild_id AND character_id=o.target AND rank_level=o.edit_rank) THEN RETURN OLD;END IF;
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
 IF members>2000 OR ranks NOT BETWEEN 3 AND 20
    OR (SELECT min(level) FROM game_guild_ranks WHERE guild_id=g) IS DISTINCT FROM 1
    OR (SELECT max(level) FROM game_guild_ranks WHERE guild_id=g) IS DISTINCT FROM ranks
    OR (s<>4 AND leaders<>1)
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

-- The real writer has already locked the Guild and this invitation in accepted order.
-- A newly proposed success must also see that existing row, not merely a final empty snapshot.
CREATE FUNCTION game_guild_disband_precondition() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.action=13 AND NEW.result=0 AND (NEW.actor<>NEW.target OR NOT EXISTS(
    SELECT 1 FROM game_guilds g JOIN game_guild_members m USING(guild_id)
    WHERE g.guild_id=NEW.guild_id AND g.world_id=NEW.world_id AND g.state IN(1,2)
      AND g.revision=NEW.previous_revision AND g.name=NEW.disband_confirmation AND m.character_id=NEW.actor AND m.rank_level=1)) THEN
  RAISE EXCEPTION 'Disband requires exact current leader and bound name confirmation' USING ERRCODE='23514';END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_guild_disband_precondition BEFORE INSERT ON game_guild_operations FOR EACH ROW EXECUTE FUNCTION game_guild_disband_precondition();

CREATE FUNCTION game_guild_decline_precondition() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.action=12 AND NEW.result=0 AND (NEW.actor<>NEW.target OR NOT EXISTS(
    SELECT 1 FROM game_guild_invitations WHERE guild_id=NEW.guild_id AND character_id=NEW.actor)) THEN
  RAISE EXCEPTION 'Decline success requires its actual invited actor' USING ERRCODE='23514';END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_guild_decline_precondition BEFORE INSERT ON game_guild_operations FOR EACH ROW EXECUTE FUNCTION game_guild_decline_precondition();

CREATE FUNCTION game_guild_operation_proven() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NEW.result=0 THEN
  IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(SELECT 1 FROM game_guilds g WHERE g.guild_id=NEW.guild_id AND g.world_id=NEW.world_id AND ((NEW.action<>13 AND g.state IN(1,2)) OR (NEW.action=13 AND g.state=3)) AND g.revision=NEW.revision)
     OR NOT EXISTS(SELECT 1 FROM game_guild_event_outbox e WHERE e.occurrence=NEW.occurrence AND e.event_id=NEW.event_id AND e.guild_id=NEW.guild_id AND e.payload_sha256=NEW.audit_sha256 AND e.created_xact_id=pg_current_xact_id())
     OR (NEW.action=13 AND (NEW.actor<>NEW.target
       OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND rank_level=1)
       OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=NEW.guild_id AND name=NEW.disband_confirmation AND state=3)
       OR NOT EXISTS(SELECT 1 FROM game_guild_job_checkpoints WHERE guild_id=NEW.guild_id AND step=1 AND reason=3 AND world_id=NEW.world_id AND previous_revision=NEW.previous_revision AND revision=NEW.revision AND created_xact_id=pg_current_xact_id())))
     OR (NEW.action=1 AND NOT EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=NEW.guild_id AND character_id=NEW.target AND inviter=NEW.actor))
     OR (NEW.action=2 AND EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=NEW.guild_id AND character_id=NEW.target))
     OR (NEW.action=12 AND (NEW.actor<>NEW.target OR EXISTS(SELECT 1 FROM game_guild_invitations WHERE guild_id=NEW.guild_id AND character_id=NEW.actor)))
     OR (NEW.action=3 AND (NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND rank_level>=3) OR EXISTS(SELECT 1 FROM game_guild_invitations WHERE character_id=NEW.actor)))
     OR (NEW.action IN(4,5) AND EXISTS(SELECT 1 FROM game_guild_members WHERE character_id=NEW.target))
     OR (NEW.action=7 AND (NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.target AND title=NEW.edit_text)
       OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND rank_level=1)))
     OR (NEW.action=8 AND (NEW.target<>NEW.actor OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=NEW.guild_id AND motd=NEW.edit_text)
       OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND rank_level<=2)))
     OR (NEW.action IN(7,8,9,10,11) AND NEW.expected_revision IS DISTINCT FROM NEW.previous_revision)
     OR (NEW.action=9 AND (NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.target AND rank_level=NEW.edit_rank)
       OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND rank_level<NEW.previous_rank AND rank_level<NEW.edit_rank)))
     OR (NEW.action=11 AND (NEW.target<>NEW.actor OR NOT game_guild_rank_names_valid(NEW.edit_rank_names)
       OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND rank_level=1)
       OR (SELECT array_agg(name ORDER BY level) FROM game_guild_ranks WHERE guild_id=NEW.guild_id) IS DISTINCT FROM NEW.edit_rank_names
       OR (SELECT count(*) FROM game_guild_ranks WHERE guild_id=NEW.guild_id)<>cardinality(NEW.edit_rank_names)
       OR EXISTS(SELECT 1 FROM game_guild_ranks WHERE guild_id=NEW.guild_id AND level>cardinality(NEW.edit_rank_names))))
     OR (NEW.action=10 AND (NEW.actor=NEW.target OR NEW.previous_rank<>2 OR NEW.edit_rank<>1
       OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.actor AND rank_level=2)
       OR NOT EXISTS(SELECT 1 FROM game_guild_members WHERE guild_id=NEW.guild_id AND character_id=NEW.target AND rank_level=1)))
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
 IF NEW.created_xact_id<>pg_current_xact_id() OR NOT EXISTS(SELECT 1 FROM game_guild_operations o WHERE o.occurrence=NEW.occurrence AND o.result=0 AND o.guild_id=NEW.guild_id AND o.event_id=NEW.event_id AND o.audit_sha256=NEW.payload_sha256 AND o.created_xact_id=pg_current_xact_id()
   AND (o.action NOT IN(7,8) OR ((convert_from(NEW.payload,'UTF8')::jsonb->>'text') IS NOT DISTINCT FROM o.edit_text
      AND (convert_from(NEW.payload,'UTF8')::jsonb->>'expected_revision')::numeric IS NOT DISTINCT FROM o.expected_revision
      AND (convert_from(NEW.payload,'UTF8')::jsonb->>'action')::smallint=o.action))
   AND (o.action NOT IN(9,10) OR ((convert_from(NEW.payload,'UTF8')::jsonb->>'expected_revision')::numeric=o.expected_revision
      AND (convert_from(NEW.payload,'UTF8')::jsonb->>'rank_before')::smallint=o.previous_rank
      AND (convert_from(NEW.payload,'UTF8')::jsonb->>'rank_after')::smallint=o.edit_rank
      AND (convert_from(NEW.payload,'UTF8')::jsonb->>'action')::smallint=o.action))
   AND (o.action<>13 OR convert_from(NEW.payload,'UTF8')::jsonb =
     jsonb_build_object('version',1,'action',13,'previous_revision',o.previous_revision,'revision',o.revision,
       'rank_before',NULL::smallint,'rank_after',NULL::smallint,'removed_invitations',0,'occurred_at_unix_ms',NEW.occurred_at,
       'confirmation',o.disband_confirmation)
     || (SELECT jsonb_object_agg(field_name,encoded_bytes) FROM
       (SELECT field_name,jsonb_agg(get_byte(uuid_send(uuid_value),byte_index) ORDER BY byte_index) AS encoded_bytes
        FROM (VALUES ('occurrence',o.occurrence),('transaction_id',o.transaction_id),('event_id',o.event_id),
          ('guild_id',o.guild_id),('world_id',o.world_id),('actor',o.actor),('target',o.target)) AS ids(field_name,uuid_value)
        CROSS JOIN generate_series(0,15) AS bytes(byte_index) GROUP BY field_name,uuid_value) AS encoded_ids))
   AND (o.action<>12 OR convert_from(NEW.payload,'UTF8')::jsonb =
     jsonb_build_object('version',1,'action',12,'previous_revision',o.previous_revision,'revision',o.revision,
       'rank_before',NULL::smallint,'rank_after',NULL::smallint,'removed_invitations',1,'occurred_at_unix_ms',NEW.occurred_at)
     || (SELECT jsonb_object_agg(field_name,encoded_bytes) FROM
       (SELECT field_name,jsonb_agg(get_byte(uuid_send(uuid_value),byte_index) ORDER BY byte_index) AS encoded_bytes
        FROM (VALUES ('occurrence',o.occurrence),('transaction_id',o.transaction_id),('event_id',o.event_id),
          ('guild_id',o.guild_id),('world_id',o.world_id),('actor',o.actor),('target',o.target)) AS ids(field_name,uuid_value)
        CROSS JOIN generate_series(0,15) AS bytes(byte_index) GROUP BY field_name,uuid_value) AS encoded_ids))
   AND (o.action<>11 OR convert_from(NEW.payload,'UTF8')::jsonb =
     jsonb_build_object('version',1,'action',11,'previous_revision',o.previous_revision,'revision',o.revision,
       'rank_before',NULL::smallint,'rank_after',NULL::smallint,'removed_invitations',0,'occurred_at_unix_ms',NEW.occurred_at,
       'expected_revision',o.expected_revision,'names',to_jsonb(o.edit_rank_names))
     || (SELECT jsonb_object_agg(field_name,encoded_bytes) FROM
       (SELECT field_name,jsonb_agg(get_byte(uuid_send(uuid_value),byte_index) ORDER BY byte_index) AS encoded_bytes
        FROM (VALUES ('occurrence',o.occurrence),('transaction_id',o.transaction_id),('event_id',o.event_id),
          ('guild_id',o.guild_id),('world_id',o.world_id),('actor',o.actor),('target',o.target)) AS ids(field_name,uuid_value)
        CROSS JOIN generate_series(0,15) AS bytes(byte_index) GROUP BY field_name,uuid_value) AS encoded_ids))
   AND (o.action<>10 OR convert_from(NEW.payload,'UTF8')::jsonb =
     jsonb_build_object('version',1,'action',10,'previous_revision',o.previous_revision,'revision',o.revision,
       'rank_before',2,'rank_after',1,'removed_invitations',0,'occurred_at_unix_ms',NEW.occurred_at,'expected_revision',o.expected_revision)
     || (SELECT jsonb_object_agg(field_name,encoded_bytes) FROM
       (SELECT field_name,jsonb_agg(get_byte(uuid_send(uuid_value),byte_index) ORDER BY byte_index) AS encoded_bytes
        FROM (VALUES ('occurrence',o.occurrence),('transaction_id',o.transaction_id),('event_id',o.event_id),
          ('guild_id',o.guild_id),('world_id',o.world_id),('actor',o.actor),('target',o.target)) AS ids(field_name,uuid_value)
        CROSS JOIN generate_series(0,15) AS bytes(byte_index) GROUP BY field_name,uuid_value) AS encoded_ids))
   ) THEN
  RAISE EXCEPTION 'Guild event has no matching operation' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guild_event_proven AFTER INSERT ON game_guild_event_outbox DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_event_proven();
-- GUILD0 §3.3/§3.4 start only. The same transaction retains its pending step and publication.
CREATE FUNCTION game_guild_job_start_allowed() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE g game_guilds%ROWTYPE;n BIGINT;
BEGIN
 SELECT * INTO g FROM game_guilds WHERE guild_id=NEW.guild_id;
 n=floor(extract(epoch FROM transaction_timestamp())*1000)::bigint;
 IF NOT FOUND OR g.world_id<>NEW.world_id OR g.revision<>NEW.previous_revision
    OR NEW.occurred_at<>n OR NEW.created_xact_id<>pg_current_xact_id()
    OR NOT ((NEW.reason=1 AND g.state=1 AND n>g.formation_deadline)
       OR (NEW.reason=2 AND g.state=2 AND g.vice_deficit_since IS NOT NULL
          AND n::numeric-g.vice_deficit_since::numeric>1209600000)
       OR (NEW.reason=3 AND g.state IN(1,2) AND EXISTS(
          SELECT 1 FROM game_guild_operations o WHERE o.created_xact_id=pg_current_xact_id()
            AND o.action=13 AND o.result=0 AND o.guild_id=g.guild_id AND o.world_id=g.world_id
            AND o.previous_revision=NEW.previous_revision AND o.revision=NEW.revision AND o.disband_confirmation=g.name)))
    OR (NEW.reason<>3 AND EXISTS(SELECT 1 FROM game_guild_operations WHERE created_xact_id=pg_current_xact_id())) THEN
  RAISE EXCEPTION 'Guild job requires its due locked Guild and exact database time' USING ERRCODE='23514';END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_guild_job_stamp BEFORE INSERT ON game_guild_job_checkpoints FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_guild_job_start_allowed BEFORE INSERT ON game_guild_job_checkpoints FOR EACH ROW EXECUTE FUNCTION game_guild_job_start_allowed();
CREATE TRIGGER game_guild_job_event_stamp BEFORE INSERT ON game_guild_job_event_outbox FOR EACH ROW EXECUTE FUNCTION game_item_stamp_created_xact_id();
CREATE TRIGGER game_guild_job_immutable BEFORE UPDATE OR DELETE ON game_guild_job_checkpoints FOR EACH ROW EXECUTE FUNCTION game_guild_rows_immutable();
CREATE TRIGGER game_guild_job_event_immutable BEFORE UPDATE OR DELETE ON game_guild_job_event_outbox FOR EACH ROW EXECUTE FUNCTION game_guild_rows_immutable();
CREATE TRIGGER game_guild_job_no_truncate BEFORE TRUNCATE ON game_guild_job_checkpoints FOR EACH STATEMENT EXECUTE FUNCTION game_guild_rows_immutable();
CREATE TRIGGER game_guild_job_event_no_truncate BEFORE TRUNCATE ON game_guild_job_event_outbox FOR EACH STATEMENT EXECUTE FUNCTION game_guild_rows_immutable();
CREATE FUNCTION game_guild_job_proven() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE j game_guild_job_checkpoints%ROWTYPE;expected JSONB;
BEGIN
 SELECT * INTO j FROM game_guild_job_checkpoints WHERE guild_id=NEW.guild_id AND step=NEW.step;
 IF NOT FOUND OR j.created_xact_id<>pg_current_xact_id()
    OR NOT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=j.guild_id AND world_id=j.world_id AND state=3 AND revision=j.revision) THEN
  RAISE EXCEPTION 'Guild job must commit its pending start' USING ERRCODE='23514';END IF;
 IF j.reason=3 THEN
  IF NOT EXISTS(SELECT 1 FROM game_guild_operations o JOIN game_guild_event_outbox e ON e.occurrence=o.occurrence
     WHERE o.guild_id=j.guild_id AND o.world_id=j.world_id AND o.action=13 AND o.result=0
       AND o.previous_revision=j.previous_revision AND o.revision=j.revision
       AND o.created_xact_id=pg_current_xact_id() AND e.created_xact_id=pg_current_xact_id()
       AND e.event_id=o.event_id AND e.payload_sha256=o.audit_sha256)
     OR EXISTS(SELECT 1 FROM game_guild_job_event_outbox WHERE guild_id=j.guild_id AND step=j.step) THEN
   RAISE EXCEPTION 'Player Disband start requires its one exact command event' USING ERRCODE='23514';END IF;
  RETURN NULL;
 END IF;
 expected=jsonb_build_object('version',1,'step',1,'reason',j.reason,'previous_revision',j.previous_revision,
   'revision',j.revision,'occurred_at_unix_ms',j.occurred_at)
   || (SELECT jsonb_object_agg(field_name,encoded_bytes) FROM
       (SELECT field_name,jsonb_agg(get_byte(uuid_send(uuid_value),byte_index) ORDER BY byte_index) AS encoded_bytes
        FROM (VALUES ('guild_id',j.guild_id),('world_id',j.world_id)) AS ids(field_name,uuid_value)
        CROSS JOIN generate_series(0,15) AS bytes(byte_index) GROUP BY field_name,uuid_value) AS encoded_ids);
 IF NOT EXISTS(SELECT 1 FROM game_guild_job_event_outbox e WHERE e.guild_id=j.guild_id AND e.step=j.step
    AND e.created_xact_id=pg_current_xact_id() AND e.payload_sha256=sha256(e.payload)
    AND convert_from(e.payload,'UTF8')::jsonb=expected) THEN
  RAISE EXCEPTION 'Guild job requires exactly bound same-transaction event DATA' USING ERRCODE='23514';END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_guild_job_proven AFTER INSERT ON game_guild_job_checkpoints DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_job_proven();
CREATE CONSTRAINT TRIGGER game_guild_job_event_proven AFTER INSERT ON game_guild_job_event_outbox DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_guild_job_proven();
"#;

#[cfg(test)]
#[path = "guild_tests.rs"]
mod tests;

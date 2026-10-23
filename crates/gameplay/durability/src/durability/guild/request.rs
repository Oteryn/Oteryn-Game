use super::*;
include!("request_data.rs");
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
    pub(super) fn code(self) -> i16 {
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
    pub fn target(self, actor: CharacterId) -> CharacterId {
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
impl GuildDisbandRequest {
    pub fn new(common: GuildRequest, confirmation: &str) -> Result<Self> {
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
    pub(super) fn operation(&self) -> GuildRequest {
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
impl GuildTextRequest {
    pub fn new(common: GuildRequest, expected_revision: u64, received: &str) -> Result<Self> {
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
            GuildCommand::SetMessage => crate::durability::chat_text::ChatText::parse(received)
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
impl GuildRankRequest {
    pub fn new(common: GuildRequest, expected_revision: u64, level: u8) -> Result<Self> {
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

pub fn rank_binding(fence: &CurrentCharacterItemFence, rank: &GuildRankRequest) -> Result<Vec<u8>> {
    let mut digest = Sha256::new();
    digest.update(b"oteryn:guild-rank-proposal:v1");
    digest.update(binding(fence, &rank.common)?);
    digest.update(rank.expected_revision.to_be_bytes());
    digest.update([rank.level.level()]);
    Ok(digest.finalize().to_vec())
}

/// Resign DATA only. Expected revision is a private proposed precondition, not a new wire ID.
impl GuildResignRequest {
    pub fn new(common: GuildRequest, expected_revision: u64) -> Result<Self> {
        if expected_revision == 0 || !matches!(common.operation, GuildCommand::Resign(_)) {
            return Err(GuildError::InvalidInput);
        }
        Ok(Self {
            common,
            expected_revision,
        })
    }
}
pub fn resign_binding(
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
impl GuildEditRanksRequest {
    pub fn new(common: GuildRequest, expected_revision: u64, names: Vec<String>) -> Result<Self> {
        if expected_revision == 0
            || common.operation != GuildCommand::EditRanks
            || oteryn_gameplay_social_rules::guild::validate_rank_count(names.len()).is_err()
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
pub fn edit_ranks_binding(
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

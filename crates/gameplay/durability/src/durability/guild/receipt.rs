use super::*;

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
    pub fn code(self) -> i16 {
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
    pub(super) fn from_code(code: i16) -> Option<Self> {
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

pub fn invalid() -> GuildError {
    DurabilityError::InvalidStoredState.into()
}

pub fn binding(fence: &CurrentCharacterItemFence, request: &GuildRequest) -> Result<Vec<u8>> {
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

//! Shared Guild identities, rank/state values and accepted contract bounds.
// GUILD-0 §3.2 uses the existing 0022 grammar and canonical key, in a separate guild namespace.
pub use super::character_name::{
    CharacterName as GuildName, InvalidCharacterName as InvalidGuildName,
};

pub const FORMATION_LIFETIME_MS: i64 = 3 * 24 * 60 * 60 * 1_000;
pub const DEFAULT_RANKS: [(u8, &str); 3] = [(1, "Leader"), (2, "Vice Leader"), (3, "Member")];

pub const MIN_RANKS: u8 = 3;
pub const MAX_RANKS: u8 = 20;
pub const REQUIRED_VICES: usize = 4;
pub const VICE_DEFICIT_GRACE_MS: i64 = 14 * 24 * 60 * 60 * 1_000;
pub const MEMBERS_MAX: usize = 2_000; // GUILD0-RL-05, PARITY_PENDING.
pub const GUILD_INVITATIONS_MAX: usize = 500;
pub const TARGET_INVITATIONS_MAX: usize = 50;
pub const INVITATION_LIFETIME_MS: i64 = 30 * 24 * 60 * 60 * 1_000; // PARITY_PENDING.

/// Game-issued identity shape only. A UUID is never membership or current Guild authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GuildId([u8; 16]);

impl GuildId {
    pub fn from_bytes(bytes: [u8; 16]) -> Result<Self, super::DomainError> {
        super::validate_uuid_v7(&bytes)?;
        Ok(Self(bytes))
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildRuleError {
    InvalidRankLevel,
    InvalidRankCount,
    GuildDisbanding,
    GuildDeadlinePassed,
    InvalidWarTerms,
}

/// Lower numbers outrank higher numbers; this is only the numeric part of command eligibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuildRank(u8);

impl GuildRank {
    pub const LEADER: Self = Self(1);
    pub const VICE: Self = Self(2);

    pub fn new(level: u8) -> Result<Self, GuildRuleError> {
        if (1..=MAX_RANKS).contains(&level) {
            Ok(Self(level))
        } else {
            Err(GuildRuleError::InvalidRankLevel)
        }
    }

    #[must_use]
    pub const fn level(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildState {
    Forming,
    Active,
    Disbanding,
    Disbanded,
}

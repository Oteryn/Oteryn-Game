//! Pure GUILD-0 §3.1–§3.3 rules, as accepted by SOCIAL-MAP §2.2.
//!
//! These checks neither authorize an actor nor establish current guild facts. A future writer
//! must resolve its session fence, membership, Account leadership and Premium requirements,
//! and read the guild facts and database transaction time under GUILD0-LO-01. The separately
//! prepared private durability owner does that; no authority or mutation lives in this module.

// GUILD-0 §3.2 uses the existing 0022 grammar and canonical key, in a separate guild namespace.
pub use super::character_name::{
    CharacterName as GuildName, InvalidCharacterName as InvalidGuildName,
};

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

    /// GUILD-0 §3.2: both the target's existing and proposed rank must be below the actor.
    #[must_use]
    pub const fn may_set_rank(self, target: Self, proposed: Self) -> bool {
        self.0 < target.0 && self.0 < proposed.0
    }

    /// GUILD-0 §3.2: only a leader or vice may exclude, and only a lower-ranked member.
    #[must_use]
    pub const fn may_exclude(self, target: Self) -> bool {
        self.0 <= Self::VICE.0 && self.0 < target.0
    }
}

/// Count validation only: name uniqueness and reassigning members of removed ranks stay with
/// the future atomic rank-edit writer. No rank-name grammar is inferred from the guild-name rule.
pub fn validate_rank_count(count: usize) -> Result<(), GuildRuleError> {
    if (usize::from(MIN_RANKS)..=usize::from(MAX_RANKS)).contains(&count) {
        Ok(())
    } else {
        Err(GuildRuleError::InvalidRankCount)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildState {
    Forming,
    Active,
    Disbanding,
    Disbanded,
}

impl GuildState {
    /// The ordinary-command gate, not the disband-step or payout-claim gate (§3.2).
    pub fn ensure_command_allowed(self) -> Result<(), GuildRuleError> {
        match self {
            Self::Forming | Self::Active => Ok(()),
            Self::Disbanding | Self::Disbanded => Err(GuildRuleError::GuildDisbanding),
        }
    }

    /// Refuse a vice-count change that would activate or cure an overdue guild (§3.2).
    ///
    /// The writer supplies locked guild facts and the database transaction time as Unix milliseconds;
    /// this helper does not obtain or attest them. A change that leaves fewer than four vices
    /// does not cure the deficit and remains allowed. Premium deficits are job-owned (§3.3) and
    /// deliberately are not inputs to this transaction gate. "Past" and "older than" are strict.
    pub fn ensure_vice_count_change_allowed(
        self,
        proposed_vices: usize,
        formation_deadline_unix_ms: i64,
        vice_deficit_since_unix_ms: Option<i64>,
        database_transaction_unix_ms: i64,
    ) -> Result<(), GuildRuleError> {
        self.ensure_command_allowed()?;
        if proposed_vices < REQUIRED_VICES {
            return Ok(());
        }
        let expired = match self {
            Self::Forming => database_transaction_unix_ms > formation_deadline_unix_ms,
            Self::Active => vice_deficit_since_unix_ms.is_some_and(|since| {
                i128::from(database_transaction_unix_ms) - i128::from(since)
                    > i128::from(VICE_DEFICIT_GRACE_MS)
            }),
            Self::Disbanding | Self::Disbanded => return Err(GuildRuleError::GuildDisbanding),
        };
        if expired {
            Err(GuildRuleError::GuildDeadlinePassed)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guild_names_reuse_character_policy_and_key_without_a_second_normalizer()
    -> Result<(), InvalidGuildName> {
        assert!(GuildName::parse(&"a".repeat(29)).is_ok());
        for invalid in ["a".repeat(30), "Élan".into(), "Al\u{200b}Dric".into()] {
            assert_eq!(GuildName::parse(&invalid), Err(InvalidGuildName));
        }
        let spaced = GuildName::parse("Al Dric")?;
        let compact = GuildName::parse("ALDRIC")?;
        assert_eq!(spaced.comparison_key(), compact.comparison_key());
        Ok(())
    }

    #[test]
    fn rank_level_and_count_boundaries_are_enforced() {
        assert_eq!(GuildRank::new(0), Err(GuildRuleError::InvalidRankLevel));
        assert_eq!(GuildRank::new(1), Ok(GuildRank::LEADER));
        assert_eq!(GuildRank::new(2), Ok(GuildRank::VICE));
        assert_eq!(GuildRank::new(20).map(GuildRank::level), Ok(20));
        assert_eq!(GuildRank::new(21), Err(GuildRuleError::InvalidRankLevel));
        assert_eq!(
            validate_rank_count(2),
            Err(GuildRuleError::InvalidRankCount)
        );
        assert_eq!(validate_rank_count(3), Ok(()));
        assert_eq!(validate_rank_count(20), Ok(()));
        assert_eq!(
            validate_rank_count(21),
            Err(GuildRuleError::InvalidRankCount)
        );
    }

    #[test]
    fn rank_changes_require_both_strict_inequalities_and_exclusion_requires_leadership()
    -> Result<(), GuildRuleError> {
        let member = GuildRank::new(3)?;
        let lower_member = GuildRank::new(4)?;
        assert!(GuildRank::LEADER.may_set_rank(member, GuildRank::VICE));
        assert!(!GuildRank::VICE.may_set_rank(member, GuildRank::VICE));
        assert!(!GuildRank::VICE.may_set_rank(GuildRank::VICE, member));
        assert!(!member.may_set_rank(GuildRank::VICE, lower_member));
        assert!(member.may_set_rank(lower_member, lower_member));
        assert!(GuildRank::VICE.may_exclude(member));
        assert!(!GuildRank::VICE.may_exclude(GuildRank::VICE));
        assert!(!member.may_exclude(lower_member));
        Ok(())
    }

    #[test]
    fn disbanding_and_terminal_states_refuse_even_a_non_curing_change() {
        for state in [GuildState::Disbanding, GuildState::Disbanded] {
            assert_eq!(
                state.ensure_command_allowed(),
                Err(GuildRuleError::GuildDisbanding)
            );
            assert_eq!(
                state.ensure_vice_count_change_allowed(0, 100, None, 0),
                Err(GuildRuleError::GuildDisbanding)
            );
        }
        assert_eq!(GuildState::Forming.ensure_command_allowed(), Ok(()));
        assert_eq!(GuildState::Active.ensure_command_allowed(), Ok(()));
    }

    #[test]
    fn forming_can_activate_at_its_deadline_but_cannot_activate_later() {
        let deadline = 3 * 24 * 60 * 60 * 1_000;
        for count in [4, 5] {
            assert_eq!(
                GuildState::Forming
                    .ensure_vice_count_change_allowed(count, deadline, None, deadline),
                Ok(())
            );
            assert_eq!(
                GuildState::Forming.ensure_vice_count_change_allowed(
                    count,
                    deadline,
                    None,
                    deadline + 1
                ),
                Err(GuildRuleError::GuildDeadlinePassed)
            );
        }
        assert_eq!(
            GuildState::Forming.ensure_vice_count_change_allowed(3, deadline, None, deadline + 1),
            Ok(())
        );
    }

    #[test]
    fn active_vice_grace_survives_reconstruction_without_extending_the_deadline() {
        // Each evaluation reconstructs the inputs from the same hypothetical persisted facts;
        // no helper-local start time can extend the grace after a restart.
        let since = 50;
        let deadline = since + 1_209_600_000; // Accepted 14-day grace, in milliseconds.
        for (now, expected) in [
            (deadline - 1, Ok(())),
            (deadline, Ok(())),
            (deadline + 1, Err(GuildRuleError::GuildDeadlinePassed)),
        ] {
            for _reconstruction in 0..2 {
                assert_eq!(
                    GuildState::Active.ensure_vice_count_change_allowed(4, 0, Some(since), now),
                    expected
                );
            }
        }
        assert_eq!(
            GuildState::Active.ensure_vice_count_change_allowed(3, 0, Some(since), deadline + 1),
            Ok(())
        );
        assert_eq!(
            GuildState::Active.ensure_vice_count_change_allowed(4, 0, None, i64::MAX),
            Ok(())
        );
    }

    #[test]
    fn vice_deadline_comparison_does_not_overflow_at_timestamp_extremes() {
        assert_eq!(
            GuildState::Active.ensure_vice_count_change_allowed(4, 0, Some(i64::MIN), i64::MAX),
            Err(GuildRuleError::GuildDeadlinePassed)
        );
        assert_eq!(
            GuildState::Active.ensure_vice_count_change_allowed(4, 0, Some(i64::MAX), i64::MIN),
            Ok(())
        );
    }
}

//! Pure GUILD0 eligibility and deadline predicates over caller-supplied locked facts.
pub use oteryn_gameplay_domain::domain::guild::*;

/// Global guild-war limits: leaders choose 10..=1000 frags and 7..=180 days.
pub const GUILD_WAR_MIN_FRAGS: u16 = 10;
pub const GUILD_WAR_MAX_FRAGS: u16 = 1_000;
pub const GUILD_WAR_MIN_DAYS: u16 = 7;
pub const GUILD_WAR_MAX_DAYS: u16 = 180;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildWarEnd {
    FragLimit,
    Duration,
}

pub fn validate_guild_war_terms(frag_limit: u16, duration_days: u16) -> Result<(), GuildRuleError> {
    if (GUILD_WAR_MIN_FRAGS..=GUILD_WAR_MAX_FRAGS).contains(&frag_limit)
        && (GUILD_WAR_MIN_DAYS..=GUILD_WAR_MAX_DAYS).contains(&duration_days)
    {
        Ok(())
    } else {
        Err(GuildRuleError::InvalidWarTerms)
    }
}

#[must_use]
pub fn guild_war_end(
    frag_limit: u16,
    frags: [u16; 2],
    ends_at: i64,
    now: i64,
) -> Option<GuildWarEnd> {
    if frags.into_iter().any(|value| value >= frag_limit) {
        Some(GuildWarEnd::FragLimit)
    } else if now >= ends_at {
        Some(GuildWarEnd::Duration)
    } else {
        None
    }
}
pub trait GuildRankRules {
    fn may_set_rank(self, target: Self, proposed: Self) -> bool;
    fn may_exclude(self, target: Self) -> bool;
}
impl GuildRankRules for GuildRank {
    /// GUILD-0 §3.2: both the target's existing and proposed rank must be below the actor.
    fn may_set_rank(self, target: Self, proposed: Self) -> bool {
        self.level() < target.level() && self.level() < proposed.level()
    }

    /// GUILD-0 §3.2: only a leader or vice may exclude, and only a lower-ranked member.
    fn may_exclude(self, target: Self) -> bool {
        self.level() <= Self::VICE.level() && self.level() < target.level()
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

pub trait GuildStateRules {
    fn ensure_command_allowed(self) -> Result<(), GuildRuleError>;
    fn ensure_vice_count_change_allowed(
        self,
        proposed_vices: usize,
        formation_deadline_unix_ms: i64,
        vice_deficit_since_unix_ms: Option<i64>,
        database_transaction_unix_ms: i64,
    ) -> Result<(), GuildRuleError>;
}
impl GuildStateRules for GuildState {
    /// The ordinary-command gate, not the disband-step or payout-claim gate (§3.2).
    fn ensure_command_allowed(self) -> Result<(), GuildRuleError> {
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
    fn ensure_vice_count_change_allowed(
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

/// Accepted GUILD0 §3.3 deadline source. Premium remains a separate daily job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildDeadlineJob {
    Formation,
    ViceDeficit,
}
impl GuildDeadlineJob {
    pub const fn code(self) -> i16 {
        match self {
            Self::Formation => 1,
            Self::ViceDeficit => 2,
        }
    }

    /// Locked facts only; this comparison grants no current World authority.
    pub fn due(
        self,
        state: GuildState,
        formation_deadline: i64,
        vice_deficit_since: Option<i64>,
        database_now: i64,
    ) -> bool {
        match (self, state) {
            (Self::Formation, GuildState::Forming) => database_now > formation_deadline,
            (Self::ViceDeficit, GuildState::Active) => vice_deficit_since.is_some_and(|since| {
                i128::from(database_now) - i128::from(since) > i128::from(VICE_DEFICIT_GRACE_MS)
            }),
            _ => false,
        }
    }
}

#[cfg(test)]
#[path = "guild/rules_tests.rs"]
mod rules_tests;

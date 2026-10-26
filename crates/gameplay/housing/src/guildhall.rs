//! Pure Guildhall policy. Persistence composes these decisions with the existing
//! House auction and Guild/Account bank transaction owners.

pub const GUILD_BALANCE_MAX: u64 = 999_999_999_999;
pub const ACCOUNT_BALANCE_MAX: u64 = 9_000_000_000_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GuildhallRefusal {
    NotGuildLeader,
    GuildAlreadyHasHall,
    GuildAlreadyBidding,
    PersonalHouseLimitExceeded,
    GuildInactive,
    InsufficientFunds,
    BalanceLimit,
    StaleLeader,
    NoChange,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AcquisitionFacts {
    pub rank: Option<u8>,
    pub guild_active: bool,
    pub guild_has_hall: bool,
    pub guild_has_bid: bool,
    /// Global permits the Hall in addition to the leader's ordinary allowance,
    /// but the leader must still satisfy the ordinary limit (zero or one).
    pub leader_personal_houses: u8,
}

pub fn qualify_acquisition(facts: AcquisitionFacts) -> Result<(), GuildhallRefusal> {
    if facts.rank != Some(1) {
        return Err(GuildhallRefusal::NotGuildLeader);
    }
    if !facts.guild_active {
        return Err(GuildhallRefusal::GuildInactive);
    }
    if facts.guild_has_hall {
        return Err(GuildhallRefusal::GuildAlreadyHasHall);
    }
    if facts.guild_has_bid {
        return Err(GuildhallRefusal::GuildAlreadyBidding);
    }
    if facts.leader_personal_houses > 1 {
        return Err(GuildhallRefusal::PersonalHouseLimitExceeded);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Escrow {
    pub guild: u64,
    pub leader: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FundingChange {
    pub escrow_before: Escrow,
    pub escrow_after: Escrow,
    pub guild_balance_after: u64,
    pub leader_balance_after: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Payment {
    pub guild: u64,
    pub leader: u64,
    pub guild_balance_after: u64,
    pub leader_balance_after: u64,
}

/// A transfer price or rent debit. This is deliberately the same Guild-first
/// ordering as escrow reservation and never mints a fallback currency source.
pub fn plan_payment(
    amount: u64,
    guild_balance: u64,
    leader_balance: u64,
) -> Result<Payment, GuildhallRefusal> {
    if guild_balance > GUILD_BALANCE_MAX || leader_balance > ACCOUNT_BALANCE_MAX {
        return Err(GuildhallRefusal::BalanceLimit);
    }
    let guild = amount.min(guild_balance);
    let leader = amount - guild;
    if leader > leader_balance {
        return Err(GuildhallRefusal::InsufficientFunds);
    }
    Ok(Payment {
        guild,
        leader,
        guild_balance_after: guild_balance - guild,
        leader_balance_after: leader_balance - leader,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReleaseCause {
    RentFailure,
    GuildDisband,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RentDecision {
    Paid(Payment),
    Evict(ReleaseCause),
}

pub fn collect_rent(rent: u64, guild: u64, leader: u64) -> RentDecision {
    match plan_payment(rent, guild, leader) {
        Ok(payment) => RentDecision::Paid(payment),
        Err(GuildhallRefusal::InsufficientFunds) => RentDecision::Evict(ReleaseCause::RentFailure),
        // Invalid retained values fail closed. The durable owner treats this as
        // an error rather than silently releasing custody.
        Err(_) => RentDecision::Evict(ReleaseCause::RentFailure),
    }
}

impl Escrow {
    pub fn total(self) -> Result<u64, GuildhallRefusal> {
        self.guild
            .checked_add(self.leader)
            .ok_or(GuildhallRefusal::BalanceLimit)
    }

    /// Reserve Guild-first and refund leader-first, preserving the original
    /// contributors. `allow_leader_debit` is false after a leadership change.
    pub fn resize(
        self,
        target: u64,
        guild_balance: u64,
        leader_balance: u64,
        allow_leader_debit: bool,
    ) -> Result<FundingChange, GuildhallRefusal> {
        if self.guild > GUILD_BALANCE_MAX
            || guild_balance > GUILD_BALANCE_MAX
            || self.leader > ACCOUNT_BALANCE_MAX
            || leader_balance > ACCOUNT_BALANCE_MAX
        {
            return Err(GuildhallRefusal::BalanceLimit);
        }
        let current = self.total()?;
        let (after, guild_after, leader_after) = if target > current {
            let needed = target - current;
            let guild_part = needed.min(guild_balance);
            let leader_part = needed - guild_part;
            if leader_part > 0 && !allow_leader_debit {
                return Err(GuildhallRefusal::StaleLeader);
            }
            if leader_part > leader_balance {
                return Err(GuildhallRefusal::InsufficientFunds);
            }
            (
                Escrow {
                    guild: self
                        .guild
                        .checked_add(guild_part)
                        .ok_or(GuildhallRefusal::BalanceLimit)?,
                    leader: self
                        .leader
                        .checked_add(leader_part)
                        .ok_or(GuildhallRefusal::BalanceLimit)?,
                },
                guild_balance - guild_part,
                leader_balance - leader_part,
            )
        } else if target < current {
            let returned = current - target;
            let leader_part = returned.min(self.leader);
            let guild_part = returned - leader_part;
            (
                Escrow {
                    guild: self.guild - guild_part,
                    leader: self.leader - leader_part,
                },
                guild_balance
                    .checked_add(guild_part)
                    .ok_or(GuildhallRefusal::BalanceLimit)?,
                leader_balance
                    .checked_add(leader_part)
                    .ok_or(GuildhallRefusal::BalanceLimit)?,
            )
        } else {
            return Err(GuildhallRefusal::NoChange);
        };
        if guild_after > GUILD_BALANCE_MAX || leader_after > ACCOUNT_BALANCE_MAX {
            return Err(GuildhallRefusal::BalanceLimit);
        }
        Ok(FundingChange {
            escrow_before: self,
            escrow_after: after,
            guild_balance_after: guild_after,
            leader_balance_after: leader_after,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HouseAccess {
    None,
    Guest,
    Subowner,
}

/// Global rank projection onto the existing House lists. Membership is read at
/// access time, so resign/kick removes access without copying stale ACL rows.
pub fn rank_access(rank: Option<u8>) -> HouseAccess {
    match rank {
        Some(1 | 2) => HouseAccess::Subowner,
        Some(_) => HouseAccess::Guest,
        None => HouseAccess::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_active_leader_with_global_allowance_qualifies() {
        let valid = AcquisitionFacts {
            rank: Some(1),
            guild_active: true,
            leader_personal_houses: 1,
            ..AcquisitionFacts::default()
        };
        assert_eq!(qualify_acquisition(valid), Ok(()));
        assert_eq!(
            qualify_acquisition(AcquisitionFacts {
                rank: Some(2),
                ..valid
            }),
            Err(GuildhallRefusal::NotGuildLeader)
        );
        assert_eq!(
            qualify_acquisition(AcquisitionFacts {
                guild_has_hall: true,
                ..valid
            }),
            Err(GuildhallRefusal::GuildAlreadyHasHall)
        );
        assert_eq!(
            qualify_acquisition(AcquisitionFacts {
                leader_personal_houses: 2,
                ..valid
            }),
            Err(GuildhallRefusal::PersonalHouseLimitExceeded)
        );
    }

    #[test]
    fn funding_is_guild_first_and_refunds_leader_first() -> Result<(), GuildhallRefusal> {
        let reserved = Escrow::default().resize(80, 60, 40, true)?;
        assert_eq!(
            reserved.escrow_after,
            Escrow {
                guild: 60,
                leader: 20
            }
        );
        let lowered = reserved.escrow_after.resize(65, 0, 20, true)?;
        assert_eq!(
            lowered.escrow_after,
            Escrow {
                guild: 60,
                leader: 5
            }
        );
        assert_eq!(
            (lowered.guild_balance_after, lowered.leader_balance_after),
            (0, 35)
        );
        Ok(())
    }

    #[test]
    fn successor_leader_cannot_spend_predecessor_account() {
        assert_eq!(
            Escrow {
                guild: 5,
                leader: 5
            }
            .resize(12, 0, 100, false),
            Err(GuildhallRefusal::StaleLeader)
        );
    }

    #[test]
    fn membership_is_the_access_revocation_source() {
        assert_eq!(rank_access(Some(1)), HouseAccess::Subowner);
        assert_eq!(rank_access(Some(2)), HouseAccess::Subowner);
        assert_eq!(rank_access(Some(3)), HouseAccess::Guest);
        assert_eq!(rank_access(None), HouseAccess::None);
    }

    #[test]
    fn rent_is_guild_first_and_failure_eviction_is_typed() {
        assert_eq!(
            collect_rent(75, 50, 30),
            RentDecision::Paid(Payment {
                guild: 50,
                leader: 25,
                guild_balance_after: 0,
                leader_balance_after: 5,
            })
        );
        assert_eq!(
            collect_rent(81, 50, 30),
            RentDecision::Evict(ReleaseCause::RentFailure)
        );
    }
}

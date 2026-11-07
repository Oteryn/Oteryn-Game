//! Closed TRANSFER conservation for native Guild/Bank owner evidence (no numeric event IDs).
use oteryn_gameplay_economy::guild_bank::{ACCOUNT_HARD_MAX, GUILD_BALANCE_MAX};

pub(super) const DEPOSIT: &str = "GUILD_DEPOSIT";
pub(super) const WITHDRAW: &str = "GUILD_WITHDRAW";
pub(super) const PAYOUT: &str = "GUILD_DISBAND_PAYOUT";
pub(super) const CLAIM: &str = "GUILD_DISBAND_CLAIM";

#[derive(Clone, Copy, Default)]
pub(super) struct Values {
    pub bank_before: u64,
    pub bank_after: u64,
    pub guild_before: u64,
    pub guild_after: u64,
    pub claim_before: u64,
    pub claim_after: u64,
}
impl Values {
    pub(super) fn conserved(self, kind: &str, amount: u64) -> bool {
        if amount > GUILD_BALANCE_MAX
            || self.bank_before > ACCOUNT_HARD_MAX
            || self.bank_after > ACCOUNT_HARD_MAX
            || [
                self.guild_before,
                self.guild_after,
                self.claim_before,
                self.claim_after,
            ]
            .into_iter()
            .any(|v| v > GUILD_BALANCE_MAX)
        {
            return false;
        }
        let sum_before = self.bank_before + self.guild_before + self.claim_before;
        let sum_after = self.bank_after + self.guild_after + self.claim_after;
        sum_before == sum_after
            && match kind {
                DEPOSIT => {
                    amount > 0
                        && self.bank_before.checked_sub(amount) == Some(self.bank_after)
                        && self.guild_before.checked_add(amount) == Some(self.guild_after)
                        && self.claim_before == self.claim_after
                }
                WITHDRAW => {
                    amount > 0
                        && self.guild_before.checked_sub(amount) == Some(self.guild_after)
                        && self.bank_before.checked_add(amount) == Some(self.bank_after)
                        && self.claim_before == self.claim_after
                }
                PAYOUT => {
                    amount == self.guild_before && self.guild_after == 0 && self.claim_before == 0
                }
                CLAIM => {
                    amount > 0
                        && self.claim_before.checked_sub(amount) == Some(self.claim_after)
                        && self.bank_before.checked_add(amount) == Some(self.bank_after)
                        && self.guild_before == self.guild_after
                }
                _ => false,
            }
    }
}

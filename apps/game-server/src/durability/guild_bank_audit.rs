//! Unregistered GUILD0 §5 audit proposal. These bytes are durable candidate evidence,
//! not an admitted EventEnvelope/protobuf schema. CP must register both bank and guild
//! payloads/retention before any production writer is activated. No wire/event IDs here.
use super::item_mint_audit::check_uuid_v7;
use prost::Message;

pub const HARD_CEILING: u64 = 9_000_000_000_000_000;
pub const BANK_MAX: u64 = super::bank_audit::BANK_BALANCE_MAX;

/// Explicit value-custody before/after facts; all lines are TRANSFER, never MINT/BURN.
#[derive(Clone, PartialEq, Message)]
pub struct GuildBankAuditProposal {
    #[prost(bytes = "vec", tag = "1")]
    pub transaction_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub guild_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub account_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub world_id: Vec<u8>,
    #[prost(string, tag = "5")]
    pub cause: String,
    #[prost(uint64, tag = "6")]
    pub guild_before: u64,
    #[prost(uint64, tag = "7")]
    pub guild_after: u64,
    #[prost(uint64, tag = "8")]
    pub account_before: u64,
    #[prost(uint64, tag = "9")]
    pub account_after: u64,
    #[prost(uint64, tag = "10")]
    pub claim_before: u64,
    #[prost(uint64, tag = "11")]
    pub claim_after: u64,
}

impl GuildBankAuditProposal {
    pub fn checked_bytes(&self) -> Option<Vec<u8>> {
        for bytes in [
            &self.transaction_id,
            &self.guild_id,
            &self.account_id,
            &self.world_id,
        ] {
            let id: [u8; 16] = bytes.as_slice().try_into().ok()?;
            check_uuid_v7(&id).ok()?;
        }
        if self.guild_before > BANK_MAX
            || self.guild_after > BANK_MAX
            || self.account_before > HARD_CEILING
            || self.account_after > HARD_CEILING
            || self.claim_before > HARD_CEILING
            || self.claim_after > HARD_CEILING
            || self.guild_before + self.account_before + self.claim_before
                != self.guild_after + self.account_after + self.claim_after
        {
            return None;
        }
        let legal = match self.cause.as_str() {
            "GUILD_DEPOSIT" => {
                self.guild_after > self.guild_before
                    && self.account_after < self.account_before
                    && self.claim_before == self.claim_after
            }
            "GUILD_WITHDRAW" => {
                self.guild_after < self.guild_before
                    && self.account_after > self.account_before
                    && self.account_after <= BANK_MAX
                    && self.claim_before == self.claim_after
            }
            "GUILD_DISBAND_PAYOUT" => {
                self.guild_after == 0
                    && self.claim_before == 0
                    && self.account_after >= self.account_before
            }
            "GUILD_DISBAND_CLAIM" => {
                self.guild_before == self.guild_after
                    && self.claim_after < self.claim_before
                    && self.account_after > self.account_before
            }
            _ => false,
        };
        legal.then(|| self.encode_to_vec())
    }
}

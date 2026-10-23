//! Event 8 has its own four-value-line accounting, independent of BANK_OPERATION.
use super::house_ownership::{HouseOwnershipError, value::ValueWrite};
use super::item_mint_audit::{EventEnvelopeV1, TransactionEventRefV1, encode_bounded};
use prost::Message;
use sha2::{Digest, Sha256};

#[cfg(test)]
#[path = "house_ownership_audit_tests.rs"]
mod tests;

pub const VALUE_LINES_MAX: usize = 4;
pub const PAYLOAD_BYTES_MAX: usize = 65_536;
pub const ENVELOPE_BYTES_MAX: usize = 131_072;

#[derive(Clone, PartialEq, Message)]
struct EligibilityView {
    #[prost(bytes = "vec", tag = "1")]
    account: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    character: Vec<u8>,
    #[prost(uint64, tag = "3")]
    maximum: u64,
    #[prost(uint64, tag = "4")]
    reached_order: u64,
    #[prost(bool, tag = "5")]
    active: bool,
    #[prost(bool, tag = "6")]
    account_matches: bool,
    #[prost(bool, tag = "7")]
    world_matches: bool,
    #[prost(uint32, tag = "8")]
    level: u32,
    #[prost(bool, tag = "9")]
    junior: bool,
    #[prost(bool, tag = "10")]
    slot_occupied: bool,
    #[prost(bool, tag = "11")]
    banned: bool,
    #[prost(bool, tag = "12")]
    premium_allowed: bool,
    #[prost(uint32, tag = "13")]
    refusal: u32,
}

#[derive(Clone, PartialEq, Message)]
struct HouseValueLine {
    #[prost(uint32, tag = "1")]
    kind: u32,
    #[prost(uint64, tag = "2")]
    amount: u64,
    #[prost(uint64, tag = "3")]
    balance_before: u64,
    #[prost(uint64, tag = "4")]
    balance_after: u64,
    #[prost(uint64, tag = "5")]
    escrow_before: u64,
    #[prost(uint64, tag = "6")]
    escrow_after: u64,
}
#[derive(Clone, PartialEq, Message)]
struct HouseOperation {
    #[prost(bytes = "vec", tag = "1")]
    auction: Vec<u8>,
    #[prost(string, tag = "2")]
    house: String,
    #[prost(bytes = "vec", tag = "3")]
    account: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    character: Vec<u8>,
    #[prost(uint32, tag = "5")]
    kind: u32,
    #[prost(uint64, tag = "6")]
    maximum: u64,
    #[prost(uint64, tag = "7")]
    first_rent: u64,
    #[prost(uint64, tag = "8")]
    price: u64,
    #[prost(message, repeated, tag = "9")]
    lines: Vec<HouseValueLine>,
    #[prost(uint64, tag = "10")]
    escrow_before: u64,
    #[prost(uint64, tag = "11")]
    escrow_after: u64,
    #[prost(uint32, tag = "12")]
    eligibility_version: u32,
    #[prost(message, repeated, tag = "13")]
    eligibility: Vec<EligibilityView>,
    #[prost(bytes = "vec", optional, tag = "14")]
    owner_before: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "15")]
    owner_after: Option<Vec<u8>>,
    #[prost(bool, optional, tag = "16")]
    slot_before: Option<bool>,
    #[prost(bool, optional, tag = "17")]
    slot_after: Option<bool>,
    #[prost(uint64, optional, tag = "18")]
    ownership_before: Option<u64>,
    #[prost(uint64, optional, tag = "19")]
    ownership_after: Option<u64>,
    #[prost(int64, optional, tag = "20")]
    paid_until_unix_us: Option<i64>,
}

pub(super) fn encode(
    write: &ValueWrite<'_>,
    event: [u8; 16],
) -> Result<Vec<u8>, HouseOwnershipError> {
    if write.eligibility.len() > 256 {
        return Err(HouseOwnershipError::AuctionFull);
    }
    let accounts: std::collections::BTreeSet<_> =
        write.eligibility.iter().map(|c| c.bid.account).collect();
    if accounts.len() != write.eligibility.len() {
        return Err(HouseOwnershipError::InvalidInput);
    }
    let valid: Vec<_> = write
        .eligibility
        .iter()
        .filter(|c| c.facts.refusal() == 0)
        .map(|c| c.bid)
        .collect();
    let quote = oteryn_gameplay_housing::house_auction::quote(&valid)?;
    let subject = write.eligibility.iter().any(|c| {
        c.bid.account == write.account
            && c.character == write.character
            && c.bid.maximum == write.maximum
            && c.bid.maximum_reached_order == write.reached
            && c.facts.refusal() == 0
    });
    let admitted = match write.kind {
        1 => write.eligibility.len() == 1 && subject,
        2 => subject && quote.is_some_and(|q| q.leader == write.account && q.price == write.price),
        3 => write.eligibility.is_empty() || quote.is_none(),
        _ => false,
    };
    if !admitted {
        return Err(HouseOwnershipError::InvalidInput);
    }
    if write.plan.lines.len()
        + usize::from(
            write
                .plan
                .lines
                .first()
                .is_some_and(|l| l.escrow_before != write.plan.escrow),
        )
        > VALUE_LINES_MAX
    {
        return Err(HouseOwnershipError::InvalidInput);
    }
    let payload = encode_bounded(
        &HouseOperation {
            auction: write.auction.to_vec(),
            house: write.house.into(),
            account: write.account.to_vec(),
            character: write.character.to_vec(),
            kind: write.kind as u32,
            maximum: write.maximum,
            first_rent: write.first_rent,
            price: write.price,
            escrow_before: write
                .plan
                .lines
                .first()
                .map_or(write.plan.escrow, |l| l.escrow_before),
            escrow_after: write.plan.escrow,
            eligibility_version: oteryn_gameplay_housing::house_auction::eligibility::VERSION,
            eligibility: write
                .eligibility
                .iter()
                .map(|c| EligibilityView {
                    account: c.bid.account.to_vec(),
                    character: c.character.to_vec(),
                    maximum: c.bid.maximum,
                    reached_order: c.bid.maximum_reached_order,
                    active: c.facts.active,
                    account_matches: c.facts.account_matches,
                    world_matches: c.facts.world_matches,
                    level: c.facts.level,
                    junior: c.facts.junior,
                    slot_occupied: c.facts.slot_occupied,
                    banned: c.facts.banned,
                    premium_allowed: c.facts.premium_allowed,
                    refusal: c.facts.refusal(),
                })
                .collect(),
            owner_before: (write.kind == 2).then(Vec::new),
            owner_after: (write.kind == 2).then(|| write.character.to_vec()),
            slot_before: (write.kind == 2).then_some(false),
            slot_after: (write.kind == 2).then_some(true),
            ownership_before: (write.kind == 2).then_some(write.ownership_before),
            ownership_after: if write.kind == 2 {
                Some(
                    write
                        .ownership_before
                        .checked_add(1)
                        .ok_or(HouseOwnershipError::InvalidInput)?,
                )
            } else {
                None
            },
            paid_until_unix_us: if write.kind == 2 {
                Some(
                    write
                        .now_us
                        .checked_add(oteryn_gameplay_housing::house_auction::FIRST_RENT_US)
                        .ok_or(HouseOwnershipError::InvalidInput)?,
                )
            } else {
                None
            },
            lines: write
                .plan
                .lines
                .iter()
                .map(|l| HouseValueLine {
                    kind: l.kind as u32,
                    amount: l.amount,
                    balance_before: l.balance_before,
                    balance_after: l.balance_after,
                    escrow_before: l.escrow_before,
                    escrow_after: l.escrow_after,
                })
                .collect(),
        },
        PAYLOAD_BYTES_MAX,
    )
    .map_err(|_| HouseOwnershipError::InvalidInput)?;
    encode_bounded(
        &EventEnvelopeV1 {
            envelope_revision: 1,
            event_id: event.to_vec(),
            event_type_id: 8,
            event_schema_revision: 1,
            durability_class: 2,
            privacy_class: 3,
            retention_profile_id: "ECONOMY_LEDGER_RETENTION_V1".into(),
            occurred_at_unix_ms: write.now_us / 1000,
            world_id: Some(write.world.as_bytes().to_vec()),
            transaction_event: Some(TransactionEventRefV1 {
                transaction_id: write.operation.to_vec(),
                ordinal: 1,
                count: 1,
            }),
            operation_id: Some(write.operation.to_vec()),
            server_build_id: super::character_authority::SERVER_BUILD_ID.into(),
            payload_sha256: Sha256::digest(&payload).to_vec(),
            payload,
            ..Default::default()
        },
        ENVELOPE_BYTES_MAX,
    )
    .map_err(|_| HouseOwnershipError::InvalidInput)
}

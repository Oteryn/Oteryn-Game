//! Negotiated display metadata for the existing NPC and player trade domains.
pub const CAPABILITY_TRADE_DETAILS_V1: u32 = 501;

/// One NPC offer's authoritative presentation and transaction bound.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcQuoteDetails {
    pub name: String,
    pub maximum_quantity: u32,
}

//! Prepared by the existing native coin/change owner, then revalidated by the fee writer.
#[derive(Clone, Debug)]
pub struct NpcSourcePaymentFacts {
    pub occurrence: [u8; 16],
    pub transaction: [u8; 16],
    pub event: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub server_build_id: String,
    pub change: crate::durability::item_fee_burn::FeeChangeFacts,
}

//! Personal boss reward source of the existing op4 custody mint. No synthetic Content claim.
use super::*;

#[derive(Clone, PartialEq, Eq, Message)]
pub struct BossRewardClaimSourceV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub death: Vec<u8>,
    #[prost(uint32, tag = "2")]
    pub item_ordinal: u32,
    #[prost(string, tag = "3")]
    pub boss_key: String,
    #[prost(uint64, tag = "4")]
    pub lease_generation: u64,
    #[prost(uint64, tag = "5")]
    pub scope_generation: u64,
}

pub(super) fn check_source(source: &OneItemRewardClaimCauseV1) -> Result<(), AuditError> {
    match (&source.reward_claim, &source.boss_reward) {
        (Some(claim), None) => check_definition(Some(claim)),
        (None, Some(boss)) => {
            check_content_key(&boss.boss_key)?;
            if boss.death.len() != 16
                || boss.death.iter().all(|byte| *byte == 0)
                || !(1..=1_000).contains(&boss.item_ordinal)
                || boss.lease_generation == 0
                || boss.scope_generation == 0
            {
                return Err(AuditError::InvalidInput);
            }
            Ok(())
        }
        _ => Err(AuditError::InvalidInput),
    }
}

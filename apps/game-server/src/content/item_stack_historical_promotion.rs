//! Seven genuine historical source frames; existing Item admission remains unchanged.
use super::{
    ProjectReferenceRecord, ProjectV2Draft,
    item_stack_false_promotion::{StackFalsePromotion, apply_rows},
    world_project_sha256,
};

pub const ITEM_STACK_HISTORICAL_PACKET: &[u8] = include_bytes!(
    "../../../../docs/agents/evidence/OTV2-20261001-item-stack-historical-promotion-v1.json"
);
pub const ITEM_STACK_HISTORICAL_PACKET_SHA256: &str =
    "3798105b2ff54722bf38022a67a0e1fd7477fa1d4cbbc16570cc761f5d5d1ddf";

pub fn apply_item_stack_historical_promotion_v1(
    draft: &mut ProjectV2Draft,
) -> Result<StackFalsePromotion, String> {
    if world_project_sha256(ITEM_STACK_HISTORICAL_PACKET) != ITEM_STACK_HISTORICAL_PACKET_SHA256 {
        return Err("historical stack source frame packet digest drift".into());
    }
    let items = draft.core.records.iter_mut().filter_map(|r| match r {
        ProjectReferenceRecord::Item {
            identity,
            stack_class,
            semantics,
            ..
        } => Some((identity.key.as_str(), *stack_class, semantics)),
        _ => None,
    });
    apply_rows(items, ITEM_STACK_HISTORICAL_PACKET, 7)
}

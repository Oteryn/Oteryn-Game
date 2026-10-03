//! Explicit own-Object immobile=no facts; preserve Item classes and admission.
use super::{
    ProjectReferenceRecord, ProjectV2Draft,
    item_physical_promotion::{PhysicalPromotion, apply_rows},
    world_project_sha256,
};

pub const ITEM_MOVABLE_PACKET: &[u8] =
    include_bytes!("../../../../docs/agents/evidence/OTV2-20261003-item-movable-promotion-v2.json");
pub const ITEM_MOVABLE_PACKET_SHA256: &str =
    "12fb60981ad76e9cd79a5845c5e4c976ce6c76f9d2fea9983c92ff7e5f5ec7b9";

pub fn apply_item_movable_promotion_v1(
    draft: &mut ProjectV2Draft,
) -> Result<PhysicalPromotion, String> {
    if world_project_sha256(ITEM_MOVABLE_PACKET) != ITEM_MOVABLE_PACKET_SHA256 {
        return Err("movable packet digest drift".into());
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
    apply_rows(items, ITEM_MOVABLE_PACKET, 5691, 5691)
}

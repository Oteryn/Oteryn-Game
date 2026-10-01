//! Explicit own-Object immobile=no facts; preserve Item classes and admission.
use super::{
    ProjectReferenceRecord, ProjectV2Draft,
    item_physical_promotion::{PhysicalPromotion, apply_rows},
    world_project_sha256,
};

pub const ITEM_MOVABLE_PACKET: &[u8] =
    include_bytes!("../../../../docs/agents/evidence/OTV2-20261001-item-movable-promotion-v1.json");
pub const ITEM_MOVABLE_PACKET_SHA256: &str =
    "e619b930e877fde117996a867c11676ce423aeca4931d3ae6f37a4afdd799939";

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
    apply_rows(items, ITEM_MOVABLE_PACKET, 5692, 5692)
}

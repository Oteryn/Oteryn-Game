//! Historical external-container data under the exact current named-House actor.
use super::*;
use crate::durability::item_loop_move::{LoopContainer, LoopGround};

pub(super) async fn external(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    context: &HouseActionContext,
    fence: CurrentCharacterItemFence,
    container: [u8; 16],
) -> Option<LoopContainer> {
    let root = admission
        .root
        .read_item_loop_ground_container_root(admission.character, fence, container)
        .await
        .ok()??;
    let roots = admission
        .root
        .read_house_death_ground(admission.holder, context)
        .await
        .ok()??;
    let root = roots
        .into_iter()
        .find(|entry| entry.item.item_instance_id == root)?;
    let cell =
        crate::durability::spell_items_abi::decode_ground_cell(&root.item.ground.spatial_position)
            .ok()?;
    let at = context.position();
    if cell.z != at[2] || cell.x.abs_diff(at[0]) > 1 || cell.y.abs_diff(at[1]) > 1 {
        return None;
    }
    admission
        .root
        .read_item_loop_ground_container(
            admission.character,
            fence,
            container,
            root.item.item_instance_id,
            LoopGround {
                spatial_position: root.item.ground.spatial_position,
                map_revision: root.item.ground.map_revision,
                content_revision: root.item.ground.content_revision,
                placement_context: root.item.ground.native_room_placement_context,
            },
        )
        .await
        .ok()?
}

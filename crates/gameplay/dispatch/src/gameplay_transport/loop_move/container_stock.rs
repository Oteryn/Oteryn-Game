//! Views of validated shared stock; custody and replay remain with map MINT and item loop.
use super::*;

pub(super) fn definition(index: &ItemDefinitionIndex, item: u16) -> Option<NonZeroU32> {
    let key = format!("oteryn:item.tibia.i{item}");
    index.definition_ref("Item", &key, index.revision(&key)?)
}
pub(super) async fn observe(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    fence: Option<CurrentCharacterItemFence>,
    target: ItemKey,
) -> Option<ContainerObservation> {
    let ItemKey::MapBase {
        bundle_digest,
        placement_key,
        reset_epoch,
    } = target
    else {
        return None;
    };
    let fence = fence?;
    let _permit = admission.spell_lane_permit().await?;
    let (index, at) = {
        let mut runtime = admission.runtime.lock().await;
        runtime
            .player_control_facts(actor, fence.game_session_id)
            .ok()?;
        let index = pinned_index(&runtime, admission.active_generation)?;
        let at = super::super::bundle_map_position(&mut runtime, admission.movement_cells, actor)?;
        (index, at)
    };
    let base = at.map.overlay.base();
    if bundle_digest != base.digest() || reset_epoch != admission.reset_epoch {
        return Some(ContainerObservation::Gone);
    }
    let (pos, ordinal) = view::placement_of(placement_key)?;
    let stock = base.container_stock(pos)?;
    if ordinal != stock.parent_ordinal {
        return None;
    }
    // A view needs the same live durable session proof as ordinary Ground containers.
    let current = super::super::FreshAdmissionStore::from_root(admission.root.clone())
        .current_session_at(fence.game_session_id)
        .await
        .ok()?
        .0;
    if current.session_state() != super::super::GameSessionState::Active
        || super::super::item_fence_of(current) != Some(fence)
    {
        return Some(ContainerObservation::Gone);
    }
    let player = TilePos {
        x: u16::try_from(at.at.x).ok()?,
        y: u16::try_from(at.at.y).ok()?,
        floor: i8::try_from(at.at.floor).ok()?,
    };
    if !crate::map::overlay::pickup::within_reach(player, pos) {
        return Some(ContainerObservation::TooFar);
    }
    let ground = loop_ground(admission, base, Some(index)).await?;
    let consumed = ground
        .overlay()
        .tile(pos)
        .is_some_and(|tile| tile.is_hidden(stock.stock_ordinal));
    let entries = if consumed {
        Vec::new()
    } else {
        vec![ViewItem {
            key: ItemKey::MapBase {
                bundle_digest,
                placement_key: view::placement_key(pos, stock.stock_ordinal)?,
                reset_epoch,
            },
            item_definition_ref: definition(index, stock.item)?,
            count: NonZeroU32::new(stock.count)?,
            sub_type: 0,
        }]
    };
    Some(ContainerObservation::Container {
        parent: None,
        capacity: stock.capacity,
        entries,
    })
}

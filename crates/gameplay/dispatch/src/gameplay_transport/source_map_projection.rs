//! Read-only original B3 display for the sealed SourceWorld activity frame.
use super::*;
use oteryn_gameplay_world::map::{boot::BundleCollisionIndexExt, view::MapFacts};

/// A Source display adds only the existing map capability, never the wider
/// mutable Bundle offer. The original non-Bundle capability gates remain intact.
pub(super) fn offered(
    base: &'static [capabilities::OfferedCapability],
    qualified: bool,
) -> &'static [capabilities::OfferedCapability] {
    use oteryn_protocol_oteryn::world_map::{
        CAPABILITY_WORLD_MAP_VIEW_REQUIRES, CAPABILITY_WORLD_MAP_VIEW_V1 as MAP,
    };
    if !qualified
        || base.iter().any(|row| row.id == MAP)
        || !CAPABILITY_WORLD_MAP_VIEW_REQUIRES
            .iter()
            .all(|id| base.iter().any(|row| row.id == *id))
    {
        return base;
    }
    type Cached = (
        &'static [capabilities::OfferedCapability],
        &'static [capabilities::OfferedCapability],
    );
    static CACHE: std::sync::Mutex<Vec<Cached>> = std::sync::Mutex::new(Vec::new());
    let mut cache = CACHE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some((_, result)) = cache.iter().find(|(key, _)| *key == base) {
        return result;
    }
    let mut result = base.to_vec();
    let index = result.partition_point(|row| row.id < MAP);
    result.insert(
        index,
        capabilities::OfferedCapability {
            id: MAP,
            requires: &CAPABILITY_WORLD_MAP_VIEW_REQUIRES,
        },
    );
    let result: &'static [capabilities::OfferedCapability] = Box::leak(result.into_boxed_slice());
    cache.push((base, result));
    result
}

pub(super) async fn observe(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    view: &mut world_map::SessionMapView,
    items: &mut item_view::SessionItemView,
) -> Option<Result<Option<world_map::MapUpdate>, world_map::MapViewError>> {
    let projection = admission.movement_cells.source_map_projection()?;
    if !projection.matches_scope(admission.movement_cells.scope()) {
        return None;
    }
    let _permit = admission.spell_lane_permit().await?;
    let (at, content_generation) = {
        let mut runtime = admission.runtime.lock().await;
        if !matches!(
            runtime.binding().runtime_scope(),
            RuntimeScopeRefV1::Instance { .. }
        ) || !projection.matches_pin(runtime.content_pin())
            || runtime.owner_fence().is_err()
        {
            return None;
        }
        let snapshot = runtime.borrow_movement_position().read(actor).ok()?;
        if snapshot.context() != runtime.pinned_movement_context() {
            return None;
        }
        let position = snapshot.position();
        (
            world_spatial::ActorPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
            runtime.content_pin().client_artifact_digest(),
        )
    };
    let map = projection.bundle().map();
    // This port displays authored B3 only. No callback/door state from the
    // origin Channel may leak into a different runtime namespace. base_only
    // retains House rotations, so this closed non-House room refuses those too.
    let [min_x, min_y, max_x, max_y, min_floor, max_floor] = projection.native_bounds();
    for floor in min_floor..=max_floor {
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let position = crate::map::overlay::TilePos {
                    x: u16::try_from(x).ok()?,
                    y: u16::try_from(y).ok()?,
                    floor: i8::try_from(floor).ok()?,
                };
                if map.facts.house_tile(position) {
                    return None;
                }
            }
        }
    }
    let facts = map.facts.base_only();
    let source = world_map::MapViewSource {
        overlay: &map.overlay,
        facts: &facts,
        content_generation,
        reset_epoch: admission.reset_epoch,
    };
    Some(view.update_source_room(items, &source, at, projection))
}

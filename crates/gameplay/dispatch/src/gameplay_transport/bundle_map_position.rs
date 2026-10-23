//! Domain-17 composition under the booted reset epoch.
use super::*;

/// MAP-CUTOVER-1b: domain 17 of the bundle World `movement_cells` carry, at `actor`'s current
/// position in `runtime`; `None` on a fixture World or when the actor has no current position.
/// This compatibility helper supplies epoch zero; production observes the boot-read active epoch.
#[cfg(test)]
pub(super) fn bundle_world_map(
    runtime: &mut ChannelRuntimeV1,
    movement_cells: &NativeEntryMovementCells,
    actor: ExactActorRef,
    view: &mut world_map::SessionMapView,
    items: &mut item_view::SessionItemView,
) -> Option<Result<Option<world_map::MapUpdate>, world_map::MapViewError>> {
    let at = bundle_map_position(runtime, movement_cells, actor)?;
    Some(at.update(view, items, None, 0))
}

/// What domain 17 needs from the Channel runtime: the bundle map, the actor's native position
/// and the content generation. It is read under the runtime lock; the view update that composes
/// and encodes the window runs after the lock is released (§1.4).
pub(super) struct BundleMapPosition<'a> {
    pub(super) map: &'a crate::map::boot::BundleMap,
    pub(super) at: ActorPosition,
    pub(super) content_generation: [u8; 32],
}

impl BundleMapPosition<'_> {
    /// The view update over the bundle map and, when given, the Channel's loop Ground items
    /// (ITEM-LOOP-1).
    fn update(
        &self,
        view: &mut world_map::SessionMapView,
        items: &mut item_view::SessionItemView,
        ground: Option<&loop_move::LoopGroundOverlay>,
        reset_epoch: u64,
    ) -> Result<Option<world_map::MapUpdate>, world_map::MapViewError> {
        // MAP-DOOR-1 (Codex 4239177726): one read of every door's state, so each door's
        // appearance and object revision in this update agree.
        let facts = self.map.facts.pinned();
        match ground {
            Some(ground) => {
                let facts = ground.facts(&facts);
                let source = world_map::MapViewSource {
                    overlay: ground.overlay(),
                    facts: &facts,
                    content_generation: self.content_generation,
                    reset_epoch,
                };
                view.update(items, &source, self.at)
            }
            None => {
                let source = world_map::MapViewSource {
                    overlay: &self.map.overlay,
                    facts: &facts,
                    content_generation: self.content_generation,
                    reset_epoch,
                };
                view.update(items, &source, self.at)
            }
        }
    }
}

/// The bundle map and `actor`'s current native position in `runtime`; `None` on a fixture World
/// or when the actor has no current position.
pub(super) fn bundle_map_position<'a>(
    runtime: &mut ChannelRuntimeV1,
    movement_cells: &'a NativeEntryMovementCells,
    actor: ExactActorRef,
) -> Option<BundleMapPosition<'a>> {
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        movement_cells.index()
    else {
        return None;
    };
    let snapshot = runtime.borrow_movement_position().read(actor).ok()?;
    if snapshot.context() != runtime.pinned_movement_context() {
        return None;
    }
    let position = snapshot.position();
    // The runtime floor is legacy `z`; the native floor is `-z`.
    Some(BundleMapPosition {
        map: index.map(),
        at: ActorPosition {
            x: position.x,
            y: position.y,
            floor: position.floor.checked_neg()?,
        },
        content_generation: runtime.content_pin().client_artifact_digest(),
    })
}

pub(super) async fn observe(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    view: &mut world_map::SessionMapView,
    items: &mut item_view::SessionItemView,
) -> Option<Result<Option<world_map::MapUpdate>, world_map::MapViewError>> {
    let (at, index) = {
        let mut runtime = admission.runtime.lock().await;
        let index = item_ref_admission::pinned_index(&runtime, admission.active_generation);
        (
            bundle_map_position(&mut runtime, admission.movement_cells, actor)?,
            index,
        )
    };
    // ITEM-LOOP-1: the dropped items, read after the runtime lock is released.
    let ground = loop_move::loop_ground(admission, at.map.overlay.base(), index).await?;
    Some(at.update(view, items, Some(&ground), admission.reset_epoch))
}

//! Crystal201189f a_piece_of_cake_config.lua:249-350,473-483,575-586.
//! The real Channel cadence and movement owner consume the durable World phase.
use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn refresh_cake_bridge_near_actor(
        &self,
        actor: ExactActorRef,
    ) -> bool {
        let near = {
            let runtime = self.runtime.lock().await;
            runtime.read_actor_position(actor).is_ok_and(|snapshot| {
                let position = snapshot.position();
                position.floor == 7
                    && (33299..=33307).contains(&position.x)
                    && (32050..=32061).contains(&position.y)
            })
        };
        !near || self.refresh_cake_bridge().await
    }

    pub(in crate::gameplay_transport) async fn refresh_cake_bridge(&self) -> bool {
        let Some(map) = self.bundle_map() else {
            return true;
        };
        if map.overlay.base().tile(33303, 32055, -7).is_none() {
            return true;
        }
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return false;
        };
        let Some(index) = native.item_index() else {
            return false;
        };
        let facts = |id| {
            Some(view::EntryFacts {
                definition: MapDefinition::Item(index.definition_ref(
                    "Item",
                    &format!("oteryn:item.tibia.i{id}"),
                    "definition-r1",
                )?),
                terrain_kind: Some(oteryn_world_bundle::bundle::TerrainKind::Ground),
                appearance_id: id,
                blocks_projectile: false,
                pickupable: false,
                bound: false,
                count: 1,
                sub_type: 0,
            })
        };
        let (Some(floor), Some(rail)) = (facts(48227), facts(12113)) else {
            return false;
        };
        let fence = {
            let runtime = self.runtime.lock().await;
            if runtime.binding().runtime_scope()
                != RuntimeScopeRefV1::channel(self.world_id, self.channel_id)
            {
                return true;
            }
            let Ok(owner) = runtime.owner_fence() else {
                return false;
            };
            if super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                .is_none()
            {
                return false;
            }
            CorpseDecayFence {
                runtime_scope: runtime.binding().runtime_scope(),
                scope_ownership_generation: owner.generation(),
            }
        };
        let Ok(reset_epoch) = i64::try_from(self.reset_epoch) else {
            return false;
        };
        let scope = TerrainScope {
            map_revision: crate::map::overlay::map_revision(map.overlay.base()),
            content_generation: native.source_digest(),
            reset_epoch,
        };
        let Ok(present) = self
            .root
            .cake_bridge_present(self.holder, fence, map.overlay.base().digest(), scope)
            .await
        else {
            return false;
        };
        let runtime = self.runtime.lock().await;
        if !runtime
            .owner_fence()
            .is_ok_and(|owner| owner.generation() == fence.scope_ownership_generation)
            || super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                .is_none()
        {
            return false;
        }
        map.facts
            .project_cake_bridge(map.overlay.base(), present, floor, rail)
    }
}

//! Crystal201189f movements_cake_base_isle.lua and movements_extraction_spot.lua.
use super::*;
use crate::durability::{cake_event::CakeErrandStep, spell_owner_commit::SpellLanePermit};
use crate::foundation::{GameSessionId, MovementPositionSnapshot};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn step_cake_errand(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        session: GameSessionId,
        entered: MovementPositionSnapshot,
    ) {
        let (Some(map), Some(native)) = (
            self.bundle_map(),
            self.active_generation
                .and_then(|active| active.native_gameplay()),
        ) else {
            return;
        };
        let Some(index) = native.item_index() else {
            return;
        };
        let position = entered.position();
        let (Ok(x), Ok(y), Ok(floor)) = (
            u16::try_from(position.x),
            u16::try_from(position.y),
            i8::try_from(-position.floor),
        ) else {
            return;
        };
        let at = TilePos { x, y, floor };
        let Ok(Some(tile)) = view::compose_tile(&map.overlay, &map.facts, at) else {
            return;
        };
        // Only a canonical base placement can arm/extract; a dropped lookalike
        // never becomes a quest source, and ambiguous stacks never choose one.
        let mut sources = tile.entries.into_iter().filter_map(|entry| {
            if !matches!(entry.source.key(), MapEntryKey::Base(_)) {
                return None;
            }
            let MapDefinition::Item(reference) = entry.facts.definition else {
                return None;
            };
            let id = super::super::item_look::typed_definition(index, reference)
                .and_then(|definition| item_id(&definition.production_key))?;
            (matches!(id, 12090..=12095)
                || (id == 8886
                    && crate::durability::raid_runner::cake::is_machine_position([
                        x,
                        y,
                        u16::try_from(position.floor).ok()?,
                    ])))
            .then_some(id)
        });
        let (Some(source_item), None) = (sources.next(), sources.next()) else {
            return;
        };
        let Ok(Some(current)) = self.current_quest_fence(session).await else {
            return;
        };
        let fence = current.item_fence();
        {
            let runtime = self.runtime.lock().await;
            if runtime.read_actor_position(actor) != Ok(entered)
                || super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                    .is_none()
                || !runtime
                    .bound_attacker_lease(actor, session)
                    .is_ok_and(|lease| {
                        lease.character_id().as_bytes() == fence.character_id.as_bytes()
                            && lease.generation() == fence.character_lease_generation
                    })
            {
                return;
            }
        }
        let Ok(reset_epoch) = i64::try_from(self.reset_epoch) else {
            return;
        };
        let mut encoded = [0; 12];
        encoded[..4].copy_from_slice(&position.x.to_be_bytes());
        encoded[4..8].copy_from_slice(&position.y.to_be_bytes());
        encoded[8..].copy_from_slice(&i32::from(position.floor).to_be_bytes());
        let request = CakeErrandStep {
            actor: actor.placement_identity(),
            movement_revision: entered.revision(),
            position: encoded,
            source_item,
            bundle: map.overlay.base().digest(),
            scope: TerrainScope {
                map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                content_generation: native.source_digest(),
                reset_epoch,
            },
        };
        if self
            .root
            .record_world_cake_step(permit, self.character, self.holder, fence, request)
            .await
            .is_err()
        {
            super::super::operator_event("cake_errand_step_unavailable");
        }
    }
}

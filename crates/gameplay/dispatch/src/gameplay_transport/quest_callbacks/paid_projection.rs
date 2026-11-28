//! Checkpoint a paid movement and restore the paid source scene on a rebuilt map.
use super::*;
use crate::durability::character_last_position::LastVitals;
use crate::durability::quest_world_callback::paid_projection::PaidArrival;
use crate::foundation::SourceStepCommitProof;
pub(super) const DREAD_LORD_SCROLL: &str = "oteryn:quest-callback/dreamer-dread-lord-scroll";

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn paid_arrival(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        session: GameSessionId,
        proof: &impl SourceStepCommitProof,
    ) -> Option<PaidArrival> {
        let (actor, _, destination, _) = proof.parts();
        let lease = runtime.bound_attacker_lease(actor, session).ok()?;
        let mut placed = self.placed_record(runtime, destination);
        let state = states.get(runtime, actor, session);
        if let Some(state) = state {
            placed.record.vitals = state
                .saved_vitals()
                .and_then(|(health, mana)| LastVitals::new(health, mana));
            placed.record.rest = Some(state.bed_rest_snapshot(self.owner_now().get()));
        }
        Some(PaidArrival {
            character: *lease.character_id().as_bytes(),
            lease_generation: lease.generation(),
            placed,
            retain_saved_vitals: state.is_none(),
        })
    }

    pub(in crate::gameplay_transport) async fn restore_paid_quest_scene(
        &self,
        map: &BundleMap,
    ) -> Option<()> {
        let source_loaded = self.callback_content().iter().any(|callback| {
            callback.key == DREAD_LORD_SCROLL
                && callback.selector.native.as_ref().is_some_and(|source| {
                    let pos = tile(source.position);
                    map.overlay.base().tile(pos.x, pos.y, pos.floor).is_some()
                })
        });
        if !source_loaded {
            return Some(());
        }
        let permit = self.spell_lane_permit().await?;
        let (binding, frame) = {
            let runtime = self.runtime.lock().await;
            runtime.owner_fence().ok()?;
            (
                runtime.binding(),
                runtime.content_pin().frame_binding_digest(),
            )
        };
        let scenes = self
            .root
            .read_paid_scroll_scenes(
                self.character,
                self.holder,
                &permit,
                binding.runtime_scope(),
                binding.scope_generation().get(),
                crate::map::overlay::map_revision(map.overlay.base()),
                frame,
            )
            .await
            .ok()?;
        let runtime = self.runtime.lock().await;
        runtime.owner_fence().ok()?;
        if runtime.binding() != binding || runtime.content_pin().frame_binding_digest() != frame {
            return None;
        }
        for (receipt, source, edits) in scenes {
            let pos = tile(source.position);
            let key = placement_key(pos, source.ordinal)?;
            let base = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
            let (id, _) = base
                .ids()
                .iter()
                .zip(base.depths())
                .filter(|(_, depth)| **depth == 0)
                .nth(usize::from(source.ordinal))?;
            let before = map.facts.base_entry(pos, source.ordinal, *id)?;
            if before.appearance_id != source.appearance || map.facts.house_tile(pos) {
                return None;
            }
            let [edit] = edits.as_slice() else {
                return None;
            };
            let position = tile(edit.position);
            if edit.remove.is_some()
                || edit.passable != Some(true)
                || map.facts.house_tile(position)
                || map
                    .overlay
                    .base()
                    .tile(position.x, position.y, position.floor)
                    .is_none()
            {
                return None;
            }
            let facts = map
                .facts
                .movement_item_facts(edit.create?)
                .filter(|facts| !facts.pickupable)?;
            if !map
                .facts
                .restore_paid_scene_item(key, position, facts, receipt)
            {
                return None;
            }
        }
        Some(())
    }
}

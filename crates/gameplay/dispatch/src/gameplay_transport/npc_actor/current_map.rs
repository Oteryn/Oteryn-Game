//! NPC-only current tile read: source/trigger facts under the runtime owner, Item facts
//! locked in the existing scope transaction, then an exact synchronous position commit.
use super::*;
use crate::content::npc_catalogue::walk_terrain;
use crate::durability::DurabilityError;
use crate::durability::native_world_items::SpellGroundTarget;
use crate::durability::spell_item_transaction::{
    assert_spell_item_scope_in_transaction, begin_spell_owner_transaction, check_scope_transaction,
    read_creature_scope_tile_in_transaction,
};
use crate::map::boot::BundleMap;
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Trigger};
use std::sync::OnceLock;

struct PreparedStep {
    step: NpcStepCandidate,
    target: SpellGroundTarget,
}

fn static_clear(map: &BundleMap, at: MovementLocalPosition) -> bool {
    static CALLBACKS: OnceLock<Option<Vec<Callback>>> = OnceLock::new();
    let Some(callbacks) =
        CALLBACKS.get_or_init(|| oteryn_gameplay_quests::quest::world_callbacks::load().ok())
    else {
        return false;
    };
    let (Ok(x), Ok(y), Ok(floor)) = (
        u16::try_from(at.x),
        u16::try_from(at.y),
        i8::try_from(-at.floor),
    ) else {
        return false;
    };
    let pos = TilePos { x, y, floor };
    let Some(tile) = map.overlay.base().tile(x, y, floor) else {
        return false;
    };
    // A registered position trigger remains a trigger even when its quest action would refuse.
    if callbacks.iter().any(|callback| {
        callback.trigger == Trigger::StepIn
            && callback
                .selector
                .position
                .is_some_and(|p| p.x == x && p.y == y && p.floor == floor)
    }) {
        return false;
    }
    for (ordinal, id) in tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .map(|(id, _)| *id)
        .enumerate()
    {
        let Ok(ordinal) = u8::try_from(ordinal) else {
            return false;
        };
        let Some(key) = crate::map::view::placement_key(pos, ordinal) else {
            return false;
        };
        if map.facts.base_hidden(key) {
            continue;
        }
        let Some(entry) = map.facts.base_entry(pos, ordinal, id) else {
            return false;
        };
        // Bound doors include step-out handlers; unique/action placements include selectors.
        if entry.bound
            || callbacks.iter().any(|callback| {
                callback.trigger == Trigger::StepIn
                    && callback.selector.position.is_none()
                    && callback.selector.unique.is_none()
                    && callback.selector.action.is_none()
                    && callback.selector.appearance == Some(entry.appearance_id)
            })
        {
            return false;
        }
    }
    true
}

impl super::super::ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn commit_npc_walk(
        &self,
        proposal: NpcWalkProposal,
        book: &NpcWalkBook,
    ) -> Option<u64> {
        let permit = self.spell_lane_permit().await?;
        let room = self.rooms.gate_monster_ai()?;
        let native = self.active_generation?.native_gameplay()?;
        let map = self.bundle_map()?;
        let expected = proposal.expected_position();
        let actor = proposal.occurrence().actor;
        let (scope, generation, steps) = {
            let mut runtime = self.runtime.lock().await;
            if native.source_digest() != runtime.content_pin().server_artifact_digest()
                || !static_clear(map, expected.position())
            {
                return None;
            }
            let candidates = proposal
                .bundle_terrain_candidates(&mut runtime, room, book)
                .ok()?;
            let mut steps = Vec::new();
            for step in candidates {
                if !static_clear(map, step.destination) {
                    continue;
                }
                let tile = room
                    .movement_cells()
                    .spell_tiles()
                    .lookup(room.movement_cells().scope(), cell(step.destination))
                    .ok()?;
                if tile
                    .ground_source_id()
                    .is_none_or(|id| walk_terrain::damaging_field(id) != Some(false))
                    || tile
                        .top_source_ids()
                        .iter()
                        .any(|id| walk_terrain::damaging_field(*id) != Some(false))
                {
                    continue;
                }
                let target =
                    SpellGroundTarget::for_native_tile_read(room, &runtime, step.destination)
                        .ok()?;
                steps.push(PreparedStep { step, target });
            }
            let binding = runtime.binding();
            (
                binding.runtime_scope(),
                binding.scope_generation().get(),
                steps,
            )
        };
        if steps.is_empty() {
            return None;
        }
        let pass = self.root.try_issue_semantic_pass().ok()?;
        let mut context = (self, &permit, steps);
        pass.run_with_context(&mut context, move |holder, deadline, context| {
            Box::pin(async move {
                let (owner, permit, steps) = context;
                let mut tx = begin_spell_owner_transaction(holder, deadline).await?;
                let authority = assert_spell_item_scope_in_transaction(
                    &mut tx,
                    permit,
                    owner.root,
                    owner.character,
                    owner.holder,
                    scope,
                    generation,
                )
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
                // Read in spatial order beneath the Item owner's runtime-scope lock;
                // preserve the shuffled preference only after every read is fenced.
                let mut order = (0..steps.len()).collect::<Vec<_>>();
                order.sort_by(|left, right| {
                    steps[*left]
                        .target
                        .spatial_position
                        .cmp(&steps[*right].target.spatial_position)
                });
                let mut clear = vec![false; steps.len()];
                for index in order {
                    let prepared = &steps[index];
                    let items = read_creature_scope_tile_in_transaction(
                        &mut tx,
                        &authority,
                        &prepared.target,
                    )
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                    clear[index] = items.items.iter().all(|item| {
                        let Some(policy) = native.item_policy(
                            &item.definition.production_key,
                            &item.definition.revision_ref,
                        ) else {
                            return false;
                        };
                        let Ok(id) = policy
                            .record()
                            .production_binding
                            .external_id
                            .parse::<u32>()
                        else {
                            return false;
                        };
                        let attributes = &policy.record().attributes;
                        let blocks =
                            oteryn_gameplay_items::native_world_items::source_barrier_collision(
                                policy,
                            )
                            .map(|flags| flags.0)
                            .or(attributes.blocks_movement);
                        blocks == Some(false)
                            && !item.blocks_movement
                            && !item.immovable_block_solid
                            && walk_terrain::has_height(id) == Some(false)
                            && walk_terrain::damaging_field(id) == Some(false)
                            && attributes.field_condition.as_ref().is_none_or(|field| {
                                field.condition_values().is_ok_and(|value| value.is_none())
                            })
                    });
                }
                check_scope_transaction(&mut tx, &authority)
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?;
                // No awaited lock follows the final transaction fence. A busy runtime
                // refuses this occurrence; it never commits from a detached previous read.
                let Ok(mut runtime) = owner.runtime.try_lock() else {
                    return Ok(None);
                };
                if runtime.binding().runtime_scope() != scope
                    || runtime.binding().scope_generation().get() != generation
                    || runtime.pinned_movement_context() != expected.context()
                    || runtime.borrow_movement_position().read(actor).ok() != Some(expected)
                    || !static_clear(map, expected.position())
                {
                    return Ok(None);
                }
                for (index, prepared) in steps.iter().enumerate() {
                    if !clear[index]
                        || !static_clear(map, prepared.step.destination)
                        || runtime.position_occupied_by_other(actor, prepared.step.destination)
                            != Ok(false)
                    {
                        continue;
                    }
                    let moved = owner.states.chat.with_npc_customer_state(actor, |talking| {
                        !talking
                            && runtime
                                .borrow_movement_position()
                                .commit_cardinal(expected, prepared.step.destination)
                                .is_ok()
                    }) == Some(true);
                    if moved {
                        return Ok(Some(
                            u64::try_from(prepared.step.duration.as_micros()).unwrap_or(u64::MAX),
                        ));
                    }
                }
                Ok(None)
            })
        })
        .await
        .ok()
        .flatten()
    }
}

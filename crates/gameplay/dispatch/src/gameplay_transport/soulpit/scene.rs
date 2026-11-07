//! Read the current shared Map/Item owners before borrowing the physical kernel.
use super::super::ComposedFreshAdmission;
use super::runtime::ActivityHandoffScene;
use crate::durability::native_world_items::SpellGroundTarget;
use crate::durability::spell_item_transaction::{
    assert_spell_item_scope_in_transaction, begin_spell_owner_transaction, check_scope_transaction,
    read_creature_scope_tile_in_transaction,
};
use crate::foundation::{ExactActorRef, GameSessionId, MovementLocalPosition};
use std::collections::BTreeMap;

pub(super) struct NativeScene {
    room: Option<std::sync::Arc<crate::content::QualifiedNativeEntryRoom>>,
    tiles: BTreeMap<(i32, i32, i16), (bool, bool)>,
    targets: Vec<(ExactActorRef, ExactActorRef)>,
    attributes: Vec<((ExactActorRef, GameSessionId), i32)>,
}
impl ActivityHandoffScene for NativeScene {
    fn emit_spawn_effect(&self, at: MovementLocalPosition, effect: u16) -> bool {
        self.room
            .as_ref()
            .is_some_and(|room| super::super::soul_war_scene::emit_effect(room, at, effect))
    }
    fn permits_spawn(&self, at: MovementLocalPosition) -> bool {
        self.tiles
            .get(&(at.x, at.y, at.floor))
            .is_some_and(|(walk, projectile)| *walk && !projectile)
    }
    fn attack_target(&self, actor: ExactActorRef) -> Option<ExactActorRef> {
        self.targets
            .iter()
            .find(|(source, _)| *source == actor)
            .map(|(_, target)| *target)
    }
    fn permits_condition(
        &self,
        source: ExactActorRef,
        target: ExactActorRef,
        from: MovementLocalPosition,
        to: MovementLocalPosition,
    ) -> bool {
        self.targets.iter().any(|(actor, _)| *actor == source)
            && self
                .attributes
                .iter()
                .any(|((actor, _), _)| *actor == target)
            && crate::durability::native_world_read::source_sight_steps(from, to).is_ok_and(
                |steps| {
                    steps.iter().all(|(at, exempt)| {
                        *exempt
                            || self
                                .tiles
                                .get(&(at.x, at.y, at.floor))
                                .is_some_and(|(_, projectile)| !projectile)
                    })
                },
            )
    }
    fn player_attribute_damage_delta(
        &self,
        target: ExactActorRef,
        session: GameSessionId,
        _: u64,
    ) -> Option<i32> {
        self.attributes
            .iter()
            .find(|((actor, id), _)| *actor == target && *id == session)
            .map(|(_, delta)| *delta)
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// One causal entry cue per genuine installed participant. The caller retains
    /// this pre-emission cursor across retries and carries it only over its barrier.
    pub(in crate::gameplay_transport) async fn emit_soulpit_entry_effects(
        &self,
        connections: &[(
            super::super::connection::AdmittedSession,
            super::super::connection::AdmittedSession,
        )],
    ) -> Result<Vec<(GameSessionId, u64)>, ()> {
        use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
        use crate::map::boot::BundleCollisionIndexExt as _;
        let room = self.states.activity_room.as_ref().ok_or(())?;
        if room.pinned_activity_source_room()
            != Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::Soulpit)
        {
            return Err(());
        }
        let runtime = self.runtime.lock().await;
        runtime.owner_fence().map_err(|_| ())?;
        let active = self.active_generation.ok_or(())?;
        if !super::super::native_cue_offer::source_available(active, room, runtime.content_pin()) {
            return Err(());
        }
        let NativeMovementCollisionIndex::Bundle(index) = room.movement_cells().index() else {
            return Err(());
        };
        let map = index.map();
        let cursor = map
            .facts
            .movement_effects_since(0)
            .last()
            .map_or(0, |row| row.0);
        let mut positions = Vec::with_capacity(connections.len());
        for (_, next) in connections {
            let actor = next.runtime_actor.ok_or(())?;
            if !runtime
                .player_control_facts(actor, next.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return Err(());
            }
            let at = runtime
                .read_actor_position(actor)
                .map_err(|_| ())?
                .position();
            if at.floor != 8 || at.y != 31151 || !(32373..=32377).contains(&at.x) {
                return Err(());
            }
            positions.push((
                next.game_session_id,
                crate::map::overlay::TilePos {
                    x: u16::try_from(at.x).map_err(|_| ())?,
                    y: u16::try_from(at.y).map_err(|_| ())?,
                    floor: -8,
                },
            ));
        }
        for (_, at) in &positions {
            if !map.facts.emit_movement_effect(*at, 11) {
                return Err(());
            }
        }
        Ok(positions
            .into_iter()
            .map(|(session, _)| (session, cursor))
            .collect())
    }

    /// Descendant of committed entry and completed physical installation. The
    /// independently owned Instance map receives the source scripted state.
    pub(in crate::gameplay_transport) async fn activate_soulpit_obelisk(&self) -> Result<(), ()> {
        self.set_soulpit_obelisk(true).await
    }

    pub(in crate::gameplay_transport) async fn reset_soulpit_obelisk(&self) -> Result<(), ()> {
        self.set_soulpit_obelisk(false).await
    }

    async fn set_soulpit_obelisk(&self, active: bool) -> Result<(), ()> {
        use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
        use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
        let room = self.states.activity_room.as_ref().ok_or(())?;
        if room.pinned_activity_source_room() != Some(PinnedActivitySourceRoom::Soulpit) {
            return Err(());
        }
        let generation = self.active_generation.ok_or(())?;
        let runtime = self.runtime.lock().await;
        if runtime.owner_fence().is_err()
            || !matches!(
                runtime.binding().runtime_scope(),
                crate::foundation::RuntimeScopeRefV1::Instance { .. }
            )
            || !super::super::native_cue_offer::source_available(
                generation,
                room,
                runtime.content_pin(),
            )
        {
            return Err(());
        }
        let NativeMovementCollisionIndex::Bundle(index) = room.movement_cells().index() else {
            return Err(());
        };
        use crate::map::boot::BundleCollisionIndexExt as _;
        let map = index.map();
        let tile = crate::map::view::compose_tile(
            &map.overlay,
            &map.facts,
            crate::map::overlay::TilePos {
                x: 32375,
                y: 31157,
                floor: -8,
            },
        )
        .map_err(|_| ())?
        .ok_or(())?;
        let mut glyphs = tile
            .entries
            .iter()
            .filter(|entry| matches!(entry.facts.appearance_id, 47367 | 47379));
        let glyph = glyphs.next().ok_or(())?;
        if glyphs.next().is_some() {
            return Err(());
        }
        let crate::map::view::EntrySource::Base { placement_key, .. } = glyph.source else {
            return Err(());
        };
        let target = if active { 47379 } else { 47367 };
        if glyph.facts.appearance_id == target {
            return Ok(());
        }
        map.facts
            .transform_movement_item(placement_key, target)
            .then_some(())
            .ok_or(())
    }

    pub(super) async fn prepare_soulpit_scene(
        &self,
        permit: &crate::durability::spell_owner_commit::SpellLanePermit,
    ) -> Result<NativeScene, ()> {
        let owned_room = self.states.activity_room.clone().ok_or(())?;
        if owned_room.pinned_activity_source_room()
            != Some(crate::content::bosses::activity_template::PinnedActivitySourceRoom::Soulpit)
        {
            return Err(());
        }
        let room = self.effective_rooms().gate_native_combat_cast().ok_or(())?;
        let native = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
            .ok_or(())?;
        let now = self.owner_now().get();
        let (scope, generation, requests, targets, attributes) = {
            let runtime = self.runtime.lock().await;
            let objects = self.door.lock().await;
            let static_owner = crate::durability::native_world_read::qualified_static_owner(
                room, &runtime, &*objects,
            )
            .map_err(|_| ())?;
            let states = self.states.spell_states.lock().await;
            let binding = runtime.binding();
            let mut requests = Vec::new();
            let mut targets = Vec::new();
            let mut attributes = Vec::new();
            for (actor, _, session) in runtime.positioned_actor_census().map_err(|_| ())? {
                if let Some(session) = session {
                    if let Some(state) = states.get(&runtime, actor, session) {
                        attributes.push((
                            (actor, session),
                            state.owned_conditions().attribute_damage_delta_at(now),
                        ));
                    }
                } else if let Some(target) = states
                    .monster_melee
                    .creature_ai
                    .state(actor)
                    .and_then(|s| s.target())
                {
                    targets.push((actor, target));
                }
            }
            let floor = runtime.content_pin().entry_start().floor;
            for y in 31132..=31153 {
                for x in 32362..=32390 {
                    let at = MovementLocalPosition { x, y, floor };
                    let Some(tile) = crate::durability::native_world_read::tile::<
                        std::convert::Infallible,
                    >(room, at)
                    .ok()
                    .flatten() else {
                        continue;
                    };
                    let flags = tile.flags();
                    let walk = tile.ground_present()
                        && !flags.block_solid
                        && !flags.protection_zone
                        && !flags.floor_change
                        && !static_owner.blocks_movement(at);
                    let target = SpellGroundTarget::for_native_tile_read(room, &runtime, at)
                        .map_err(|_| ())?;
                    requests.push((at, target, walk, flags.block_projectile));
                }
            }
            (
                binding.runtime_scope(),
                binding.scope_generation(),
                requests,
                targets,
                attributes,
            )
        };
        // Owned target addresses above retain the exact frame/map/content proof; no
        // runtime, spell-state or physical-kernel borrow crosses the Item read await.
        let pass = self.root.try_issue_semantic_pass().map_err(|_| ())?;
        let mut context = (self, permit, requests, native);
        let tiles = pass.run_with_context(&mut context, move |holder,deadline,context| Box::pin(async move {
            let (owner,permit,requests,native) = context;
            let mut tx = begin_spell_owner_transaction(holder,deadline).await?;
            let authority = assert_spell_item_scope_in_transaction(&mut tx,permit,owner.root,owner.character,owner.holder,scope,generation.get()).await.map_err(|_| crate::durability::DurabilityError::Unavailable)?;
            let mut tiles = BTreeMap::new();
            for (at,target,walk,projectile) in requests.iter() {
                let Ok(items) = read_creature_scope_tile_in_transaction(&mut tx,&authority,target).await else { return Err(crate::durability::DurabilityError::Unavailable); };
                let mut walk = *walk;
                let mut projectile = *projectile;
                let mut known = true;
                for item in items.items {
                    let Some(policy) = native.item_policy(&item.definition.production_key,&item.definition.revision_ref) else { known=false; break; };
                    let attributes = &policy.record().attributes;
                    let flags = oteryn_gameplay_items::native_world_items::source_barrier_collision(policy)
                        .or_else(|| Some((attributes.blocks_movement?,attributes.blocks_projectile?)));
                    let Some((solid,blocks_projectile)) = flags else {known=false;break;};
                    if item.blocks_movement != solid || item.blocks_projectile != blocks_projectile {known=false;break;}
                    walk &= !solid;
                    projectile |= blocks_projectile;
                }
                if known { tiles.insert((at.x,at.y,at.floor),(walk,projectile)); }
            }
            check_scope_transaction(&mut tx,&authority).await.map_err(|_| crate::durability::DurabilityError::Unavailable)?;
            tx.rollback().await.map_err(|_| crate::durability::DurabilityError::Unavailable)?;
            Ok(tiles)
        })).await.map_err(|_| ())?;
        Ok(NativeScene {
            room: Some(owned_room),
            tiles,
            targets,
            attributes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srx3_soulpit_source_zone_rejects_projectile_blocking_ground() {
        // Crystal zones.lua:22 and tile.lua:46: randomPosition excludes a
        // projectile-blocked tile even when ordinary movement can enter it.
        let at = MovementLocalPosition {
            x: 32374,
            y: 31145,
            floor: 8,
        };
        let scene = NativeScene {
            room: None,
            tiles: BTreeMap::from([((at.x, at.y, at.floor), (true, true))]),
            targets: Vec::new(),
            attributes: Vec::new(),
        };
        assert!(!scene.permits_spawn(at));
    }
}

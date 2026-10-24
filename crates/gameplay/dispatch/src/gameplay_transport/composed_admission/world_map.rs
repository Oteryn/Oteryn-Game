use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// MAP-CUTOVER-1b: the map view of a bundle World, which its movement cells carry; `None`
    /// on a fixture World.
    pub(in crate::gameplay_transport) fn bundle_map(&self) -> Option<&crate::map::boot::BundleMap> {
        match self.movement_cells.index() {
            crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) => {
                Some(index.map())
            }
            _ => None,
        }
    }

    pub(in crate::gameplay_transport) fn observation(
        runtime: &ChannelRuntimeV1,
        position: crate::foundation::MovementLocalPosition,
    ) -> WorldSpatialObservation {
        WorldSpatialObservation {
            content_generation: runtime.content_pin().client_artifact_digest(),
            actor_position: ActorPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
        }
    }

    pub(in crate::gameplay_transport) async fn observe_actor_position(
        &self,
        actor: ExactActorRef,
    ) -> Option<WorldSpatialObservation> {
        let mut runtime = self.runtime.lock().await;
        let snapshot = runtime.borrow_movement_position().read(actor).ok()?;
        if snapshot.context() != runtime.pinned_movement_context() {
            return None;
        }
        Some(Self::observation(&runtime, snapshot.position()))
    }

    pub(in crate::gameplay_transport) async fn observe_channel_entities(
        &self,
        actor: ExactActorRef,
    ) -> Option<world_spatial::ChannelEntities> {
        use world_spatial::VisibleKind;
        let runtime = self.runtime.lock().await;
        let visible = runtime.visible_entities();
        let entities = (visible
            .players
            .iter()
            .map(|entry| (VisibleKind::Player, entry)))
        .chain(
            visible
                .creatures
                .iter()
                .map(|entry| (VisibleKind::Creature, entry)),
        )
        .chain(visible.npcs.iter().map(|entry| (VisibleKind::Npc, entry)))
        .map(|(kind, entry)| world_spatial::ChannelEntity {
            kind,
            identity: entry.actor.placement_identity(),
            generation: entry.generation,
            position: ActorPosition {
                x: entry.position.x,
                y: entry.position.y,
                floor: entry.position.floor,
            },
            revision: entry.revision,
        })
        .chain(item_ref_admission::visible_corpses(
            &runtime,
            self.active_generation,
        ))
        .collect();
        // CLOCK-1 (§11.4): an actor's light is its brightest current LIGHT condition, from the
        // runtime actor-condition owner (players and creatures) and, for a player, its spell
        // state, read under the same runtime lock (lock order: runtime, then spell_states). A
        // store whose owner time is ahead of `now_us` gives no light. Equipped items and monster
        // content carry no light source yet, so they add none. The census includes players whose
        // control was lost: they stay visible during reconnect grace, so their light stays too.
        let now_us = self.owner_now().get();
        let mut lights = world_spatial::EntityLights::new();
        if let Ok(census) = runtime.positioned_melee_census() {
            let states = self.states.spell_states.lock().await;
            for (actor, _, session) in census {
                let spell = session
                    .and_then(|session| states.get(&runtime, actor, session))
                    .map(|state| state.owned_conditions())
                    .filter(|conditions| conditions.accepts_time(now_us))
                    .and_then(|conditions| conditions.light_at(now_us));
                let owned = runtime
                    .actor_conditions_at(actor, session, now_us)
                    .ok()
                    .and_then(|conditions| conditions.light_at(now_us));
                let brightest = match (spell, owned) {
                    (Some(a), Some(b)) => Some(if b.0 > a.0 { b } else { a }),
                    (a, b) => a.or(b),
                };
                if let Some((level, color)) = brightest
                    && level != 0
                {
                    lights.insert(
                        actor.placement_identity(),
                        world_spatial::EntityLight {
                            level,
                            color: color.unwrap_or(crate::world_clock::LIGHT_COLOR),
                        },
                    );
                }
            }
        }
        Some(world_spatial::ChannelEntities {
            content_generation: runtime.content_pin().client_artifact_digest(),
            observer: actor.placement_identity(),
            entities,
            lights,
            observed_at_ms: now_us / 1_000,
        })
    }
}

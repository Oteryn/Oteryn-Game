use super::*;
#[path = "house_hireling_view.rs"]
mod house_hireling_view;

pub(in crate::gameplay_transport) use oteryn_protocol_oteryn::world_spatial::native_floor;

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Bundle runtimes own positive z; entry fixtures retain their authored frame.
    pub(in crate::gameplay_transport) fn runtime_floor(&self, floor: i16) -> i16 {
        if self.bundle_map().is_some() {
            oteryn_protocol_oteryn::world_spatial::runtime_floor(floor)
        } else {
            floor
        }
    }

    /// MAP-CUTOVER-1b: the map view of a bundle World, which its movement cells carry; `None`
    /// on a fixture World.
    pub(in crate::gameplay_transport) fn bundle_map(&self) -> Option<&crate::map::boot::BundleMap> {
        match self.movement_cells().index() {
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
                floor: native_floor(position.floor),
            },
        }
    }

    pub(in crate::gameplay_transport) async fn observe_actor_position(
        &self,
        actor: ExactActorRef,
    ) -> Option<WorldSpatialObservation> {
        let mut runtime = self.runtime.lock().await;
        if let Some((_, state)) = runtime.house_observer(actor).ok()? {
            return Some(Self::observation(
                &runtime,
                crate::foundation::MovementLocalPosition {
                    x: state.position.x,
                    y: state.position.y,
                    floor: state.position.floor,
                },
            ));
        }
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
        let states = self.states.spell_states.lock().await;
        let now = self.owner_now();
        if let Some(mut view) =
            house_hireling_view::entities(&runtime, actor, self.owner_now().get() / 1_000).ok()?
        {
            view.entities.retain(|entry| {
                !states.player_ref_hidden_from(
                    &runtime,
                    actor,
                    world_spatial::EntityRef {
                        identity: entry.identity,
                        generation: entry.generation,
                    },
                    now,
                )
            });
            self.states.staff_world.filter(actor, &mut view);
            return Some(view);
        }
        let visible = runtime.visible_entities();
        let origin = visible
            .players
            .iter()
            .find(|entry| entry.actor == actor)?
            .position;
        let origin =
            crate::movement::interest::VisibilityPosition::new(origin.x, origin.y, origin.floor)
                .ok()?;
        let in_view = |position: crate::foundation::MovementLocalPosition| {
            crate::movement::interest::VisibilityPosition::new(
                position.x,
                position.y,
                position.floor,
            )
            .is_ok_and(|position| {
                crate::movement::interest::VisibilitySettings::REFERENCE.can_see(origin, position)
            })
        };
        // Use the wire interest owner's exact floor/rectangle rules before cloning
        // health/policy and reading conditions. A full World has tens of thousands
        // of distant creatures; none need per-connection enrichment.
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
        .filter(|(_, entry)| in_view(entry.position))
        .filter(|(_, entry)| !states.player_hidden_from(&runtime, actor, entry.actor, now))
        .map(|(kind, entry)| world_spatial::ChannelEntity {
            display_name: None,
            health_percent: runtime
                .companion_snapshot(entry.actor)
                .ok()
                .map_or(100, |s| {
                    u8::try_from(
                        (i128::from(s.health) * 100 / i128::from(s.maximum_health)).clamp(1, 100),
                    )
                    .unwrap_or(100)
                }),
            direction: match runtime
                .read_actor_position(entry.actor)
                .ok()
                .and_then(|at| at.facing())
            {
                Some(crate::foundation::MovementFacing::North) => StepDirection::North,
                Some(crate::foundation::MovementFacing::East) => StepDirection::East,
                Some(crate::foundation::MovementFacing::West) => StepDirection::West,
                Some(crate::foundation::MovementFacing::South) | None => StepDirection::South,
            },
            kind,
            identity: entry.actor.placement_identity(),
            generation: entry.generation,
            position: ActorPosition {
                x: entry.position.x,
                y: entry.position.y,
                floor: native_floor(entry.position.floor),
            },
            revision: entry.revision,
        })
        .chain(item_ref_admission::visible_corpses(
            &runtime,
            self.active_generation,
        ))
        .chain(self.source_volatile_entities(&runtime))
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
            for (actor, position, session) in census {
                if !in_view(position.position()) {
                    continue;
                }
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
        let mut view = world_spatial::ChannelEntities {
            content_generation: runtime.content_pin().client_artifact_digest(),
            observer: actor.placement_identity(),
            entities,
            lights,
            observed_at_ms: now_us / 1_000,
        };
        self.states.staff_world.filter(actor, &mut view);
        Some(view)
    }
}

#[cfg(test)]
mod native_floor_tests {
    use super::native_floor;

    #[test]
    fn spatial_floors_use_the_map_view_frame() {
        // Ground is legacy z 7 in the movement runtime and -7 in the map view origin.
        assert_eq!(native_floor(7), -7);
        assert_eq!(native_floor(-7), -7);
        assert_eq!(native_floor(6), -6);
        assert_eq!(native_floor(0), 0);
        assert_eq!(native_floor(15), -15);
    }
}

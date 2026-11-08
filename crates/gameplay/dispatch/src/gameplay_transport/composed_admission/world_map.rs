use super::*;
#[path = "house_hireling_view.rs"]
mod house_hireling_view;

impl ComposedFreshAdmission<'_, '_, '_> {
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
                floor: position.floor,
            },
        }
    }

    pub(in crate::gameplay_transport) fn wire_observation(
        &self,
        runtime: &ChannelRuntimeV1,
        position: crate::foundation::MovementLocalPosition,
    ) -> Option<WorldSpatialObservation> {
        Some(WorldSpatialObservation {
            content_generation: runtime.content_pin().client_artifact_digest(),
            actor_position: self.wire_position_frame(runtime)?.project(position)?,
        })
    }

    pub(in crate::gameplay_transport) async fn observe_actor_position(
        &self,
        actor: ExactActorRef,
    ) -> Option<WorldSpatialObservation> {
        let mut runtime = self.runtime.lock().await;
        if let Some((_, state)) = runtime.house_observer(actor).ok()? {
            return self.wire_observation(
                &runtime,
                crate::foundation::MovementLocalPosition {
                    x: state.position.x,
                    y: state.position.y,
                    floor: state.position.floor,
                },
            );
        }
        let snapshot = runtime.borrow_movement_position().read(actor).ok()?;
        if snapshot.context() != runtime.pinned_movement_context() {
            return None;
        }
        self.wire_observation(&runtime, snapshot.position())
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
        let frame = self.wire_position_frame(&runtime)?;
        let visible = runtime.visible_entities();
        // Reuse the same spell-state guard for visibility, source icons and
        // LIGHTS; never reacquire the held mutex during this projection.
        let census = runtime.positioned_melee_census().ok();
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
        .filter(|(_, entry)| !states.player_hidden_from(&runtime, actor, entry.actor, now))
        .map(|(kind, entry)| world_spatial::ChannelEntity {
            source_icons: source_icons(
                &runtime,
                kind,
                entry.actor,
                census.as_ref().and_then(|members| {
                    let (_, _, session) =
                        members.iter().find(|(actor, _, _)| *actor == entry.actor)?;
                    states
                        .get(&runtime, entry.actor, (*session)?)?
                        .soul_war_torment_icon_count()
                }),
            ),
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
                floor: entry.position.floor,
            },
            revision: entry.revision,
        })
        .chain(item_ref_admission::visible_corpses(
            &runtime,
            self.active_generation,
        ))
        .chain(self.source_volatile_entities(&runtime))
        .map(|mut entry| {
            entry.position = frame.project(crate::foundation::MovementLocalPosition {
                x: entry.position.x,
                y: entry.position.y,
                floor: entry.position.floor,
            })?;
            Some(entry)
        })
        .collect::<Option<Vec<_>>>()?;
        // CLOCK-1 (§11.4): an actor's light is its brightest current LIGHT condition, from the
        // runtime actor-condition owner (players and creatures) and, for a player, its spell
        // state, read under the same runtime lock (lock order: runtime, then spell_states). A
        // store whose owner time is ahead of `now_us` gives no light. Equipped items and monster
        // content carry no light source yet, so they add none. The census includes players whose
        // control was lost: they stay visible during reconnect grace, so their light stays too.
        let now_us = self.owner_now().get();
        let mut lights = world_spatial::EntityLights::new();
        if let Some(census) = census {
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

/// Source metadata is projected under the same current physical-owner lock as
/// position and health. It carries no authority to change the source state.
fn source_icons(
    runtime: &ChannelRuntimeV1,
    kind: world_spatial::VisibleKind,
    actor: ExactActorRef,
    torment: Option<u16>,
) -> oteryn_protocol_oteryn::world_spatial_entities::NativeCreatureIcons {
    use oteryn_protocol_oteryn::world_spatial_entities::{
        NativeCreatureIcon, NativeCreatureIcons, SoulpitCreatureIcon,
    };
    let icons: Vec<_> = match kind {
        world_spatial::VisibleKind::Player => {
            let keys = runtime.rotten_blood_keyed_icons(actor).unwrap_or_default();
            let hazard = runtime.positioned_actor_census().ok().and_then(|census| {
                let (_, _, session) = census
                    .iter()
                    .find(|(candidate, _, _)| *candidate == actor)?;
                oteryn_gameplay_combat::hazard::effective_level(runtime, actor, (*session)?)
            });
            project_player_source_icons(keys, hazard, torment)
        }
        world_spatial::VisibleKind::Creature => runtime
            .soulpit_variant(actor)
            .and_then(|tag| SoulpitCreatureIcon::from_stack(tag.stack as u8).ok())
            .map(|icon| NativeCreatureIcon {
                category: 1,
                id: icon.icon(),
                count: u16::from(icon.count()),
            })
            .into_iter()
            .collect(),
        _ => Vec::new(),
    };
    NativeCreatureIcons::new(&icons).unwrap_or_default()
}

// All source keys must be combined before Creature's three-icon wire bound.
fn project_player_source_icons(
    keys: [Option<(&'static str, u8, u8, u16)>; 4],
    hazard: Option<u16>,
    torment: Option<u16>,
) -> Vec<oteryn_protocol_oteryn::world_spatial_entities::NativeCreatureIcon> {
    use oteryn_protocol_oteryn::world_spatial_entities::NativeCreatureIcon;
    let mut keyed: Vec<_> = keys
        .into_iter()
        .flatten()
        .chain(
            hazard
                .filter(|count| *count != 0)
                .map(|count| ("hazard", 0, 22, count)),
        )
        .chain(
            torment
                .filter(|count| *count != 0)
                .map(|count| ("goshnars-hatred-damage", 0, 2, count)),
        )
        .collect();
    // Crystal Creature::icons is std::map<string,CreatureIcon>; the protocol
    // takes its first three entries, including distinct keys sharing an ID.
    keyed.sort_unstable_by_key(|(key, _, _, _)| *key);
    keyed
        .into_iter()
        .take(3)
        .map(|(_, category, id, count)| NativeCreatureIcon {
            category,
            id,
            count,
        })
        .collect()
}

#[cfg(test)]
mod source_icon_tests {
    use super::project_player_source_icons;
    #[test]
    fn srx3_general_hazard_merges_lexical_keys_before_first_three() {
        let actual = project_player_source_icons(
            [
                Some(("agony-stacks", 0, 2, 0)),
                Some(("rotten-area", 0, 17, 100)),
                Some(("rotten-hazard", 0, 22, 4)),
                Some(("white-cross", 0, 2, 0)),
            ],
            Some(12),
            None,
        );
        let actual: Vec<_> = actual
            .into_iter()
            .map(|icon| (icon.category, icon.id, icon.count))
            .collect();
        assert_eq!(actual, vec![(0, 2, 0), (0, 22, 12), (0, 17, 100)]);
    }
    #[test]
    fn srx3_general_and_rotten_hazard_keep_distinct_source_keys() {
        let actual = project_player_source_icons(
            [
                None,
                None,
                Some(("rotten-hazard", 0, 22, 4)),
                Some(("white-cross", 0, 2, 0)),
            ],
            Some(12),
            None,
        );
        let actual: Vec<_> = actual
            .into_iter()
            .map(|icon| (icon.category, icon.id, icon.count))
            .collect();
        assert_eq!(actual, vec![(0, 22, 12), (0, 22, 4), (0, 2, 0)]);
    }

    #[test]
    fn srx3_torment_hazard_and_rotten_share_one_lexical_first_three() {
        let actual = project_player_source_icons(
            [
                Some(("agony-stacks", 0, 2, 0)),
                Some(("rotten-area", 0, 17, 100)),
                Some(("rotten-hazard", 0, 22, 4)),
                Some(("white-cross", 0, 2, 0)),
            ],
            Some(12),
            Some(24),
        );
        let actual: Vec<_> = actual
            .into_iter()
            .map(|icon| (icon.category, icon.id, icon.count))
            .collect();
        assert_eq!(actual, vec![(0, 2, 0), (0, 2, 24), (0, 22, 12)]);
    }
}

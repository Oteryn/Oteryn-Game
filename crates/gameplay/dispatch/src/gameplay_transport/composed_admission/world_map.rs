use super::*;
#[path = "house_hireling_view.rs"]
mod house_hireling_view;

fn health_percent(
    runtime: &ChannelRuntimeV1,
    states: &actor_spell::ChannelSpellStates,
    actor: ExactActorRef,
) -> u8 {
    if let Some((_, state)) = player_state(runtime, states, actor) {
        let vitals = state.vitals();
        return (u64::from(vitals.health) * 100)
            .checked_div(u64::from(vitals.max_health))
            .and_then(|percent| u8::try_from(percent.min(100)).ok())
            .unwrap_or(100);
    }
    runtime.companion_snapshot(actor).ok().map_or(100, |s| {
        u8::try_from((i128::from(s.health) * 100 / i128::from(s.maximum_health)).clamp(1, 100))
            .unwrap_or(100)
    })
}

type ItemLightKey<'a> = (&'a str, &'a str, &'a str);

#[derive(Default)]
struct NativeLights<'a> {
    digest: [u8; 32],
    items: std::collections::BTreeMap<ItemLightKey<'a>, world_spatial::EntityLight>,
    creatures: std::collections::BTreeMap<(&'a str, &'a str), world_spatial::EntityLight>,
}

fn native_lights(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> NativeLights<'_> {
    use crate::content::ProjectV2AuthoringProfileData;
    let presentations: std::collections::BTreeMap<_, _> = native
        .presentation_profiles()
        .records
        .iter()
        .filter_map(|record| {
            let ProjectV2AuthoringProfileData::Presentation(profile) = &record.data else {
                return None;
            };
            if profile.light_level == 0 {
                return None;
            }
            let color = match profile.light_color_binding.as_deref() {
                Some(binding) => binding
                    .strip_prefix("canary.appearance:light-color/")?
                    .parse::<u8>()
                    .ok()?,
                None => crate::world_clock::LIGHT_COLOR,
            };
            Some((
                (record.target.key.as_str(), record.target.revision.as_str()),
                world_spatial::EntityLight {
                    level: profile.light_level,
                    color,
                },
            ))
        })
        .collect();
    let mut lights = NativeLights {
        digest: native.source_digest(),
        ..NativeLights::default()
    };
    for record in native.creature_profiles().records.iter() {
        if let Some(light) = presentations.get(&(
            record.presentation.key.as_str(),
            record.presentation.revision.as_str(),
        )) {
            lights.creatures.insert(
                (
                    record.profile.target.key.as_str(),
                    record.profile.target.revision.as_str(),
                ),
                *light,
            );
        }
    }
    for record in native.item_profiles() {
        if let Some(presentation) = &record.authoring.presentation
            && let Some(light) =
                presentations.get(&(presentation.key.as_str(), presentation.revision.as_str()))
        {
            let definition = &record.production_definition;
            lights.items.insert(
                (
                    definition.family.as_str(),
                    definition.production_key.as_str(),
                    definition.revision_ref.as_str(),
                ),
                *light,
            );
        }
    }
    lights
}

fn player_state<'s>(
    runtime: &ChannelRuntimeV1,
    states: &'s actor_spell::ChannelSpellStates,
    actor: ExactActorRef,
) -> Option<(GameSessionId, &'s crate::spell::cast::PlayerSpellState)> {
    states
        .actors
        .iter()
        .find(|(held, session, _)| {
            *held == actor
                && (runtime.player_control_facts(actor, *session).is_ok()
                    || runtime.house_player_control_facts(actor, *session).is_ok())
        })
        .map(|(_, session, state)| (*session, state))
}

fn brighter(
    a: world_spatial::EntityLight,
    b: world_spatial::EntityLight,
) -> world_spatial::EntityLight {
    if b.level > a.level { b } else { a }
}

fn actor_lights(
    runtime: &ChannelRuntimeV1,
    states: &actor_spell::ChannelSpellStates,
    actors: &[ExactActorRef],
    now: u64,
    native: &NativeLights<'_>,
) -> world_spatial::EntityLights {
    let mut lights = world_spatial::EntityLights::new();
    for &actor in actors {
        let player = player_state(runtime, states, actor);
        let spell = player
            .map(|(_, state)| state.owned_conditions())
            .filter(|conditions| conditions.accepts_time(now))
            .and_then(|conditions| conditions.light_at(now));
        let owned = runtime
            .actor_conditions_at(actor, player.map(|(session, _)| session), now)
            .ok()
            .and_then(|conditions| conditions.light_at(now));
        let condition =
            spell
                .into_iter()
                .chain(owned)
                .map(|(level, color)| world_spatial::EntityLight {
                    level,
                    color: color.unwrap_or(crate::world_clock::LIGHT_COLOR),
                });
        let innate = (native.digest == runtime.content_pin().server_artifact_digest())
            .then(|| runtime.companion_snapshot(actor).ok())
            .flatten()
            .and_then(|creature| {
                native
                    .creatures
                    .get(&(
                        creature.state.policy.definition_key.as_str(),
                        creature.state.policy.definition_revision.as_str(),
                    ))
                    .copied()
            });
        if let Some(light) = condition
            .chain(innate)
            .filter(|light| light.level != 0)
            .reduce(brighter)
        {
            lights.insert(actor.placement_identity(), light);
        }
    }
    lights
}

struct EquipmentLight {
    actor: ExactActorRef,
    session: GameSessionId,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    light: world_spatial::EntityLight,
}

fn light_fence_current(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
) -> bool {
    if fence.game_session_id != session {
        return false;
    }
    if let Ok(house) = runtime.house_actor_state(actor, session) {
        return house.scope == fence.runtime_scope
            && house.generation == fence.scope_ownership_generation
            && house.lease.character_id().as_bytes() == fence.character_id.as_bytes()
            && house.lease.generation() == fence.character_lease_generation;
    }
    let binding = runtime.binding();
    binding.runtime_scope() == fence.runtime_scope
        && binding.scope_generation() == fence.scope_ownership_generation
        && runtime.player_control_facts(actor, session).is_ok()
        && runtime
            .bound_attacker_lease(actor, session)
            .is_ok_and(|lease| {
                lease.character_id().as_bytes() == fence.character_id.as_bytes()
                    && lease.generation() == fence.character_lease_generation
            })
}

impl ComposedFreshAdmission<'_, '_, '_> {
    async fn visible_equipment_lights(
        &self,
        players: &[(ExactActorRef, GameSessionId)],
        native: &NativeLights<'_>,
    ) -> Vec<EquipmentLight> {
        let mut lights = Vec::new();
        for &(actor, session) in players {
            let Ok((current, _)) = FreshAdmissionStore::from_root(self.root.clone())
                .current_session_at(session)
                .await
            else {
                continue;
            };
            if current.session_state() == GameSessionState::Terminal {
                continue;
            }
            let Some(fence) = item_fence_of(current) else {
                continue;
            };
            let house = {
                let runtime = self.runtime.lock().await;
                if native.digest != runtime.content_pin().server_artifact_digest()
                    || !light_fence_current(&runtime, actor, session, fence)
                {
                    continue;
                }
                runtime.house_actor_state(actor, session).is_ok()
            };
            let read = if house {
                self.root
                    .read_house_equipment(
                        self.character,
                        self.holder,
                        &fence,
                        *self.channel_id.as_bytes(),
                        native.digest,
                    )
                    .await
            } else {
                self.root
                    .read_movement_equipment(self.character, self.holder, &fence, native.digest)
                    .await
            };
            let Ok(read) = read else { continue };
            if read.fence() != &fence || read.equipment().content_digest != native.digest {
                continue;
            }
            let light = read
                .equipment()
                .items
                .iter()
                .filter_map(|item| {
                    let definition = &item.definition;
                    native
                        .items
                        .get(&(
                            definition.family.as_str(),
                            definition.production_key.as_str(),
                            definition.revision_ref.as_str(),
                        ))
                        .copied()
                })
                .reduce(brighter);
            if let Some(light) = light {
                lights.push(EquipmentLight {
                    actor,
                    session,
                    fence,
                    light,
                });
            }
        }
        lights
    }
}

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
        let native = self
            .active_generation
            .and_then(|active| active.native_gameplay())
            .map(native_lights)
            .unwrap_or_default();
        let mut observed = Vec::new();
        let runtime = self.runtime.lock().await;
        let states = self.states.spell_states.lock().await;
        let now = self.owner_now();
        let mut view = if let Some(mut view) =
            house_hireling_view::entities(&runtime, &states, actor, self.owner_now().get() / 1_000)
                .ok()?
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
            observed.extend(states.actors.iter().filter_map(|(held, session, _)| {
                (runtime.house_player_control_facts(*held, *session).is_ok()
                    && view.entities.iter().any(|entry| {
                        entry.kind == VisibleKind::Player
                            && entry.identity == held.placement_identity()
                            && entry.generation == held.actor_local_generation()
                    }))
                .then_some(*held)
            }));
            view
        } else {
            let visible = runtime.visible_entities();
            let origin = visible
                .players
                .iter()
                .find(|entry| entry.actor == actor)?
                .position;
            let origin = crate::movement::interest::VisibilityPosition::new(
                origin.x,
                origin.y,
                origin.floor,
            )
            .ok()?;
            let in_view = |position: crate::foundation::MovementLocalPosition| {
                crate::movement::interest::VisibilityPosition::new(
                    position.x,
                    position.y,
                    position.floor,
                )
                .is_ok_and(|position| {
                    crate::movement::interest::VisibilitySettings::REFERENCE
                        .can_see(origin, position)
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
            .map(|(kind, entry)| {
                observed.push(entry.actor);
                world_spatial::ChannelEntity {
                    display_name: None,
                    health_percent: health_percent(&runtime, &states, entry.actor),
                    direction: match runtime
                        .read_actor_position(entry.actor)
                        .ok()
                        .and_then(|at| at.facing())
                    {
                        Some(crate::foundation::MovementFacing::North) => StepDirection::North,
                        Some(crate::foundation::MovementFacing::East) => StepDirection::East,
                        Some(crate::foundation::MovementFacing::West) => StepDirection::West,
                        Some(crate::foundation::MovementFacing::South) | None => {
                            StepDirection::South
                        }
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
                }
            })
            .chain(item_ref_admission::visible_corpses(
                &runtime,
                self.active_generation,
            ))
            .chain(self.source_volatile_entities(&runtime))
            .collect();
            let now_us = now.get();
            world_spatial::ChannelEntities {
                content_generation: runtime.content_pin().client_artifact_digest(),
                observer: actor.placement_identity(),
                entities,
                lights: world_spatial::EntityLights::new(),
                observed_at_ms: now_us / 1_000,
            }
        };
        view.lights = actor_lights(&runtime, &states, &observed, now.get(), &native);
        let players: Vec<_> = observed
            .iter()
            .filter_map(|&actor| {
                player_state(&runtime, &states, actor).map(|(session, _)| (actor, session))
            })
            .collect();
        drop(states);
        drop(runtime);
        // Equipment is best-effort enrichment. Its bounded reads hold no gameplay guards.
        if !native.items.is_empty() && !players.is_empty() {
            let equipment = tokio::time::timeout(
                COMMAND_HOLDER_WINDOW,
                self.visible_equipment_lights(&players, &native),
            )
            .await
            .unwrap_or_default();
            let runtime = self.runtime.lock().await;
            if runtime.content_pin().server_artifact_digest() == native.digest {
                for equipment in equipment {
                    if light_fence_current(
                        &runtime,
                        equipment.actor,
                        equipment.session,
                        equipment.fence,
                    ) {
                        view.lights
                            .entry(equipment.actor.placement_identity())
                            .and_modify(|light| *light = brighter(*light, equipment.light))
                            .or_insert(equipment.light);
                    }
                }
            }
        }
        self.states.staff_world.filter(actor, &mut view);
        Some(view)
    }
}

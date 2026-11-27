//! A runtime-mutable bundle tile must not abort other creatures' owner turns.
use super::*;
use crate::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_foundation::foundation::condition::{ApplicationFacts, ConditionDefinition};
use oteryn_world_bundle::spawn::{Creature, Direction, Period, Point, Source, Table};

#[test]
fn unknown_bundle_creature_and_player_tiles_keep_owner_ticks_running() -> TestResult {
    bundle_creature_case(false)
}

#[test]
fn adjacent_rat_lands_melee_attributed_to_its_exact_actor() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    bundle_creature_case(true)
}

fn bundle_creature_case(test_melee: bool) -> TestResult {
    run_case(async move |harness| {
        prepare_current_session(harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_loop_monster(
                deps.admitted.world_id,
                false,
            );
        let active = controller.active().ok_or("active generation")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let index = native.item_index().ok_or("item index")?;
        let reference = |key| index.definition_ref("Item", key, "definition-r1");
        let (bytes, mut pins) = crate::map::boot::tests::bundle_with_item_ids(
            (
                "oteryn:item.tibia.i2854",
                reference("oteryn:item.tibia.i2854").ok_or("bag")?.get() - 1,
            ),
            (
                "oteryn:item.tibia.i3031",
                reference("oteryn:item.tibia.i3031").ok_or("coin")?.get() - 1,
            ),
        );
        let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
        bundle
            .manifest
            .palette
            .first_mut()
            .ok_or("ground palette")?
            .key = "oteryn:item.tibia.i100".into();
        if test_melee {
            // Put the pickupable stack under the caster while the adjacent Rat
            // starts on the unchanged ground tile at x=2.
            let tiles = &mut bundle.sectors.first_mut().ok_or("sector")?.tiles;
            let coin_stack = tiles
                .iter()
                .find(|tile| tile.x == 7)
                .ok_or("coin tile")?
                .items
                .clone();
            tiles
                .iter_mut()
                .find(|tile| tile.x == 3)
                .ok_or("caster tile")?
                .items = coin_stack;
        }
        bundle.spawns = Table {
            creatures: vec![Creature {
                key: "oteryn:creature.rat".into(),
                period: Period::All,
            }],
            sources: vec![Source {
                key: "oteryn:spawn/unknown-bundle-field".into(),
                floor: -7,
                x: 3,
                y: 0,
                points: [2, 7]
                    .into_iter()
                    .map(|x| Point {
                        creature: 0,
                        x,
                        y: 0,
                        direction: Direction::West,
                        respawn_ms: 60_000,
                        boss_policy: None,
                    })
                    .collect(),
            }],
        };
        bundle.manifest.spawns = oteryn_world_bundle::bundle::SpawnCounts {
            sources: 1,
            points: 2,
        };
        let bytes =
            oteryn_world_bundle::writer::write(&bundle.manifest, &bundle.sectors, &bundle.spawns)
                .map_err(debug)?;
        pins.digest = oteryn_world_bundle::bundle::read(&bytes)
            .map_err(debug)?
            .digest;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let world = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 3,
                    y: 0,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                let revision = index.revision(key)?;
                let policy = native.item_policy(key, revision)?;
                Some(crate::map::facts::ItemDefinition {
                    reference: index.definition_ref("Item", key, revision)?,
                    solid: policy.record().attributes.blocks_movement,
                    blocks_projectile: policy.record().attributes.blocks_projectile.unwrap_or(true),
                    pickupable: key == "oteryn:item.tibia.i3031",
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let entry = pin.qualified_room();
        let cells = world
            .movement_cells(entry.movement_cells())
            .map_err(debug)?;
        let room = entry
            .bundle_view(
                cells,
                world.frame_binding_digest(deps.admitted.world_id),
                world.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 3,
                    y: 0,
                    floor: 7,
                },
            )
            .ok_or("bundle view")?;
        assert!(
            room.movement_cells()
                .spell_tiles()
                .lookup(
                    room.movement_cells().scope(),
                    crate::content::LogicalCell { x: 2, y: 0, z: 7 }
                )
                .is_ok()
        );
        assert_eq!(
            room.movement_cells().spell_tiles().lookup(
                room.movement_cells().scope(),
                crate::content::LogicalCell { x: 7, y: 0, z: 7 }
            ),
            Err(crate::content::SpellTileLookupError::Unknown)
        );
        let channel_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            room.frame_binding().digest(),
            room.map_revision_digest(),
            (3, 0, 7),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            8,
            channel_pin,
        )
        .map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let sources = world
            .activation_spawns(|key| {
                let policy = runtime.companion_policy(key).ok()?;
                Some((
                    u64::try_from(policy.maximum_health).ok()?,
                    policy.blocked_by_nearby_players,
                ))
            })
            .ok_or("activation spawns")?;
        runtime
            .begin_activation_spawn_sources(sources)
            .map_err(debug)?;
        assert_eq!(
            runtime
                .realize_activation_spawn_window(0)
                .map_err(debug)?
                .realized,
            2
        );
        let creatures = runtime.visible_entities().creatures;
        let unknown = creatures
            .iter()
            .find(|entry| entry.position.x == 7)
            .ok_or("unknown-tile creature")?
            .actor;
        let before = runtime.companion_snapshot(unknown).map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        deps.room = room;
        deps.combat_room = Some(deps.room.clone());
        let mut owner = deps.owner(harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = deps.room.movement_cells().into();
        if test_melee {
            let rat = creatures
                .iter()
                .find(|entry| entry.position.x == 2)
                .ok_or("adjacent Rat")?
                .actor;
            return rat_attributed_hit(&owner, &deps, rat).await;
        }
        let permit = owner.spell_lane_permit().await.ok_or("lane")?;
        let health_names = owner.read_native_health_names(&permit).await.map_err(debug)?;
        let mut runtime = deps.runtime.lock().await;
        let native_health_names = owner.seal_native_health_names(&runtime, health_names).map_err(debug)?;
        let mut states = owner.states.spell_states.lock().await;
        let creature = crate::content::ProjectV2DefinitionRef {
            family: crate::content::ProjectV2Family::Creature,
            key: "oteryn:creature.rat".into(),
            revision: before.state.policy.definition_revision.clone(),
        };
        for entry in &creatures {
            states
                .monster_melee
                .creature_ai
                .admit_native_creature(&runtime, native, entry.actor, &creature, 0)
                .map_err(debug)?;
        }
        let mut attack = owner.states.attack.lock().await;
        let revisions = crate::ability::RevisionSet::new(
            "rules-1",
            "content-1",
            "world-1",
            "formula-1",
            "sim-1",
        )
        .map_err(debug)?;
        for now in [10_000_000, 10_050_000] {
            assert!(
                owner
                    .drive_monster_profiles(
                        &mut runtime,
                        &mut states,
                        &mut attack,
                        &deps.room,
                        native,
                        &permit,
                        &revisions,
                        &[],
                        now,
                        &native_health_names,
                    )
                    .await
                    .is_ok(),
                "an unknown actor tile must not abort the entire channel pass"
            );
            assert!(owner.root.is_ready(), "the semantic holder was returned");
        }
        let after = runtime.companion_snapshot(unknown).map_err(debug)?;
        assert_eq!(after.health, before.health);
        assert_eq!(
            after.state.conditions.instances(),
            before.state.conditions.instances(),
            "a pickupable coin must not invent field conditions"
        );
        for entry in &creatures {
            let current = runtime.companion_snapshot(entry.actor).map_err(debug)?;
            assert!(current.state.conditions.accepts_time(10_050_000));
            assert!(
                !current.state.conditions.accepts_time(10_049_999),
                "both field cursors advanced through the second owner turn"
            );
            assert!(
                states
                    .monster_melee
                    .creature_ai
                    .last_report_for_test(entry.actor)
                    .is_some(),
                "both native think turns completed"
            );
        }
        let wrong_world =
            crate::map::spell_tiles::tests::movement_arena::rectangle((0, 0), (7, 1), -7);
        let wrong_cells = wrong_world
            .movement_cells(entry.movement_cells())
            .map_err(debug)?;
        let mut wrong_owner = deps.owner(harness, &authority);
        wrong_owner.movement_cells = (&wrong_cells).into();
        assert!(
            wrong_owner
                .logout_zones_for_cell(&runtime, crate::content::LogicalCell { x: 7, y: 0, z: 7 })
                .is_none(),
            "another map cannot supply the creature's protection-zone facts"
        );
        drop(attack);
        drop(states);
        drop(runtime);
        drop(permit);
        // Exercise the player's real periodic dispatcher on the same pickupable
        // bundle tile, rather than qualifying only fixture spell metadata.
        let session = deps.admitted.game_session_id;
        {
            let mut runtime = deps.runtime.lock().await;
            for x in 4..=7 {
                let before = runtime.read_actor_position(deps.actor).map_err(debug)?;
                let mut position = before.position();
                position.x = x;
                runtime
                    .borrow_movement_position()
                    .commit_cardinal(before, position)
                    .map_err(debug)?;
            }
            let mut states = owner.states.spell_states.lock().await;
            assert!(
                states
                    .initialize(
                        &runtime,
                        deps.actor,
                        session,
                        crate::gameplay_transport::actor_spell::tests::FACTS,
                        (0, 0),
                        oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
                    )
                    .is_some()
            );
            states
                .get_mut(&runtime, deps.actor, session)
                .ok_or("player state")?
                .restore_saved_vitals(100, 10);
            let definition = ConditionDefinition::new(
                "test.bundle.regeneration",
                1,
                crate::domain::condition::ConditionValues::SpellRegeneration {
                    duration_ms: 10_000,
                    sub_id: 0,
                    health_gain: 5,
                    health_interval_ms: 1_000,
                    mana_gain: 7,
                    mana_interval_ms: 1_000,
                },
            )
            .ok_or("regeneration")?;
            let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([91; 32]);
            let facts = ApplicationFacts {
                now: 0,
                base_speed: 220,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes([91; 16]),
            };
            let next = states
                .get(&runtime, deps.actor, session)
                .ok_or("player state")?
                .stage_owned_test_conditions(&[definition], &facts)
                .ok_or("regeneration application")?;
            assert!(states.commit(&runtime, deps.actor, session, next));
            assert_eq!(
                crate::gameplay_transport::actor_spell::qualified_protection_zone(
                    &mut runtime,
                    deps.room.movement_cells(),
                    deps.actor,
                ),
                Some(false),
                "immutable bundle zones survive unknown stack metadata"
            );
            assert_eq!(
                crate::gameplay_transport::actor_spell::qualified_protection_zone(
                    &mut runtime,
                    &wrong_cells,
                    deps.actor,
                ),
                None,
                "another map cannot supply the player's protection-zone facts"
            );
        }
        let result = owner
            .tick_spell_periodic_at(
                deps.actor,
                session,
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1_000_000),
            )
            .await;
        assert!(
            result.is_some_and(|(_, vitals)| vitals.health == 105 && vitals.mana == 17),
            "the real periodic owner must regenerate a player on a pickupable bundle tile"
        );
        Ok(())
    })
}

async fn rat_attributed_hit(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    rat: ExactActorRef,
) -> TestResult {
    use crate::gameplay_transport::capabilities::{
        BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
    };
    use oteryn_protocol_oteryn::analyser::{
        AnalyserDamageSource, AnalyserFact, decode_attributed_analyser_facts,
    };
    use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
    use std::time::Duration;

    let mut admitted = deps.admitted;
    admitted.runtime_actor = Some(deps.actor);
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &[6, 10, 486])
            .ok_or("hit attribution capability selection")?;
    let facts = crate::gameplay_transport::actor_spell::tests::FACTS;
    {
        let mut runtime = deps.runtime.lock().await;
        let position = runtime
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        let rat_at = runtime.read_actor_position(rat).map_err(debug)?.position();
        assert_eq!(
            crate::ai_think::chebyshev_distance(position, rat_at),
            Some(1)
        );
        let cells = deps.room.movement_cells();
        assert_eq!(
            cells.spell_tiles().lookup(
                cells.scope(),
                crate::content::LogicalCell {
                    x: position.x,
                    y: position.y,
                    z: i32::from(position.floor),
                }
            ),
            Err(crate::content::SpellTileLookupError::Unknown)
        );
        assert_eq!(
            crate::gameplay_transport::actor_spell::qualified_protection_zone(
                &mut runtime,
                cells,
                deps.actor,
            ),
            Some(false)
        );
        owner
            .states
            .spell_states
            .lock()
            .await
            .initialize(
                &runtime,
                deps.actor,
                admitted.game_session_id,
                facts,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .ok_or("initialize caster")?;
    }
    owner
        .analyser_snapshot(admitted)
        .await
        .ok_or("analyser stream")?;
    let health = || async {
        let runtime = deps.runtime.lock().await;
        owner
            .states
            .spell_states
            .lock()
            .await
            .get(&runtime, deps.actor, admitted.game_session_id)
            .map(|state| state.vitals().health)
            .ok_or("caster health")
    };
    let before = health().await?;
    let attacker = EntityRef {
        identity: rat.placement_identity(),
        generation: rat.actor_local_generation(),
    };
    let interval = Duration::from_millis(crate::ai_think::CREATURE_THINK_INTERVAL_MILLIS);
    let mut elapsed = Duration::ZERO;
    let mut attributed_hit = false;
    while elapsed < Duration::from_secs(30) {
        // Advance simulation time while leaving real PostgreSQL deadlines running normally.
        tokio::time::pause();
        let step_started = tokio::time::Instant::now();
        tokio::time::advance(interval).await;
        elapsed += step_started.elapsed();
        tokio::time::resume();
        owner
            .drain_monster_melee_at(
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(u64::try_from(
                    elapsed.as_micros(),
                )?),
            )
            .await;
        while let Some(payload) = owner.analyser_next(admitted).await {
            attributed_hit |= decode_attributed_analyser_facts(&payload)
                .map_err(debug)?
                .iter()
                .any(|event| {
                    event.attacker == Some(attacker)
                        && matches!(
                            event.fact,
                            AnalyserFact::DamageInput {
                                source: AnalyserDamageSource::Creature(_),
                                ..
                            }
                        )
                });
        }
        if attributed_hit {
            break;
        }
    }
    assert!(
        attributed_hit && health().await? < before,
        "adjacent Rat {attacker:?} must land an attributed melee hit within 30 seconds"
    );
    Ok(())
}

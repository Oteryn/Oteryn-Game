//! A mutable bundle stack does not hide the retained zone from the real creature field owner.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::content::native_cell_lookup::NativeMovementCollisionIndex;
use crate::durability::DurabilityError;
use crate::foundation::{ExactActorRef, MovementLocalPosition};
use crate::movement::monster_map::CurrentMonsterMap;
use oteryn_gameplay_domain::domain::condition::{ConditionValues, DotElement};
use oteryn_gameplay_entities::carrier::runtime_actor_periodic::CreatureFieldEntry;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_foundation::foundation::condition::ConditionDefinition;

#[test]
fn creature_field_bundle_pickupable_stack_keeps_current_zone_and_residual_ticks_postgres()
-> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "creature bundle field scenario panicked")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "creature_bundle_field", false, false)
                .await?;
        let result = exercise(&harness).await;
        let cleanup = harness.cleanup().await;
        result?;
        cleanup
    })
}

async fn exercise(harness: &Harness) -> TestResult {
    prepare_current_session(harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let original = Dependencies::new()?;
    let (controller, pin) = crate::content::native_gameplay::test_support::activated_loop_monster(
        original.admitted.world_id,
        false,
    );
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let (deps, actors) = dependencies(original, active, &pin)?;
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = deps.room.movement_cells().into();
    let permit = owner.spell_lane_permit().await.ok_or("field lane")?;
    let mut runtime = deps.runtime.lock().await;
    let objects = deps.door.lock().await;
    let states = owner.states.spell_states.lock().await;
    // TEST_PARAMETERS: an existing residual Drown condition distinguishes PZ suppression
    // from ordinary damage. Its native store and due cursor remain the production owners.
    for (ordinal, actor) in actors.iter().enumerate() {
        let field = CreatureFieldEntry {
            definition: ConditionDefinition::new(
                "test.bundle.creature.residual",
                1,
                ConditionValues::DamageOverTime {
                    element: DotElement::Drown,
                    total_min: 10,
                    total_max: 10,
                    per_tick: 5,
                    interval_ms: 1000,
                    delayed: true,
                },
            )
            .ok_or("residual condition")?,
            source: None,
            lineage: None,
            occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(id(
                u8::try_from(ordinal + 1)?,
            )),
        };
        runtime
            .commit_creature_field_entry(*actor, 0, Some(&field))
            .map_err(debug)?;
        assert_eq!(
            runtime
                .companion_snapshot(*actor)
                .map_err(debug)?
                .state
                .conditions
                .instances()
                .len(),
            1
        );
    }
    let mut wrong_map = deps.room.map_revision_digest();
    wrong_map[0] ^= 1;
    let mismatch = pin
        .qualified_room()
        .bundle_view(
            deps.room.movement_cells().clone(),
            deps.room.frame_binding().digest(),
            wrong_map,
            deps.room.entry_start(),
        )
        .ok_or("different pinned map view")?;
    let (mut source_deps, _) = super::loop_monster::fixture::dependencies(active, &pin)?;
    let mut source_context = (
        source_deps.runtime.get_mut(),
        &source_deps.room,
        source_deps.door.get_mut(),
    );
    let mut context = (
        &owner,
        &mut *runtime,
        &*states,
        &deps.room,
        &*objects,
        native,
        &permit,
        actors,
        &mismatch,
        &mut source_context,
    );
    let pass = owner.root.try_issue_semantic_pass().map_err(debug)?;
    pass.run_with_context(&mut context, |holder, deadline, context| {
        Box::pin(async move {
            let (
                owner,
                runtime,
                states,
                room,
                objects,
                native,
                permit,
                actors,
                mismatch,
                source_context,
            ) = context;
            let mut tx = crate::durability::spell_item_transaction::begin_spell_owner_transaction(
                holder, deadline,
            )
            .await?;
            let binding = runtime.binding();
            let authority =
                crate::durability::spell_item_transaction::assert_spell_item_scope_in_transaction(
                    &mut tx,
                    permit,
                    owner.root,
                    owner.character,
                    owner.holder,
                    binding.runtime_scope(),
                    binding.scope_generation().get(),
                )
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
            let before = actors
                .iter()
                .map(|actor| runtime.companion_snapshot(*actor).map(|s| s.health))
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| DurabilityError::Unavailable)?;
            let mut map =
                CurrentMonsterMap::new(&mut tx, &authority, runtime, room, objects, native);
            for (x, pz) in [(2, false), (3, true)] {
                assert_eq!(
                    room.movement_cells().spell_tiles().lookup(
                        room.movement_cells().scope(),
                        crate::content::LogicalCell { x, y: 0, z: 7 },
                    ),
                    Err(crate::content::SpellTileLookupError::Unknown)
                );
                assert_eq!(map.protection_zone(runtime, at(x)), Some(pz));
                assert_eq!(
                    map.read(runtime, at(x)).await,
                    None,
                    "zone evidence grants no movement/spell facts"
                );
            }
            assert_eq!(
                map.protection_zone(runtime, at(20)),
                None,
                "absent cells stay absent"
            );
            for now in [1_000_000, 2_000_000] {
                owner
                    .tick_creature_fields(&mut map, runtime, states, native, now)
                    .await?;
                assert!(map.validate_transaction(runtime).await);
                for (ordinal, actor) in actors.iter().enumerate() {
                    let after = runtime
                        .companion_snapshot(*actor)
                        .map_err(|_| DurabilityError::Unavailable)?;
                    if ordinal == 0 {
                        assert!(
                            after.health < before[ordinal],
                            "ordinary tile consumes residual damage"
                        );
                    } else {
                        assert_eq!(after.health, before[ordinal], "PZ suppresses Drown damage");
                    }
                    if now == 1_000_000 {
                        assert_eq!(after.state.conditions.instances().len(), 1);
                        assert_eq!(after.state.conditions.instances()[0].remaining_total(), 5);
                    } else {
                        assert!(
                            after.state.conditions.instances().is_empty(),
                            "both real condition cursors exhaust"
                        );
                    }
                }
            }
            drop(map);
            let NativeMovementCollisionIndex::Bundle(index) = room.movement_cells().index() else {
                return Err(DurabilityError::Unavailable);
            };
            let clipped = room
                .with_activity_bundle_index(index.clone(), [1, 0, 2, 0, 7, 7])
                .ok_or(DurabilityError::Unavailable)?;
            let map =
                CurrentMonsterMap::new(&mut tx, &authority, runtime, &clipped, objects, native);
            assert_eq!(map.protection_zone(runtime, at(2)), Some(false));
            assert_eq!(
                map.protection_zone(runtime, at(3)),
                None,
                "the parent base cannot bypass child bounds"
            );
            drop(map);
            let map =
                CurrentMonsterMap::new(&mut tx, &authority, runtime, mismatch, objects, native);
            assert_eq!(
                map.protection_zone(runtime, at(2)),
                None,
                "independently pinned map mismatch refuses"
            );
            drop(map);
            let (source_runtime, source_room, source_objects) = &mut **source_context;
            let source = source_room
                .source_world()
                .ok_or(DurabilityError::Unavailable)?;
            let placements = source.pending_mutable_placements();
            let cell = placements
                .iter()
                .find(|candidate| {
                    candidate.pickupable()
                        && placements
                            .iter()
                            .filter(|placement| placement.cell() == candidate.cell())
                            .all(|placement| {
                                !placement.has_height()
                                    && matches!(
                                        placement.source_kind(),
                                        None | Some("door") | Some("container")
                                    )
                            })
                })
                .ok_or(DurabilityError::Unavailable)?
                .cell();
            let position = MovementLocalPosition {
                x: cell.x,
                y: cell.y,
                floor: i16::try_from(cell.z).map_err(|_| DurabilityError::Unavailable)?,
            };
            assert_eq!(
                source_room
                    .movement_cells()
                    .spell_tiles()
                    .lookup(source_room.movement_cells().scope(), cell),
                Err(crate::content::SpellTileLookupError::Unknown)
            );
            let expected = source
                .immutable_tile(source.scope(), cell)
                .map_err(|_| DurabilityError::Unavailable)?
                .flags()
                .protection_zone;
            let mut map = CurrentMonsterMap::new(
                &mut tx,
                &authority,
                source_runtime,
                source_room,
                source_objects,
                native,
            );
            assert_eq!(
                map.protection_zone(source_runtime, position),
                Some(expected)
            );
            assert_eq!(
                map.read(source_runtime, position).await,
                None,
                "retained Source PZ does not supply missing mutable initialization receipts"
            );
            assert!(map.validate_transaction(source_runtime).await);
            drop(map);
            let mut map =
                CurrentMonsterMap::new(&mut tx, &authority, runtime, room, objects, native);
            assert!(
                sqlx::query("SELECT 1/0")
                    .execute(&mut **map.transaction_for_test())
                    .await
                    .is_err()
            );
            assert!(!map.validate_transaction(runtime).await);
            assert_eq!(
                map.protection_zone(runtime, at(2)),
                None,
                "invalidated physical transaction grants no zone"
            );
            drop(map);
            tx.rollback().await?;
            Ok(())
        })
    })
    .await
    .map_err(debug)?;
    Ok(())
}

fn at(x: i32) -> MovementLocalPosition {
    MovementLocalPosition { x, y: 0, floor: 7 }
}

fn dependencies(
    original: Dependencies,
    active: &crate::content::ActiveGeneration,
    pin: &crate::content::NativeEntryContentPin,
) -> TestResult<(Dependencies, [ExactActorRef; 2])> {
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let coin_key = "oteryn:item.tibia.i3031";
    let item_index = native.item_index().ok_or("native Item index")?;
    let coin_revision = item_index
        .revision(coin_key)
        .ok_or("native coin revision")?;
    let coin_reference = item_index
        .definition_ref("Item", coin_key, coin_revision)
        .ok_or("native coin reference")?;
    let (bytes, mut bundle_pin) = crate::map::boot::tests::bundle();
    let mut data = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    data.manifest.palette[0].key = "oteryn:terrain.tibia.i100".into();
    data.manifest.palette[4].key = coin_key.into();
    data.manifest.palette[4].id = coin_reference.get() - 1;
    // Existing bundle fixture, with one loose coin on each independently flagged tile.
    for (index, flags) in [(1, 0), (2, 1)] {
        data.sectors[0].tiles[index].flags = flags;
        data.sectors[0].tiles[index]
            .items
            .push(oteryn_world_bundle::sector::Item {
                palette: 4,
                depth: 0,
                attrs: Default::default(),
            });
    }
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &data.sectors, &data.spawns)
        .map_err(debug)?;
    bundle_pin.digest = oteryn_world_bundle::bundle::read(&bytes)
        .map_err(debug)?
        .digest;
    let base = crate::map::load(&bytes, &bundle_pin).map_err(debug)?;
    let world = crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: bundle_pin,
            start: crate::map::overlay::TilePos {
                x: 1,
                y: 0,
                floor: -7,
            },
        },
        original.admitted.world_id,
        original.admitted.channel_id,
        |key| match key {
            "item:box" => Some(crate::map::facts::ItemDefinition {
                reference: std::num::NonZeroU32::new(1)?,
                solid: Some(true),
                pickupable: false,
                blocks_projectile: true,
                door: false,
            }),
            "oteryn:item.tibia.i3031" => {
                let index = native.item_index()?;
                let revision = index.revision(key)?;
                let policy = native.item_policy(key, revision)?;
                let crate::content::ReferenceItemField::Known(physical) =
                    &policy.record().semantics.physical
                else {
                    return None;
                };
                Some(crate::map::facts::ItemDefinition {
                    reference: index.definition_ref("Item", key, revision)?,
                    solid: Some(false),
                    pickupable: physical.pickupable
                        == crate::content::ReferenceItemField::Known(true),
                    blocks_projectile: false,
                    door: false,
                })
            }
            _ => None,
        },
    )
    .map_err(debug)?;
    let entry = pin.qualified_room();
    let room = entry
        .bundle_view(
            world
                .movement_cells(entry.movement_cells())
                .map_err(debug)?,
            world.frame_binding_digest(original.admitted.world_id),
            world.map_revision_digest(),
            crate::content::NativeEntryStart {
                x: 1,
                y: 0,
                floor: 7,
            },
        )
        .ok_or("bundle combat view")?;
    let channel_pin = ChannelContentPin::from_activation(
        original.admitted.world_id,
        pin.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (1, 0, 7),
    );
    let mut deps = Dependencies::with_content_and_capacity(channel_pin, room, original.spells, 8)?;
    deps.combat_room = Some(deps.room.clone());
    let runtime = deps.runtime.get_mut();
    native.install_companion_policies(runtime).map_err(debug)?;
    runtime
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    use oteryn_world_bundle::spawn::{Creature, Direction, Period, Point, Source, Table};
    let source = Table {
        creatures: vec![Creature {
            key: "oteryn:creature.rat".into(),
            period: Period::All,
        }],
        sources: vec![Source {
            key: "oteryn:spawn/bundle-field-zones".into(),
            floor: -7,
            x: 2,
            y: 0,
            points: [2, 3]
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
    let facts = crate::map::boot::table_activation_spawns(&source, |key| {
        let policy = runtime.companion_policy(key).ok()?;
        Some((
            u64::try_from(policy.maximum_health).ok()?,
            policy.blocked_by_nearby_players,
        ))
    })
    .ok_or("native source spawn policies")?;
    runtime
        .begin_activation_spawn_sources(facts)
        .map_err(debug)?;
    assert_eq!(
        runtime
            .realize_activation_spawn_window(0)
            .map_err(debug)?
            .realized,
        2
    );
    let actors: [ExactActorRef; 2] = runtime
        .visible_entities()
        .creatures
        .into_iter()
        .map(|visible| visible.actor)
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "two source-spawned creatures")?;
    for (ordinal, actor) in actors.iter().enumerate() {
        assert_eq!(
            runtime
                .read_actor_position(*actor)
                .map_err(debug)?
                .position(),
            at(i32::try_from(ordinal + 2)?)
        );
    }
    Ok((deps, actors))
}

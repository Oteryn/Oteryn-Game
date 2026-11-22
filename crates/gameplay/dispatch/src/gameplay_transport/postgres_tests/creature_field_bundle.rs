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

#[derive(Clone, Copy, PartialEq, Eq)]
enum FieldCase {
    BundleZones,
    StepOff,
    NoField,
    Ordinals,
    FreshField,
}

#[test]
fn creature_field_bundle_pickupable_stack_keeps_current_zone_and_residual_ticks_postgres()
-> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, FieldCase::BundleZones)
}

#[test]
fn creature_field_step_off_keeps_due_ticks_and_cleans_entry_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, FieldCase::StepOff)
}

#[test]
fn creature_without_field_or_conditions_skips_tile_and_zone_reads_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, FieldCase::NoField)
}

#[test]
fn creature_field_gate_preserves_owner_work_ordinals() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, FieldCase::Ordinals)
}

#[test]
fn creature_field_fresh_entry_on_live_ground_field_stays_candidate_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, FieldCase::FreshField)
}

#[test]
fn creature_field_pass_skips_protected_stop_and_ticks_later_creature_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        let harness = Harness::create_with_admission_seed(
            admin,
            "creature_field_protected_stop",
            false,
            false,
        )
        .await?;
        let result = exercise_protected_stop(&harness).await;
        let cleanup = harness.cleanup().await;
        result?;
        cleanup
    })
}

async fn exercise_protected_stop(harness: &Harness) -> TestResult {
    use oteryn_gameplay_encounters::bosses::phase_controller::{cults, gates};

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
    let stop_key = format!("{}_stop", cults::plan().roles[5].creature);
    let (deps, [stop, later]) =
        dependencies(original, active, &pin, false, Some(stop_key.clone()))?;
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = deps.room.movement_cells().into();
    let permit = owner.spell_lane_permit().await.ok_or("field lane")?;
    let names = owner
        .read_native_health_names(&permit)
        .await
        .map_err(debug)?;
    let mut runtime = deps.runtime.lock().await;
    assert!(runtime.matches_live_creature_identity(stop, stop_key.as_bytes()));
    // Use the same carrier hook that CultsEncounter::bind installs on its stop forms.
    gates::protect(&mut runtime, &[stop]).map_err(debug)?;
    let hooks = runtime.encounter_hooks().ok_or("Cults damage hook")?;
    assert!(hooks.contains(stop));
    assert!(!hooks.contains(later));
    assert!(!runtime.has_creature_periodic_state(stop));
    let stop_before = runtime.companion_snapshot(stop).map_err(debug)?;
    let residual = CreatureFieldEntry {
        definition: ConditionDefinition::new(
            "test.bundle.creature.residual",
            1,
            ConditionValues::Agony {
                // Agony has no element and can tick on the second bundle tile's PZ.
                total_min: 10,
                total_max: 10,
                per_tick: 5,
                interval_ms: 1_000,
                delayed: true,
            },
        )
        .ok_or("residual condition")?,
        source: None,
        lineage: None,
        occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(id(1)),
    };
    runtime
        .commit_creature_field_entry(later, 0, Some(&residual))
        .map_err(debug)?;
    let later_before = runtime.companion_snapshot(later).map_err(debug)?;
    let names = owner
        .seal_native_health_names(&runtime, names)
        .map_err(debug)?;
    let revisions =
        crate::ability::RevisionSet::new("rules-1", "content-1", "world-1", "formula-1", "sim-1")
            .map_err(debug)?;
    let mut states = owner.states.spell_states.lock().await;
    let mut attack = owner.states.attack.lock().await;
    let before = runtime.issue_owner_work().map_err(debug)?;
    let (_, actors, completed) = owner
        .drive_monster_profiles(
            &mut runtime,
            &mut states,
            &mut attack,
            &deps.room,
            native,
            &permit,
            &revisions,
            &[],
            1_000_000,
            &names,
        )
        .await
        .map_err(|()| "protected stop aborted the monster profile pass")?;
    assert!(
        completed,
        "the profile pass finished fields and its remaining stages"
    );
    assert!(actors.is_empty(), "the regression uses passive creatures");
    assert_eq!(
        runtime.companion_snapshot(stop).map_err(debug)?,
        stop_before
    );
    assert!(!runtime.has_creature_periodic_state(stop));
    let after = runtime.issue_owner_work().map_err(debug)?;
    // Between the markers: one empty AI batch and one periodic turn for the rat.
    assert_eq!(after.ordinal().get(), before.ordinal().get() + 3);
    let later_after = runtime.companion_snapshot(later).map_err(debug)?;
    assert_eq!(later_after.health, later_before.health - 5);
    let [condition] = later_after.state.conditions.instances() else {
        return Err("one residual condition must remain after the tick".into());
    };
    assert_eq!(condition.remaining_total(), 5);
    Ok(())
}

fn run_case(admin: String, case: FieldCase) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            let executor = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            executor
                .block_on(Box::pin(scenario(admin, case)))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "creature bundle field scenario panicked")?
        .map_err(Into::into)
}

async fn scenario(admin: String, case: FieldCase) -> TestResult {
    let name = match case {
        FieldCase::BundleZones => "creature_bundle_field",
        FieldCase::StepOff => "creature_field_step_off",
        FieldCase::NoField => "creature_field_skip_reads",
        FieldCase::Ordinals => "creature_field_ordinals",
        FieldCase::FreshField => "creature_field_fresh_entry",
    };
    let harness = Harness::create_with_admission_seed(admin, name, false, false).await?;
    let result = exercise(&harness, case).await;
    let cleanup = harness.cleanup().await;
    result?;
    cleanup
}

async fn exercise(harness: &Harness, case: FieldCase) -> TestResult {
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
    let (deps, actors) = dependencies(original, active, &pin, true, None)?;
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = deps.room.movement_cells().into();
    if case == FieldCase::FreshField {
        owner.required_capabilities =
            crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[11, 30],
                deps.actor.runtime_scope(),
                deps.actor.scope_generation().get(),
                "policy-1",
                crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
            )
            .map_err(debug)?;
    }
    let permit = owner.spell_lane_permit().await.ok_or("field lane")?;
    if case == FieldCase::FreshField {
        seed_live_field(harness, &owner, &deps, &permit).await?;
    }
    let health_names = owner
        .read_native_health_names(&permit)
        .await
        .map_err(debug)?;
    let mut runtime = deps.runtime.lock().await;
    let native_health_names = owner
        .seal_native_health_names(&runtime, health_names)
        .map_err(debug)?;
    let objects = deps.door.lock().await;
    let states = owner.states.spell_states.lock().await;
    // TEST_PARAMETERS: an existing residual Drown condition distinguishes PZ suppression
    // from ordinary damage. Its native store and due cursor remain the production owners.
    let entered = match case {
        FieldCase::BundleZones => &actors[..],
        FieldCase::StepOff => &actors[..1],
        FieldCase::Ordinals => &actors[1..],
        FieldCase::NoField | FieldCase::FreshField => &actors[..0],
    };
    for (ordinal, actor) in entered.iter().enumerate() {
        let field = CreatureFieldEntry {
            definition: ConditionDefinition::new(
                "test.bundle.creature.residual",
                1,
                if case == FieldCase::Ordinals {
                    // Synthetic lethal Agony is independent of the second tile's PZ.
                    ConditionValues::Agony {
                        total_min: 1_000,
                        total_max: 1_000,
                        per_tick: 1_000,
                        interval_ms: 1_000,
                        delayed: true,
                    }
                } else {
                    ConditionValues::DamageOverTime {
                        element: DotElement::Drown,
                        total_min: 10,
                        total_max: 10,
                        per_tick: 5,
                        interval_ms: 1000,
                        delayed: true,
                    }
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
    if case == FieldCase::StepOff {
        let on_field = runtime.read_actor_position(actors[0]).map_err(debug)?;
        let stepped = runtime
            .borrow_movement_position()
            .commit_cardinal(on_field, at(1))
            .map_err(debug)?;
        assert_eq!(stepped.position(), at(1));
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
    let (mut source_deps, _) = super::loop_monster_fixtures::fixture::dependencies(active, &pin)?;
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
        &native_health_names,
    );
    let pass = owner.root.try_issue_semantic_pass().map_err(debug)?;
    pass.run_with_context(&mut context, move |holder, deadline, context| {
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
                native_health_names,
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
            if case != FieldCase::BundleZones {
                assert_eq!(runtime.visible_entities().creatures.len(), 2);
                if case == FieldCase::FreshField {
                    assert!(actors.iter().all(|actor| !runtime.has_creature_periodic_state(*actor)));
                }
                let before_stamp = if case == FieldCase::Ordinals {
                    Some(runtime.issue_owner_work().map_err(|_| DurabilityError::Unavailable)?)
                } else {
                    None
                };
                owner
                    .tick_creature_fields(
                        &mut map, runtime, states, native, 1_000_000, native_health_names,
                    )
                    .await?;
                match case {
                    FieldCase::StepOff => {
                        assert_eq!(map.field_read_counts_for_test(), [1, 1, 1]);
                        let after = runtime
                            .companion_snapshot(actors[0])
                            .map_err(|_| DurabilityError::Unavailable)?;
                        assert!(after.health < before[0], "the first residual was due");
                        let after_first_tick = after.health;
                        assert_eq!(after.state.conditions.instances()[0].remaining_total(), 5);
                        owner
                            .tick_creature_fields(
                                &mut map, runtime, states, native, 2_000_000, native_health_names,
                            )
                            .await?;
                        let after = runtime
                            .companion_snapshot(actors[0])
                            .map_err(|_| DurabilityError::Unavailable)?;
                        assert!(after.health < after_first_tick, "the last residual was due");
                        assert!(!runtime.has_creature_periodic_state(actors[0]));
                    }
                    FieldCase::NoField => {
                        assert_eq!(map.field_read_counts_for_test(), [0, 0, 0]);
                    }
                    FieldCase::Ordinals => {
                        assert_eq!(map.field_read_counts_for_test(), [1, 1, 1]);
                        let before_stamp = before_stamp.ok_or(DurabilityError::Unavailable)?;
                        let next = runtime.issue_owner_work().map_err(|_| DurabilityError::Unavailable)?;
                        assert_eq!(next.ordinal().get(), before_stamp.ordinal().get() + 3);
                        let later = actors[1];
                        let mut death = runtime.borrow_combat_death();
                        let receipt = death.committed_lethal_receipt(later)
                            .map_err(|_| DurabilityError::Unavailable)?;
                        let corpse = death.project_committed_lethal(receipt)
                            .map_err(|_| DurabilityError::Unavailable)?;
                        // On the ungated path the first rat's empty turn takes +1.
                        let expected = format!("owner-tick/{}/{}/{}/{}",
                            later.actor_local_id(), later.actor_local_generation(),
                            before_stamp.generation().get(), before_stamp.ordinal().get() + 2);
                        assert_eq!(
                            corpse.occurrence().commit_binding().split(|byte| *byte == 0).next(),
                            Some(expected.as_bytes()),
                        );
                    }
                    FieldCase::FreshField => {
                        assert_eq!(map.field_read_counts_for_test(), [1, 1, 1]);
                        let after = runtime.companion_snapshot(actors[0])
                            .map_err(|_| DurabilityError::Unavailable)?;
                        assert!(after.health < before[0]);
                        assert!(after.state.conditions.instances().iter().any(|condition|
                            condition.provenance().source_kind ==
                                oteryn_gameplay_foundation::foundation::condition::ConditionSourceKind::Field
                        ));
                    }
                    FieldCase::BundleZones => {}
                }
                assert!(map.validate_transaction(runtime).await);
                drop(map);
                tx.rollback().await?;
                return Ok(());
            }
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
                    .tick_creature_fields(&mut map, runtime, states, native, now, native_health_names)
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

async fn seed_live_field(
    harness: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    permit: &crate::durability::spell_owner_commit::SpellLanePermit,
) -> TestResult {
    use crate::durability::native_world_items::{SpellGroundTarget, qualify_source_field_chain};
    use crate::durability::spell_item_transaction::{
        apply_spell_items_in_transaction, assert_spell_item_authority_in_transaction,
    };
    use crate::durability::spell_items_abi::*;
    use crate::foundation::{CommandId, CommandRef};

    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native fields")?;
    // The same yellow flame as field_runes_postgres_creature_entry_and_residual_tick.
    let policy = native
        .item_policy_for_source_id(21465)
        .ok_or("native yellow flame")?;
    let definition = qualify_source_field_chain(
        native,
        QualifiedItemDefinition::from_native_policy(policy).map_err(debug)?,
    )
    .map_err(debug)?;
    let placement = {
        let runtime = deps.runtime.lock().await;
        SpellGroundTarget::for_native_tile_read(&deps.room, &runtime, at(2))?
            .placement(b"fixture:creature-field-fresh-entry".to_vec())
    };
    let (current, _) =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(harness.root.clone())
            .current_session_at(deps.admitted.game_session_id)
            .await?;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("field fence")?;
    let command = CommandRef::new(
        deps.admitted.game_session_id,
        CommandId::new(4900).map_err(debug)?,
    );
    let request = SpellItemTransactionRequest {
        swing_ordinal: None,
        command,
        spell: crate::durability::item_mint::TypedDefinitionRef {
            family: "Spell".into(),
            production_key: "fixture:creature-field".into(),
            revision_ref: "definition-r1".into(),
        },
        catalog_digest: native.source_digest(),
        transaction_id: id(188),
        event_id: id(189),
        cost: CastCostBinding {
            vitals_revision_before: 1,
            vitals_revision_after: 2,
            mana_before: 200,
            mana_after: 180,
            soul_before: 100,
            soul_after: 100,
            cooldowns_before: Vec::new(),
            cooldowns_after: vec![("spell:fixture_creature_field".into(), 100_000)],
            caster_digest_before: [11; 32],
            caster_digest_after: [12; 32],
        },
        caster_origin: Some(SourceCasterOrigin {
            actor: deps.actor,
            character_lease_generation: fence.character_lease_generation,
        }),
        operations: vec![SpellItemOperation::MintGround {
            item_instance_id: id(180),
            definition,
            quantity: 1,
            placement,
            lifetime_millis: None,
            blocks_movement: policy
                .record()
                .attributes
                .blocks_movement
                .ok_or("field movement")?,
            blocks_projectile: policy
                .record()
                .attributes
                .blocks_projectile
                .ok_or("field projectile")?,
            description: None,
        }],
        companion: None,
        direct_companion: None,
    };
    let mut tx = harness.pool.begin().await?;
    let authority = assert_spell_item_authority_in_transaction(
        &mut tx,
        permit,
        owner.root,
        owner.character,
        owner.holder,
        &fence,
        command,
        native.source_digest(),
    )
    .await
    .map_err(debug)?;
    let outcome = apply_spell_items_in_transaction(
        &mut tx,
        &authority,
        &request,
        &crate::gameplay_transport::world_required_field_guard::player_origin(
            &owner.required_capabilities,
        ),
    )
    .await
    .map_err(debug)?;
    assert!(matches!(outcome, SpellItemTransactionOutcome::Applied(_)));
    tx.commit().await?;
    Ok(())
}

#[test]
fn creature_field_pass_completes_beside_a_hooked_guardian_postgres() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| hooked_scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "hooked guardian field scenario panicked")?
        .map_err(Into::into)
}

fn hooked_scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "creature_field_hooked", false, false)
                .await?;
        let result = exercise_hooked(&harness).await;
        let cleanup = harness.cleanup().await;
        result?;
        cleanup
    })
}

/// A Cults stop form is protected by the encounter damage gates, so the periodic
/// owner refuses its turn. The field pass still completes for every other creature,
/// including the one census-ordered after the hooked guardian.
async fn exercise_hooked(harness: &Harness) -> TestResult {
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
    let (deps, [guardian, rat]) = dependencies(original, active, &pin, false, None)?;
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = deps.room.movement_cells().into();
    let permit = owner.spell_lane_permit().await.ok_or("field lane")?;
    let health_names = owner
        .read_native_health_names(&permit)
        .await
        .map_err(debug)?;
    let mut runtime = deps.runtime.lock().await;
    let native_health_names = owner
        .seal_native_health_names(&runtime, health_names)
        .map_err(debug)?;
    let objects = deps.door.lock().await;
    let states = owner.states.spell_states.lock().await;
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
        occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(id(1)),
    };
    runtime
        .commit_creature_field_entry(rat, 0, Some(&field))
        .map_err(debug)?;
    oteryn_gameplay_encounters::bosses::phase_controller::gates::protect(&mut runtime, &[guardian])
        .map_err(debug)?;
    assert!(
        runtime
            .encounter_hooks()
            .is_some_and(|hooks| hooks.contains(guardian))
    );
    let mut context = (
        &owner,
        &mut *runtime,
        &*states,
        &deps.room,
        &*objects,
        native,
        &permit,
        (rat, guardian),
        &native_health_names,
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
                native_health_names,
            ) = context;
            let (rat, guardian) = *actors;
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
            let health = |runtime: &crate::foundation::ChannelRuntimeV1, actor| {
                runtime
                    .companion_snapshot(actor)
                    .map(|snapshot| snapshot.health)
                    .map_err(|_| DurabilityError::Unavailable)
            };
            let (rat_before, guardian_before) = (health(runtime, rat)?, health(runtime, guardian)?);
            let mut map =
                CurrentMonsterMap::new(&mut tx, &authority, runtime, room, objects, native);
            let pass = owner
                .tick_creature_fields(
                    &mut map,
                    runtime,
                    states,
                    native,
                    1_000_000,
                    native_health_names,
                )
                .await;
            assert!(
                pass.is_ok(),
                "a hooked guardian aborted the field pass: {pass:?}"
            );
            assert!(map.validate_transaction(runtime).await);
            drop(map);
            let after = runtime
                .companion_snapshot(rat)
                .map_err(|_| DurabilityError::Unavailable)?;
            assert!(
                after.health < rat_before,
                "the ordinary creature still ticks"
            );
            assert_eq!(after.state.conditions.instances()[0].remaining_total(), 5);
            assert_eq!(health(runtime, guardian)?, guardian_before);
            tx.rollback().await?;
            Ok(())
        })
    })
    .await
    .map_err(debug)?;
    Ok(())
}

#[test]
fn creature_field_pass_defers_a_reserved_source_dot_until_release_postgres() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| reserved_source_scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "reserved source field scenario panicked")?
        .map_err(Into::into)
}

fn reserved_source_scenario() -> TestResult {
    run(async |admin| {
        let harness = Harness::create_with_admission_seed(
            admin,
            "creature_field_reserved_source",
            false,
            false,
        )
        .await?;
        let result = exercise_reserved_source(&harness).await;
        let cleanup = harness.cleanup().await;
        result?;
        cleanup
    })
}

/// A commandless owner event holding the live Player source, as a pending
/// source callback does until its durable outcome is observed.
struct HeldSource {
    player: ExactActorRef,
    session: crate::foundation::GameSessionId,
}
impl crate::foundation::source_reservation_seal::Sealed for HeldSource {}
impl crate::foundation::SourceActorReservationProof for HeldSource {
    fn owner(&self) -> (ExactActorRef, crate::foundation::GameSessionId) {
        (self.player, self.session)
    }
    fn event_identity(&self) -> [u8; 16] {
        id(9)
    }
    fn source_binding_digest(&self) -> [u8; 32] {
        [9; 32]
    }
    fn affected_actors(&self) -> &[ExactActorRef] {
        &[]
    }
    fn current_for(&self, runtime: &crate::foundation::ChannelRuntimeV1) -> bool {
        runtime
            .player_control_facts(self.player, self.session)
            .is_ok()
    }
}

/// The periodic owner refuses a DOT whose live Player source is reserved. That
/// creature waits for a later pass, the pass still completes for the creature
/// census-ordered after it, and the deferred DOT ticks once the source is released.
async fn exercise_reserved_source(harness: &Harness) -> TestResult {
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
    let (deps, [sourced, plain]) = dependencies(original, active, &pin, false, None)?;
    let (player, session) = (deps.actor, deps.admitted.game_session_id);
    let mut owner = deps.owner(harness, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = deps.room.movement_cells().into();
    let permit = owner.spell_lane_permit().await.ok_or("field lane")?;
    let health_names = owner
        .read_native_health_names(&permit)
        .await
        .map_err(debug)?;
    let mut runtime = deps.runtime.lock().await;
    let native_health_names = owner
        .seal_native_health_names(&runtime, health_names)
        .map_err(debug)?;
    let objects = deps.door.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    states
        .initialize(
            &runtime,
            player,
            session,
            crate::gameplay_transport::actor_spell::tests::FACTS,
            (0, 0),
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
        )
        .ok_or("player spell state")?;
    // A live Player source carries its bound attacker lease, as every admitted
    // attacker does before its first hit.
    runtime
        .bind_attacker_lease(
            player,
            session,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                1,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let residual = || {
        ConditionDefinition::new(
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
        .ok_or("residual condition")
    };
    runtime
        .commit_creature_field_entry(
            plain,
            0,
            Some(&CreatureFieldEntry {
                definition: residual()?,
                source: None,
                lineage: None,
                occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(id(1)),
            }),
        )
        .map_err(debug)?;
    // TEST_PARAMETERS: the same residual DOT, owned by the live Player source.
    let before = runtime.companion_snapshot(sourced).map_err(debug)?;
    let mut next = before.state.clone();
    let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([8; 32]);
    next.conditions
        .apply(
            &residual()?,
            Some(player),
            oteryn_gameplay_foundation::foundation::condition::ConditionSourceKind::Player,
            &[],
            &oteryn_gameplay_foundation::foundation::condition::ApplicationFacts {
                now: 0,
                base_speed: 100,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: false,
                decision_root: &root,
                occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(id(2)),
            },
        )
        .map_err(debug)?;
    runtime
        .compare_companion_state(&before, next)
        .map_err(debug)?;
    let mut context = (
        &owner,
        &mut *runtime,
        &*states,
        &deps.room,
        &*objects,
        native,
        &permit,
        (sourced, plain, player, session),
        &native_health_names,
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
                native_health_names,
            ) = context;
            let (sourced, plain, player, session) = *actors;
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
            let health = |runtime: &crate::foundation::ChannelRuntimeV1, actor| {
                runtime
                    .companion_snapshot(actor)
                    .map(|snapshot| snapshot.health)
                    .map_err(|_| DurabilityError::Unavailable)
            };
            let remaining = |runtime: &crate::foundation::ChannelRuntimeV1, actor| {
                runtime
                    .companion_snapshot(actor)
                    .map(|snapshot| {
                        snapshot
                            .state
                            .conditions
                            .instances()
                            .first()
                            .map(|instance| instance.remaining_total())
                    })
                    .map_err(|_| DurabilityError::Unavailable)
            };
            let proof = HeldSource { player, session };
            let mut held = runtime
                .prepare_source_actor_reservation(&proof)
                .map_err(|_| DurabilityError::Unavailable)?;
            runtime
                .reserve_source_actors(&mut held, &proof)
                .map_err(|_| DurabilityError::Unavailable)?;
            assert!(runtime.actor_spell_reserved(player));
            let (sourced_before, plain_before) =
                (health(runtime, sourced)?, health(runtime, plain)?);
            let mut map =
                CurrentMonsterMap::new(&mut tx, &authority, runtime, room, objects, native);
            let pass = owner
                .tick_creature_fields(
                    &mut map,
                    runtime,
                    states,
                    native,
                    1_000_000,
                    native_health_names,
                )
                .await;
            assert!(
                pass.is_ok(),
                "a reserved DOT source aborted the field pass: {pass:?}"
            );
            assert!(map.validate_transaction(runtime).await);
            drop(map);
            assert!(
                health(runtime, plain)? < plain_before,
                "the creature after the deferred one still ticks"
            );
            assert_eq!(
                health(runtime, sourced)?,
                sourced_before,
                "the reserved source's DOT waits"
            );
            assert_eq!(
                remaining(runtime, sourced)?,
                Some(10),
                "no due tick is consumed while reserved"
            );
            runtime
                .release_source_actors_after_observed_outcome(held, &proof)
                .map_err(|_| DurabilityError::Unavailable)?;
            assert!(!runtime.actor_spell_reserved(player));
            let mut map =
                CurrentMonsterMap::new(&mut tx, &authority, runtime, room, objects, native);
            let pass = owner
                .tick_creature_fields(
                    &mut map,
                    runtime,
                    states,
                    native,
                    2_000_000,
                    native_health_names,
                )
                .await;
            assert!(
                pass.is_ok(),
                "the released source's DOT pass failed: {pass:?}"
            );
            assert!(map.validate_transaction(runtime).await);
            drop(map);
            assert!(
                health(runtime, sourced)? < sourced_before,
                "the deferred DOT ticks once the source is released"
            );
            assert!(remaining(runtime, sourced)?.is_none_or(|left| left < 10));
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
    second_tile_pz: bool,
    first_creature: Option<String>,
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
    // The second creature's tile is a protection zone only when the case asks for one.
    for (index, flags) in [(1, 0), (2, u32::from(second_tile_pz))] {
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
    let rat = u32::from(first_creature.is_some());
    let source = Table {
        creatures: first_creature
            .into_iter()
            .chain(std::iter::once("oteryn:creature.rat".into()))
            .map(|key| Creature {
                key,
                period: Period::All,
            })
            .collect(),
        sources: vec![Source {
            key: "oteryn:spawn/bundle-field-zones".into(),
            floor: -7,
            x: 2,
            y: 0,
            points: [(2, 0), (3, rat)]
                .into_iter()
                .map(|(x, creature)| Point {
                    creature,
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

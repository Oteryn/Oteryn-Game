use super::fixture::{Harness, debug, id, run};
use super::house_transfer_fixtures::{RecoverySource, prepare_owned_session};
use super::{Dependencies, TestResult};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{AuthenticatedTransportRefV1, ChannelContentPin, ChannelId, WorldId};
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::connection::{
    ControlLossResult, FirstEntryOutcome, FreshAdmissionAuthority, ResumeAttempt, UseCommand,
};
use crate::gameplay_transport::equipment_move::{move_equipment, observe_character_equipment};
use crate::gameplay_transport::item_move::{EquipmentMove, EquipmentMoveResult};
use crate::gameplay_transport::item_ref_admission::definition_ref;
use crate::gameplay_transport::item_view::ItemKey;
use crate::map::boot::tests::{floor_items, floor_pins, teleport_bundle};
use oteryn_protocol_oteryn::item_view::EquipmentSlot;
use std::time::Duration;

#[test]
fn stealth_ring_equip_hides_until_expiry_postgres() -> TestResult {
    run_ring("stealth_ring", 3049, 3086, 600_000, Case::Lifecycle)
}

#[test]
fn life_ring_regen_pauses_and_resumes_postgres() -> TestResult {
    run_ring("life_ring", 3052, 3089, 1_200_000, Case::Lifecycle)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Lifecycle,
    Replacement,
    StalledCadence,
    Status,
    ExpiryPublication,
    InterruptedMove,
    ParkedReplay,
    QualifiedVisibility,
    DeferredFinalPulse,
    ReservedFinalPulse,
    RecoveredWearStart,
    ZoneBoundary,
    ReadmittedStatus,
}

#[test]
fn timed_worn_rings_replacement_has_its_own_regeneration_clock_postgres() -> TestResult {
    run_ring("ring_replacement", 3052, 3089, 1_200_000, Case::Replacement)
}

#[test]
fn timed_worn_rings_stall_credits_elapsed_wear_postgres() -> TestResult {
    run_ring("ring_stalled", 3052, 3089, 1_200_000, Case::StalledCadence)
}

#[test]
fn timed_worn_rings_project_condition_status_postgres() -> TestResult {
    run_ring("stealth_status", 3049, 3086, 600_000, Case::Status)?;
    run_ring("life_status", 3052, 3089, 1_200_000, Case::Status)
}

#[test]
fn timed_worn_rings_clear_effect_before_expiry_publication_postgres() -> TestResult {
    run_ring(
        "stealth_expiry_publication",
        3049,
        3086,
        600_000,
        Case::ExpiryPublication,
    )?;
    run_ring(
        "life_expiry_publication",
        3052,
        3089,
        1_200_000,
        Case::ExpiryPublication,
    )
}

#[test]
fn timed_worn_rings_reconcile_interrupted_move_publication_postgres() -> TestResult {
    run_ring(
        "ring_interrupted_move",
        3052,
        3089,
        1_200_000,
        Case::InterruptedMove,
    )
}

#[test]
fn timed_worn_rings_parked_item_replay_keeps_its_revision_postgres() -> TestResult {
    run_ring(
        "parked_item_replay_keeps_its_revision",
        3052,
        3089,
        1_200_000,
        Case::ParkedReplay,
    )
}

#[test]
fn timed_worn_rings_stealth_visibility_requires_positive_wear_postgres() -> TestResult {
    run_ring(
        "stealth_visibility_requires_positive_wear",
        3049,
        3086,
        600_000,
        Case::QualifiedVisibility,
    )
}

#[test]
fn timed_worn_rings_expiry_retains_the_final_pulse_postgres() -> TestResult {
    run_ring(
        "expiry_retains_the_final_pulse",
        3052,
        3089,
        1_200_000,
        Case::DeferredFinalPulse,
    )
}

#[test]
fn timed_worn_rings_reserved_actor_recovers_the_final_pulse_postgres() -> TestResult {
    run_ring(
        "reserved_ring_expiry",
        3052,
        3089,
        1_200_000,
        Case::ReservedFinalPulse,
    )
}

#[test]
fn timed_worn_rings_interrupted_equip_recovers_elapsed_wear_postgres() -> TestResult {
    run_ring(
        "interrupted_equip_recovers_elapsed_wear",
        3052,
        3089,
        1_200_000,
        Case::RecoveredWearStart,
    )
}

#[test]
fn timed_worn_rings_pz_crossing_settles_at_the_boundary_postgres() -> TestResult {
    run_ring(
        "pz_crossing_settles_at_the_boundary",
        3052,
        3089,
        1_200_000,
        Case::ZoneBoundary,
    )
}

#[test]
fn timed_worn_rings_admission_projects_wear_before_first_sweep_postgres() -> TestResult {
    run_ring(
        "admission_projects_wear_before_first_sweep",
        3049,
        3086,
        600_000,
        Case::ReadmittedStatus,
    )
}

fn run_ring(name: &'static str, base: u32, worn: u32, full_ms: i64, case: Case) -> TestResult {
    super::journal_watchdog::run(name, move |stage| {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, name, true, false).await?;
            eprintln!("POSTGRES_STARTED {name}");
            stage.set("equip, reconnect, and expire");
            let result = exercise_ring(&h, base, worn, full_ms, case).await;
            stage.set("cleanup");
            let cleanup = h.cleanup().await;
            result.and(cleanup)
        })
    })
}

async fn exercise_ring(h: &Harness, base: u32, worn: u32, full_ms: i64, case: Case) -> TestResult {
    let recovery = RecoverySource::start(h).await?;
    prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
    let active = controller.active().ok_or("active generation")?;
    let index = active
        .native_gameplay()
        .ok_or("native gameplay")?
        .item_index()
        .ok_or("item index")?;
    let channel = ChannelId::decode(&id(43)).map_err(debug)?;
    let (map_bytes, map_pins) = teleport_bundle();
    let map = crate::map::boot::boot(
        &map_bytes,
        &floor_pins(map_pins),
        world,
        channel,
        floor_items,
    )
    .map_err(debug)?;
    let cells = map
        .movement_cells(pin.qualified_room().movement_cells())
        .map_err(debug)?;
    let frame = map.frame_binding_digest(world);
    let deps = Dependencies::with_content(
        ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            frame,
            map.map_revision_digest(),
            (6, 1, 8),
        ),
        pin.qualified_room().clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    deps.runtime
        .lock()
        .await
        .initialize_first_entry_position(deps.actor)
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    owner.evidence = &recovery.evidence;
    let actor = deps.actor;
    let session = deps.admitted.game_session_id;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "fence")?
        .ok_or("current fence")?;
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = Some(fence.item_fence());
    owner.states.command_ingress.admit(session);

    let base_key = format!("oteryn:item.tibia.i{base}");
    let food_revision = if case == Case::ParkedReplay {
        Some(
            index
                .revision("oteryn:item.tibia.i29409")
                .ok_or("gourmet definition")?,
        )
    } else {
        None
    };
    seed_custody(
        h,
        &[
            (
                "oteryn:item.tibia.i2854",
                index
                    .revision("oteryn:item.tibia.i2854")
                    .ok_or("backpack revision")?,
            ),
            (&base_key, index.revision(&base_key).ok_or("ring revision")?),
            (&base_key, index.revision(&base_key).ok_or("ring revision")?),
        ],
        food_revision,
    )
    .await?;
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(active.identity().server_artifact_digest().as_slice())
        .bind(active.identity().client_artifact_digest().as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;

    deps.initialize(&owner).await;
    let (max_health, max_mana) = {
        let mut runtime = owner.runtime.lock().await;
        assert_eq!(
            crate::gameplay_transport::actor_spell::qualified_protection_zone(
                &mut runtime,
                owner.movement_cells(),
                actor,
            ),
            Some(false),
            "the fixture entry is outside protection zones"
        );
        let mut states = owner.states.spell_states.lock().await;
        let state = states
            .get_mut(&runtime, actor, session)
            .ok_or("actor spell state")?;
        state.restore_saved_vitals(25, 0);
        let facts = state.character_facts();
        (facts.max_health, facts.max_mana)
    };
    let invisible = base == 3049;
    let starting = ring_effects(&owner, &deps).await?;
    assert_eq!(starting, (false, 0, 25, 0));
    let item = ItemKey::Instance(id(90));
    let command = |admitted: super::AdmittedSession, command_id| UseCommand {
        game_session_id: admitted.game_session_id,
        command_id,
        item_fence: admitted.item_fence,
    };
    let equip = EquipmentMove::Equip {
        item,
        to: EquipmentSlot::Ring,
    };
    let unequip = EquipmentMove::Unequip {
        item,
        from: EquipmentSlot::Ring,
    };
    if case == Case::RecoveredWearStart {
        use crate::durability::character_equipment::EquipmentMove as DurableMove;
        use crate::foundation::{CommandId, CommandRef};
        use crate::gameplay_transport::item_ref_admission::NativeItemWeights;

        owner
            .root
            .commit_equipment_move(
                owner.character,
                owner.holder,
                &fence.item_fence(),
                CommandRef::new(session, CommandId::new(1).map_err(debug)?),
                &NativeItemWeights(active.native_gameplay().ok_or("native gameplay")?),
                DurableMove {
                    item: id(90),
                    from_slot: None,
                    to_slot: oteryn_protocol_oteryn::item_view::durable_slot(EquipmentSlot::Ring),
                },
            )
            .await
            .map_err(debug)?
            .map_err(debug)?;
        // The timestamp comes from the committed receipt's clock, so let that
        // clock advance before the first owner observation of the new slot.
        tokio::time::sleep(Duration::from_millis(6_050)).await;
        owner.tick_timed_items().await;
        assert_eq!(ring_effects(&owner, &deps).await?.2, 27);
        return Ok(());
    }
    let moved = move_equipment(&owner, actor, command(admitted, 1), equip).await;
    if moved != EquipmentMoveResult::Committed {
        return Err(format!("equip {base} through the durable equipment policy: {moved:?}").into());
    }
    let worn_key = format!("oteryn:item.tibia.i{worn}");
    let worn_ref = definition_ref(
        index,
        &TypedDefinitionRef {
            family: "Item".into(),
            revision_ref: index.revision(&worn_key).ok_or("worn revision")?.into(),
            production_key: worn_key,
        },
    )
    .ok_or("worn definition ref")?;
    let view = observe_character_equipment(&owner, session)
        .await
        .ok_or("worn view")?;
    assert_eq!(view.len(), 1);
    assert_eq!(
        (view[0].0, view[0].1.key, view[0].1.item_definition_ref),
        (EquipmentSlot::Ring, item, worn_ref)
    );
    assert_eq!(
        ring_effects(&owner, &deps).await?,
        (invisible, usize::from(!invisible), 25, 0)
    );

    match case {
        Case::Replacement => {
            for _ in 0..5 {
                tick_one_second(&mut owner, &deps).await;
            }
            let replacement = EquipmentMove::Equip {
                item: ItemKey::Instance(id(94)),
                to: EquipmentSlot::Ring,
            };
            assert_eq!(
                move_equipment(&owner, actor, command(admitted, 2), replacement).await,
                EquipmentMoveResult::Committed
            );
            tick_one_second(&mut owner, &deps).await;
            let (_, _, health, mana) = ring_effects(&owner, &deps).await?;
            assert_eq!(
                (health, mana),
                (25, 1),
                "the replacement must wait for its own pulse"
            );
            return Ok(());
        }
        Case::StalledCadence => {
            tick_regen_pulse(&mut owner, &deps).await;
            let (_, _, health, mana) = ring_effects(&owner, &deps).await?;
            owner.clock_origin -= Duration::from_secs(7);
            owner.tick_timed_items().await;
            let _ = owner.tick_spell_periodic(actor, session).await;
            let (_, _, next_health, next_mana) = ring_effects(&owner, &deps).await?;
            assert_eq!(
                next_health - health,
                2,
                "the seven-second stall includes a six-second pulse"
            );
            assert!(next_mana - mana >= 8);
            return Ok(());
        }
        Case::Status => {
            owner.clock_origin -= Duration::from_secs(7);
            return assert_ring_status(&owner, admitted, invisible, full_ms - 7_000).await;
        }
        Case::ExpiryPublication => {
            use crate::gameplay_transport::timed_item_clock::RETIRED_EQUIPMENT_READ;
            let entered = std::sync::Arc::new(tokio::sync::Notify::new());
            let proceed = std::sync::Arc::new(tokio::sync::Notify::new());
            owner.clock_origin -= Duration::from_millis(u64::try_from(full_ms)?);
            let expiry = RETIRED_EQUIPMENT_READ
                .scope((entered.clone(), proceed.clone()), owner.tick_timed_items());
            let inspect = async {
                entered.notified().await;
                let effects = ring_effects(&owner, &deps).await;
                proceed.notify_one();
                effects
            };
            let ((), effects) = tokio::join!(expiry, inspect);
            let (hidden, regen, _, _) = effects?;
            assert_eq!(
                (hidden, regen),
                (false, 0),
                "commit must retire the effect before reread"
            );
            return Ok(());
        }
        Case::InterruptedMove => {
            use crate::durability::character_equipment::EquipmentMove as DurableMove;
            use crate::foundation::{CommandId, CommandRef};
            use crate::gameplay_transport::item_ref_admission::NativeItemWeights;

            let native = active.native_gameplay().ok_or("native gameplay")?;
            let ring_slot = oteryn_protocol_oteryn::item_view::durable_slot(EquipmentSlot::Ring);
            for (command, from_slot, to_slot, expected) in
                [(2, ring_slot, None, 0), (3, None, ring_slot, 1)]
            {
                owner
                    .root
                    .commit_equipment_move(
                        owner.character,
                        owner.holder,
                        &fence.item_fence(),
                        CommandRef::new(session, CommandId::new(command).map_err(debug)?),
                        &NativeItemWeights(native),
                        DurableMove {
                            item: id(90),
                            from_slot,
                            to_slot,
                        },
                    )
                    .await
                    .map_err(debug)?
                    .map_err(debug)?;
                // The transaction committed, but its caller never published the new effects.
                owner.clock_origin -= Duration::from_secs(1);
                owner.tick_timed_items().await;
                assert_eq!(ring_effects(&owner, &deps).await?.1, expected);
            }
            return Ok(());
        }
        Case::ParkedReplay => {
            use crate::durability::item_use::{
                ItemUseFlaskPlan, ItemUseKind, ItemUseRequest, gourmet::GourmetUseRequest,
            };
            use crate::gameplay_transport::item_use::gourmet::{PendingUse, resolve};
            use crate::spell::actor_conditions::{gourmet_conditions, stage_gourmet_food};

            let item_fence = fence.item_fence();
            let inventory = owner
                .root
                .read_item_use_inventory(owner.character, owner.holder, item_fence)
                .await
                .map_err(debug)?;
            let item = &inventory
                .backpack
                .as_ref()
                .ok_or("gourmet backpack")?
                .entries
                .iter()
                .find(|entry| entry.item.item_instance_id == id(96))
                .ok_or("seeded gourmet item")?
                .item;
            let request;
            let before = {
                let runtime = owner.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                let state = states.get(&runtime, actor, session).ok_or("actor")?;
                let revision = state.revision();
                let now = owner.owner_now().get() / 1_000;
                let prepared = stage_gourmet_food(state, 29409, now).map_err(debug)?;
                request = GourmetUseRequest {
                    wire_binding: [0x39; 32],
                    conditions: gourmet_conditions(&prepared, now * 1_000),
                    item: ItemUseRequest {
                        kind: ItemUseKind::HirelingFood {
                            origin_channel: *owner.channel_id.as_bytes(),
                        },
                        command_id: 70,
                        content_revision: inventory.content_revision.clone(),
                        ruleset_revision: inventory.ruleset_revision.clone(),
                        sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION
                            .into(),
                        transaction_id: id(180),
                        event_id: id(181),
                        occurred_at_unix_ms:
                            crate::gameplay_transport::npc_trade_command::unix_ms_now(),
                        server_build_id: crate::durability::character_authority::SERVER_BUILD_ID
                            .into(),
                        source_item_instance_id: id(96),
                        source_definition: item.definition.clone(),
                        source_quantity_before: item.quantity,
                        flask: ItemUseFlaskPlan::None,
                    },
                };
                assert!(
                    PendingUse::new(
                        actor,
                        session,
                        item_fence,
                        request.clone(),
                        prepared,
                        revision
                    )
                    .park(&mut states)
                );
                revision
            };
            // Persist the write but leave its successor parked as on an unknown outcome.
            owner
                .root
                .commit_gourmet_use(owner.character, owner.holder, item_fence, request)
                .await
                .map_err(debug)?;
            owner.clock_origin -= Duration::from_secs(7);
            owner.tick_timed_items().await;
            {
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                assert_eq!(
                    states
                        .get(&runtime, actor, session)
                        .ok_or("actor")?
                        .revision(),
                    before
                );
            }
            assert!(
                resolve(&owner, actor, session).await,
                "the committed receipt must replay"
            );
            owner.clock_origin -= Duration::from_secs(1);
            owner.tick_timed_items().await;
            assert_eq!(ring_effects(&owner, &deps).await?.2, 27);
            let receipts: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_hireling_gourmet_receipts")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(receipts, 1);
            return Ok(());
        }
        Case::QualifiedVisibility => {
            owner.clock_origin -= Duration::from_millis(u64::try_from(full_ms)?);
            assert!(
                !ring_effects(&owner, &deps).await?.0,
                "expiry must hide the cached effect before SQL runs"
            );
            return Ok(());
        }
        Case::DeferredFinalPulse | Case::ReservedFinalPulse => {
            owner.clock_origin -= Duration::from_millis(u64::try_from(full_ms - 1_000)?);
            owner.tick_timed_items().await;
            let reservation = {
                let mut runtime = owner.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                let state = states.get_mut(&runtime, actor, session).ok_or("actor")?;
                state.restore_saved_vitals(25, 0);
                if case == Case::ReservedFinalPulse {
                    use crate::foundation::{CommandId, CommandRef};
                    use oteryn_gameplay_entities::carrier::runtime_actor_spell_types::{
                        OwnerCombatBatch, SpellAnchor, SpellOccurrenceBinding,
                    };
                    runtime
                        .bind_attacker_lease(
                            actor,
                            session,
                            crate::foundation::CharacterLease::new(
                                crate::foundation::CharacterId::decode(
                                    fence.character_id.as_bytes(),
                                )
                                .map_err(debug)?,
                                fence.character_lease_generation,
                            )
                            .map_err(debug)?,
                        )
                        .map_err(debug)?;
                    let now = owner.owner_now();
                    let mut staged = runtime
                        .stage_spell_batch(&OwnerCombatBatch {
                            caster: actor,
                            attacker: crate::foundation::CharacterId::decode(
                                fence.character_id.as_bytes(),
                            )
                            .map_err(debug)?,
                            current_lease_generation: fence.character_lease_generation,
                            command: CommandRef::new(session, CommandId::new(37).map_err(debug)?),
                            occurrence: SpellOccurrenceBinding {
                                id: "ring-owner-reservation".into(),
                                revisions: std::array::from_fn(|_| "fixture:1".into()),
                            },
                            binding: b"ring-owner-reservation".to_vec(),
                            anchor: Some(SpellAnchor {
                                expected_revision: state.revision(),
                                next_revision: state.revision() + 1,
                                paid_mana: 0,
                                paid_soul: 0,
                                cooldown_deadlines: vec![],
                            }),
                            now_us: Some(now),
                            now_ms: now.get() / 1_000,
                            effects: vec![],
                            deferred: None,
                        })
                        .map_err(debug)?;
                    runtime.reserve_spell_batch(&mut staged).map_err(debug)?;
                    assert!(runtime.actor_spell_reserved(actor));
                    Some(staged)
                } else {
                    states.enchanting_pending.push((actor, session));
                    None
                }
            };
            owner.clock_origin -= Duration::from_secs(2);
            owner.tick_timed_items().await;
            assert_eq!(
                ring_effects(&owner, &deps).await?.2,
                25,
                "the pending write owns vitals"
            );
            let held: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_character_equipment_slots WHERE item_instance_id=encode($1,'hex')::uuid)",
            ).bind(id(90).as_slice()).fetch_one(&h.pool).await?;
            assert!(
                !held,
                "expiry commits while the actor cannot publish vitals"
            );
            if let Some(reservation) = reservation {
                owner
                    .runtime
                    .lock()
                    .await
                    .release_definitely_uncommitted_spell_batch(&reservation)
                    .map_err(debug)?;
            } else {
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .enchanting_pending
                    .retain(|pending| *pending != (actor, session));
            }
            for _ in 0..2 {
                owner.clock_origin -= Duration::from_secs(1);
                owner.tick_timed_items().await;
                assert_eq!(ring_effects(&owner, &deps).await?, (false, 0, 27, 8));
            }
            return Ok(());
        }
        Case::ZoneBoundary => {
            owner.clock_origin -= Duration::from_secs(7);
            let (result, _) = owner
                .ordinary_paced_step(
                    actor,
                    session,
                    oteryn_protocol_oteryn::world_spatial::StepDirection::West,
                    None,
                    None,
                )
                .await;
            assert!(
                result.moved_to.is_some(),
                "fixture must move onto the PZ tile"
            );
            assert_eq!(
                crate::gameplay_transport::actor_spell::qualified_protection_zone(
                    &mut *owner.runtime.lock().await,
                    owner.movement_cells(),
                    actor,
                ),
                Some(true),
            );
            owner.tick_timed_items().await;
            assert_eq!(
                ring_effects(&owner, &deps).await?.2,
                27,
                "credit the wear before the zone crossing"
            );
            return Ok(());
        }
        Case::ReadmittedStatus => {
            owner
                .states
                .timed_items
                .sessions
                .lock()
                .await
                .remove(&session);
            let equipment = owner
                .read_fenced_equipment_effects(fence.item_fence())
                .await
                .map_err(debug)?;
            {
                let runtime = owner.runtime.lock().await;
                owner
                    .install_admission_suppressions(&runtime, actor, session, equipment)
                    .await
                    .ok_or("equipment publication")?;
                let states = owner.states.spell_states.lock().await;
                let page = crate::gameplay_transport::condition_status::snapshot(
                    &runtime,
                    &states,
                    actor,
                    session,
                    owner.owner_now().get(),
                )
                .ok_or("mechanics status snapshot")?;
                assert!(
                    page.conditions
                        .iter()
                        .any(|effect| effect.icon == 9 && effect.remaining_ms > 0)
                );
            }
            return assert_ring_status(&owner, admitted, invisible, full_ms).await;
        }
        Case::RecoveredWearStart => {
            return Err("wear recovery must precede the normal equip".into());
        }
        Case::Lifecycle => {}
    }
    tick_regen_pulse(&mut owner, &deps).await;
    owner.clock_origin -= Duration::from_secs(54);
    owner.tick_timed_items().await;
    assert!((full_ms - 62_000..=full_ms - 60_000).contains(&budget(h).await?));
    let (_, _, health, mana) = ring_effects(&owner, &deps).await?;
    // The hungry monk also gains one mana on the first six native ticks.
    assert_eq!(
        (health, mana),
        if invisible {
            (25, 1)
        } else {
            (45.min(max_health), 81.min(max_mana))
        }
    );

    assert_eq!(
        move_equipment(&owner, actor, command(admitted, 2), unequip).await,
        EquipmentMoveResult::Committed
    );
    let paused = budget(h).await?;
    assert_eq!(ring_effects(&owner, &deps).await?, (false, 0, health, mana));
    assert_eq!(
        owner.lose_control(admitted, Duration::ZERO, None).await,
        ControlLossResult::Recorded
    );
    owner.clock_origin -= Duration::from_secs(30);
    owner.tick_timed_items().await;
    let token = RecoverySource::token(2)?;
    let resumed = owner
        .resume(ResumeAttempt {
            game_session_id: session,
            recovery_material: token.as_bytes(),
            last_applied_server_sequence: admitted.continuity.server_sequence,
            transport: AuthenticatedTransportRefV1::decode(&[2; 16]).map_err(debug)?,
            supported_capabilities: &[],
        })
        .await
        .map_err(debug)?;
    assert_eq!(resumed.continuity.connection_generation, 2);
    assert_eq!(budget(h).await?, paused);
    let _ = owner.tick_spell_periodic(actor, session).await;
    assert_eq!(ring_effects(&owner, &deps).await?, (false, 0, health, mana));

    assert_eq!(
        move_equipment(&owner, actor, command(resumed, 3), equip).await,
        EquipmentMoveResult::Committed
    );
    assert_eq!(
        budget(h).await?,
        paused,
        "re-equip retains the partial budget"
    );
    tick_regen_pulse(&mut owner, &deps).await;
    owner.clock_origin -= Duration::from_secs(54);
    owner.tick_timed_items().await;
    let (hidden, regen, health, mana) = ring_effects(&owner, &deps).await?;
    assert_eq!(
        (hidden, regen, health, mana),
        if invisible {
            (true, 0, 26, 3)
        } else {
            (false, 1, 66.min(max_health), 163.min(max_mana))
        }
    );
    let remaining = budget(h).await?;
    assert!((paused - 62_000..=paused - 60_000).contains(&remaining));

    owner.clock_origin -= Duration::from_millis(u64::try_from(remaining - 2_000)?);
    owner.tick_timed_items().await;
    assert_eq!(ring_effects(&owner, &deps).await?.0, invisible);
    assert!((1..=2_000).contains(&budget(h).await?));
    owner.clock_origin -= Duration::from_secs(3);
    owner.tick_timed_items().await;
    let final_pulses = u32::try_from(paused / 6_000 - 10)?;
    let (health, mana) = if invisible {
        (health, mana)
    } else {
        (
            (health + 2 * final_pulses).min(max_health),
            (mana + 8 * final_pulses).min(max_mana),
        )
    };
    assert_eq!(ring_effects(&owner, &deps).await?, (false, 0, health, mana));
    assert!(
        observe_character_equipment(&owner, session)
            .await
            .ok_or("expired view")?
            .is_empty()
    );
    let lifecycle: i16 = sqlx::query_scalar(
        "SELECT lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(id(90).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(lifecycle, 2, "expiry consumes the ring");
    Ok(())
}

async fn assert_ring_status(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: super::AdmittedSession,
    invisible: bool,
    full_ms: i64,
) -> TestResult {
    use crate::gameplay_transport::capabilities::{OfferedCapability, SelectedCapabilities};
    use oteryn_protocol_oteryn::condition_status::{
        CAPABILITY_CONDITION_STATUS_V1, COMMAND_TYPE_CONDITION_STATUS_QUERY, ConditionStatus,
    };

    let fence = admitted.item_fence.ok_or("condition status item fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("condition status actor")?,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        &[OfferedCapability {
            id: CAPABILITY_CONDITION_STATUS_V1,
            requires: &[],
        }],
        &[CAPABILITY_CONDITION_STATUS_V1],
    )
    .ok_or("condition-status capability")?;
    let command_id = admitted.continuity.next_command_id;
    let (status, bytes) = super::cyclopedia_fixtures::support::exchange(
        owner,
        &mut admitted,
        command_id,
        COMMAND_TYPE_CONDITION_STATUS_QUERY,
        &[],
    )
    .await?;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    let page = ConditionStatus::decode(&bytes).ok_or("condition status decode")?;
    let effect = page
        .conditions
        .iter()
        .find(|effect| effect.icon == if invisible { 9 } else { 21 })
        .ok_or("the worn ring effect is missing from condition status")?;
    assert!((full_ms - 2_000..=full_ms).contains(&effect.remaining_ms));
    Ok(())
}

async fn tick_one_second(owner: &mut ComposedFreshAdmission<'_, '_, '_>, deps: &Dependencies) {
    owner.clock_origin -= Duration::from_secs(1);
    owner.tick_timed_items().await;
    let _ = owner
        .tick_spell_periodic(deps.actor, deps.admitted.game_session_id)
        .await;
}

async fn tick_regen_pulse(owner: &mut ComposedFreshAdmission<'_, '_, '_>, deps: &Dependencies) {
    for _ in 0..6 {
        tick_one_second(owner, deps).await;
    }
}

async fn ring_effects(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
) -> TestResult<(bool, usize, u32, u32)> {
    let runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let state = states
        .get(&runtime, deps.actor, deps.admitted.game_session_id)
        .ok_or("actor spell state")?;
    let vitals = state.vitals();
    Ok((
        state
            .owned_invisible_at(owner.owner_now().get())
            .map_err(debug)?,
        state.equipped_regeneration().len(),
        vitals.health,
        vitals.mana,
    ))
}

async fn budget(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT remaining_ms FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(id(90).as_slice()).fetch_one(&h.pool).await?)
}

async fn seed_custody(
    h: &Harness,
    items: &[(&str, &str); 3],
    food_revision: Option<&str>,
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let seeded = [70, 90, 94]
        .into_iter()
        .zip(items.iter().copied())
        .chain(food_revision.map(|revision| (96, ("oteryn:item.tibia.i29409", revision))));
    for (seed, (key, revision)) in seeded {
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
            .bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(key).bind(revision).bind(id(seed + 1).as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(42).as_slice()).bind(id(72).as_slice()).execute(&mut *tx).await?;
    for (ordinal, seed) in [(1, 90), (2, 94)]
        .into_iter()
        .chain(food_revision.map(|_| (3, 96)))
    {
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($6,'hex')::uuid)")
            .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(ordinal).bind(id(seed + 2).as_slice()).execute(&mut *tx).await?;
    }
    sqlx::query("INSERT INTO game_character_build_state(character_id,vocation,magic_level,mana_spent,fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries,committed_character_revision,last_build_occurrence_id) VALUES(encode($1,'hex')::uuid,'monk',10,0,10,0,10,0,10,0,10,0,10,0,10,0,10,0,2,encode($2,'hex')::uuid) ON CONFLICT(character_id) DO UPDATE SET vocation='monk'")
        .bind(id(41).as_slice()).bind(id(30).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

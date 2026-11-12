//! Source-bound bank population through NPC_TALK and the fenced bank writer.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{
    ChannelContentPin, MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements,
};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::sync::Arc;

#[test]
fn s10_healing_postgres_restores_only_emergency_health() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "S10 healing PostgreSQL thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED S10 healing duration={:?}",
        started.elapsed()
    );
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        reset();
        let h = Harness::create_with_admission_seed(admin, "s10_healing", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) = activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active content")?;
        let deps = Dependencies::with_content(
            ChannelContentPin::from_activation(
                world,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                pin.frame_binding().digest(),
                pin.qualified_room().map_revision_digest(),
                (2, 0, 7),
            ),
            pin.qualified_room().clone(),
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        deps.initialize(&owner).await;
        let catalogue = NpcDataCatalogue::load(
            std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
            std::ffi::OsStr::new("world"),
            npc_catalogue_preproduction_limits(),
        )
        .map_err(debug)?;
        let model = catalogue.service_model_for_project().map_err(debug)?;
        owner
            .states
            .chat
            .install_npc_talk(Arc::new(NpcTalkBook::from_model(&model)));
        let session = deps.admitted.game_session_id;
        owner
            .states
            .chat
            .admit(session, "Fixture Hero", owner.owner_now());
        owner.states.chat.set_npc_service(session);
        let npc = {
            let mut runtime = deps.runtime.lock().await;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let at = runtime
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position();
            let placed = runtime
                .place_npcs(
                    &NpcPlacementTable {
                        npcs: vec![NpcPlacements {
                            key: "oteryn:npc.norf".into(),
                            placements: vec![NpcPlacement {
                                x: u16::try_from(at.x + 1)?,
                                y: u16::try_from(at.y)?,
                                floor: -i8::try_from(at.floor)?,
                                direction: MovementFacing::South,
                            }],
                        }],
                    },
                    |_| true,
                    true,
                )
                .map_err(debug)?;
            EntityRef {
                identity: placed.actors[0].placement_identity(),
                generation: placed.actors[0].actor_local_generation(),
            }
        };
        super::npc_paid_travel::talk(&owner, &deps, npc, 1, "hi").await?;
        use crate::domain::condition::{ConditionType, ConditionValues, DotElement, StatusKind};
        use oteryn_gameplay_foundation::foundation::condition::{
            ApplicationFacts, ConditionDefinition, DamageSchedule, DamageSegment,
        };
        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes([91; 32]);
        {
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            crate::gameplay_transport::actor_spell::tests::wound(
                &mut states,
                deps.actor,
                session,
                10,
            );
            let mut definitions = Vec::new();
            for (key, element) in [
                ("s10.fire", DotElement::Fire),
                ("s10.poison", DotElement::Poison),
            ] {
                definitions.push(
                    ConditionDefinition::new_damage_schedule(
                        key,
                        1,
                        element,
                        DamageSchedule::Fixed {
                            delayed: true,
                            segments: vec![DamageSegment {
                                count: 100,
                                interval_ms: 1000,
                                amount: 1,
                            }],
                        },
                    )
                    .ok_or("condition")?,
                );
            }
            definitions.push(
                ConditionDefinition::new(
                    "s10.drunk",
                    1,
                    ConditionValues::TimedStatus {
                        kind: StatusKind::Drunk,
                        duration_ms: 100_000,
                    },
                )
                .ok_or("status")?,
            );
            let facts = ApplicationFacts {
                now: owner.owner_now().get(),
                base_speed: 220,
                mana_shield_capacity: 0,
                target_reentry_protected: false,
                source_reentry_protected: false,
                target_is_player: true,
                decision_root: &root,
                occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(
                    [92; 16],
                ),
            };
            let next = states
                .get(&runtime, deps.actor, session)
                .ok_or("vitals")?
                .stage_owned_test_conditions(&definitions, &facts)
                .ok_or("stage conditions")?;
            assert!(states.commit(&runtime, deps.actor, session, next));
        }
        for (command, health, fire, poison) in [
            (2, 10, false, true),
            (3, 10, false, false),
            (4, 40, false, false),
            (5, 40, false, false),
        ] {
            owner.clock_origin -= std::time::Duration::from_secs(1);
            super::npc_paid_travel::talk(&owner, &deps, npc, command, "heal").await?;
            let runtime = deps.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states.get(&runtime, deps.actor, session).ok_or("vitals")?;
            assert_eq!(state.vitals().health, health, "command={command}");
            let has = |kind| {
                state
                    .owned_conditions()
                    .instances()
                    .iter()
                    .any(|instance| instance.definition().condition_type() == kind)
            };
            assert_eq!(has(ConditionType::DamageOverTime(DotElement::Fire)), fire);
            assert_eq!(
                has(ConditionType::DamageOverTime(DotElement::Poison)),
                poison
            );
            assert!(
                has(ConditionType::Status(StatusKind::Drunk)),
                "unrelated condition must survive"
            );
        }
        {
            let mut states = owner.states.spell_states.lock().await;
            crate::gameplay_transport::actor_spell::tests::wound(
                &mut states,
                deps.actor,
                session,
                10,
            );
        }
        owner.clock_origin -= std::time::Duration::from_secs(1);
        super::npc_paid_travel::talk(&owner, &deps, npc, 4, "heal").await?;
        {
            let runtime = deps.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert_eq!(
                states
                    .get(&runtime, deps.actor, session)
                    .ok_or("vitals")?
                    .vitals()
                    .health,
                10,
                "retry cannot heal a subsequent injury"
            );
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

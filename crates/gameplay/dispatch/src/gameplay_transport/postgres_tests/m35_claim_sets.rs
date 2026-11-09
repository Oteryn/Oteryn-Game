//! Crystal 201189f8 chest.lua:917-931: both independent Alawar claims, in either order.
use super::fixture::{Harness, debug, id, run};
use super::quest_callbacks::{source_world_choices, wire_use_targets_retained};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[test]
fn m35_alawar_dispatch_completes_only_on_last_chest() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            for reverse in [false, true] {
                scenario(reverse, false).map_err(|e| e.to_string())?;
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "M35 dispatch scenario panicked")??;
    println!("M35_ALAWAR_POSTGRES_CASE_COMPLETED");
    Ok(())
}
// Crystal 201189f8 chest.lua:2131-2161: these placements share ChestsWine storage.
#[test]
fn m35_dreamer_wine_alternatives_do_not_complete_other_required_groups() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            for reverse in [false, true] {
                scenario(reverse, true).map_err(|e| e.to_string())?;
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "M35 wine dispatch scenario panicked")??;
    println!("M35_WINE_ALTERNATIVES_POSTGRES_CASE_COMPLETED");
    Ok(())
}
fn definition(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}
fn scenario(reverse: bool, wine: bool) -> TestResult {
    let (chest, appearance, quest) = if wine {
        (
            (32749, 32341, -14),
            2469,
            "oteryn:quest.dreamer_s_challenge_quest",
        )
    } else {
        (
            (32105, 31567, -9),
            2472,
            "oteryn:quest.alawar_s_vault_quest",
        )
    };
    let choices: &[(u16, u16)] = if wine {
        &[(32749, 6197), (32751, 6198), (32753, 6199), (32755, 6200)]
    } else {
        &[(32105, 6045), (32109, 6046)]
    };
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "m35_claim_sets", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        if wine {
            for (key, weight) in [
                ("oteryn:item.tibia.i6498", 150),
                ("oteryn:item.tibia.i3602", 400),
                ("oteryn:item.tibia.i2874", 180),
            ] {
                let facts = generation_item_facts(native, &definition(key)).map_err(debug)?;
                assert_eq!(facts.weight, Some(weight));
                assert_ne!(
                    facts.stack,
                    crate::durability::item_transfer::ItemStackClass::Unknown
                );
            }
        }
        let world = source_world_choices(chest, choices, appearance)?;
        let bound = crate::content::world_reward_claims::bind_embedded_reward_claims(
            world.base().as_ref(),
            &|_| true,
        )?;
        for claim in &bound.served {
            assert!(
                claim
                    .placements
                    .iter()
                    .all(|p| p.placement.quest_transition.is_some()),
                "source chest must bind a claim-set transition"
            );
        }
        assert_eq!(
            bound.served.len(),
            if wine { 1 } else { 2 },
            "left out: {:?}; unbound: {:?}",
            bound.left_out,
            bound.unbound
        );
        let catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let at = MovementLocalPosition {
            x: i32::from(chest.0) - 1,
            y: i32::from(chest.1),
            floor: -i16::from(chest.2),
        };
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (at.x, at.y, at.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, at)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence,
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence,
                &NativeItemWeights(native),
                &crate::durability::starter_grant::StarterTemplate {
                    definition: definition("oteryn:item.tibia.i2854"),
                    ..crate::durability::starter_grant::StarterTemplate::main()
                },
            )
            .await
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);

        let channel_states = std::sync::Arc::new(
            crate::gameplay_transport::channel_states::ChannelSessionStates::default(),
        );
        let clock_origin = std::time::Instant::now();
        let mut position = at;
        let mut command = 1;
        for index in 0..choices.len() {
            let position_index = if reverse {
                choices.len() - index - 1
            } else {
                index
            };
            let x = choices[position_index].0;
            // Rebuild the caller while retaining the Channel's command-stream truth.
            // Drop its quest cache so admission must reload the durable claim tracks.
            let mut owner = deps.owner(&h, &authority);
            owner.states = std::sync::Arc::clone(&channel_states);
            owner.clock_origin = clock_origin;
            owner
                .states
                .quest_sessions
                .lock()
                .map_err(|_| "quest cache")?
                .remove(&admitted.game_session_id);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.quest_catalogue = catalogue.clone();
            owner.admit_progression_session(&mut admitted).await;
            owner.admit_quest_session(&admitted).await;
            deps.initialize(&owner).await;
            walk(
                &owner,
                &mut admitted,
                &mut command,
                &mut position,
                MovementLocalPosition {
                    x: i32::from(x) - 1,
                    ..at
                },
            )
            .await?;
            let key = oteryn_world_bundle::bundle::placement_key(chest.2, x, chest.1, 1)
                .ok_or("placement")?;
            let mut placement = world.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let target = WorldObjectTarget {
                placement,
                expected_revision: 0,
            };
            let first_result = if wine && index > 0 {
                UseDisposition::NothingToUse
            } else {
                UseDisposition::Committed
            };
            assert_eq!(
                wire_use_targets_retained(
                    &owner,
                    &mut admitted,
                    &[
                        (command, &target),
                        (command, &target),
                        (command + 1, &target)
                    ]
                )
                .await?,
                [
                    (CommandStatus::Accepted, first_result),
                    (CommandStatus::DuplicateReplay, first_result),
                    (CommandStatus::Accepted, UseDisposition::NothingToUse)
                ]
            );
            command += 2;
            let counts: (i64, i64, i64, i64) = sqlx::query_as(
                "SELECT (SELECT count(*) FROM game_reward_claims), \
                 (SELECT count(*) FROM game_character_quest_receipts), \
                 (SELECT count(*) FROM game_character_quest_obligations WHERE state='CONSUMED'), \
                 (SELECT count(*) FROM game_character_quest_obligations WHERE state <> 'CONSUMED')",
            )
            .fetch_one(&h.pool)
            .await?;
            let claims = if wine { 1 } else { index as i64 + 1 };
            assert_eq!(counts, (claims, claims, claims, 0));
            let snapshot = h
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            let state = snapshot.states().get(quest).ok_or("quest state")?;
            assert_eq!(state.completed, !wine && index == 1);
            if wine {
                let fluids: Vec<(i64, i16, bool, bool)> = sqlx::query_as(
                    "SELECT i.quantity, f.fluid_subtype, m.root_transaction_id=r.transaction_id, \
                     f.last_transaction_id=m.transaction_id FROM game_reward_claim_mint_receipts r \
                     JOIN game_item_nested_entries e ON e.parent_item_instance_id=r.item_instance_id \
                     JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id \
                     JOIN game_item_fluid_states f ON f.item_instance_id=i.item_instance_id \
                     JOIN game_item_fluid_reward_mints m ON m.item_instance_id=i.item_instance_id \
                     WHERE i.definition_production_key='oteryn:item.tibia.i2874' LIMIT 2")
                    .fetch_all(&h.pool).await?;
                assert_eq!(fluids, vec![(1, 1, true, true)]);
                let children: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_item_nested_entries")
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(children, 3);
                assert_eq!(
                    snapshot.tracks().get(
                        "oteryn:quest-progress/quest/u7_9/dreamers_challenge/chests_wine/claimed"
                    ),
                    Some(&1)
                );
                assert_eq!(
                    snapshot
                        .tracks()
                        .iter()
                        .filter(|(key, _)| key.ends_with("/claimed"))
                        .count(),
                    1
                );
            }
        }
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        Ok(())
    })
}
async fn walk(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    stream: &mut crate::gameplay_transport::connection::AdmittedSession,
    command: &mut u64,
    position: &mut MovementLocalPosition,
    destination: MovementLocalPosition,
) -> TestResult {
    use oteryn_protocol_oteryn::world_spatial::{
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
        encode_step_intent,
    };
    assert_eq!(position.floor, destination.floor);
    while *position != destination {
        let (direction, dx, dy) = if position.x < destination.x {
            (StepDirection::East, 1, 0)
        } else if position.x > destination.x {
            (StepDirection::West, -1, 0)
        } else if position.y < destination.y {
            (StepDirection::South, 0, 1)
        } else {
            (StepDirection::North, 0, -1)
        };
        let reply = super::quest_world_callbacks::wire(
            owner,
            stream,
            *command,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(direction),
        )
        .await?;
        assert_eq!(
            decode_step_result(&reply).map_err(debug)?,
            StepDisposition::Moved
        );
        *command += 1;
        position.x += dx;
        position.y += dy;
    }
    Ok(())
}

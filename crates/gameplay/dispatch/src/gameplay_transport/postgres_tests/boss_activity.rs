//! A real command 178 moves the unique native player and its live spell state,
//! commits the durable Instance session, then crosses domain 99 before the new
//! owner's initial snapshot. All room/boss values here are isolated test data.
use super::fixture::{self, Harness, debug, run};
use super::{Dependencies, TestResult, reset};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{
    ChannelRuntimeV1, CharacterLease, CommandStatus, MessageType, MovementLocalPosition,
    RuntimeScopeRefV1, decode_wire_envelope,
};
use crate::gameplay_transport::activity_runtime::{ActivityRuntimeAssets, ActivityTemplateKey};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, IDLE_LIVENESS, serve_admitted,
};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use crate::gameplay_transport::{ComposedFreshAdmission, SeamRooms, item_fence_of};
use oteryn_protocol_oteryn::activity_scope::{
    ActivityScope, ActivityScopeSnapshot, STATE_DOMAIN_ACTIVITY_SCOPE, decode_activity_scope,
};
use oteryn_protocol_oteryn::boss_encounter::{
    self as wire, BossActivityRefusal, BossActivityReply,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, decode_command_result, decode_snapshot_chunk, encode_client_command,
};
use tokio::io::AsyncWriteExt;

const BOSS: &str = "oteryn:boss.native_activity_fixture";
const CREATURE: &str = "fixture:activity_boss";
const TEMPLATE: &str = "oteryn:room.native_activity_fixture";

#[path = "boss_activity/recovery.rs"]
mod recovery;
#[path = "boss_activity/rewards.rs"]
mod rewards;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Entry,
    Rewards,
    LateRewards,
    Recovery,
    RecoveryCancelled,
    RecoveryProtection,
    CapabilityMismatch,
}

#[test]
fn boss_activity_postgres_real_wire_native_entry_and_scope_barrier() -> TestResult {
    run_scenario(Case::Entry)
}

#[test]
fn boss_activity_postgres_real_instance_native_boss_death_and_reward_custody() -> TestResult {
    run_scenario(Case::Rewards)
}

#[test]
fn boss_activity_postgres_late_native_death_drains_and_returns_without_rewards() -> TestResult {
    run_scenario(Case::LateRewards)
}

#[test]
fn boss_activity_postgres_authenticated_recovery_keeps_exact_native_state() -> TestResult {
    run_scenario(Case::Recovery)
}

#[test]
fn boss_activity_postgres_cancelled_committed_recovery_returns_and_releases() -> TestResult {
    run_scenario(Case::RecoveryCancelled)
}

#[test]
fn boss_activity_postgres_recovery_protection_survives_entry_loss_and_return() -> TestResult {
    run_scenario(Case::RecoveryProtection)
}

#[test]
fn boss_activity_postgres_capability_mismatch_returns_before_terminal_release() -> TestResult {
    run_scenario(Case::CapabilityMismatch)
}

fn run_scenario(case: Case) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("BOSS_ACTIVITY POSTGRES_CASE_SKIPPED: no admin URL");
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "boss activity fixture panicked")??;
    Ok(())
}

fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        let with_rewards = matches!(case, Case::Rewards | Case::LateRewards);
        let late = case == Case::LateRewards;
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "native_activity_wire", true, false).await?;
        let evidence = super::house_transfer::RecoverySource::start(&h).await?;
        super::house_transfer::prepare_owned_session(&h).await?;
        let world = fixture::fence(1)?.runtime_scope.world_id();
        let (controller, source_pin) = if with_rewards {
            crate::content::native_gameplay::test_support::activated_reward_boss_fixture(
                world, None,
            )
        } else {
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None)
        };
        let source_room = source_pin.qualified_room().clone();
        let pin = source_pin.into_channel_pin();
        let content_pin = pin.clone();
        let deps = Dependencies::with_content(
            pin,
            source_room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.evidence = &evidence.evidence;
        owner.active_generation = controller.active();
        owner.rooms = SeamRooms::new(Some(&deps.room), Some(&deps.room));
        let (boss_key, policy) = if with_rewards {
            rewards::source_boss(&owner)?
        } else {
            (BOSS.to_owned(), fixture_policy())
        };
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = item_fence_of(
            FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(admitted.game_session_id)
                .await?
                .0,
        );
        let requested = if case == Case::CapabilityMismatch {
            vec![
                wire::CAPABILITY_BOSS_ENCOUNTER_V1,
                oteryn_protocol_oteryn::boss_difficulty::CAPABILITY_BOSS_DIFFICULTY_V1,
            ]
        } else {
            vec![wire::CAPABILITY_BOSS_ENCOUNTER_V1]
        };
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, &requested)
                .ok_or("boss capability missing")?;
        let fence = admitted.item_fence.ok_or("source fence")?;
        if with_rewards {
            super::item_look::seed_main_backpack(&h, &[]).await?;
            sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT(character_id) DO NOTHING")
                .bind(fence.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
        }
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    admitted.game_session_id,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            runtime
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
        }
        deps.initialize(&owner).await;
        owner
            .register_current_runtime()
            .await
            .map_err(|_| "register source owner")?;
        owner
            .activity_runtimes
            .remember(admitted)
            .map_err(|_| "remember source")?;
        let inherited_protection = if case == Case::RecoveryProtection {
            Some(recovery::prepare_source_reentry(&h, &owner, &mut admitted).await?)
        } else {
            None
        };
        // The protection case performs a real Channel recovery first. Every
        // subsequent request must use its actual fresh connection generation.
        let fence = admitted.item_fence.ok_or("current source fence")?;
        let before_state = {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let state = states
                .get(&runtime, deps.actor, admitted.game_session_id)
                .ok_or("source live state")?;
            (state.revision(), state.vitals(), state.saved_vitals())
        };
        let source_cell = owner
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();

        issue_content_and_reset(&h, &content_pin).await?;
        let instance = super::activity_runtime::allocate_ready_instance(&h, 88).await?;
        let assignment = h
            .root
            .read_runtime_scope_assignment(instance)
            .await
            .map_err(debug)?
            .ok_or("allocated Instance")?;
        // Two independent test activation invocations produce equal qualified
        // content. Production uses the consumed activity-template activation API.
        let (_instance_controller, instance_pin) = if with_rewards {
            crate::content::native_gameplay::test_support::activated_reward_boss_fixture(
                world, None,
            )
        } else {
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None)
        };
        let instance_room = instance_pin.qualified_room().clone();
        let pin = instance_pin.into_channel_pin();
        assert_eq!(
            pin.server_artifact_digest(),
            content_pin.server_artifact_digest()
        );
        let mut runtime = ChannelRuntimeV1::from_committed_scope_assignment(
            instance,
            h.node.fact().node_id(),
            h.node.fact().registration_revision(),
            assignment.ownership_generation,
            assignment.source_revision,
            &assignment.decision_identity,
            4,
            pin.clone(),
        )
        .map_err(debug)?;
        let reference =
            crate::world_runtime::ReferenceContentGeneration::from_content(instance_room.door())
                .map_err(debug)?;
        let generation =
            crate::foundation::ScopeOwnershipGeneration::new(assignment.ownership_generation)
                .map_err(debug)?;
        let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_test(
            instance, generation, reference,
        );
        let door = crate::world_runtime::bind_native_entry_door(
            instance_room.door(),
            &door_fence,
            instance,
            generation,
        )
        .map_err(debug)?;
        let start = instance_room.entry_start();
        let entry = MovementLocalPosition {
            x: start.x,
            y: start.y,
            floor: start.floor,
        };
        let cells = instance_room.movement_cells();
        let boss_cell = (-8_i32..=8)
            .flat_map(|dy| (-8_i32..=8).map(move |dx| (dx, dy)))
            .map(|(dx, dy)| MovementLocalPosition {
                x: start.x + dx,
                y: start.y + dy,
                floor: start.floor,
            })
            .find(|cell| {
                *cell != entry
                    && crate::gameplay_transport::respawn_cell_admissible(
                        cells.index(),
                        cells.scope(),
                        world,
                        pin.server_artifact_digest(),
                        door.blocking_cells(),
                        *cell,
                    )
            })
            .ok_or("a second actual qualified walkable cell")?;
        runtime
            .install_companion_policies(
                crate::foundation::CompiledCreaturePolicies::from_active_artifact(
                    pin.server_artifact_digest(),
                    vec![policy.clone()],
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        runtime
            .realize_native_qualification_spawn(pin, &policy.definition_key, boss_cell)
            .map_err(debug)?;
        sqlx::query("INSERT INTO game_boss_room_catalogue(boss_key,content_revision,boss_creature_key,minimum_players,maximum_players,minimum_level,cooldown_seconds,time_limit_seconds,lever_positions,room_template_key,reward_key,enabled,template_revision,entry_positions,exit_position,boss_position,after_kill_seconds) VALUES($1,'content-1',$2,1,1,1,20,$8,$3,$4,'fixture:reward',true,'fixture-r1',$5,$6,$7,1)")
            .bind(&boss_key).bind(&policy.definition_key).bind(serde_json::json!([cell_json(source_cell)]))
            .bind(TEMPLATE).bind(serde_json::json!([cell_json(entry)]))
            .bind(cell_json(source_cell)).bind(cell_json(boss_cell)).bind(if late { 1_i32 } else { 900 }).execute(&h.pool).await?;

        let (_, refused, switch) = exchange(&owner, &mut admitted, &boss_key, false).await?;
        assert_eq!(
            refused,
            BossActivityReply::Refused(BossActivityRefusal::ContentUnavailable)
        );
        assert!(switch.is_none());
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            source_cell
        );
        owner
            .register_activity_runtime(
                ActivityTemplateKey {
                    key: TEMPLATE.into(),
                    revision: "fixture-r1".into(),
                },
                &assignment,
                runtime,
                door,
                ActivityRuntimeAssets {
                    movement_cells: cells,
                    rooms: SeamRooms::new(Some(&instance_room), Some(&instance_room)),
                },
            )
            .await
            .map_err(|_| "register genuine destination owner")?;

        // Prove fixture readiness independently of the typed wire result. An
        // unavailable catalogue or unready allocation must not be mistaken for
        // the expected pre-registration ContentUnavailable negative case above.
        let handoffs =
            crate::durability::activity_handoff::ActivityHandoffOwner::new(h.root.clone());
        let definition = handoffs
            .read_boss_room(
                &authority,
                &h.node,
                admitted.item_fence.ok_or("current source fence")?,
                &boss_key,
            )
            .await?
            .ok_or("authored fixture catalogue is visible to the current source fence")?;
        let allocations = handoffs
            .ready_allocations(&authority, &h.node, world, None)
            .await?;
        assert!(
            allocations
                .iter()
                .any(|allocation| allocation.scope == instance
                    && allocation.generation.get() == assignment.ownership_generation
                    && allocation.source_revision == assignment.source_revision),
            "allocated Instance remains independently ready: {allocations:?}"
        );
        let registered = owner
            .activity_runtimes
            .get(instance, assignment.ownership_generation)
            .ok_or("registered Instance")?;
        {
            let runtime = registered.runtime.lock().await;
            assert!(
                crate::gameplay_transport::native_activity::boss_room_native_ready(
                    &runtime,
                    &definition
                ),
                "the fresh room contains its exact authored living boss before entry"
            );
        }
        let old = admitted;
        let (command, reply, barrier) = exchange(&owner, &mut admitted, &boss_key, true).await?;
        let BossActivityReply::Accepted {
            handoff,
            deadline_unix_ms: Some(_),
        } = reply
        else {
            return Err(format!("expected real entry: {reply:?}").into());
        };
        let barrier = barrier.ok_or("scope barrier missing")?;
        assert_eq!(barrier.handoff, handoff);
        assert_eq!(barrier.source_session, *old.game_session_id.as_bytes());
        assert_eq!(barrier.session, *admitted.game_session_id.as_bytes());
        assert_ne!(old.game_session_id, admitted.game_session_id);
        let RuntimeScopeRefV1::Instance {
            world_id,
            instance_id,
        } = instance
        else {
            return Err("Instance expected".into());
        };
        assert_eq!(
            barrier.scope,
            ActivityScope::Instance {
                world: *world_id.as_bytes(),
                instance: instance_id
            }
        );
        assert_eq!(barrier.connection_generation, 1);
        assert_eq!(barrier.scope_generation, assignment.ownership_generation);
        assert_eq!(
            barrier.lease_generation,
            fence.character_lease_generation + 1
        );
        assert_eq!(
            barrier.selection.as_slice(),
            old.continuity.selected_capabilities.as_slice()
        );
        assert_ne!(
            admitted
                .controller
                .ok_or("destination controller")?
                .transport,
            old.controller.ok_or("source controller")?.transport
        );
        assert!(
            owner
                .runtime
                .lock()
                .await
                .player_control_facts(deps.actor, old.game_session_id)
                .is_err()
        );
        let destination = owner
            .resolve_runtime_view(admitted)
            .await
            .map_err(|_| "resolve Instance owner")?
            .ok_or("owner actually changed")?;
        let next_actor = admitted.runtime_actor.ok_or("destination actor")?;
        {
            let runtime = destination.runtime.lock().await;
            assert_eq!(runtime.binding().runtime_scope(), instance);
            assert_eq!(
                runtime
                    .read_actor_position(next_actor)
                    .map_err(debug)?
                    .position(),
                entry
            );
            let states = destination.states.spell_states.lock().await;
            let state = states
                .get(&runtime, next_actor, admitted.game_session_id)
                .ok_or("moved live state")?;
            assert_eq!(
                (state.revision(), state.vitals(), state.saved_vitals()),
                before_state
            );
        }
        let store = FreshAdmissionStore::from_root(h.root.clone());
        assert_eq!(
            store
                .current_session_at(old.game_session_id)
                .await?
                .0
                .session_state(),
            crate::foundation::GameSessionState::Terminal
        );
        assert_eq!(
            item_fence_of(store.current_session_at(admitted.game_session_id).await?.0),
            admitted.item_fence
        );
        let replay = owner
            .retry_native_activity(old, command, &boss_key)
            .await
            .map_err(|_| "same attempt replay")?
            .ok_or("retained attempt")?;
        let crate::gameplay_transport::native_activity::NativeActivityOutcome::Committed(replayed) =
            replay
        else {
            return Err("replay refused".into());
        };
        assert_eq!(replayed.handoff_id, handoff);
        owner
            .publish_activity_commit(&replayed)
            .map_err(|_| "idempotent publication")?;
        assert!(
            owner
                .activity_runtimes
                .pending_switch(old)
                .map_err(|_| "pending read")?
                .is_none(),
            "replay must not resurrect a delivered barrier"
        );
        let counts:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_activity_handoff_groups),(SELECT count(*) FROM game_boss_room_activities)").fetch_one(&h.pool).await?;
        assert_eq!(counts, (1, 1));
        let mut forged = admitted;
        forged
            .item_fence
            .as_mut()
            .ok_or("fence")?
            .scope_ownership_generation =
            crate::foundation::ScopeOwnershipGeneration::new(assignment.ownership_generation + 1)
                .map_err(debug)?;
        assert!(owner.resolve_runtime_view(forged).await.is_err());
        if with_rewards {
            rewards::settle_and_collect(&h, &owner, &destination, admitted, &policy, late).await?;
            if late {
                eprintln!("BOSS_ACTIVITY_LATE_REWARD_TIMEOUT POSTGRES_CASE_COMPLETED");
            } else {
                eprintln!("BOSS_ACTIVITY_INSTANCE_REWARD_CUSTODY POSTGRES_CASE_COMPLETED");
            }
        }
        match case {
            Case::Recovery => recovery::assert_recovery(&h, &owner, old, admitted).await?,
            Case::RecoveryCancelled => {
                recovery::assert_cancelled_recovery(&h, &owner, old, admitted).await?
            }
            Case::RecoveryProtection => {
                recovery::assert_inherited_protection(
                    &h,
                    &owner,
                    admitted,
                    inherited_protection.ok_or("source protection")?,
                )
                .await?
            }
            Case::CapabilityMismatch => {
                recovery::assert_capability_mismatch(&h, &owner, admitted).await?
            }
            _ => {}
        }
        eprintln!("BOSS_ACTIVITY_NATIVE_ENTRY POSTGRES_CASE_COMPLETED");
        eprintln!("BOSS_ACTIVITY_SCOPE_BARRIER POSTGRES_CASE_COMPLETED");
        eprintln!("BOSS_ACTIVITY_STATE_AND_REPLAY POSTGRES_CASE_COMPLETED");
        Ok(())
    })
}

fn cell_json(cell: MovementLocalPosition) -> serde_json::Value {
    serde_json::json!({"x":cell.x,"y":cell.y,"floor":cell.floor})
}

fn fixture_policy() -> crate::foundation::CompiledCreaturePolicy {
    crate::foundation::CompiledCreaturePolicy {
        definition_key: CREATURE.into(),
        definition_revision: "fixture-r1".into(),
        display_name: "Activity fixture".into(),
        maximum_health: 37,
        forge_difficulty: None,
        base_speed: 110,
        outfit_look_type: 21,
        object_look_type: None,
        summonable: false,
        convinceable: false,
        mana_cost: None,
        is_familiar: false,
        condition_immunities: vec![],
        defense: Some(1),
        armor: Some(1),
        mitigation: None,
        resistances: vec![],
        damage_immunities: vec![],
        healing_from_damage: vec![],
        preferred_distance: Some(1),
        reward_boss: Some(true),
        blocked_by_nearby_players: false,
        flags: crate::foundation::CreatureFlags {
            attackable: true,
            illusionable: false,
            health_hidden: false,
        },
    }
}

async fn issue_content_and_reset(
    h: &Harness,
    pin: &oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
) -> TestResult {
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = fixture::fence(1)?.runtime_scope
    else {
        return Err("fixture Channel".into());
    };
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice())
        .bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
    // Initial reset fixture follows the existing kill-reward PG harness. Runtime
    // entry never writes a control grant, activation or world-reset row.
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'activity-fixture',$2,$3,$4,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-10')")
        .bind(world_id.as_bytes().as_slice()).bind(pin.server_artifact_digest().as_slice())
        .bind(pin.client_artifact_digest().as_slice()).bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
        .bind(world_id.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    Ok(())
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    boss: &str,
    expect_switch: bool,
) -> TestResult<(u64, BossActivityReply, Option<ActivityScopeSnapshot>)> {
    let before = *admitted;
    let id = before.continuity.next_command_id;
    let body = wire::encode_boss_key(boss).map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(16384);
    let serving = async {
        let end = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        write_frame(
            &mut client,
            &encode_client_command(
                before.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: id,
                    command_type: wire::COMMAND_TYPE_BOSS_LEVER,
                    payload: &body,
                },
            )
            .map_err(debug)?,
        )
        .await?;
        let mut reply = None;
        let mut barrier = None;
        let mut commits_after_barrier = 0;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            match envelope.message_type() {
                MessageType::CommandResult => {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, id);
                    let decoded = wire::decode_boss_activity_reply(result.payload);
                    assert_eq!(
                        result.status,
                        if expect_switch {
                            CommandStatus::Accepted
                        } else {
                            CommandStatus::Rejected
                        },
                        "boss command {id}, session {:?}, generation {}: {decoded:?}; raw payload {:?}",
                        before.game_session_id,
                        before.continuity.connection_generation,
                        result.payload
                    );
                    reply = Some(decoded.map_err(debug)?);
                    if !expect_switch {
                        break;
                    }
                }
                MessageType::SnapshotChunk => {
                    let (_, domains) = decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
                    for domain in domains {
                        if domain.domain_id == STATE_DOMAIN_ACTIVITY_SCOPE {
                            assert!(reply.is_some(), "initiator result precedes scope barrier");
                            let projected = decode_activity_scope(domain.payload).map_err(debug)?;
                            if let Some(prior) = barrier.as_ref() {
                                // The new owner's full initial snapshot includes
                                // its binding too. It is the same projection, not
                                // another source-stream switch or reset.
                                assert!(commits_after_barrier >= 1, "one source-stream barrier");
                                assert_eq!(
                                    prior, &projected,
                                    "initial scope projection agrees with committed barrier"
                                );
                            } else {
                                barrier = Some(projected);
                            }
                        }
                    }
                }
                MessageType::SnapshotCommit if barrier.is_some() => {
                    commits_after_barrier += 1;
                    if commits_after_barrier == 2 {
                        break;
                    }
                }
                _ => {}
            }
        }
        client.shutdown().await?;
        Ok((id, reply.ok_or("command result")?, barrier))
    };
    let (ended, result): (_, TestResult<_>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        fixture::join_two(serving, driving),
    )
    .await?;
    if let ConnectionEnd::AdmittedThenDisconnected(next)
    | ConnectionEnd::AdmittedThenClosed(next, _) = ended
    {
        *admitted = next;
    }
    result
}

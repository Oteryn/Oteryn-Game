//! Exact donor Ahau entry and committed native STEP return, using the shared owner.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, reset};
use crate::content::bosses::activity_template::captured_boss_levers;
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::{boss_encounter as boss, world_spatial as movement};

#[test]
fn m25_ahau_native_step_exit_returns_exact_current_instance_participant_postgres() -> TestResult {
    scenario(false)
}
#[test]
fn m25_ahau_player_push_exit_returns_only_pushed_participant_postgres() -> TestResult {
    scenario(true)
}
fn scenario(push: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("m25-ahau-native-exit", move || {
        run(async |admin| {
            reset();
            let scene = super::srx3_soul_war_world_fixture::source_world(MovementLocalPosition {
                x: 34037,
                y: 31714,
                floor: 10,
            })?;
            let active = scene.controller.active().ok_or("actual active World")?;
            let native = active
                .native_gameplay()
                .ok_or("actual native definitions")?;
            let mitmah = captured_boss_levers()
                .map_err(debug)?
                .into_iter()
                .find(|row| row.creature_key.as_deref() == Some("oteryn:creature.mitmah_vanguard"))
                .ok_or("actual Mitmah source candidate")?;
            scene
                .controller
                .qualify_activity_template(
                    &scene.room,
                    &scene.room,
                    native.training_formula().ok_or("active interpretation")?.content_revision(),
                    &mitmah,
                )
                .map_err(debug)?;
            let candidate = captured_boss_levers()
                .map_err(debug)?
                .into_iter()
                .find(|row| row.creature_key.as_deref() == Some("oteryn:creature.ahau"))
                .ok_or("actual Ahau source candidate")?;
            for [x, y, z] in [
                [34002, 31706, 10],
                [34008, 31696, 10],
                [34008, 31703, 10],
                [34036, 31717, 10],
                [34033, 31714, 10],
                [34034, 31714, 10],
                [34035, 31714, 10],
                [34036, 31714, 10],
                [34037, 31714, 10],
            ] {
                let cells = scene.room.movement_cells();
                assert_eq!(
                    cells.index().lookup(
                        cells.scope(),
                        crate::content::LogicalCell { x, y, z },
                    ),
                    Ok(crate::content::CollisionClass::Walkable),
                    "actual published Ahau source placement {x},{y},{z}"
                );
            }
            let template = scene
                .controller
                .qualify_activity_template(
                    &scene.room,
                    &scene.room,
                    native
                        .training_formula()
                        .ok_or("active interpretation")?
                        .content_revision(),
                    &candidate,
                )
                .map_err(debug)?;
            let exit = template
                .exit_teleporter()
                .ok_or("actual sealed Ahau exit")?;
            assert_eq!(
                (exit.trigger().x, exit.trigger().y, exit.trigger().z),
                (34002, 31706, 10)
            );
            assert_eq!(
                (
                    exit.destination().x,
                    exit.destination().y,
                    exit.destination().z
                ),
                (34036, 31717, 10)
            );
            let deps: Dependencies = scene.dependencies()?;
            let h =
                Harness::create_with_admission_seed(admin, "m25_ahau_native", false, false).await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            owner.active_generation = Some(active);
            owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&scene.room));
            owner
                .activity_runtimes
                .register_template(&template)
                .map_err(|_| "register actual Ahau template")?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            crate::durability::activity_handoff::ActivityHandoffOwner::new(h.root.clone())
                .publish_qualified_boss_rooms(&h.node, std::slice::from_ref(&template))
                .await?;
            let mut admitted = deps.admitted;
            let supported = [4, 6, 7, 10, 11, 15, 17, 18, 23, 30, 92, 117, 385];
            let (offers, scope, generation) = {
                let runtime = owner.runtime.lock().await;
                (crate::gameplay_transport::world_required_capabilities::with_native_map_presentation_offer(
                crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,Some(active),Some(&scene.room),runtime.content_pin()),runtime.binding().runtime_scope(),runtime.binding().scope_generation().get())
            };
            owner.required_capabilities=crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(&[11,18,30],scope,generation,"policy-1",offers).map_err(debug)?;
            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::connection::fresh_capability_selection(
                    &owner, &supported,
                )
                .await
                .ok_or("actual native capability negotiation")?;
            owner
                .required_capabilities
                .authenticate_resume_support(&supported, admitted.continuity.selected_capabilities)
                .map_err(debug)?;
            super::srx3_activity_source_fixture::prepare_player(&h, &owner, &mut admitted, 1999)
                .await?;
            let peer = if push {
                Some(super::m25_ahau_push::prepare_peer_and_party(&h, &owner, &mut admitted).await?)
            } else {
                None
            };
            let source = admitted;
            let instance = super::activity_runtime::allocate_ready_instance(&h, 237).await?;
            let payload = boss::encode_boss_key(template.boss_key()).map_err(debug)?;
            let mut entry_frames = Vec::new();
            let command_id = admitted.continuity.next_command_id;
            let (status, reply, barrier, actual_entry) = super::guild_wire::exchange_capture(
                &owner,
                &mut admitted,
                command_id,
                boss::COMMAND_TYPE_BOSS_LEVER,
                |_| Ok(payload.clone()),
                true,
                &mut entry_frames,
            )
            .await?;
            assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
            assert!(matches!(
                boss::decode_boss_activity_reply(&reply).map_err(debug)?,
                boss::BossActivityReply::Accepted { .. }
            ));
            assert!(barrier.is_some());
            let fence = admitted.item_fence.ok_or("current Instance fence")?;
            assert_eq!(fence.runtime_scope, instance);
            let registered = owner
                .activity_runtimes
                .get(instance, fence.scope_ownership_generation.get())
                .ok_or("actual Ahau Instance")?;
            let mut view = owner.runtime_view(&registered);
            let actor = admitted.runtime_actor.ok_or("current Instance actor")?;
            if let Some(peer) = peer {
                super::m25_ahau_push::exercise(&h, &owner, &mut view, admitted, peer).await?;
                drop(view);
                drop(registered);
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            let entry_snapshot = view
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?;
            let target = {
                let runtime = view.runtime.lock().await;
                view.wire_observation(
                    &runtime,
                    MovementLocalPosition {
                        x: exit.trigger().x,
                        y: exit.trigger().y,
                        floor: i16::try_from(exit.trigger().z)?,
                    },
                )
                .ok_or("actual exit wire position")?
            };
            let mut route = view
                .item_walk_route(
                    actor,
                    admitted.game_session_id,
                    crate::gameplay_transport::paced_movement::AutoWalkTarget::Cell(
                        target.actor_position,
                    ),
                )
                .await
                .map_err(|_| "actual bounded native exit route")?
                .ok_or("real exit reachable")?;
            let final_step = route.steps.pop_back().ok_or("real STEP onto exit")?;
            let mut first_movement = None;
            for direction in route.steps {
                view.clock_origin = view
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(1))
                    .ok_or("actual movement cadence")?;
                let (step, _) = view
                    .paced_step(actor, admitted.game_session_id, direction)
                    .await;
                assert_eq!(
                    step.disposition,
                    movement::StepDisposition::Moved,
                    "actual walk toward exit"
                );
                if first_movement.is_none() {
                    first_movement = view
                        .runtime
                        .lock()
                        .await
                        .committed_movement_occurrence(actor)
                        .map_err(debug)?;
                }
            }
            view.clock_origin = view
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(1))
                .ok_or("actual final step cadence")?;
            let before_exit = admitted;
            let before_exit_snapshot = view
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?;
            assert_ne!(
                entry_snapshot, before_exit_snapshot,
                "actual source movement produced a new snapshot"
            );
            let current_movement = view
                .runtime
                .lock()
                .await
                .committed_movement_occurrence(actor)
                .map_err(debug)?
                .ok_or("actual penultimate movement receipt")?;
            let stale_movement = first_movement.ok_or("actual first route movement receipt")?;
            assert_ne!(
                stale_movement, current_movement,
                "real subsequent movement replaces the canonical latest receipt"
            );
            owner
                .source_boss_exit_movement(actor, admitted.game_session_id, current_movement)
                .await;
            view.source_boss_exit_movement(actor, source.game_session_id, current_movement)
                .await;
            let boss_actor = {
                let runtime = view.runtime.lock().await;
                runtime
                    .visible_entities()
                    .creatures
                    .into_iter()
                    .find(|candidate| {
                        runtime.matches_live_creature_identity(
                            candidate.actor,
                            b"oteryn:creature.ahau",
                        )
                    })
                    .ok_or("actual native Ahau")?
                    .actor
            };
            view.source_boss_exit_movement(boss_actor, admitted.game_session_id, current_movement)
                .await;
            view.source_boss_exit_movement(actor, admitted.game_session_id, stale_movement)
                .await;
            assert_eq!(
                return_count(&h).await?,
                0,
                "wrong owner scope, stale session, Creature/player mispair and stale receipt never issue a return"
            );
            assert_eq!(
                view.runtime
                    .lock()
                    .await
                    .read_actor_position(actor)
                    .map_err(debug)?,
                before_exit_snapshot
            );
            let step_payload = movement::encode_step_intent(final_step);
            let mut exit_frames = Vec::new();
            let command_id = admitted.continuity.next_command_id;
            let (status, reply, barrier, actual_exit) =
                super::guild_wire::exchange_capture_switch_effect(
                    &view,
                    &mut admitted,
                    command_id,
                    movement::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                    |_| Ok(step_payload.clone()),
                    11,
                    &mut exit_frames,
                )
                .await?;
            assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
            assert_eq!(
                movement::decode_step_result(&reply).map_err(debug)?,
                movement::StepDisposition::Moved
            );
            assert!(
                barrier.is_some(),
                "real STEP must cross the shared return barrier"
            );
            assert!(matches!(
                admitted.item_fence.ok_or("returned fence")?.runtime_scope,
                RuntimeScopeRefV1::Channel { .. }
            ));
            assert_ne!(before_exit.game_session_id, admitted.game_session_id);
            let actual = {
                let runtime = owner.runtime.lock().await;
                runtime
                    .read_actor_position(admitted.runtime_actor.ok_or("returned actor")?)
                    .map_err(debug)?
                    .position()
            };
            assert_eq!(
                (actual.x, actual.y, i32::from(actual.floor)),
                (
                    exit.destination().x,
                    exit.destination().y,
                    exit.destination().z
                )
            );
            let exact:i64=sqlx::query_scalar("SELECT count(*) FROM game_activity_runtime_returns WHERE source_game_session_id=encode($1,'hex')::uuid AND destination_game_session_id=encode($2,'hex')::uuid").bind(before_exit.game_session_id.as_bytes().as_slice()).bind(admitted.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(exact, 1, "one durable shared return receipt");
            let actual_exit_movement = view
                .source_exit_movement_for_test(before_exit.game_session_id)
                .await
                .ok_or("actual retained committed movement receipt")?;
            assert_eq!(actual_exit_movement.mover(), actor);
            let after = actual_exit_movement.after().position();
            assert_eq!(
                (after.x, after.y, i32::from(after.floor)),
                (exit.trigger().x, exit.trigger().y, exit.trigger().z)
            );
            let row:(i16,Option<String>,serde_json::Value)=sqlx::query_as("SELECT reason,command_id::text,native_step_exit FROM game_activity_runtime_returns WHERE source_game_session_id=encode($1,'hex')::uuid").bind(before_exit.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(row.0, 1);
            assert!(
                row.1.is_none(),
                "the native movement event does not fabricate a client command"
            );
            let hex = |bytes: &[u8]| {
                bytes
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>()
            };
            assert_eq!(
                row.2["movement_binding"],
                hex(&actual_exit_movement.binding())
            );
            assert_eq!(
                row.2["session"],
                hex(before_exit.game_session_id.as_bytes())
            );
            let RuntimeScopeRefV1::Instance {
                world_id,
                instance_id,
            } = instance
            else {
                return Err("actual Instance scope".into());
            };
            // runtime_scope_assignment uses tag3 for an Activity scope key.
            let mut expected_scope = vec![3_u8];
            expected_scope.extend_from_slice(world_id.as_bytes());
            expected_scope.extend_from_slice(&instance_id);
            assert_eq!(
                row.2["scope"],
                hex(&expected_scope),
                "exact tagged Instance scope, not an origin Channel"
            );
            super::m25_step_exit_matrix::assert_native_step_exit_matrix(
                &h,
                before_exit.game_session_id,
            )
            .await?;
            view.source_boss_exit_movement(
                actor,
                before_exit.game_session_id,
                actual_exit_movement,
            )
            .await;
            assert_eq!(
                return_count(&h).await?,
                1,
                "retired source actor/session cannot replay the return"
            );
            assert!(
                view.emit_source_boss_exit_effect(before_exit)
                    .await
                    .is_err(),
                "retired source cannot emit a destination effect"
            );
            assert_eq!(
                return_count(&h).await?,
                1,
                "unavailable stale visual cannot change committed custody"
            );

            if let Some(path) = std::env::var_os("SRX3_M25_CAPTURE_PATH") {
                export(
                    path,
                    [source, before_exit, admitted],
                    [&entry_frames, &exit_frames],
                    [&actual_entry, &actual_exit],
                )?;
            }
            drop(view);
            drop(registered);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })?;
    eprintln!(
        "M25_AHAU_NATIVE_STEP_EXIT_POSTGRES_EXECUTED duration={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn return_count(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_activity_runtime_returns")
            .fetch_one(&h.pool)
            .await?,
    )
}

fn export(
    path: std::ffi::OsString,
    [source, before_exit, returned]: [crate::gameplay_transport::connection::AdmittedSession; 3],
    [entry, exit]: [&[Vec<u8>]; 2],
    [entry_payload, exit_payload]: [&[u8]; 2],
) -> TestResult {
    use oteryn_protocol_oteryn::{MessageType, decode_wire_envelope};
    let hex = |raw: &[u8]| {
        raw.iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    };
    let metadata = |s: crate::gameplay_transport::connection::AdmittedSession| serde_json::json!({"game_session_id":hex(s.game_session_id.as_bytes()),"world_id":hex(s.world_id.as_bytes()),"channel_id":hex(s.channel_id.as_bytes()),"connection_generation":s.continuity.connection_generation,"current_server_sequence":s.continuity.server_sequence,"next_command_id":s.continuity.next_command_id,"schema_revision":crate::gameplay_transport::connection::SERVER_SCHEMA_REVISION,"selected_capabilities":s.continuity.selected_capabilities.as_slice()});
    let boundary = |frames: &[Vec<u8>]| -> TestResult<usize> {
        for (i, frame) in frames.iter().enumerate() {
            if decode_wire_envelope(frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                return Ok(i + 1);
            }
        }
        Err("actual initial snapshot commit".into())
    };
    let a = boundary(entry)?;
    let b = boundary(exit)?;
    let encode = |frames: &[Vec<u8>]| frames.iter().map(|frame| hex(frame)).collect::<Vec<_>>();
    let character = source
        .item_fence
        .ok_or("capture actual character")?
        .character_id;
    let position = |frames: &[Vec<u8>]| -> TestResult<[i32; 3]> {
        for frame in frames {
            let envelope = decode_wire_envelope(frame).map_err(debug)?;
            if envelope.message_type() != MessageType::SnapshotChunk {
                continue;
            }
            let (_, domains) =
                oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
            for domain in domains {
                if domain.domain_id == movement::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY {
                    use oteryn_protocol_oteryn::world_spatial_entities::{
                        EntityFields, SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                        decode_world_spatial_entities_snapshot_with_fields,
                    };
                    assert_eq!(domain.snapshot_type, SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2);
                    let (snapshot, _) = decode_world_spatial_entities_snapshot_with_fields(
                        domain.payload,
                        EntityFields::selected(source.continuity.selected_capabilities.as_slice()),
                    )
                    .map_err(debug)?;
                    let at = snapshot.actor_position;
                    return Ok([at.x, at.y, i32::from(at.floor)]);
                }
            }
        }
        Err("actual captured own spatial observation".into())
    };
    let source_position = position(&entry[..a])?;
    let entry_position = position(&entry[a..])?;
    let before_exit_position = position(&exit[..b])?;
    let returned_position = position(&exit[b..])?;
    let capture = serde_json::json!({"schema":"OTERYN_NATIVE_SCOPE_REPLAY/v1","mechanic":"M25_AHAU_EXIT","character_id":hex(character.as_bytes()),"source_admission":metadata(source),"exit_admission":metadata(before_exit),"returned_admission":metadata(returned),"initial_frames":encode(&entry[..a]),"entry_frames":encode(&entry[a..]),"entry_command":{"kind":boss::COMMAND_TYPE_BOSS_LEVER,"payload":hex(entry_payload)},"before_exit_frames":encode(&exit[..b]),"exit_frames":encode(&exit[b..]),"exit_command":{"kind":movement::COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,"payload":hex(exit_payload)},"source_position":source_position,"entry_position":entry_position,"before_exit_position":before_exit_position,"exit_trigger_source_position":[34002,31706,10],"returned_position":returned_position});
    std::fs::write(path, serde_json::to_vec_pretty(&capture)?)?;
    Ok(())
}

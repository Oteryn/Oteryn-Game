//! SEAM-2: holder contention cannot change a resumed command's ordered frames.
use super::*;
use crate::foundation::{AuthenticatedTransportRefV1, FoundationProtocolError};
use crate::gameplay_transport::connection::{
    ConnectionEnd, ControlLossResult, FreshAdmissionAuthority, IDLE_LIVENESS, ResumeAttempt,
    UseCommand, serve_admitted,
};
use crate::gameplay_transport::tcp_tls::write_frame;
use crate::gameplay_transport::world_spatial::*;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, DomainSnapshot, encode_client_command,
    encode_command_protocol_error, encode_command_result, encode_single_chunk_snapshot,
    encode_state_delta,
};
use std::sync::Arc;
use tokio::io::AsyncReadExt;

#[test]
fn seam_second_same_session_resume_has_deterministic_frames_with_busy_holder() -> TestResult {
    resume_frames(false)
}

#[test]
fn seam_resume_waits_for_durable_loss_runtime_publication() -> TestResult {
    resume_frames(true)
}

fn resume_frames(delay_loss_publication: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |admin| {
                let h =
                    Harness::create_with_admission_seed(admin, "seam_resume_frames", false, false)
                        .await?;
                let recovery = super::house_transfer::RecoverySource::start(&h).await?;
                super::house_transfer::prepare_owned_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                // Movement must use the room's real content pin. Dependencies::new()
                // deliberately uses a synthetic pin for tests that do not move actors.
                let world = fence(1)?.runtime_scope.world_id();
                let room = crate::content::qualify_native_entry_room(world).map_err(debug)?;
                let start = room.entry_start();
                let pin = crate::content::channel_content_pin::ChannelContentPin::from_activation(
                    world,
                    1,
                    room.compiled().server_digest(),
                    room.compiled().client_digest(),
                    room.frame_binding().digest(),
                    room.map_revision_digest(),
                    (start.x, start.y, start.floor),
                );
                let deps = Dependencies::with_content(
                    pin,
                    room,
                    crate::spell::cast::v1_spell_book().map_err(debug)?,
                )?;
                let mut owner = deps.owner(&h, &authority);
                owner.evidence = &recovery.evidence;
                {
                    let runtime = owner.runtime.lock().await;
                    assert!(
                        owner
                            .states
                            .spell_states
                            .lock()
                            .await
                            .initialize(
                                &runtime,
                                deps.actor,
                                deps.admitted.game_session_id,
                                crate::spell::cast::CharacterCastFacts {
                                    vocation: crate::spell::Vocation::None,
                                    level: 1,
                                    magic_level: 0,
                                    max_health: 150,
                                    max_mana: 55,
                                    max_soul: 100,
                                },
                                (0, 0),
                                owner.owner_now(),
                            )
                            .is_some()
                    );
                }
                owner
                    .runtime
                    .lock()
                    .await
                    .initialize_first_entry_position(deps.actor)
                    .map_err(debug)?;
                let mut admitted = deps.admitted;
                admitted.first_entry = FirstEntryOutcome::Positioned;
                admitted.item_fence = crate::gameplay_transport::item_fence_of(
                    FreshAdmissionStore::from_root(h.root.clone())
                        .current_session(admitted.game_session_id)
                        .await?,
                );
                owner.states.command_ingress.admit(admitted.game_session_id);
                for generation in [2, 3] {
                    let token = super::house_transfer::RecoverySource::token(generation as u8)?;
                    let attempt = ResumeAttempt {
                        game_session_id: admitted.game_session_id,
                        recovery_material: token.as_bytes(),
                        transport: AuthenticatedTransportRefV1::decode(&[generation as u8; 16])
                            .map_err(debug)?,
                        last_applied_server_sequence: admitted.continuity.server_sequence,
                        supported_capabilities: &[],
                    };
                    admitted = if delay_loss_publication {
                        let barrier =
                            Arc::new((tokio::sync::Notify::new(), tokio::sync::Notify::new()));
                        let loss =
                            crate::gameplay_transport::control_loss::LOSS_PUBLICATION_BARRIER
                                .scope(
                                    barrier.clone(),
                                    owner.lose_control(admitted, Duration::ZERO, None),
                                );
                        let resume = async {
                            barrier.0.notified().await;
                            let current = FreshAdmissionStore::from_root(h.root.clone())
                                .current_session(admitted.game_session_id)
                                .await?;
                            assert_eq!(
                                current.session_state(),
                                crate::foundation::GameSessionState::Reconnectable
                            );
                            assert!(
                                !owner
                                    .states
                                    .lost
                                    .lock()
                                    .map_err(debug)?
                                    .contains_key(&admitted.game_session_id)
                            );
                            let mut resume = std::pin::pin!(owner.resume(attempt));
                            // Hold the exact publication gap open: admission must remain pending,
                            // rather than permanently reject a durably reconnectable session.
                            assert!(
                                tokio::time::timeout(Duration::from_millis(500), &mut resume)
                                    .await
                                    .is_err()
                            );
                            barrier.1.notify_one();
                            resume.await.map_err(debug)
                        };
                        let (lost, resumed) =
                            tokio::time::timeout(Duration::from_secs(20), async {
                                tokio::join!(loss, resume)
                            })
                            .await?;
                        assert_eq!(lost, ControlLossResult::Recorded);
                        resumed?
                    } else {
                        assert_eq!(
                            owner.lose_control(admitted, Duration::ZERO, None).await,
                            ControlLossResult::Recorded
                        );
                        owner.resume(attempt).await.map_err(debug)?
                    };
                    assert_eq!(admitted.continuity.connection_generation, generation);
                    // Force the original-work lookup to meet an occupied holder. Its first poll
                    // must wait, not report an unknown original and close the resumed stream.
                    let held = h.root.try_issue_semantic_pass()?;
                    let command = UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: admitted.continuity.next_command_id,
                        item_fence: admitted.item_fence,
                    };
                    let mut lookup = std::pin::pin!(owner.resume_npc_bank(deps.actor, command));
                    std::future::poll_fn(|cx| {
                        use std::future::Future;
                        assert!(
                            lookup.as_mut().poll(cx).is_pending(),
                            "busy holder must wait for reconciliation"
                        );
                        std::task::Poll::Ready(())
                    })
                    .await;
                    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                    assert!(lookup.await.is_none(), "no retained NPC work exists");

                    let before = owner.observe(deps.actor).await.ok_or("position")?;
                    let (vitals_revision, vitals) = owner
                        .observe_vitals(deps.actor, admitted.game_session_id)
                        .await
                        .ok_or("vitals")?;
                    let vitals = oteryn_protocol_oteryn::actor_spell::encode_actor_vitals(&vitals)
                        .map_err(debug)?;
                    let overlay = owner.observe_world_object_overlay().await;
                    let overlay_payload =
                        oteryn_protocol_oteryn::world_object::encode_world_object_overlay_snapshot(
                            overlay.as_slice(),
                        )
                        .map_err(debug)?;
                    let n = admitted.continuity.next_command_id;
                    let sequence = admitted.continuity.server_sequence;
                    let revision = admitted.continuity.spatial_revision;
                    let direction = if generation == 2 {
                        StepDirection::East
                    } else {
                        StepDirection::West
                    };
                    let mut after = before;
                    after.actor_position.x += if generation == 2 { 1 } else { -1 };
                    let expected = encode_single_chunk_snapshot(
                        generation,
                        1,
                        sequence,
                        &[
                            DomainSnapshot {
                                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                                revision,
                                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                                payload: &encode_world_spatial(&before),
                            },
                            DomainSnapshot {
                                domain_id: 2,
                                revision: 0,
                                snapshot_type: 1,
                                payload: &overlay_payload,
                            },
                            DomainSnapshot {
                                domain_id: 3,
                                revision: vitals_revision,
                                snapshot_type: 1,
                                payload: &vitals,
                            },
                        ],
                    )
                    .map_err(debug)?;
                    let mut expected: Vec<Vec<u8>> = expected.into();
                    expected.extend([
                        encode_command_result(
                            generation,
                            sequence + 1,
                            n,
                            CommandStatus::Accepted,
                            &encode_step_result(StepDisposition::Moved),
                        )
                        .map_err(debug)?,
                        encode_state_delta(
                            generation,
                            sequence + 2,
                            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                            revision,
                            revision + 1,
                            DELTA_TYPE_WORLD_SPATIAL_V1,
                            &encode_world_spatial(&after),
                        )
                        .map_err(debug)?,
                        encode_command_protocol_error(
                            FoundationProtocolError::CommandSequenceGap,
                            generation,
                            n + 2,
                            n + 1,
                        )
                        .map_err(debug)?,
                    ]);
                    let (mut server, mut client) = tokio::io::duplex(1 << 20);
                    for command_id in [n, n + 2] {
                        write_frame(
                            &mut client,
                            &encode_client_command(
                                generation,
                                &ClientCommandValue {
                                    command_id,
                                    command_type: COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                                    payload: &encode_step_intent(direction),
                                },
                            )
                            .map_err(debug)?,
                        )
                        .await?;
                    }
                    let end =
                        Box::pin(serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS))
                            .await;
                    drop(server);
                    let mut bytes = Vec::new();
                    client.read_to_end(&mut bytes).await?;
                    let expected: Vec<u8> = expected
                        .iter()
                        .flat_map(|frame| {
                            (frame.len() as u32)
                                .to_be_bytes()
                                .into_iter()
                                .chain(frame.iter().copied())
                        })
                        .collect();
                    assert_eq!(bytes, expected, "generation {generation}");
                    admitted = match end {
                        ConnectionEnd::AdmittedThenClosed(
                            current,
                            FoundationProtocolError::CommandSequenceGap,
                        ) => current,
                        end => return Err(format!("unexpected resumed end: {end:?}").into()),
                    };
                    assert_eq!(admitted.continuity.next_command_id, n + 1);
                    assert_eq!(admitted.continuity.server_sequence, sequence + 2);
                }
                drop(owner);
                drop(seal);
                h.cleanup().await
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "resume frames test thread panicked")??;
    Ok(())
}

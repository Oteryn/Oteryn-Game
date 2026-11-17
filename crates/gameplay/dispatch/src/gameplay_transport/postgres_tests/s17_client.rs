//! Canary livestream.cpp:249,260: real control caller starts/stops a session-only cast.
use super::{
    Dependencies, TestResult,
    fixture::{Harness, debug, run},
    prepare_current_session,
};
use oteryn_protocol_oteryn::cast::Message;
#[path = "s17_client/settings_host.rs"]
pub(super) mod settings_host;
#[test]
fn s17a_client_start_stop_dispatch_commits_audit_and_rejects_stale_revision() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            run(async |admin| {
                let started = std::time::Instant::now();
                let h =
                    Harness::create_with_admission_seed(admin, "s17_client_control", true, false)
                        .await?;
                prepare_current_session(&h).await?;
                // Like node/serve.rs::maintain: long-lived workers need ongoing
                // holder upkeep, rather than the one-shot helper of ordinary cases.
                let upkeep_root = h.root.clone();
                let upkeep = tokio::spawn(async move {
                    let mut cadence = tokio::time::interval(std::time::Duration::from_millis(10));
                    loop {
                        cadence.tick().await;
                        if upkeep_root.has_ready_demand()
                            && upkeep_root.maintain_ready_once().await.is_err()
                        {
                            upkeep_root.request_ready();
                        }
                    }
                });
                let result = async {
                    let seal = h.recovery.seal_current().map_err(debug)?;
                    let authority = h
                        .root
                        .open_character_authority(&seal)
                        .await
                        .map_err(debug)?;
                    let deps = Dependencies::new()?;
                    let owner = deps.owner(&h, &authority);
                    deps.initialize(&owner).await;
                    let current =
                        crate::durability::fresh_admission::FreshAdmissionStore::from_root(
                            h.root.clone(),
                        )
                        .current_session_at(deps.admitted.game_session_id)
                        .await?
                        .0;
                    let mut admitted = deps.admitted;
                    admitted.continuity.selected_capabilities =
                        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                            &[crate::gameplay_transport::capabilities::OfferedCapability {
                                id: 290,
                                requires: &[],
                            }],
                            &[290],
                        )
                        .ok_or("selection")?;
                    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
                    admitted.controller.as_mut().ok_or("controller")?.transport =
                        current.current_transport().ok_or("transport")?;
                    admitted.first_entry =
                        crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
                    let fence = admitted.item_fence.ok_or("item fence")?;
                    {
                        let mut runtime = deps.runtime.lock().await;
                        runtime
                            .bind_attacker_lease(
                                deps.actor,
                                admitted.game_session_id,
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
                        runtime
                            .initialize_first_entry_position(deps.actor)
                            .map_err(debug)?;
                    }
                    let (access, host) =
                        settings_host::host(*h.node.fact().node_id().as_bytes()).await?;
                    use oteryn_gameplay_social::cast::preferences::PreferencesStore;
                    access
                        .load(admitted.controller.ok_or("controller")?.account_id)
                        .await?;
                    let (_, vitals) = owner
                        .observe_actor_vitals(deps.actor, admitted.game_session_id)
                        .await
                        .ok_or("caster vitals unavailable")?;
                    assert!(vitals.health > 0);
                    assert!(
                        deps.runtime
                            .lock()
                            .await
                            .player_control_facts(deps.actor, admitted.game_session_id)
                            .map_err(debug)?
                            .control_loss
                            .is_none()
                    );
                    let service = oteryn_gameplay_social::cast::service::CastService::spawn(
                        h.root.clone(),
                        h.node.clone(),
                        50,
                        access.clone(),
                    )?;
                    owner
                        .states
                        .casts
                        .set(crate::gameplay_transport::cast::CastNode {
                            service: service.clone(),
                            access,
                        })
                        .map_err(|_| "cast already set")?;
                    // One real transport: disconnect is itself a cast-stop boundary.
                    let (mut server, mut client) = tokio::io::duplex(16384);
                    let serving = crate::gameplay_transport::connection::serve_admitted(
                        &mut server,
                        admitted,
                        &owner,
                        crate::gameplay_transport::connection::IDLE_LIVENESS,
                    );
                    let driving = async {
                        use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
                        use oteryn_protocol_oteryn::{
                            ClientCommandValue, CommandStatus, MessageType, decode_command_result,
                            decode_wire_envelope, encode_client_command,
                        };
                        use tokio::io::AsyncWriteExt;
                        loop {
                            let bytes = read_frame(&mut client).await?;
                            if decode_wire_envelope(&bytes).map_err(debug)?.message_type()
                                == MessageType::SnapshotCommit
                            {
                                break;
                            }
                        }
                        let mut revision = 0;
                        for step in 0..3 {
                            let message = match step {
                                0 => Message::Start {
                                    revision: 0,
                                    access_revision: 1,
                                    delay: 0,
                                    cap: 50,
                                    password: false,
                                },
                                1 => Message::Stop {
                                    revision: revision + 1,
                                },
                                _ => Message::Stop { revision },
                            };
                            let payload = message.encode().map_err(debug)?;
                            let id = admitted.continuity.next_command_id + step;
                            let bytes = encode_client_command(
                                admitted.continuity.connection_generation,
                                &ClientCommandValue {
                                    command_id: id,
                                    command_type: 502,
                                    payload: &payload,
                                },
                            )
                            .map_err(debug)?;
                            write_frame(&mut client, &bytes).await?;
                            loop {
                                let bytes = read_frame(&mut client).await?;
                                let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
                                if envelope.message_type() != MessageType::CommandResult {
                                    continue;
                                }
                                let result =
                                    decode_command_result(envelope.payload()).map_err(debug)?;
                                assert_eq!(result.command_id, id);
                                if step == 1 {
                                    assert_eq!(result.status, CommandStatus::Rejected);
                                } else {
                                    assert_eq!(
                                        result.status,
                                        CommandStatus::Accepted,
                                        "control step {step}"
                                    );
                                    let reply = Message::decode(result.payload).map_err(debug)?;
                                    if step == 0 {
                                        let Message::CasterState {
                                            revision: current, ..
                                        } = reply
                                        else {
                                            return Err("start reply".into());
                                        };
                                        revision = current;
                                    } else {
                                        assert_eq!(reply, Message::End);
                                    }
                                }
                                break;
                            }
                        }
                        client.shutdown().await?;
                        Ok::<(), Box<dyn std::error::Error>>(())
                    };
                    let (_, result) = tokio::time::timeout(
                        std::time::Duration::from_secs(30),
                        super::fixture::join_two(serving, driving),
                    )
                    .await?;
                    result?;
                    let rows: i64 =
                        sqlx::query_scalar("SELECT count(*) FROM game_cast_audit_outbox")
                            .fetch_one(&h.pool)
                            .await?;
                    assert_eq!(rows, 2);
                    let _ = service
                        .apply(oteryn_gameplay_social::cast::service::Operation::Shutdown)
                        .await;
                    host.abort();
                    eprintln!(
                        "S17a real-dispatch PostgreSQL: {:.3}s",
                        started.elapsed().as_secs_f64()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                }
                .await;
                upkeep.abort();
                h.cleanup().await?;
                result
            })
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "S17a thread failed")?
        .map_err(Into::into)
}

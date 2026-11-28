use super::autowalk_fixtures::{ack_probe, read, send};
use super::fixture::join_two;
use super::full_world_support::*;
use crate::durability::DurabilityRoot;
use crate::gameplay_transport::connection::{
    AFTER_COMMAND_INPUT_GUARDS, ConnectionEnd, IDLE_LIVENESS, serve_admitted,
};
use oteryn_protocol_oteryn::item_view::{self, ItemHandle};
use oteryn_protocol_oteryn::world_object::{
    CAPABILITY_RUNE_USE_V1, COMMAND_TYPE_USE_INTENT, UseDisposition, decode_use_result,
    encode_use_item_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityDetail, decode_world_spatial_entities_delta_with_item_handles,
};
use oteryn_protocol_oteryn::zones_nologout::{self, ServerMessage};
use oteryn_protocol_oteryn::{
    CommandStatus, MessageType, decode_command_result, decode_state_delta, decode_wire_envelope,
};
use std::time::Instant;
use tokio::io::{AsyncWriteExt, DuplexStream};
use tokio::sync::Notify;

#[test]
fn p11_demon_removal_handle_opens_on_next_use_and_delivers_loot_notice() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("p11-corpse-use", || {
        demon_corpse_use(None)
    })
}

#[test]
fn w13_corpse_use_waits_for_holder_and_opens_once_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(Contention::Released)
}

#[test]
fn w13_corpse_use_busy_refusal_replays_and_next_use_opens_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    run(Contention::Exhausted)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Contention {
    Released,
    Exhausted,
}

fn run(contention: Contention) -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("w13-corpse-use", move || {
        let mut result = Ok(());
        let lines = oteryn_error_codes::capture(|| result = demon_corpse_use(Some(contention)));
        result?;
        let outcome = match contention {
            Contention::Released => "retry_finished",
            Contention::Exhausted => "holder_busy",
        };
        assert!(
            lines.iter().any(|line| {
                line.contains("event=session_holder_wait")
                    && line.contains("purpose=corpse_open")
                    && line.contains("p11_corpse_use.rs")
                    && line.contains("held_ms=")
                    && line.contains("waited_ms=")
                    && line.contains(&format!("outcome={outcome}"))
            }),
            "{lines:?}"
        );
        Ok(())
    })
}

fn demon_corpse_use(contention: Option<Contention>) -> TestResult {
    world_scenario!(
        "p11_corpse_use",
        |h,
         request,
         seal,
         authority,
         world_id,
         native,
         world,
         cells,
         room,
         pin,
         deps,
         actor,
         session,
         owner,
         fence,
         admitted| {
            let _ = &request;
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                owner.offered_capabilities(),
                &[
                    4,
                    6,
                    15,
                    CAPABILITY_RUNE_USE_V1,
                    zones_nologout::CAPABILITY_ZONES_NOLOGOUT_V1,
                ],
            )
            .ok_or("selected capabilities")?;
            assert!(
                admitted
                    .continuity
                    .selected_capabilities
                    .contains(CAPABILITY_RUNE_USE_V1)
            );
            assert!(
                admitted
                    .continuity
                    .selected_capabilities
                    .contains(zones_nologout::CAPABILITY_ZONES_NOLOGOUT_V1)
            );
            let target = {
                let mut runtime = owner.runtime.lock().await;
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .get_mut(&runtime, actor, session)
                    .ok_or("killer state")?
                    .facts
                    .level = 1_000_000;
                runtime
                    .realize_native_qualification_spawn(
                        pin.clone(),
                        "oteryn:creature.demon",
                        MovementLocalPosition {
                            x: ENTRY.0 + 1,
                            y: ENTRY.1,
                            floor: 7,
                        },
                    )
                    .map_err(debug)?
                    .actor
            };
            let entity = EntityRef {
                identity: target.placement_identity(),
                generation: target.actor_local_generation(),
            };
            let (mut server, mut client) = tokio::io::duplex(1 << 20);
            let barrier = Arc::new((Notify::new(), Notify::new()));
            let drive = async {
                let result = async {
                    loop {
                        let frame = read(&mut client).await?;
                        if decode_wire_envelope(&frame)?.message_type()
                            == MessageType::SnapshotCommit
                        {
                            break;
                        }
                    }
                    assert_eq!(
                        owner
                            .select_attack_target(actor, session, 1, Some(entity))
                            .await,
                        AttackIntentDisposition::Ok
                    );
                    owner.refresh_attack_weapon(actor, session).await;
                    let started = owner.owner_now();
                    for tick in 1..=16 {
                        let swing = started.checked_add(tick * 3_000_000).map_err(debug)?;
                        owner.drain_auto_attacks_at(swing).await;
                        if !owner.runtime.lock().await.contains_live_creature(target) {
                            break;
                        }
                    }
                    assert!(!owner.runtime.lock().await.contains_live_creature(target));
                    assert!(matches!(
                        owner.drain_kill_rewards(session).await,
                        DrainOutcome::Drained
                    ));

                    let mut proof = CorpseWireProof::default();
                    let handle = loop {
                        proof
                            .read(&mut client, entity, target.corpse_identity())
                            .await?;
                        if proof.removed
                            && let Some(handle) = proof.handle
                        {
                            break handle;
                        }
                    };
                    let mut command_id = admitted.continuity.next_command_id;
                    let payload = encode_use_item_intent(handle);
                    send(&mut client, command_id, COMMAND_TYPE_USE_INTENT, &payload).await?;
                    if let Some(case) = contention {
                        barrier.0.notified().await;
                        let mut holder = owner.root.try_acquire_ready()?;
                        let checkout_started = tokio::time::Instant::now();
                        barrier.1.notify_one();

                        if case == Contention::Exhausted {
                            // Keep the holder across the terminal result and its replay.
                            // Neither can publish an open or re-enter the durable read.
                            for status in [CommandStatus::Rejected, CommandStatus::DuplicateReplay] {
                                while proof.reply.is_none() {
                                    proof
                                        .read(&mut client, entity, target.corpse_identity())
                                        .await?;
                                }
                                assert_eq!(
                                    proof.reply.take(),
                                    Some((
                                        command_id,
                                        status,
                                        UseDisposition::Rejected
                                    ))
                                );
                                assert_eq!(proof.open_deltas, 0);
                                if status == CommandStatus::Rejected {
                                    assert!(checkout_started.elapsed()
                                        >= crate::gameplay_transport::AdmissionRetryBudget::holder_wait_window());
                                    send(
                                        &mut client,
                                        command_id,
                                        COMMAND_TYPE_USE_INTENT,
                                        &payload,
                                    )
                                    .await?;
                                }
                            }
                        } else {
                            // Longer than the old command window, shorter than the P18 read window.
                            tokio::time::sleep(
                                crate::gameplay_transport::COMMAND_HOLDER_WINDOW
                                    + Duration::from_millis(250),
                            )
                            .await;
                        }
                        assert_eq!(
                            holder
                                .return_to_pool_observed_until(
                                    Instant::now() + crate::durability::DB_PASS_DEADLINE,
                                )
                                .await,
                            sqlx::pool::PoolConnectionReturnDisposition::ReturnedToIdle
                        );

                        if case == Contention::Exhausted {
                            command_id += 1;
                            send(&mut client, command_id, COMMAND_TYPE_USE_INTENT, &payload)
                                .await?;
                            barrier.0.notified().await;
                            barrier.1.notify_one();
                        }
                    }
                    while !proof.loot_notice || proof.opened.is_none() || proof.reply.is_none() {
                        proof
                            .read(&mut client, entity, target.corpse_identity())
                            .await?;
                        if let Some((reply_id, status, disposition)) = proof.reply {
                            assert_eq!(
                                (reply_id, status, disposition),
                                (
                                    command_id,
                                    CommandStatus::Accepted,
                                    UseDisposition::Committed
                                )
                            );
                        }
                    }
                    assert_eq!(proof.opened, Some(handle));
                    assert_eq!(proof.open_deltas, 1);
                    if contention.is_some() {
                        proof.reply = None;
                        send(&mut client, command_id, COMMAND_TYPE_USE_INTENT, &payload).await?;
                        while proof.reply.is_none() {
                            proof
                                .read(&mut client, entity, target.corpse_identity())
                                .await?;
                        }
                        assert_eq!(proof.reply, Some((
                            command_id, CommandStatus::DuplicateReplay, UseDisposition::Committed,
                        )));
                        assert_eq!(proof.open_deltas, 1);
                    }
                    TestResult::Ok(())
                }
                .await;
                client.shutdown().await?;
                result
            };
            let (end, result) = tokio::time::timeout(
                std::time::Duration::from_secs(20),
                join_two(
                    async {
                        let serve = DurabilityRoot::with_holder_owner(serve_admitted(
                            &mut server,
                            admitted,
                            &owner,
                            IDLE_LIVENESS,
                        ));
                        if contention.is_some() {
                            AFTER_COMMAND_INPUT_GUARDS
                                .scope(barrier.clone(), serve)
                                .await
                        } else {
                            serve.await
                        }
                    },
                    drive,
                ),
            )
            .await?;
            result?;
            assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        }
    )
}

#[derive(Default)]
struct CorpseWireProof {
    removed: bool,
    handle: Option<ItemHandle>,
    opened: Option<ItemHandle>,
    open_deltas: usize,
    reply: Option<(u64, CommandStatus, UseDisposition)>,
    loot_notice: bool,
}

impl CorpseWireProof {
    async fn read(
        &mut self,
        client: &mut DuplexStream,
        killed: EntityRef,
        corpse: [u8; 16],
    ) -> TestResult {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        match envelope.message_type() {
            MessageType::ProtocolError => {
                let error = oteryn_protocol_oteryn::decode_protocol_error(envelope.payload())?;
                return Err(format!("unexpected protocol error E{}", error.error_code).into());
            }
            MessageType::LivenessProbe => ack_probe(client, &frame).await?,
            MessageType::CommandResult => {
                let result = decode_command_result(envelope.payload())?;
                self.reply = Some((
                    result.command_id,
                    result.status,
                    decode_use_result(result.payload).map_err(debug)?,
                ));
            }
            MessageType::StateDelta => {
                let delta = decode_state_delta(envelope.payload())?;
                match delta.domain_id {
                    crate::gameplay_transport::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY => {
                        let spatial = decode_world_spatial_entities_delta_with_item_handles(delta.payload)
                            .map_err(debug)?;
                        self.removed |= spatial.leave.contains(&killed);
                        if let Some(handle) = spatial.enter.iter().find_map(|entry| match entry.detail {
                                EntityDetail::Object { item_handle, .. }
                                | EntityDetail::ObjectWithAppearance { item_handle, .. }
                                    if entry.entity.identity == corpse => item_handle,
                                _ => None,
                            }) {
                            self.handle = Some(handle);
                        }
                    }
                    item_view::STATE_DOMAIN_OPEN_CONTAINER => {
                        self.open_deltas += 1;
                        self.opened = item_view::decode_open_container(delta.payload).map_err(debug)?.container_handle;
                    }
                    zones_nologout::STATE_DOMAIN_SERVER_MESSAGES => {
                        if let Ok(ServerMessage::StatusText(text)) = zones_nologout::decode_server_message(delta.payload) {
                            self.loot_notice |= text.starts_with("Loot of a demon: ");
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }
}

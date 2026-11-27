use super::autowalk_fixtures::{read, send};
use super::fixture::join_two;
use super::full_world_support::*;
use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
use crate::gameplay_transport::kill_reward::{KillSink, record_creature_kill};
use oteryn_protocol_oteryn::attack::{
    CAPABILITY_ATTACK_V1, COMMAND_TYPE_ATTACK_TARGET_INTENT, COMMAND_TYPE_FIGHT_MODES_INTENT,
    FightModes, STATE_DOMAIN_ACTOR_COMBAT_STATE, decode_actor_combat_state,
    decode_attack_intent_result, encode_attack_target_intent, encode_fight_modes_intent,
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
use tokio::io::{AsyncWriteExt, DuplexStream};

#[test]
fn p11_demon_removal_handle_opens_on_next_use_and_delivers_loot_notice() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e_fixtures::scenario_thread::run("p11-corpse-use", demon_corpse_use)
}

fn demon_corpse_use() -> TestResult {
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
                    CAPABILITY_ATTACK_V1,
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
            let attack_id = admitted.continuity.next_command_id;
            let (mut server, mut client) = tokio::io::duplex(1 << 20);
            let drive = async {
                let result = async {
                    loop {
                        let frame = read(&mut client).await?;
                        if decode_wire_envelope(&frame)?.message_type() == MessageType::SnapshotCommit {
                            break;
                        }
                    }
                    send(
                        &mut client,
                        attack_id,
                        COMMAND_TYPE_ATTACK_TARGET_INTENT,
                        &encode_attack_target_intent(Some(&entity)).map_err(debug)?,
                    ).await?;
                    let mut proof = CorpseWireProof::default();
                    proof.await_combat_reply(&mut client, entity, target.corpse_identity(), attack_id).await?;
                    while !proof.target_seen {
                        proof.read(&mut client, entity, target.corpse_identity()).await?;
                    }

                    let mode_id = attack_id.checked_add(1).ok_or("fight modes command id")?;
                    let barrier_id = mode_id.checked_add(1).ok_or("barrier command id")?;
                    let observed = owner.owner_now();
                    let death_ms = observed.get() / 1_000;
                    let principal = {
                        let mut runtime = owner.runtime.lock().await;
                        let lease = runtime.bound_attacker_lease(actor, session).map_err(debug)?;
                        let health = runtime.companion_snapshot(target).map_err(debug)?.health;
                        let target_identity = runtime.borrow_combat_death().creature_target_identity(target).map_err(debug)?.to_vec();
                        // Cross the exact damage owner with enough damage to kill regardless of swing rolls.
                        let hit = runtime.borrow_exact_actor_commit_at(death_ms)
                            .commit_damage_for_bound_attacker(
                                target,
                                actor,
                                CommandRef::new(session, CommandId::new(900).map_err(debug)?),
                                0,
                                crate::foundation::OwnerDamageCommand {
                                    target: &target_identity,
                                    occurrence: session.as_bytes(),
                                    binding: b"p11-demon-lethal",
                                    damage: health,
                                },
                            ).map_err(debug)?;
                        assert!(hit.applied && hit.health_before > 0);
                        assert_eq!(hit.health_after, 0, "the fixture hit must kill the demon");
                        assert!(!runtime.contains_live_creature(target));
                        crate::combat::CapturedRewardPrincipal {
                            actor, session, character: lease.character_id(), lease_generation: lease.generation(),
                        }
                    };
                    let pending = owner.observe_channel_entities(actor).await.ok_or("pending death view")?;
                    assert!(pending.entities.iter().any(|entry| {
                        entry.identity == entity.identity
                            && entry.kind == crate::gameplay_transport::world_spatial::VisibleKind::PendingDeath
                    }), "hold the observed creature until its corpse commits");
                    // Each reply precedes its own combat refresh. The second reply proves
                    // the first refresh ran on this connection while the corpse was pending.
                    for id in [mode_id, barrier_id] {
                        send(&mut client, id, COMMAND_TYPE_FIGHT_MODES_INTENT, &encode_fight_modes_intent(&FightModes::DEFAULT)).await?;
                        proof.await_combat_reply(&mut client, entity, target.corpse_identity(), id).await?;
                    }
                    assert!(proof.handle.is_none(), "the corpse is still pending");
                    assert!(!proof.target_cleared, "the pending demon must stay targeted on the wire");
                    {
                        let mut runtime = owner.runtime.lock().await;
                        let states = owner.states.spell_states.lock().await;
                        let attack = owner.states.attack.lock().await;
                        record_creature_kill(
                            &mut runtime,
                            KillSink {
                                prey: Some(&states),
                                queue: attack.kills(),
                                rewards: native.reward_table(),
                                activity: &[],
                                charms: None,
                            },
                            target, principal, death_ms, Some(observed),
                        );
                    }

                    // The live combat cadence drains the queued death and publishes its corpse.
                    let handle = loop {
                        proof.read(&mut client, entity, target.corpse_identity()).await?;
                        if let Some(handle) = proof.handle {
                            break handle;
                        }
                    };
                    let command_id = barrier_id.checked_add(1).ok_or("corpse USE command id")?;
                    proof.use_command = Some(command_id);
                    send(&mut client, command_id, COMMAND_TYPE_USE_INTENT, &encode_use_item_intent(handle))
                        .await?;
                    while !proof.target_cleared || !proof.loot_notice || proof.opened.is_none() || proof.reply.is_none() {
                        proof.read(&mut client, entity, target.corpse_identity()).await?;
                        if let Some((reply_id, status, disposition)) = proof.reply {
                            assert_eq!((reply_id, status, disposition),
                                (command_id, CommandStatus::Accepted, UseDisposition::Committed));
                        }
                    }
                    assert_eq!(proof.opened, Some(handle));
                    assert!(matches!(owner.drain_kill_rewards(session).await, DrainOutcome::Drained));
                    TestResult::Ok(())
                }.await;
                client.shutdown().await?;
                result
            };
            let (end, result) = tokio::time::timeout(
                std::time::Duration::from_secs(20),
                join_two(
                    serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS),
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
    target_seen: bool,
    target_cleared: bool,
    combat_reply: Option<u64>,
    use_command: Option<u64>,
    handle: Option<ItemHandle>,
    opened: Option<ItemHandle>,
    reply: Option<(u64, CommandStatus, UseDisposition)>,
    loot_notice: bool,
}

impl CorpseWireProof {
    async fn await_combat_reply(
        &mut self,
        client: &mut DuplexStream,
        killed: EntityRef,
        corpse: [u8; 16],
        command: u64,
    ) -> TestResult {
        while self.combat_reply != Some(command) {
            self.read(client, killed, corpse).await?;
        }
        Ok(())
    }

    async fn read(
        &mut self,
        client: &mut DuplexStream,
        killed: EntityRef,
        corpse: [u8; 16],
    ) -> TestResult {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        match envelope.message_type() {
            MessageType::CommandResult => {
                let result = decode_command_result(envelope.payload())?;
                if let Some(command_id) = self.use_command {
                    assert_eq!(result.command_id, command_id);
                    self.reply = Some((
                        result.command_id,
                        result.status,
                        decode_use_result(result.payload).map_err(debug)?,
                    ));
                } else {
                    assert_eq!(result.status, CommandStatus::Accepted);
                    assert_eq!(
                        decode_attack_intent_result(result.payload).map_err(debug)?,
                        AttackIntentDisposition::Ok
                    );
                    self.combat_reply = Some(result.command_id);
                }
            }
            MessageType::StateDelta => {
                let delta = decode_state_delta(envelope.payload())?;
                match delta.domain_id {
                    crate::gameplay_transport::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY => {
                        let spatial = decode_world_spatial_entities_delta_with_item_handles(delta.payload)
                            .map_err(debug)?;
                        if spatial.leave.contains(&killed) {
                            self.handle = spatial.enter.iter().find_map(|entry| match entry.detail {
                                EntityDetail::Object { item_handle, .. }
                                | EntityDetail::ObjectWithAppearance { item_handle, .. }
                                    if entry.entity.identity == corpse => item_handle,
                                _ => None,
                            });
                            assert!(self.handle.is_some(), "the removal delta must publish the durable corpse handle");
                        }
                    }
                    STATE_DOMAIN_ACTOR_COMBAT_STATE => {
                        let state = decode_actor_combat_state(delta.payload).map_err(debug)?;
                        self.target_seen |= state.target == Some(killed);
                        if self.target_seen && state.target.is_none() {
                            assert!(self.handle.is_some(), "target clear preceded the demon removal and committed corpse handle");
                            self.target_cleared = true;
                        }
                    }
                    item_view::STATE_DOMAIN_OPEN_CONTAINER => {
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

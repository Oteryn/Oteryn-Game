use super::autowalk_fixtures::{read, send};
use super::fixture::join_two;
use super::full_world_support::*;
use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
use oteryn_protocol_oteryn::item_view::{self, ItemHandle};
use oteryn_protocol_oteryn::world_object::{
    CAPABILITY_RUNE_USE_V1, COMMAND_TYPE_USE_INTENT, UseDisposition, decode_use_result,
    encode_use_item_intent,
};
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityDetail, decode_world_spatial_entities_delta_with_item_handles,
};
use oteryn_protocol_oteryn::zones_nologout::{self, ServerMessage};
use oteryn_protocol_oteryn::{CommandStatus, MessageType, decode_command_result, decode_state_delta, decode_wire_envelope};
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
        |h, request, seal, authority, world_id, native, world, cells, room, pin, deps,
         actor, session, owner, fence, admitted| {
            let _ = &request;
            admitted.continuity.selected_capabilities =
                SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 15, 22])
                    .ok_or("selected capabilities")?;
            assert!(admitted.continuity.selected_capabilities.contains(CAPABILITY_RUNE_USE_V1));
            let target = {
                let mut runtime = owner.runtime.lock().await;
                owner.states.spell_states.lock().await
                    .get_mut(&runtime, actor, session).ok_or("killer state")?
                    .facts.level = 1_000_000;
                runtime.realize_native_qualification_spawn(
                    pin.clone(),
                    "oteryn:creature.demon",
                    MovementLocalPosition { x: ENTRY.0 + 1, y: ENTRY.1, floor: 7 },
                ).map_err(debug)?.actor
            };
            let entity = EntityRef {
                identity: target.placement_identity(),
                generation: target.actor_local_generation(),
            };
            let (mut server, mut client) = tokio::io::duplex(1 << 20);
            let drive = async {
                let result = async {
                    loop {
                        let frame = read(&mut client).await?;
                        if decode_wire_envelope(&frame)?.message_type() == MessageType::SnapshotCommit {
                            break;
                        }
                    }
                    assert_eq!(
                        owner.select_attack_target(actor, session, 1, Some(entity)).await,
                        AttackIntentDisposition::Ok
                    );
                    owner.refresh_attack_weapon(actor, session).await;
                    let swing = owner.owner_now().checked_add(2_000_000).map_err(debug)?;
                    owner.drain_auto_attacks_at(swing).await;
                    assert!(!owner.runtime.lock().await.contains_live_creature(target));
                    assert!(matches!(owner.drain_kill_rewards(session).await, DrainOutcome::Drained));

                    let mut proof = CorpseWireProof::default();
                    let handle = loop {
                        proof.read(&mut client, entity, target.corpse_identity()).await?;
                        if let Some(handle) = proof.handle {
                            break handle;
                        }
                    };
                    let command_id = admitted.continuity.next_command_id;
                    send(&mut client, command_id, COMMAND_TYPE_USE_INTENT, &encode_use_item_intent(handle))
                        .await?;
                    while !proof.loot_notice || proof.opened.is_none() || proof.reply.is_none() {
                        proof.read(&mut client, entity, target.corpse_identity()).await?;
                        if let Some((reply_id, status, disposition)) = proof.reply {
                            assert_eq!((reply_id, status, disposition),
                                (command_id, CommandStatus::Accepted, UseDisposition::Committed));
                        }
                    }
                    assert_eq!(proof.opened, Some(handle));
                    TestResult::Ok(())
                }.await;
                client.shutdown().await?;
                result
            };
            let (end, result) = tokio::time::timeout(
                std::time::Duration::from_secs(20),
                join_two(serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS), drive),
            ).await?;
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
    handle: Option<ItemHandle>,
    opened: Option<ItemHandle>,
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
            MessageType::CommandResult => {
                let result = decode_command_result(envelope.payload())?;
                self.reply = Some((result.command_id, result.status, decode_use_result(result.payload).map_err(debug)?));
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

//! Original compiled map and NPCframe; no source NPC is reseeded beside a synthetic actor.
use super::*;
use crate::content::{ReferenceItemField as F, ReferenceItemType};
pub(super) const ENTRY: (i32, i32, i16) = (33228, 31832, 7);
pub(super) fn world(
    world: crate::foundation::WorldId,
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<(crate::map::boot::BundleWorld, NpcPlacementTable)> {
    let bytes = std::fs::read(
        std::env::var_os("OTERYN_SRV7_FURNITURE_SHOPS_BUNDLE")
            .ok_or("actual retailer source bundle")?,
    )?;
    let b = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
    let table = NpcPlacementTable {
        npcs: b
            .npcs
            .npcs
            .iter()
            .map(|n| NpcPlacements {
                key: n.key.clone(),
                placements: n
                    .placements
                    .iter()
                    .map(|p| NpcPlacement {
                        x: p.x,
                        y: p.y,
                        floor: p.floor,
                        direction: match p.direction {
                            oteryn_world_bundle::npc::Direction::North => MovementFacing::North,
                            oteryn_world_bundle::npc::Direction::East => MovementFacing::East,
                            oteryn_world_bundle::npc::Direction::South => MovementFacing::South,
                            oteryn_world_bundle::npc::Direction::West => MovementFacing::West,
                        },
                    })
                    .collect(),
            })
            .collect(),
    };
    let edvard = table
        .npcs
        .iter()
        .find(|n| n.key == "oteryn:npc.edvard")
        .ok_or("real Edvard source placement")?;
    assert_eq!(edvard.placements.len(), 1);
    assert_eq!(
        (
            edvard.placements[0].x,
            edvard.placements[0].y,
            edvard.placements[0].floor
        ),
        (33229, 31832, -7)
    );
    let pins = crate::map::BundlePins {
        digest: b.digest,
        project_format_version: b.manifest.identity.project_format_version.clone(),
        world_schema_version: b.manifest.identity.world_schema_version.clone(),
        content_revision: b.manifest.identity.content_revision.clone(),
        production: false,
    };
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let index = native.item_index().ok_or("retailer native Item index")?;
    let boot=crate::map::boot::boot(&bytes,&crate::map::boot::BootPins{bundle:pins,map_revision:crate::map::overlay::map_revision(&base),start:crate::map::overlay::TilePos{x:33228,y:31832,floor:-7}},world,crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,|key|{
 let rev=index.revision(key)?;let policy=native.item_policy(key,rev).map(|p|p.record());
 Some(crate::map::facts::ItemDefinition{reference:index.definition_ref("Item",key,rev)?,solid:policy.and_then(|p|p.attributes.blocks_movement),blocks_projectile:policy.and_then(|p|p.attributes.blocks_projectile).unwrap_or(true),pickupable:policy.is_some_and(|p|matches!(&p.semantics.physical,F::Known(p) if p.pickupable==F::Known(true))),door:policy.is_some_and(|p|matches!(&p.semantics.classification,F::Known(c) if c.item_type==F::Known(ReferenceItemType::Door)))})
 }).map_err(debug)?;
    Ok((boot, table))
}

use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, ConnectionEnd, IDLE_LIVENESS, serve_admitted},
    tcp_tls::{read_frame, write_frame},
};
use oteryn_protocol_oteryn::npc_service::{
    NpcIntentDisposition, NpcTalkIntent, NpcTradeIntent, decode_npc_intent_result,
    encode_npc_talk_intent, encode_npc_trade_intent,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::AsyncWriteExt;
pub(super) async fn trade(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    first_id: u64,
    intent: &NpcTradeIntent,
    replay: bool,
) -> TestResult<Vec<NpcIntentDisposition>> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let before = *admitted;
    let viewer = {
        let runtime = owner.runtime.lock().await;
        let (actor, position) = runtime
            .positioned_player_for_session(before.game_session_id)
            .map_err(debug)?
            .ok_or("actual retailer viewer")?;
        let p = position.position();
        serde_json::json!({"actor_identity":actor.placement_identity(),"actor_generation":actor.actor_local_generation(),"position":[p.x,p.y,p.floor],"world":hex(runtime.binding().world_id().as_bytes()),"scope_generation":runtime.binding().scope_generation().get(),"content_generation":hex(&runtime.content_pin().client_artifact_digest()),"server_artifact_digest":hex(&runtime.content_pin().server_artifact_digest()),"selected_capabilities":before.continuity.selected_capabilities.as_slice()})
    };
    let mut commands = vec![
        (
            first_id,
            7,
            encode_npc_talk_intent(&NpcTalkIntent {
                npc_actor: intent.npc_actor,
                text: "hi".into(),
            })
            .map_err(debug)?,
        ),
        (
            first_id + 1,
            7,
            encode_npc_talk_intent(&NpcTalkIntent {
                npc_actor: intent.npc_actor,
                text: "trade".into(),
            })
            .map_err(debug)?,
        ),
        (
            first_id + 2,
            8,
            encode_npc_trade_intent(intent).map_err(debug)?,
        ),
    ];
    if replay {
        commands.push((
            first_id + 2,
            8,
            encode_npc_trade_intent(intent).map_err(debug)?,
        ));
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async {
        let mut capture = Vec::new();
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            capture_packet(&frame, &mut capture)?;
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        let mut results = Vec::new();
        for (command_id, command_type, payload) in commands {
            write_frame(
                &mut client,
                &encode_client_command(
                    before.continuity.connection_generation,
                    &ClientCommandValue {
                        command_id,
                        command_type,
                        payload: &payload,
                    },
                )
                .map_err(debug)?,
            )
            .await?;
            loop {
                let frame = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                capture_packet(&frame, &mut capture)?;
                if envelope.message_type() != MessageType::CommandResult {
                    continue;
                }
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, command_id);
                let disposition = decode_npc_intent_result(result.payload).map_err(debug)?;
                if command_type == 8 {
                    results.push(disposition);
                } else {
                    assert_eq!(result.status, CommandStatus::Accepted);
                    assert_eq!(disposition, NpcIntentDisposition::Ok);
                }
                break;
            }
        }
        client.shutdown().await?;
        Ok((results, capture))
    };
    let (ended, result): (
        _,
        TestResult<(Vec<NpcIntentDisposition>, Vec<serde_json::Value>)>,
    ) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::super::fixture::join_two(serving, driving),
    )
    .await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = &ended
    {
        *admitted = *current;
    }
    let (results, capture) =
        result.map_err(|error| format!("actual retailer shop ended {ended:?}: {error}"))?;
    println!(
        "SRV7_FURNITURE_RETAILER_ACTUAL_CAPTURE {}",
        serde_json::json!({"source":{"producer_test":"srv7_npc_edvard_actual_world_placement_wraps_birdcage_and_replays","npc":"oteryn:npc.edvard","donor_revision":"201189f871d23597b82caf88fd611454a2ddefb8","source_position":[33229,31832,7]},"viewer":viewer,"packets":capture})
    );
    Ok(results)
}

fn capture_packet(frame: &[u8], output: &mut Vec<serde_json::Value>) -> TestResult {
    let envelope = decode_wire_envelope(frame).map_err(debug)?;
    if matches!(
        envelope.message_type(),
        MessageType::SnapshotBegin
            | MessageType::SnapshotChunk
            | MessageType::SnapshotCommit
            | MessageType::StateDelta
            | MessageType::CommandResult
    ) {
        output.push(serde_json::json!({"kind":"wire","message_type":format!("{:?}",envelope.message_type()),"server_sequence":envelope.server_sequence(),"envelope_hex":hex(frame),"payload_hex":hex(envelope.payload())}));
    }
    match envelope.message_type() {
        MessageType::SnapshotChunk => {
            let (_, domains) =
                oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
            for domain in domains {
                if [1, 7, 8, 9, 17].contains(&domain.domain_id) {
                    output.push(serde_json::json!({"kind":"snapshot","domain_id":domain.domain_id,"revision":domain.revision,"snapshot_type":domain.snapshot_type,"server_sequence":envelope.server_sequence(),"payload_hex":hex(domain.payload)}));
                }
            }
        }
        MessageType::StateDelta => {
            let delta =
                oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
            if [1, 7, 8, 9, 17].contains(&delta.domain_id) {
                output.push(serde_json::json!({"kind":"delta","domain_id":delta.domain_id,"base_revision":delta.base_revision,"new_revision":delta.new_revision,"delta_type":delta.delta_type,"server_sequence":envelope.server_sequence(),"payload_hex":hex(delta.payload),"state_delta_hex":hex(envelope.payload())}));
            }
        }
        _ => {}
    }
    Ok(())
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

use super::*;
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use oteryn_protocol_oteryn::npc_service::{
    NpcIntentDisposition, NpcTalkIntent, decode_npc_intent_result, encode_npc_talk_intent,
    NpcTradeIntent, encode_npc_trade_intent,
};
use oteryn_protocol_oteryn::{ClientCommandValue, CommandStatus, decode_command_result, encode_client_command};
use tokio::io::AsyncWriteExt;

/// Capability 3 is offered only on a bundle World. Keep the native NPC test on
/// that production route, with a tiny source-qualified ground plane and no items.
pub(super) fn world(world: crate::foundation::WorldId) -> TestResult<crate::map::boot::BundleWorld> {
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut bundle = oteryn_world_bundle::bundle::read(&bytes)?;
    bundle.manifest.palette.truncate(1);
    bundle.manifest.palette[0].key = "oteryn:item.tibia.i100".into();
    for sector in &mut bundle.sectors {
        sector.tiles.retain(|tile| tile.x <= 3);
        for tile in &mut sector.tiles { tile.items.retain(|item| item.palette == 0); }
    }
    let bytes = oteryn_world_bundle::writer::write(&bundle.manifest, &bundle.sectors, &Default::default())?;
    pins.digest = oteryn_world_bundle::bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(&bytes, &crate::map::boot::BootPins {
        map_revision: crate::map::overlay::map_revision(&base), bundle: pins,
        start: crate::map::overlay::TilePos { x: 2, y: 0, floor: -7 },
    }, world, crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?, |_| None).map_err(debug)
}

/// Keep one conversation for a complete choice/confirmation sequence. A new connection
/// greets again; session command receipts still deduplicate repeated command ids.
pub(super) async fn talk(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    npc: EntityRef, commands: &[(u64, &str)],
) -> TestResult {
    conversation(owner, admitted, npc, commands, Some(NpcIntentDisposition::Ok), None).await
}

pub(super) async fn talk_refused(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    npc: EntityRef, commands: &[(u64, &str)],
) -> TestResult {
    conversation(owner, admitted, npc, commands, Some(NpcIntentDisposition::Rejected), None).await
}

/// Park foreign work only after this connection has passed its retained-NPC recovery
/// and the task choice has been acknowledged. The confirmation then reaches the
/// generic pre-mutation lane gate without bypassing the production recovery owner.
pub(super) async fn talk_refused_with_parked_confirmation(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    npc: EntityRef, commands: &[(u64, &str)],
) -> TestResult {
    let (command_id, text) = commands.last().ok_or("confirmation command")?;
    assert_eq!(*text, "yes");
    conversation(owner, admitted, npc, commands, Some(NpcIntentDisposition::Rejected), Some(*command_id)).await
}

pub(super) async fn talk_interrupted(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    npc: EntityRef, commands: &[(u64, &str)],
) -> TestResult {
    conversation(owner, admitted, npc, commands, None, None).await
}

async fn conversation(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    npc: EntityRef, commands: &[(u64, &str)], expected_last: Option<NpcIntentDisposition>,
    park_before: Option<u64>,
) -> TestResult {
    let fresh = commands.iter().filter(|(id, _)| *id >= admitted.continuity.next_command_id)
        .map(|(id, _)| *id).collect::<std::collections::BTreeSet<_>>();
    // Retained command replay exits ingress before NPC dispatch and does not consume
    // the four-utterance source window.
    assert!(fresh.len() <= 4, "source NPC talk window");
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let before = *admitted;
    let parked_lane = park_before.map(|id| (id, owner.spell_lane.clone()));
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async {
        let mut expected_next = before.continuity.next_command_id;
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        for (offset, (command_id, text)) in commands.iter().enumerate() {
            let last = offset + 1 == commands.len();
            let replay = *command_id < expected_next;
            if !replay { expected_next = command_id.checked_add(1).ok_or("command exhaustion")?; }
            if let Some((target, lane)) = parked_lane.as_ref() && command_id == target {
                assert!(last && !replay, "park only before the fresh confirmation");
                let mut permit = lane.acquire().await.map_err(|_| "unexpected parked lane")?;
                permit.open_commit_window((), |_| Box::new(())).park();
                drop(permit);
            }
            let payload = encode_npc_talk_intent(&NpcTalkIntent { npc_actor: npc, text: (*text).into() }).map_err(debug)?;
            let frame = encode_client_command(before.continuity.connection_generation,
                &ClientCommandValue { command_id: *command_id, command_type: 7, payload: &payload }).map_err(debug)?;
            write_frame(&mut client, &frame).await?;
            loop {
                let frame = match read_frame(&mut client).await {
                    Err(error) if last && expected_last.is_none() && error.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
                    result => result?,
                };
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, *command_id);
                    let expected = if last { expected_last.ok_or("unknown command was sequenced")? }
                        else { NpcIntentDisposition::Ok };
                    let status = if replay { CommandStatus::DuplicateReplay }
                        else if expected == NpcIntentDisposition::Rejected { CommandStatus::Rejected }
                        else { CommandStatus::Accepted };
                    assert_eq!(result.status, status, "{command_id}: {text}; NPC payload {:?}", result.payload);
                    assert_eq!(decode_npc_intent_result(result.payload).map_err(debug)?,
                        expected, "{command_id}: {text}");
                    break;
                }
            }
        }
        client.shutdown().await?;
        Ok(())
    };
    let (ended, result): (_, TestResult) = tokio::time::timeout(std::time::Duration::from_secs(30),
        super::super::fixture::join_two(serving, driving)).await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current) | ConnectionEnd::AdmittedThenClosed(current, _) = &ended {
        *admitted = *current;
    }
    result.map_err(|error| format!("Grizzly conversation ended {ended:?}: {error}").into())
}

/// Real command 8 after command 7 opens the standard window. A fixture-only point change
/// between those commands proves that the display cache is never mutation authority.
pub(super) async fn trade(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    h: &Harness, first_id: u64, intent: &NpcTradeIntent,
    points_after_window: Option<i64>, replay: bool,
) -> TestResult<Vec<NpcIntentDisposition>> {
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let before = *admitted;
    let mut commands = vec![
        (first_id, 7, encode_npc_talk_intent(&NpcTalkIntent { npc_actor: intent.npc_actor, text: "hi".into() }).map_err(debug)?),
        (first_id + 1, 7, encode_npc_talk_intent(&NpcTalkIntent { npc_actor: intent.npc_actor, text: "trade".into() }).map_err(debug)?),
        (first_id + 2, 8, encode_npc_trade_intent(intent).map_err(debug)?),
    ];
    if replay { commands.push((first_id + 2, 8, encode_npc_trade_intent(intent).map_err(debug)?)); }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let ended = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type() == MessageType::SnapshotCommit { break; }
        }
        let mut results = Vec::new();
        for (command_id, command_type, payload) in commands {
            write_frame(&mut client, &encode_client_command(before.continuity.connection_generation,
                &ClientCommandValue { command_id, command_type, payload: &payload }).map_err(debug)?).await?;
            loop {
                let frame = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() != MessageType::CommandResult { continue; }
                let result = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(result.command_id, command_id);
                let disposition = decode_npc_intent_result(result.payload).map_err(debug)?;
                if command_type == 8 { results.push(disposition); }
                else {
                    assert_eq!(result.status, CommandStatus::Accepted);
                    assert_eq!(disposition, NpcIntentDisposition::Ok);
                }
                break;
            }
            if command_id == first_id + 1 && let Some(points) = points_after_window {
                seed(h, &[(quest::POINTS_STORAGE, points)]).await?;
            }
        }
        client.shutdown().await?;
        Ok(results)
    };
    let (ended, result): (_, TestResult<Vec<NpcIntentDisposition>>) = tokio::time::timeout(
        std::time::Duration::from_secs(30), super::super::fixture::join_two(serving, driving)).await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current) | ConnectionEnd::AdmittedThenClosed(current, _) = &ended {
        *admitted = *current;
    }
    result.map_err(|error| format!("Grizzly shop ended {ended:?}: {error}").into())
}

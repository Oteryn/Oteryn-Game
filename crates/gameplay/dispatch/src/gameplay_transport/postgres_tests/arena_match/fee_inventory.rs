use super::{A, AdmittedSession, ComposedFreshAdmission, D, TestResult, debug, wire};
use crate::gameplay_transport::connection::{IDLE_LIVENESS, serve_admitted};
use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
use oteryn_protocol_oteryn::item_view::{
    DELTA_TYPE_CHARACTER_INVENTORY_V1, STATE_DOMAIN_CHARACTER_INVENTORY, decode_character_inventory,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, MessageType, decode_command_result, decode_snapshot_chunk,
    decode_state_delta, decode_wire_envelope, encode_client_command_for_session,
};

pub(super) async fn enter_and_check_inventory_frame(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
) -> TestResult<wire::ArenaResult> {
    let (mut server, mut client) = tokio::io::duplex(1 << 16);
    let serving = async move { serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS).await };
    let driving = async move {
        let mut initial_inventory = None;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            match envelope.message_type() {
                MessageType::SnapshotChunk => {
                    let (_, domains) = decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
                    for domain in domains {
                        if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                            initial_inventory =
                                Some(decode_character_inventory(domain.payload).map_err(debug)?);
                        }
                    }
                }
                MessageType::SnapshotCommit => break,
                _ => {}
            }
        }
        let initial = initial_inventory.ok_or("arena inventory snapshot")?;
        assert_eq!(initial.entries.len(), 2, "10 platinum plus 50 gold");
        assert!(initial.entries.iter().any(|entry| entry.count.get() == 10));
        let gold_definition = initial
            .entries
            .iter()
            .find(|entry| entry.count.get() == 50)
            .ok_or("initial gold stack")?
            .item_definition_ref;

        let command_id = admitted.continuity.next_command_id;
        let status_id = command_id.checked_add(1).ok_or("status command ID")?;
        let command = |id, action| -> TestResult<Vec<u8>> {
            let payload = wire::encode_match_request(action).map_err(debug)?;
            encode_client_command_for_session(
                admitted.continuity.connection_generation,
                &ClientCommandValue {
                    command_id: id,
                    command_type: wire::COMMAND_TYPE_ARENA_MATCH,
                    payload: &payload,
                },
                admitted.game_session_id.as_bytes(),
            )
            .map_err(debug)
        };
        write_frame(&mut client, &command(command_id, A::Enter)?).await?;
        let mut entered = None;
        let mut inventory_after_fee = None;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            match envelope.message_type() {
                MessageType::CommandResult => {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.status, CommandStatus::Accepted);
                    if result.command_id == status_id {
                        break;
                    }
                    assert_eq!(result.command_id, command_id);
                    let entry = wire::decode_match_result(result.payload).map_err(debug)?;
                    assert_eq!(entry.disposition, D::Applied);
                    entered = Some(entry);
                    // A later command is a barrier for the entry's same-turn deltas.
                    write_frame(&mut client, &command(status_id, A::Status)?).await?;
                }
                MessageType::StateDelta => {
                    let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
                    if delta.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                        assert!(
                            entered.is_some(),
                            "the entry result precedes its inventory frame"
                        );
                        assert_eq!(delta.delta_type, DELTA_TYPE_CHARACTER_INVENTORY_V1);
                        assert!(inventory_after_fee.is_none());
                        inventory_after_fee =
                            Some(decode_character_inventory(delta.payload).map_err(debug)?);
                    }
                }
                _ => {}
            }
        }
        let inventory = inventory_after_fee.ok_or("W17_arena_fee_inventory_frame_missing")?;
        let [gold] = inventory.entries.as_slice() else {
            return Err("arena fee must leave only the gold stack".into());
        };
        assert_eq!(gold.item_definition_ref, gold_definition);
        assert_eq!(gold.count.get(), 50);
        entered.ok_or_else(|| "arena entry result".into())
    };
    let (_, result) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::super::fixture::join_two(serving, driving),
    )
    .await?;
    result
}

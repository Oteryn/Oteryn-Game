//! SEAM-1: deferred admission enrichment must not terminate the command stream.
use super::*;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::wheel as wire;
use std::sync::Arc;

#[test]
fn seam_missing_progression_wheel_query_keeps_connection() -> TestResult {
    scenario(false)
}

#[test]
fn seam_busy_admission_wheel_query_keeps_connection() -> TestResult {
    scenario(true)
}

fn scenario(initialized: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async |admin| {
                let h = Harness::create_with_admission_seed(
                    admin,
                    "seam_wheel_join",
                    initialized,
                    false,
                )
                .await?;
                let request = prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let deps = Dependencies::new()?;
                let mut owner = deps.owner(&h, &authority);
                let data = oteryn_gameplay_progression::wheel_gem_data::WheelGemData::embedded()
                    .map_err(debug)?;
                owner.wheel_ruleset = Some(Arc::new(
                    crate::durability::character_wheel::WheelRuleset::from_catalogue(
                        data.revision(),
                        data.wheel(),
                    )
                    .map_err(debug)?,
                ));
                super::tutorial_vitals_fixture::admit(&owner, &deps, &request).await?;
                let session = deps.admitted.game_session_id;
                owner.refresh_quest_session(session, true).await;
                let held = if initialized {
                    Some(h.root.try_issue_semantic_pass()?)
                } else {
                    None
                };
                owner.admit_quest_session(&deps.admitted).await;
                assert!(
                    owner
                        .states
                        .wheel_sessions
                        .lock()
                        .map_err(debug)?
                        .get(&session)
                        .is_some_and(|allocation| if initialized {
                            allocation.is_none()
                        } else {
                            allocation.as_ref().is_some_and(|allocation| {
                                allocation.current
                                    && allocation.wheel_revision == 0
                                    && allocation.slots
                                        == crate::durability::character_wheel::WheelSlots::ZERO
                            })
                        })
                );
                if let Some(held) = held {
                    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                }
                let current = FreshAdmissionStore::from_root(h.root.clone())
                    .current_session(session)
                    .await?;
                let mut admitted = deps.admitted;
                admitted.first_entry = FirstEntryOutcome::Positioned;
                admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
                admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                    owner.offered_capabilities(),
                    &[wire::CAPABILITY_WHEEL_ALLOCATION_V1],
                )
                .ok_or("Wheel capability")?;
                let query = wire::encode_wheel_request(&wire::WheelAllocationRequest {
                    action: 0,
                    occurrence: [0; 16],
                    expected_revision: 0,
                    points: [0; 36],
                    vessels: None,
                })
                .map_err(debug)?;
                query_then_command(&owner, admitted, &query, !initialized).await?;
                drop(owner);
                drop(seal);
                h.cleanup().await
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Wheel admission scenario panicked")??;
    Ok(())
}

async fn query_then_command(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    query: &[u8],
    query_has_allocation: bool,
) -> TestResult {
    use crate::foundation::{MessageType, decode_wire_envelope};
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
    use oteryn_protocol_oteryn::{
        ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
    };
    use tokio::io::AsyncWriteExt;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = Box::pin(serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS)).await;
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
        // Both results must arrive on this physical connection. Only then does the
        // peer close; a deferred Wheel must not turn server readiness into a wire fault.
        for (command_id, command_type, payload) in [
            (1, wire::COMMAND_TYPE_WHEEL_ALLOCATION, query),
            (2, 0x7fff, &[][..]),
        ] {
            write_frame(
                &mut client,
                &encode_client_command(
                    admitted.continuity.connection_generation,
                    &ClientCommandValue {
                        command_id,
                        command_type,
                        payload,
                    },
                )
                .map_err(debug)?,
            )
            .await?;
            loop {
                let frame = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                assert_ne!(envelope.message_type(), MessageType::ProtocolError);
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, command_id);
                    if command_id == 1 && query_has_allocation {
                        // A query reads the known empty allocation; progression eligibility
                        // gates mutations, not this read-only projection (WHEEL-0 §4 Load).
                        assert_eq!(result.status, CommandStatus::Accepted);
                        let expected = wire::encode_wheel_reply(&wire::WheelAllocationReply {
                            refused: false,
                            revision: 0,
                            points: [0; 36],
                            vessels: None,
                        })
                        .map_err(debug)?;
                        assert_eq!(result.payload, expected.as_slice());
                    } else {
                        assert_eq!(result.status, CommandStatus::Rejected);
                        assert!(result.payload.is_empty());
                    }
                    break;
                }
            }
        }
        client.shutdown().await?;
        Ok(())
    };
    let (end, result): (_, TestResult) =
        tokio::time::timeout(Duration::from_secs(5), fixture::join_two(serving, driving)).await?;
    result?;
    assert!(
        matches!(end, ConnectionEnd::AdmittedThenDisconnected(current) if current.continuity.next_command_id == 3),
        "{end:?}"
    );
    Ok(())
}

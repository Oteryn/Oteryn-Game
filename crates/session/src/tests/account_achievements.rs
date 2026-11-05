use super::*;
use oteryn_protocol_oteryn::account_achievements::{
    self as wire, AccountAchievementsQuery, AccountAchievementsResult,
};

async fn peer(
    mut stream: DuplexStream,
    status: CommandStatus,
    payload: Vec<u8>,
) -> Result<(), BoxError> {
    let bootstrap = read_frame(&mut stream).await?;
    assert_eq!(
        decode_wire_envelope(&bootstrap)?
            .client_bootstrap()?
            .admission_material,
        b"grant"
    );
    write_frame(
        &mut stream,
        &encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&uuid_v7(1))?,
            world_id: WorldId::decode(&uuid_v7(2))?,
            channel_id: ChannelId::decode(&uuid_v7(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 7,
            schema_revision: 1,
            selected_capabilities: &[],
        })?,
    )
    .await?;
    let spatial_payload = spatial(0);
    let overlay_payload = encode_world_object_overlay_snapshot(&[WireOverlayEntry {
        content_generation: CONTENT_GENERATION,
        placement: b"oteryn:cell/entry-door".to_vec(),
        state: b"oteryn:reference.state.closed".to_vec(),
        revision: 0,
    }])
    .map_err(|error| format!("overlay snapshot: {error:?}"))?;
    for frame in encode_single_chunk_snapshot(
        1,
        1,
        40,
        &[
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 5,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &spatial_payload,
            },
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 2,
                snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload: &overlay_payload,
            },
        ],
    )? {
        write_frame(&mut stream, &frame).await?;
    }

    let bytes = read_frame(&mut stream).await?;
    let command = decode_wire_envelope(&bytes)?.client_command(1)?;
    assert_eq!((command.command_id, command.command_type), (7, 10));
    assert_eq!(
        wire::decode_account_achievements_query(command.payload),
        Ok(AccountAchievementsQuery { page: 3 })
    );
    write_frame(
        &mut stream,
        &encode_command_result(1, 41, 7, status, &payload)?,
    )
    .await?;
    Ok(())
}
fn scenario(
    status: CommandStatus,
    payload: Vec<u8>,
    wanted: Option<account_achievements::Response>,
) -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            block_on(async {
                let (client, server) = tokio::io::duplex(65536);
                let task = tokio::spawn(peer(server, status, payload));
                let mut session = Session::admit(client, admission()?).await?;
                let result = session.account_achievements(3).await;
                if let Some(wanted) = wanted {
                    assert_eq!(result?, wanted);
                } else {
                    assert!(matches!(
                        result,
                        Err(SessionError::Protocol(
                            FoundationProtocolError::MalformedFrame
                        ))
                    ));
                    assert!(matches!(
                        session.account_achievements(0).await,
                        Err(SessionError::SessionUnusable)
                    ));
                }
                task.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "achievements session scenario panicked")?
}
#[test]
fn account_achievements_core_command_needs_no_notice_capability() -> Result<(), BoxError> {
    let page = AccountAchievementsResult {
        page: 3,
        total_points: 42,
        ..Default::default()
    };
    scenario(
        CommandStatus::Accepted,
        wire::encode_account_achievements_result(&page).map_err(|e| format!("{e:?}"))?,
        Some(account_achievements::Response::Page(page)),
    )?;
    scenario(
        CommandStatus::Rejected,
        vec![],
        Some(account_achievements::Response::Refused),
    )
}
#[test]
fn account_achievements_malformed_or_wrong_page_poisons_session() -> Result<(), BoxError> {
    scenario(CommandStatus::Accepted, vec![255], None)?;
    scenario(CommandStatus::Accepted, vec![], None)
}

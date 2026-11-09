//! Initial destination and recovery projection regressions.
use super::*;

#[test]
fn srx3_activity_scope_rejects_changed_destination_projection_atomically() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let (client, mut peer) = tokio::io::duplex(256 * 1024);
                let server = tokio::spawn(async move {
                    join_peer_with(&mut peer, &[117], &[117], &[], (1, spatial(100)), &[]).await?;
                    let scope = switch(1, 10, 1, false);
                    publish_switch(&mut peer, 1, 41, &scope).await?;
                    let mut changed = scope;
                    changed.handoff = uuid_v7(99);
                    destination_projection(&mut peer, 1, 200, changed).await?;
                    let mut rest = Vec::new();
                    let _ = peer.read_to_end(&mut rest).await;
                    Ok::<(), BoxError>(())
                });
                let mut session = Session::admit(client, admission_with(&[117])?).await?;
                assert!(session.wait_for_events(DEADLINE).await.is_err());
                assert_eq!(
                    session.join_snapshot().game_session_id,
                    GameSessionId::decode(&uuid_v7(1))?
                );
                assert_eq!(session.world_spatial().actor_position.x, 100);
                assert_eq!(session.next_command_id(), 7);
                drop(session);
                server.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "scope projection thread panicked")?
}

#[test]
fn srx3_activity_scope_cold_current_projection_binds_without_resetting_command_cursor()
-> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let (client, mut peer) = tokio::io::duplex(256 * 1024);
                let server = tokio::spawn(async move {
                    read_frame(&mut peer).await?;
                    write_frame(
                        &mut peer,
                        &encode_server_accepted(&ServerAcceptedValue {
                            game_session_id: GameSessionId::decode(&uuid_v7(1))?,
                            world_id: WorldId::decode(&uuid_v7(2))?,
                            channel_id: ChannelId::decode(&uuid_v7(3))?,
                            connection_generation: 1,
                            current_server_sequence: 0,
                            next_command_id: 7,
                            schema_revision: 1,
                            selected_capabilities: &[117],
                        })?,
                    )
                    .await?;
                    destination_projection(&mut peer, 1, 200, switch(90, 1, 1, false)).await?;
                    let frame = read_frame(&mut peer).await?;
                    let envelope = decode_wire_envelope(&frame)?;
                    assert_eq!(envelope.client_command(1)?.command_id, 7);
                    assert_eq!(
                        envelope.client_command_session_id()?,
                        Some(uuid_v7(1).as_slice())
                    );
                    write_frame(
                        &mut peer,
                        &encode_command_result(
                            1,
                            2,
                            7,
                            CommandStatus::Accepted,
                            &encode_step_result(StepDisposition::Blocked),
                        )?,
                    )
                    .await?;
                    Ok::<(), BoxError>(())
                });
                let mut session = Session::admit(client, admission_with(&[117])?).await?;
                assert_eq!(session.next_command_id(), 7);
                assert_eq!(session.last_server_sequence(), 1);
                assert_eq!(session.activity_scope(), Some(&switch(90, 1, 1, false)));
                assert_eq!(
                    session.step(StepDirection::North).await?.disposition,
                    StepDisposition::Blocked
                );
                server.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "cold scope projection thread panicked")?
}

fn combat_state(target: Option<EntityRef>, in_fight: bool) -> ActorCombatState {
    ActorCombatState {
        target,
        modes: FightModes::DEFAULT,
        in_fight,
    }
}

fn target_ref() -> EntityRef {
    EntityRef {
        identity: [7; world_spatial_entities::ENTITY_IDENTITY_BYTES],
        generation: 3,
    }
}

/// Capability 17 is selected: attack-target and fight-mode intents reach the wire with their
/// codec payloads, each result is decoded, and domain 10 (snapshot and delta) is kept.
async fn attack_server(mut stream: DuplexStream, with_combat_state: bool) -> Result<(), BoxError> {
    read_frame(&mut stream).await?;
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
            selected_capabilities: &[
                CAPABILITY_WORLD_SPATIAL_ENTITIES,
                attack::CAPABILITY_ATTACK_V1,
            ],
        })?,
    )
    .await?;
    let (spatial_type, spatial_payload) = entity_snapshot(vec![]);
    let overlay = encode_world_object_overlay_snapshot(&[])
        .map_err(|error| format!("overlay snapshot: {error:?}"))?;
    let combat = attack::encode_actor_combat_state(&combat_state(None, false))
        .map_err(|error| format!("combat: {error:?}"))?;
    let mut domains = vec![
        DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            revision: 5,
            snapshot_type: spatial_type,
            payload: &spatial_payload,
        },
        DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            revision: 2,
            snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
            payload: &overlay,
        },
    ];
    if with_combat_state {
        domains.push(DomainSnapshot {
            domain_id: attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
            revision: 1,
            snapshot_type: attack::SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1,
            payload: &combat,
        });
    }
    for frame in encode_single_chunk_snapshot(1, 1, 40, &domains)? {
        write_frame(&mut stream, &frame).await?;
    }
    let command = read_frame(&mut stream).await?;
    let command = decode_wire_envelope(&command)?.client_command(1)?;
    assert_eq!(
        (command.command_id, command.command_type),
        (7, attack::COMMAND_TYPE_ATTACK_TARGET_INTENT)
    );
    assert_eq!(
        attack::decode_attack_target_intent(command.payload),
        Ok(Some(target_ref()))
    );
    write_frame(
        &mut stream,
        &encode_command_result(
            1,
            41,
            7,
            CommandStatus::Accepted,
            &attack::encode_attack_intent_result(AttackIntentDisposition::Ok),
        )?,
    )
    .await?;
    write_frame(
        &mut stream,
        &encode_state_delta(
            1,
            42,
            attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
            1,
            2,
            attack::DELTA_TYPE_ACTOR_COMBAT_STATE_V1,
            &attack::encode_actor_combat_state(&combat_state(Some(target_ref()), true))
                .map_err(|error| format!("combat: {error:?}"))?,
        )?,
    )
    .await?;
    let command = read_frame(&mut stream).await?;
    let command = decode_wire_envelope(&command)?.client_command(1)?;
    assert_eq!(
        (command.command_id, command.command_type),
        (8, attack::COMMAND_TYPE_FIGHT_MODES_INTENT)
    );
    assert_eq!(
        attack::decode_fight_modes_intent(command.payload),
        Ok(FightModes {
            fight_mode: FightMode::Offensive,
            chase: ChaseMode::Stand,
            secure: true,
        })
    );
    write_frame(
        &mut stream,
        &encode_command_result(
            1,
            43,
            8,
            CommandStatus::Rejected,
            &attack::encode_attack_intent_result(AttackIntentDisposition::Rejected),
        )?,
    )
    .await?;
    Ok(())
}

#[test]
fn attack_and_fight_mode_intents_are_sent_and_the_combat_state_is_kept() -> Result<(), BoxError> {
    block_on(async {
        let (client, server) = tokio::io::duplex(64 * 1024);
        let server = tokio::spawn(attack_server(server, true));
        let mut admission = admission()?;
        admission.supported_capabilities = &[
            CAPABILITY_WORLD_SPATIAL_ENTITIES,
            attack::CAPABILITY_ATTACK_V1,
        ];
        let mut session = Session::admit(client, admission).await?;
        assert_eq!(session.combat_state(), Some(&combat_state(None, false)));
        let attack = session.attack_target(Some(&target_ref())).await?;
        assert_eq!(attack.status, CommandStatus::Accepted);
        assert_eq!(attack.disposition, AttackIntentDisposition::Ok);
        // The delta follows the result; it applies on the next read, here the next command.
        let modes = FightModes {
            fight_mode: FightMode::Offensive,
            chase: ChaseMode::Stand,
            secure: true,
        };
        let refused = session.set_fight_modes(&modes).await?;
        server.await??;
        assert_eq!(refused.status, CommandStatus::Rejected);
        assert_eq!(refused.disposition, AttackIntentDisposition::Rejected);
        assert_eq!(
            session.combat_state(),
            Some(&combat_state(Some(target_ref()), true))
        );
        assert!(matches!(
            session.take_events().as_slice(),
            [SessionEvent::ActorCombatState(delta)] if delta.new_revision == 2
        ));
        Ok(())
    })?
}

#[test]
fn a_join_snapshot_without_domain_10_is_refused_when_capability_17_is_selected()
-> Result<(), BoxError> {
    block_on(async {
        let (client, server) = tokio::io::duplex(64 * 1024);
        let server = tokio::spawn(attack_server(server, false));
        let mut admission = admission()?;
        admission.supported_capabilities = &[
            CAPABILITY_WORLD_SPATIAL_ENTITIES,
            attack::CAPABILITY_ATTACK_V1,
        ];
        let refused = Session::admit(client, admission).await.err();
        server.abort();
        assert!(matches!(
            refused,
            Some(SessionError::MissingDomain(
                attack::STATE_DOMAIN_ACTOR_COMBAT_STATE
            ))
        ));
        Ok(())
    })?
}

#[test]
fn attack_intents_are_refused_before_sending_when_capability_17_is_not_selected()
-> Result<(), BoxError> {
    block_on(async {
        let (mut session, _peer) = vitals_pair().await?;
        assert!(matches!(
            session.attack_target(None).await,
            Err(SessionError::CapabilityNotSelected { capability: 17 })
        ));
        assert_eq!(session.next_command_id(), 7);
        Ok(())
    })?
}

/// Admits with capability 21 selected, then answers each logout with the next `results`
/// entry (`None` is a `REJECTED` with an empty payload), checking the intent on the wire.
async fn logout_server(
    mut stream: DuplexStream,
    results: Vec<(CommandStatus, Option<LogoutResult>)>,
) -> Result<(), BoxError> {
    read_frame(&mut stream).await?;
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
            selected_capabilities: &[logout::CAPABILITY_LOGOUT_V1],
        })?,
    )
    .await?;
    let overlay = encode_world_object_overlay_snapshot(&[])
        .map_err(|error| format!("overlay snapshot: {error:?}"))?;
    let spatial = spatial(0);
    let domains = [
        DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            revision: 5,
            snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
            payload: &spatial,
        },
        DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            revision: 2,
            snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
            payload: &overlay,
        },
    ];
    for frame in encode_single_chunk_snapshot(1, 1, 40, &domains)? {
        write_frame(&mut stream, &frame).await?;
    }
    for (index, (status, result)) in (0_u64..).zip(results) {
        let command = read_frame(&mut stream).await?;
        let command = decode_wire_envelope(&command)?.client_command(1)?;
        assert_eq!(
            (command.command_id, command.command_type),
            (7 + index, logout::COMMAND_TYPE_LOGOUT_INTENT)
        );
        assert!(command.payload.is_empty());
        let payload = match result {
            Some(result) => logout::encode_logout_result(result)
                .map_err(|error| format!("logout: {error:?}"))?,
            None => Vec::new(),
        };
        write_frame(
            &mut stream,
            &encode_command_result(1, 41 + index, 7 + index, status, &payload)?,
        )
        .await?;
    }
    Ok(())
}

async fn logout_session(
    results: Vec<(CommandStatus, Option<LogoutResult>)>,
) -> Result<
    (
        Session<DuplexStream>,
        tokio::task::JoinHandle<Result<(), BoxError>>,
    ),
    BoxError,
> {
    let (client, server) = tokio::io::duplex(64 * 1024);
    let server = tokio::spawn(logout_server(server, results));
    let mut admission = admission()?;
    admission.supported_capabilities = &[logout::CAPABILITY_LOGOUT_V1];
    Ok((Session::admit(client, admission).await?, server))
}

#[test]
fn a_refused_logout_keeps_the_session_playing_and_an_accepted_one_ends_it() -> Result<(), BoxError>
{
    block_on(async {
        let (mut session, server) = logout_session(vec![
            (
                CommandStatus::Rejected,
                Some(LogoutResult::InFight {
                    retry_after_ms: 1_500,
                }),
            ),
            (CommandStatus::Rejected, Some(LogoutResult::Busy)),
            (CommandStatus::Rejected, None),
            (CommandStatus::Accepted, Some(LogoutResult::Accepted)),
        ])
        .await?;
        let in_fight = session.logout().await?;
        assert_eq!(
            (in_fight.command_id, in_fight.status, in_fight.result),
            (
                7,
                CommandStatus::Rejected,
                Some(LogoutResult::InFight {
                    retry_after_ms: 1_500
                })
            )
        );
        assert_eq!(session.logout().await?.result, Some(LogoutResult::Busy));
        assert_eq!(session.logout().await?.result, None);
        let accepted = session.logout().await?;
        server.await??;
        assert_eq!(
            (
                accepted.status,
                accepted.result,
                accepted.result_server_sequence
            ),
            (CommandStatus::Accepted, Some(LogoutResult::Accepted), 44)
        );
        // The server ended the session: nothing further is sent on it.
        assert!(matches!(
            session.logout().await,
            Err(SessionError::SessionUnusable)
        ));
        assert_eq!(session.next_command_id(), 11);
        Ok(())
    })?
}

#[test]
fn a_logout_result_whose_status_disagrees_with_its_code_fails_the_session() -> Result<(), BoxError>
{
    block_on(async {
        let (mut session, server) =
            logout_session(vec![(CommandStatus::Accepted, Some(LogoutResult::Busy))]).await?;
        assert!(matches!(
            session.logout().await,
            Err(SessionError::InconsistentCommandResult {
                command_id: 7,
                status: CommandStatus::Accepted
            })
        ));
        server.await??;
        assert!(matches!(
            session.logout().await,
            Err(SessionError::SessionUnusable)
        ));
        Ok(())
    })?
}

#[test]
fn a_logout_is_refused_before_sending_when_capability_21_is_not_selected() -> Result<(), BoxError> {
    block_on(async {
        let (mut session, _peer) = vitals_pair().await?;
        assert!(matches!(
            session.logout().await,
            Err(SessionError::CapabilityNotSelected { capability: 21 })
        ));
        assert_eq!(session.next_command_id(), 7);
        Ok(())
    })?
}

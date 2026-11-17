#[test]
fn a_too_early_step_is_retried_with_backoff_and_needs_capability_13() -> Result<(), BoxError> {
    block_on(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer(&mut server, &[13], &[13], &[]).await?;
            for (id, sequence) in [(7, 41), (8, 42)] {
                let command = read_frame(&mut server).await?;
                assert_eq!(
                    decode_wire_envelope(&command)?
                        .client_command(1)?
                        .command_id,
                    id
                );
                write_frame(
                    &mut server,
                    &encode_command_result(
                        1,
                        sequence,
                        id,
                        CommandStatus::Rejected,
                        &encode_step_result(StepDisposition::TooEarly),
                    )?,
                )
                .await?;
            }
            read_frame(&mut server).await?;
            write_frame(
                &mut server,
                &encode_command_result(
                    1,
                    43,
                    9,
                    CommandStatus::Accepted,
                    &encode_step_result(StepDisposition::Blocked),
                )?,
            )
            .await?;
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, admission()?).await?;
        assert_eq!(session.selected_capabilities(), &[13]);
        let started = std::time::Instant::now();
        let outcome = session.step_retrying(StepDirection::East).await?;
        peer.await??;
        assert_eq!(outcome.disposition, StepDisposition::Blocked);
        assert_eq!(outcome.command_id, 9);
        assert!(started.elapsed() >= STEP_RETRY_INITIAL_BACKOFF * 3);
        Ok(())
    })?
}

#[test]
fn too_early_without_capability_13_fails_closed() -> Result<(), BoxError> {
    block_on(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer(&mut server, &[13], &[], &[]).await?;
            read_frame(&mut server).await?;
            write_frame(
                &mut server,
                &encode_command_result(
                    1,
                    41,
                    7,
                    CommandStatus::Rejected,
                    &encode_step_result(StepDisposition::TooEarly),
                )?,
            )
            .await?;
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, admission()?).await?;
        assert!(session.selected_capabilities().is_empty());
        assert!(matches!(
            session.step(StepDirection::East).await,
            Err(SessionError::WorldSpatial(_))
        ));
        peer.await??;
        Ok(())
    })?
}

#[test]
fn a_selection_outside_the_advertised_set_is_refused() -> Result<(), BoxError> {
    block_on(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move { join_peer(&mut server, &[13], &[7], &[]).await });
        let result = Session::admit(client, admission()?).await;
        let _ = peer.await?;
        assert!(matches!(
            result,
            Err(SessionError::CapabilityNotRequested(7))
        ));
        Ok(())
    })?
}

#[test]
fn gated_domains_and_commands_route_by_selected_capability() -> Result<(), BoxError> {
    block_on(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer(
                &mut server,
                &[GATED_CAPABILITY, 13],
                &[GATED_CAPABILITY, 13],
                &[(GATED_DOMAIN, 3), (4242, 1)],
            )
            .await?;
            let command = read_frame(&mut server).await?;
            assert_eq!(
                decode_wire_envelope(&command)?
                    .client_command(1)?
                    .command_type,
                GATED_COMMAND
            );
            write_frame(
                &mut server,
                &encode_command_result(1, 41, 7, CommandStatus::Accepted, b"ok")?,
            )
            .await?;
            write_frame(
                &mut server,
                &encode_state_delta(1, 42, GATED_DOMAIN, 3, 4, 1, b"delta")?,
            )
            .await?;
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, admission_with(&[GATED_CAPABILITY, 13])?).await?;
        // The unregistered domain 4242 is ignored; the selected chat domain is kept raw.
        let snapshot = session
            .gated_snapshot(GATED_DOMAIN)
            .ok_or("chat snapshot")?;
        assert_eq!(
            (snapshot.revision, snapshot.payload.as_slice()),
            (3, &b"snap"[..])
        );
        assert!(session.gated_snapshot(4242).is_none());
        // A command another capability owns is refused before anything is sent.
        assert!(matches!(
            session.gated_command(13, GATED_COMMAND, b"x").await,
            Err(SessionError::CapabilityNotSelected { capability: 13 })
        ));
        let result = session
            .gated_command(GATED_CAPABILITY, GATED_COMMAND, b"x")
            .await?;
        assert_eq!(
            (result.command_id, result.payload.as_slice()),
            (7, &b"ok"[..])
        );
        let delta = session.read_gated_delta(GATED_DOMAIN, 1).await?;
        peer.await??;
        assert_eq!((delta.base_revision, delta.new_revision), (3, 4));
        assert_eq!(delta.payload, b"delta");
        Ok(())
    })?
}

#[test]
fn unselected_gated_commands_and_domains_are_refused() -> Result<(), BoxError> {
    block_on(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer(&mut server, &[13], &[13], &[]).await?;
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, admission()?).await?;
        peer.await??;
        assert!(matches!(
            session
                .gated_command(GATED_CAPABILITY, GATED_COMMAND, b"x")
                .await,
            Err(SessionError::CapabilityNotSelected {
                capability: GATED_CAPABILITY
            })
        ));
        assert!(matches!(
            session.read_gated_delta(GATED_DOMAIN, 1).await,
            Err(SessionError::CapabilityNotSelected {
                capability: GATED_CAPABILITY
            })
        ));
        // Nothing was sent, so the session stays usable.
        assert_eq!(session.next_command_id(), 7);

        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer(&mut server, &[13], &[13], &[(GATED_DOMAIN, 3)]).await
        });
        let result = Session::admit(client, admission()?).await;
        let _ = peer.await?;
        assert!(matches!(
            result,
            Err(SessionError::UnselectedDomain {
                domain_id: GATED_DOMAIN
            })
        ));
        Ok(())
    })?
}

#[test]
fn a_resume_never_changes_the_selected_set() {
    assert!(check_resume_selection(&[7, 13], &[13, 7]).is_ok());
    assert!(check_resume_selection(&[], &[]).is_ok());
    assert!(matches!(
        check_resume_selection(&[13], &[7, 13]),
        Err(SessionError::ResumeSelectionChanged)
    ));
    assert!(matches!(
        check_resume_selection(&[7, 13], &[13]),
        Err(SessionError::ResumeSelectionChanged)
    ));
}

#[test]
fn p17c_client_advertises_exiva_and_field_prerequisites() {
    for id in [140, 147, 91, 4, 6, 11, 15, 18, 22, 30] {
        assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&id), "missing {id}");
    }
}

#[test]
fn cl26s3_client_advertises_daily_reward_v2() {
    assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&182));
}

#[test]
fn p17c_field_functions_select_with_prerequisites_on_the_default_client() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                use oteryn_protocol_oteryn::world_map::{
                    MapViewHeader, WorldMapViewSnapshot, encode_world_map_snapshot,
                };
                let selected: &[u32] = &[4, 6, 11, 15, 18, 22, 30, 91];
                let (client, mut server) = tokio::io::duplex(256 * 1024);
                let peer = tokio::spawn(async move {
                    let map = encode_world_map_snapshot(&WorldMapViewSnapshot {
                        header: MapViewHeader {
                            content_generation: CONTENT_GENERATION,
                            bundle_digest: [0x22; 32],
                            reset_epoch: 0,
                            origin: at(0, 0),
                            first_visible_floor: 0,
                        },
                        tiles: vec![],
                    })
                    .map_err(|e| format!("{e:?}"))?;
                    let inv = item_view::encode_character_inventory(&CharacterInventory::default())
                        .map_err(|e| format!("{e:?}"))?;
                    let container = item_view::encode_open_container(&OpenContainer::default())
                        .map_err(|e| format!("{e:?}"))?;
                    join_peer_with(
                        &mut server,
                        CLIENT_SUPPORTED_CAPABILITIES,
                        selected,
                        &[],
                        item_snapshot(vec![])?,
                        &[(9, 0, 1, inv), (11, 0, 1, container), (17, 0, 1, map)],
                    )
                    .await?;
                    let frame = read_frame(&mut server).await?;
                    let envelope = decode_wire_envelope(&frame)?;
                    let command = envelope.client_command(1)?;
                    assert_eq!(command.command_type, 2);
                    write_frame(
                        &mut server,
                        &encode_command_result(
                            1,
                            41,
                            command.command_id,
                            CommandStatus::Accepted,
                            &world_object::encode_use_result(UseDisposition::TargetIllegal)
                                .map_err(|e| format!("{e:?}"))?,
                        )?,
                    )
                    .await?;
                    Ok::<(), BoxError>(())
                });
                let mut session = Session::admit(client, entity_admission()?).await?;
                assert!(session.is_selected(91));
                for capability in selected {
                    assert!(session.is_selected(*capability));
                }
                assert_eq!(
                    session.use_item(handle(1)).await?.disposition,
                    UseDisposition::TargetIllegal
                );
                peer.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "field selection thread panicked")?
}

#[test]
fn unselected_diagonal_step_and_pending_recovery_send_no_bytes() -> Result<(), BoxError> {
    block_on(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer(&mut server, &[13], &[], &[]).await?;
            let mut bytes = Vec::new();
            server.read_to_end(&mut bytes).await?;
            assert!(
                bytes.is_empty(),
                "unselected diagonal must not reach the transport"
            );
            Ok::<(), BoxError>(())
        });
        let mut session = Session::admit(client, admission()?).await?;
        for direction in [
            StepDirection::NorthEast,
            StepDirection::SouthEast,
            StepDirection::SouthWest,
            StepDirection::NorthWest,
        ] {
            assert!(matches!(
                session.step(direction).await,
                Err(SessionError::CapabilityNotSelected { capability: 371 })
            ));
            assert_eq!(session.next_command_id(), 7);
        }
        assert!(matches!(
            session
                .disconnect_after_step(StepDirection::NorthEast)
                .await,
            Err(SessionError::CapabilityNotSelected { capability: 371 })
        ));
        peer.await??;
        Ok(())
    })?
}

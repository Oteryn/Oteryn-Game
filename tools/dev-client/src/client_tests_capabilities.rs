#[test]
fn explicit_tracker_offer_survives_tls_join_and_command_without_changing_defaults()
-> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| block_on(tracker_offer_cases()))?
        .join()
        .map_err(|_| "tracker offer test thread failed")??
}

async fn tracker_offer_cases() -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::cyclopedia_tracker as tracker;
    let capability = tracker::CAPABILITY_CYCLOPEDIA_TRACKER_V1;
    assert!(!CLIENT_SUPPORTED_CAPABILITIES.contains(&capability));
    for opt_in in [false, true] {
        let (certificate, acceptor, listener, address) = tls_test_listener().await?;
        let server = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await?;
            let mut stream = acceptor.accept(tcp).await?;
            let bytes = read_frame(&mut stream).await?;
            let envelope = decode_wire_envelope(&bytes)?;
            let bootstrap = envelope.client_bootstrap()?;
            let selected = if bootstrap.supported_capabilities.contains(&capability) {
                vec![capability]
            } else {
                Vec::new()
            };
            write_frame(
                &mut stream,
                &encode_server_accepted(&ServerAcceptedValue {
                    game_session_id: GameSessionId::decode(&test_uuid_v7(1))?,
                    world_id: WorldId::decode(&test_uuid_v7(2))?,
                    channel_id: ChannelId::decode(&test_uuid_v7(3))?,
                    connection_generation: GENERATION,
                    current_server_sequence: JOIN_SEQUENCE,
                    next_command_id: FIRST_COMMAND_ID,
                    schema_revision: 1,
                    selected_capabilities: &selected,
                })?,
            )
            .await?;
            let spatial = spatial_payload(0, 0, CONTENT_GENERATION);
            let overlay = encode_world_object_overlay_snapshot(&[])
                .map_err(|error| format!("overlay: {error:?}"))?;
            let empty = tracker::TrackerSnapshot { boss: false, entries: Vec::new() };
            let trackers = tracker::encode_domain_snapshot(
                &empty,
                &tracker::TrackerSnapshot { boss: true, entries: Vec::new() },
            ).ok_or("tracker snapshot encode")?;
            let mut domains = vec![
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: JOIN_SPATIAL_REVISION,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &spatial,
                },
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    revision: JOIN_DOOR_REVISION,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    payload: &overlay,
                },
            ];
            if !selected.is_empty() {
                domains.push(DomainSnapshot {
                    domain_id: tracker::STATE_DOMAIN_CYCLOPEDIA_TRACKER,
                    revision: 1,
                    snapshot_type: tracker::SNAPSHOT_TYPE_CYCLOPEDIA_TRACKER_V1,
                    payload: &trackers,
                });
            }
            send(&mut stream, &encode_single_chunk_snapshot(
                GENERATION, 1, JOIN_SEQUENCE, &domains,
            )?).await?;
            if !selected.is_empty() {
                let (id, command, payload) = read_command(&mut stream).await?;
                assert_eq!(id, FIRST_COMMAND_ID);
                assert_eq!(command, tracker::COMMAND_TYPE_BESTIARY_TRACKER);
                assert_eq!(tracker::decode_request(&payload), Some(tracker::TrackerRequest::Read));
                let payload = tracker::encode_snapshot(&empty).ok_or("tracker reply encode")?;
                write_frame(&mut stream, &command_result_frame(
                    JOIN_SEQUENCE + 1, id, CommandStatus::Accepted, &payload,
                )?).await?;
            }
            Ok::<_, BoxError>(())
        });
        let request = JoinRequest {
            address,
            server_name: "localhost",
            root_certificate: &certificate,
            schema_revision: 1,
            character_id: CharacterId::decode(&test_uuid_v7(4))?,
            admission_material: b"fixture-grant",
            client_build_id: CLIENT_BUILD_ID,
            deadline: TEST_DEADLINE,
        };
        let mut offered = CLIENT_SUPPORTED_CAPABILITIES.to_vec();
        offered.push(capability);
        offered.sort_unstable();
        offered.dedup();
        let mut client = if opt_in {
            connect_session_with_capabilities(request, &offered).await?
        } else {
            connect_session(request).await?
        };
        assert_eq!(client.selected_capabilities().contains(&capability), opt_in,
            "capability 197 must be selected exactly when explicitly offered");
        assert_eq!(client.session_mut().gated_snapshot(tracker::STATE_DOMAIN_CYCLOPEDIA_TRACKER).is_some(), opt_in);
        if opt_in {
            let request = tracker::encode_request(&tracker::TrackerRequest::Read)
                .ok_or("tracker request encode")?;
            let reply = client.session_mut().gated_command(
                capability, tracker::COMMAND_TYPE_BESTIARY_TRACKER, &request,
            ).await?;
            assert_eq!(reply.status, CommandStatus::Accepted);
            assert_eq!(tracker::decode_snapshot(&reply.payload), Some(tracker::TrackerSnapshot {
                boss: false, entries: Vec::new(),
            }));
        }
        server.await??;
    }
    Ok(())
}

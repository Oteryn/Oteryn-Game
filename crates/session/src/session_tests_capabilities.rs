use oteryn_protocol_oteryn::charm::{
    COMMAND_TYPE_CHARM_ASSIGN_INTENT, STATE_DOMAIN_CHARACTER_CHARMS,
};


#[test]
fn prey_native_capability_routes_tier_and_confirmed_pick_and_refuses_without_selection() -> Result<(), BoxError> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        block_on(async {
            for selected in [false, true] {
                let (client, mut server) = tokio::io::duplex(128 * 1024);
                let requests = [
                    prey::Request::SetTier { slot: 1, tier: 3 },
                    prey::Request::OfferedToday { slot: 1, after: String::new() },
                    prey::Request::PickAgain { slot: 1, creature: "oteryn:creature.dragon".into(), expected_price_gold: 45000 },
                ];
                let expected = requests.clone();
                let peer = tokio::spawn(async move {
                    join_peer_with(&mut server, CLIENT_SUPPORTED_CAPABILITIES,
                        if selected { &[140] } else { &[] }, &[], (1, spatial(0)), &[]).await?;
                    if selected {
                        for (index, expected) in expected.into_iter().enumerate() {
                            let frame = read_frame(&mut server).await?;
                            let envelope = decode_wire_envelope(&frame)?;
                            let command = envelope.client_command(1)?;
                            let id = 7 + index as u64;
                            assert_eq!((command.command_type, command.command_id), (160, id));
                            assert_eq!(prey::decode_request(command.payload).map_err(|_| "request")?, expected);
                            let reply = prey::Reply {
                                status: 0, wildcard_balance: 10, slots: Vec::new(),
                                list_size: 12, expansion_owned: true,
                                offered_today: vec![prey::OfferedCreature {
                                    creature: "oteryn:creature.dragon".into(), price_gold: 45000, occupied_slot: 0,
                                }],
                                offered_next: String::new(),
                            };
                            let payload = prey::encode_reply(&reply).map_err(|_| "reply")?;
                            write_frame(&mut server, &encode_command_result(1, 34 + id, id,
                                CommandStatus::Accepted, &payload)?).await?;
                        }
                    }
                    Ok::<(), BoxError>(())
                });
                let mut admission = admission()?;
                admission.supported_capabilities = CLIENT_SUPPORTED_CAPABILITIES;
                let mut session = Session::admit(client, admission).await?;
                for request in requests {
                    let payload = prey::encode_request(&request).map_err(|_| "request")?;
                    let result = session.gated_command(140, 160, &payload).await;
                    if selected {
                        let reply = prey::decode_reply(&result?.payload).map_err(|_| "reply")?;
                        assert_eq!(reply.list_size, 12);
                        assert_eq!(reply.offered_today[0].price_gold, 45000);
                    } else {
                        assert!(matches!(result, Err(SessionError::CapabilityNotSelected { capability: 140 })));
                    }
                }
                peer.await??;
            }
            Ok::<(), BoxError>(())
        })?
    })?.join().map_err(|_| "Prey selection thread panicked")?
}

#[test]
fn derived_gated_routes_match_previous_table() {
    // Former GATED_ROUTES contents plus the PKG5 hotkey command and carried extension.
    // Compare sets because
    // the protocol registry generates its entries in capability order.
    let expected: &[(u32, &[u32], &[u32])] = &[
        (3, &[7, 8], &[7, 8]),
        (51, &[60, 61], &[]),
        (40, &[40, 41], &[]),
        (42, &[45, 46], &[]),
        (43, &[48, 49, 50], &[]),
        (60, &[78], &[26]),
        (62, &[79], &[27]),
        (63, &[80], &[28]),
        (68, &[85], &[]),
        (72, &[89], &[]),
        (81, &[97], &[]),
        (140, &[160], &[]),
        (165, &[220], &[]),
        (16, &[22], &[16]),
        (18, &[], &[17]),
        (37, &[36], &[]),
        (31, &[], &[23]),
        (25, &[25], &[21]),
        (1, &[4, 5], &[4, 5]),
        (8, &[], &[13]),
        (9, &[14], &[]),
        (10, &[], &[15]),
        (14, &[21], &[14]),
        (179, &[348], &[]),
        (180, &[], &[]),
        // PREY-DIFF: native Prey controls use the existing command160.
        (140, &[160], &[]),
        (147, &[170], &[]),
        // Wheel and Atelier are playable through the selected capability 148.
        (148, &[161, 308, 309, 424], &[]),
        // Live balances use their own selected capability without changing Atelier V1.
        (272, &[484], &[]),
        // pkg4 additions: raid server messages arrive on domain 57 under 92 zones_nologout;
        // animus mastery (168) and loyalty skills (274) query through their gated commands.
        (92, &[], &[57]),
        (168, &[232], &[]),
        (274, &[488], &[]),
        // CL25J: native feature adapters now use these existing gated wire routes.
        (151, &[177], &[]),
        (152, &[181], &[]),
        (153, &[182], &[]),
        (154, &[183, 184, 185, 186], &[]),
        (195, &[378, 379, 380, 392, 393, 394, 395], &[]),
        (197, &[383, 384], &[98]),
        (199, &[387, 388], &[]),
        (225, &[431], &[]),
        (230, &[440], &[]),
    ];
    let expected: std::collections::BTreeSet<_> = expected.iter().copied().collect();
    let derived: std::collections::BTreeSet<_> = gated_routes().collect();
    assert_eq!(derived, expected);
}

/// A peer that closes before `ServerAccepted` fails admission closed: no session.
#[test]
fn a_stream_closed_before_admission_fails_closed() -> Result<(), BoxError> {
    block_on(async {
        let (client, server) = tokio::io::duplex(1024);
        drop(server);
        let result = Session::admit(client, admission()?).await;
        assert!(matches!(result, Err(SessionError::Io(_))));
        Ok(())
    })?
}
/// Reads the bootstrap, answers with one `ProtocolError` at generation 0, then closes.
async fn refuse_admission(
    error: FoundationProtocolError,
) -> Result<Result<Session<DuplexStream>, SessionError>, BoxError> {
    let (client, mut server) = tokio::io::duplex(1024);
    let peer = tokio::spawn(async move {
        read_frame(&mut server).await?;
        write_frame(
            &mut server,
            &oteryn_protocol_oteryn::encode_protocol_error(error, 0)?,
        )
        .await?;
        Ok::<_, BoxError>(())
    });
    let result = Session::admit(client, admission()?).await;
    peer.await??;
    Ok(result)
}

/// N8: a refusal frame with a code in 1100..=1199 surfaces the code, and the code maps to
/// its FND-04A class; any other `ProtocolError` stays `NotAdmitted`.
#[test]
fn an_admission_refusal_frame_surfaces_its_code() -> Result<(), BoxError> {
    block_on(async {
        for (error, class) in [
            (
                FoundationProtocolError::AdmissionGrantExpired,
                (
                    AdmissionProgression::Terminal,
                    AdmissionPublicClass::RetryLogin,
                ),
            ),
            (
                FoundationProtocolError::AdmissionGrantSecurityEvidenceStale,
                (
                    AdmissionProgression::Retryable,
                    AdmissionPublicClass::TemporarilyUnavailable,
                ),
            ),
            (
                FoundationProtocolError::AdmissionIncumbentProtected,
                (
                    AdmissionProgression::Terminal,
                    AdmissionPublicClass::CharacterAlreadyActive,
                ),
            ),
        ] {
            let result = refuse_admission(error).await?;
            let Err(SessionError::AdmissionRefused { code }) = result else {
                return Err(format!("{error:?}: {:?}", result.err()).into());
            };
            assert_eq!(code, error.code());
            assert_eq!(admission_refusal_class(code), Some(class));
        }
        let result = refuse_admission(FoundationProtocolError::ProtocolMajorMismatch).await?;
        assert!(matches!(
            result,
            Err(SessionError::NotAdmitted(MessageType::ProtocolError))
        ));
        Ok(())
    })?
}

/// An unassigned code in the admission range is still a refusal; the client reads it as
/// generic `TEMPORARILY_UNAVAILABLE` with no retry.
#[test]
fn an_unknown_admission_code_is_generic_and_terminal() -> Result<(), BoxError> {
    block_on(async {
        let (client, mut server) = tokio::io::duplex(1024);
        let peer = tokio::spawn(async move {
            read_frame(&mut server).await?;
            // Type 14, generation 0; payload: code 1150 (varint fe 08), TRANSPORT_FATAL.
            let frame = [0x08, 0x0e, 0x22, 0x05, 0x08, 0xfe, 0x08, 0x10, 0x04];
            write_frame(&mut server, &frame).await?;
            Ok::<_, BoxError>(())
        });
        let result = Session::admit(client, admission()?).await;
        peer.await??;
        let Err(SessionError::AdmissionRefused { code }) = result else {
            return Err(format!("{:?}", result.err()).into());
        };
        assert_eq!(code, 1150);
        assert_eq!(
            admission_refusal_class(code),
            Some((
                AdmissionProgression::Terminal,
                AdmissionPublicClass::TemporarilyUnavailable
            ))
        );
        Ok(())
    })?
}

/// N8 fixes the refusal shape: a refusal-range code with any other disposition or with a
/// correlation field is not an admission refusal and fails closed as `NotAdmitted`.
#[test]
fn a_misshapen_admission_refusal_fails_closed() -> Result<(), BoxError> {
    block_on(async {
        // Type 14, generation 0; code 1103 (varint cf 08) plus the listed fields.
        for payload in [
            &[0x08, 0xcf, 0x08, 0x10, 0x01][..],
            &[0x08, 0xcf, 0x08, 0x10, 0x04, 0x18, 0x01],
            &[0x08, 0xcf, 0x08, 0x10, 0x04, 0x20, 0x01],
            &[0x08, 0xcf, 0x08, 0x10, 0x04, 0x28, 0x01],
        ] {
            let mut frame = vec![0x08, 0x0e, 0x22, u8::try_from(payload.len())?];
            frame.extend_from_slice(payload);
            let (client, mut server) = tokio::io::duplex(1024);
            let peer = tokio::spawn(async move {
                read_frame(&mut server).await?;
                write_frame(&mut server, &frame).await?;
                Ok::<_, BoxError>(())
            });
            let result = Session::admit(client, admission()?).await;
            peer.await??;
            assert!(
                matches!(
                    result,
                    Err(SessionError::NotAdmitted(MessageType::ProtocolError))
                ),
                "{payload:02x?}: {:?}",
                result.err()
            );
        }
        Ok(())
    })?
}

const GATED_DOMAIN: u32 = STATE_DOMAIN_CHARACTER_CHARMS;
const GATED_COMMAND: u32 = COMMAND_TYPE_CHARM_ASSIGN_INTENT;
const GATED_CAPABILITY: u32 = CAPABILITY_BESTIARY_CHARMS_V1;

/// Reads the bootstrap (asserting the advertised set), admits with `selected`, and sends a
/// join snapshot carrying the two core domains plus `extra` (domain, revision) payload `b"snap"`.
async fn join_peer(
    stream: &mut DuplexStream,
    advertised: &[u32],
    selected: &[u32],
    extra: &[(u32, u64)],
) -> Result<(), BoxError> {
    join_peer_with(
        stream,
        advertised,
        selected,
        extra,
        (SNAPSHOT_TYPE_WORLD_SPATIAL_V1, spatial(0)),
        &[],
    )
    .await
}

/// `join_peer` with the domain-1 snapshot `(type, payload)` the test chooses.
pub(super) async fn join_peer_with(
    stream: &mut DuplexStream,
    advertised: &[u32],
    selected: &[u32],
    extra: &[(u32, u64)],
    spatial_snapshot: (u32, Vec<u8>),
    raw: &[(u32, u64, u32, Vec<u8>)],
) -> Result<(), BoxError> {
    let bootstrap = read_frame(stream).await?;
    assert_eq!(
        decode_wire_envelope(&bootstrap)?
            .client_bootstrap()?
            .supported_capabilities,
        advertised
    );
    write_frame(
        stream,
        &encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&uuid_v7(1))?,
            world_id: WorldId::decode(&uuid_v7(2))?,
            channel_id: ChannelId::decode(&uuid_v7(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 7,
            schema_revision: 1,
            selected_capabilities: selected,
        })?,
    )
    .await?;
    let (spatial_type, spatial_payload) = spatial_snapshot;
    let overlay_payload = encode_world_object_overlay_snapshot(&[])
        .map_err(|error| format!("overlay snapshot: {error:?}"))?;
    let mut domains = vec![DomainSnapshot {
        domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
        revision: 5,
        snapshot_type: spatial_type,
        payload: &spatial_payload,
    }];
    // Bundle Worlds replace the legacy overlay with capability 18's map snapshot.
    if !selected.contains(&oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1) {
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            revision: 2,
            snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
            payload: &overlay_payload,
        });
    }
    for &(domain_id, revision) in extra {
        domains.push(DomainSnapshot {
            domain_id,
            revision,
            snapshot_type: 1,
            payload: b"snap",
        });
    }
    for (domain_id, revision, snapshot_type, payload) in raw {
        domains.push(DomainSnapshot {
            domain_id: *domain_id,
            revision: *revision,
            snapshot_type: *snapshot_type,
            payload,
        });
    }
    for frame in encode_single_chunk_snapshot(1, 1, 40, &domains)? {
        write_frame(stream, &frame).await?;
    }
    Ok(())
}

pub(super) fn admission_with(supported: &'static [u32]) -> Result<Admission<'static>, BoxError> {
    Ok(Admission {
        appearance_items: None,
        supported_capabilities: supported,
        ..admission()?
    })
}

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
fn p17c_exiva_selection_routes_command_170_and_refuses_without_selection() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for selected in [false, true] {
                    let (client, mut server) = tokio::io::duplex(64 * 1024);
                    let peer = tokio::spawn(async move {
                        join_peer_with(
                            &mut server,
                            CLIENT_SUPPORTED_CAPABILITIES,
                            if selected { &[147] } else { &[] },
                            &[],
                            (1, spatial(0)),
                            &[],
                        )
                        .await?;
                        if selected {
                            for (id, payload, reply) in [
                                (7, vec![6, b'A', b'l'], vec![1]),
                                (8, vec![5], vec![2, 2, 0]),
                            ] {
                                let frame = read_frame(&mut server).await?;
                                let envelope = decode_wire_envelope(&frame)?;
                                let command = envelope.client_command(1)?;
                                assert_eq!(
                                    (command.command_type, command.command_id, command.payload),
                                    (170, id, payload.as_slice())
                                );
                                write_frame(
                                    &mut server,
                                    &encode_command_result(
                                        1,
                                        34 + id,
                                        id,
                                        CommandStatus::Accepted,
                                        &reply,
                                    )?,
                                )
                                .await?;
                            }
                        }
                        Ok::<(), BoxError>(())
                    });
                    let mut admission = admission()?;
                    admission.supported_capabilities = CLIENT_SUPPORTED_CAPABILITIES;
                    let mut session = Session::admit(client, admission).await?;
                    let result = session.gated_command(147, 170, &[6, b'A', b'l']).await;
                    if selected {
                        assert_eq!(result?.payload, [1]);
                        let result = session.gated_command(147, 170, &[5]).await?;
                        assert_eq!(
                            exiva::PlayerList::decode(&result.payload)
                                .ok_or("list")?
                                .modes,
                            2
                        );
                    } else {
                        assert!(matches!(
                            result,
                            Err(SessionError::CapabilityNotSelected { capability: 147 })
                        ));
                    }
                    peer.await??;
                }
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "Exiva selection thread panicked")?
}

#[test]
fn selected_mechanic_panels_have_raw_command_routes() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                // The adapters for CLIENT-E2E-1 all call gated_command, including opt-in panels.
                for (capability, command_type) in [
                    (40, 40),
                    (42, 45),
                    (43, 48),
                    (60, 78),
                    (62, 79),
                    (63, 80),
                    (68, 85),
                    (72, 89),
                    (81, 97),
                    (140, 160),
                    (165, 220),
                    // CL25J: include every newly composed feature route.
                    (151, 177),
                    (152, 181),
                    (153, 182),
                    (154, 183),
                    (179, 348),
                    (195, 378),
                    (197, 383),
                    (199, 387),
                    (225, 431),
                    (230, 440),
                ] {
                    assert!(
                        gated_routes().any(|(id, commands, _)| id == capability
                            && commands.contains(&command_type)),
                        "missing panel route for capability {capability}"
                    );
                }
                let (client, mut server) = tokio::io::duplex(64 * 1024);
                let peer = tokio::spawn(async move {
                    join_peer(&mut server, &[40], &[40], &[]).await?;
                    let frame = read_frame(&mut server).await?;
                    assert_eq!(
                        decode_wire_envelope(&frame)?
                            .client_command(1)?
                            .command_type,
                        40
                    );
                    write_frame(
                        &mut server,
                        &encode_command_result(1, 41, 7, CommandStatus::Accepted, b"guild")?,
                    )
                    .await?;
                    Ok::<(), BoxError>(())
                });
                let mut session = Session::admit(client, admission_with(&[40])?).await?;
                assert!(session.is_selected(40));
                assert_eq!(session.gated_command(40, 40, b"").await?.payload, b"guild");
                peer.await??;
                Ok(())
            })?
        })?
        .join()
        .map_err(|_| "panel routes thread panicked")?
}

#[test]
fn npc_service_and_staff_prerequisites_route_with_interleaved_domains() -> Result<(), BoxError> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        block_on(async {
            let npc = EntityRef { identity: [9; 16], generation: 2 };
            let conversation = npc_service::NpcConversation { open: Some((npc, npc_service::NpcConversationState::Open)), lines: vec!["Trade?".into()] };
            let window = npc_service::NpcTradeWindow { open: Some((npc, 7)), offers: vec![npc_service::NpcOffer { item_definition_ref: 5, side: NpcTradeSide::Buy, count: 1, unit_price: 12 }] };
            let expected = window.clone();
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let peer = tokio::spawn(async move {
                join_peer_with(&mut server, CLIENT_SUPPORTED_CAPABILITIES, &[3, 51], &[], (1, spatial(0)), &[
                    (7, 0, 1, npc_service::encode_npc_conversation(&Default::default()).map_err(|e| format!("conversation: {e:?}"))?),
                    (8, 0, 1, npc_service::encode_npc_trade_window(&Default::default()).map_err(|e| format!("window: {e:?}"))?),
                ]).await?;
                let frame = read_frame(&mut server).await?;
                let envelope = decode_wire_envelope(&frame)?;
                let command = envelope.client_command(1)?;
                assert_eq!(command.command_type, 7);
                assert_eq!(npc_service::decode_npc_talk_intent(command.payload).map_err(|e| format!("talk: {e:?}"))?.text, "trade");
                write_frame(&mut server, &encode_command_result(1, 41, 7, CommandStatus::Accepted, &npc_service::encode_npc_intent_result(NpcIntentDisposition::Ok))?).await?;
                write_frame(&mut server, &encode_state_delta(1, 42, 7, 0, 1, 1, &npc_service::encode_npc_conversation(&conversation).map_err(|e| format!("conversation: {e:?}"))?)?).await?;
                write_frame(&mut server, &encode_state_delta(1, 43, 8, 0, 1, 1, &npc_service::encode_npc_trade_window(&window).map_err(|e| format!("window: {e:?}"))?)?).await?;
                let frame = read_frame(&mut server).await?;
                let envelope = decode_wire_envelope(&frame)?;
                let command = envelope.client_command(1)?;
                assert_eq!(command.command_type, 60);
                assert_eq!(staff::decode_envelope(command.payload)?.principal, "operator");
                let result = staff::StaffResult { code: staff::StaffCode::TierInsufficient, disposition: 2, control_seq: 1, link_sha256: vec![3; 32], mute_until_ms: 0 };
                write_frame(&mut server, &encode_state_delta(1, 44, 8, 1, 2, 1, &npc_service::encode_npc_trade_window(&Default::default()).map_err(|e| format!("window: {e:?}"))?)?).await?;
                write_frame(&mut server, &encode_command_result(1, 45, 8, CommandStatus::Accepted, &staff::encode_result(&result)?)?).await?;
                Ok::<(), BoxError>(())
            });
            let mut admission = admission()?;
            admission.supported_capabilities = CLIENT_SUPPORTED_CAPABILITIES;
            let mut session = Session::admit(client, admission).await?;
            assert!(matches!(session.gated_command(81, 97, b"").await, Err(SessionError::CapabilityNotSelected { capability: 81 })));
            let payload = npc_service::encode_npc_talk_intent(&npc_service::NpcTalkIntent { npc_actor: npc, text: "trade".into() }).map_err(|e| format!("talk: {e:?}"))?;
            let result = session.gated_command(3, 7, &payload).await?;
            assert_eq!(npc_service::decode_npc_intent_result(&result.payload).map_err(|e| format!("result: {e:?}"))?, NpcIntentDisposition::Ok);
            let delta = session.read_gated_delta(8, 1).await?;
            assert_eq!(npc_service::decode_npc_trade_window(&delta.payload).map_err(|e| format!("window: {e:?}"))?, expected);
            assert!(matches!(session.take_events().as_slice(), [SessionEvent::NpcConversation(_)]));
            let envelope = staff::SignedEnvelope { principal: "operator".into(), canonical: vec![1], signature: vec![2] };
            let result = session.gated_command(51, 60, &staff::encode_envelope(&envelope)?).await?;
            assert_eq!(staff::decode_result(&result.payload)?.code, staff::StaffCode::TierInsufficient);
            assert!(matches!(session.take_events().as_slice(), [SessionEvent::NpcTradeWindow(_)]));
            peer.await??;
            Ok::<(), BoxError>(())
        })?
    })?.join().map_err(|_| "NPC session thread panicked")?
}

#[test]
fn highscores_query_uses_selected_session_and_preserves_live_result() -> Result<(), BoxError> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| {
        block_on(async {
            assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&9));
            for selected in [false, true] {
                let (client, mut server) = tokio::io::duplex(64 * 1024);
                let peer = tokio::spawn(async move {
                    let caps: &[u32] = if selected { &[9] } else { &[] };
                    join_peer(&mut server, &[9], caps, &[]).await?;
                    if selected {
                        let frame = read_frame(&mut server).await?;
                        let envelope = decode_wire_envelope(&frame)?;
                        let command = envelope.client_command(1)?;
                        assert_eq!(command.command_type, 14);
                        let query = highscores::decode_highscores_query(command.payload).map_err(|e| format!("{e:?}"))?;
                        assert_eq!((query.category, query.page, query.snapshot_id), (1, 1, 0));
                        let reply = highscores::HighscoresResult { category: 1, snapshot_id: 77, computed_at: 1234, page: 1, has_more: false, rows: vec![], own_row: None };
                        let payload = highscores::encode_highscores_result(&reply).map_err(|e| format!("{e:?}"))?;
                        write_frame(&mut server, &encode_command_result(1, 41, command.command_id, CommandStatus::Accepted, &payload)?).await?;
                    }
                    Ok::<(), BoxError>(())
                });
                let mut session = Session::admit(client, admission_with(&[9])?).await?;
                let payload = highscores::encode_highscores_query(highscores::HighscoresQuery { category: 1, vocation: 0, page: 1, snapshot_id: 0 }).map_err(|e| format!("{e:?}"))?;
                let result = session.gated_command(9, 14, &payload).await;
                if selected {
                    let reply = highscores::decode_highscores_result(&result?.payload).map_err(|e| format!("{e:?}"))?;
                    assert_eq!((reply.snapshot_id, reply.computed_at), (77, 1234));
                } else {
                    assert!(matches!(result, Err(SessionError::CapabilityNotSelected { capability: 9 })));
                }
                peer.await??;
            }
            Ok(())
        })?
    })?.join().map_err(|_| "highscores session test panicked")?
}

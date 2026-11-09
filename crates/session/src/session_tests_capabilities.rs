use oteryn_protocol_oteryn::charm::{
    COMMAND_TYPE_CHARM_ASSIGN_INTENT, STATE_DOMAIN_CHARACTER_CHARMS,
};

#[test]
fn prey_native_capability_routes_tier_and_confirmed_pick_and_refuses_without_selection()
-> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for selected in [false, true] {
                    let (client, mut server) = tokio::io::duplex(128 * 1024);
                    let requests = [
                        prey::Request::SetTier { slot: 1, tier: 3 },
                        prey::Request::OfferedToday {
                            slot: 1,
                            after: String::new(),
                        },
                        prey::Request::PickAgain {
                            slot: 1,
                            creature: "oteryn:creature.dragon".into(),
                            expected_price_gold: 45000,
                        },
                    ];
                    let expected = requests.clone();
                    let peer = tokio::spawn(async move {
                        join_peer_with(
                            &mut server,
                            CLIENT_SUPPORTED_CAPABILITIES,
                            if selected { &[140] } else { &[] },
                            &[],
                            (1, spatial(0)),
                            &[],
                        )
                        .await?;
                        if selected {
                            for (index, expected) in expected.into_iter().enumerate() {
                                let frame = read_frame(&mut server).await?;
                                let envelope = decode_wire_envelope(&frame)?;
                                let command = envelope.client_command(1)?;
                                let id = 7 + index as u64;
                                assert_eq!((command.command_type, command.command_id), (160, id));
                                assert_eq!(
                                    prey::decode_request(command.payload).map_err(|_| "request")?,
                                    expected
                                );
                                let reply = prey::Reply {
                                    status: 0,
                                    wildcard_balance: 10,
                                    slots: Vec::new(),
                                    list_size: 12,
                                    expansion_owned: true,
                                    offered_today: vec![prey::OfferedCreature {
                                        creature: "oteryn:creature.dragon".into(),
                                        price_gold: 45000,
                                        occupied_slot: 0,
                                    }],
                                    offered_next: String::new(),
                                };
                                let payload = prey::encode_reply(&reply).map_err(|_| "reply")?;
                                write_frame(
                                    &mut server,
                                    &encode_command_result(
                                        1,
                                        34 + id,
                                        id,
                                        CommandStatus::Accepted,
                                        &payload,
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
                    for request in requests {
                        let payload = prey::encode_request(&request).map_err(|_| "request")?;
                        let result = session.gated_command(140, 160, &payload).await;
                        if selected {
                            let reply =
                                prey::decode_reply(&result?.payload).map_err(|_| "reply")?;
                            assert_eq!(reply.list_size, 12);
                            assert_eq!(reply.offered_today[0].price_gold, 45000);
                        } else {
                            assert!(matches!(
                                result,
                                Err(SessionError::CapabilityNotSelected { capability: 140 })
                            ));
                        }
                    }
                    peer.await??;
                }
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "Prey selection thread panicked")?
}

#[test]
fn derived_gated_routes_match_previous_table() {
    // Former routes plus PKG5 hotkeys and CP01 property/service commands.
    // Compare sets because
    // the protocol registry generates its entries in capability order.
    let expected: &[(u32, &[u32], &[u32])] = &[
        (3, &[7, 8], &[7, 8]),
        (51, &[60, 61], &[]),
        // P-C06 routes existing timed-item, cyclopedia-map and party-skull queries/intents.
        (39, &[38], &[]),
        (57, &[72, 73], &[]),
        (134, &[155], &[]),
        (40, &[40, 41], &[]),
        (42, &[45, 46], &[]),
        (43, &[48, 49, 50], &[]),
        (53, &[64], &[]),
        (54, &[66, 67], &[]),
        (55, &[68, 69], &[]),
        // P-C04: existing move-out, scope-transfer and guildhall leadership routes.
        (59, &[76, 77], &[]),
        (86, &[102], &[51]),
        (99, &[115], &[]),
        // P-C01 already routes charm progression and expert fight modes on this base.
        (67, &[84], &[]),
        (87, &[103], &[52]),
        (60, &[78], &[26]),
        (62, &[79], &[27]),
        (63, &[80], &[28]),
        (64, &[81], &[]),
        // P-C01 already enables charm progression and PvP expert; preserve that exact table.
        (67, &[84], &[]),
        (87, &[103], &[52]),
        (68, &[85], &[]),
        (72, &[89], &[]),
        (75, &[], &[40]),
        (81, &[97], &[]),
        (82, &[98], &[]),
        // P-C03 routes the existing character-info and raid-schedule commands.
        (97, &[113], &[]),
        (149, &[171, 172, 173], &[]),
        (98, &[114], &[63]),
        (132, &[153], &[97]),
        (200, &[390, 391], &[]),
        (232, &[444, 445, 446], &[]),
        (285, &[501], &[]),
        (287, &[], &[100]),
        (115, &[124], &[]),
        // P-C05 exposes the existing 117 routes (133, 169, 178-180, 340, 491); no table change.
        // C49: the boss status panel uses the registry-owned capability 117 route.
        (117, &[133, 169, 178, 179, 180, 340, 491, 710], &[99]),
        (140, &[160], &[]),
        (165, &[220], &[]),
        (16, &[22], &[16]),
        (18, &[], &[17]),
        (19, &[], &[18]),
        (23, &[], &[19, 20]),
        (288, &[], &[101]),
        // CLCAST: native caster controls and room; Watch 289 stays account-only.
        (290, &[502, 505], &[]),
        (291, &[504], &[103]),
        (292, &[503], &[]),
        (330, &[560, 561], &[]),
        (37, &[36], &[]),
        (31, &[], &[23]),
        (25, &[25], &[21]),
        (26, &[], &[22]),
        (1, &[4, 5], &[4, 5]),
        (8, &[], &[13]),
        (9, &[14], &[]),
        (10, &[], &[15]),
        // C12 enables the existing world-interaction and item-look command routes.
        (11, &[15, 31], &[]),
        (28, &[30], &[]),
        (14, &[21], &[14]),
        (179, &[348], &[]),
        (180, &[], &[]),
        // PREY-DIFF: native Prey controls use the existing command160.
        (140, &[160], &[]),
        (147, &[170], &[]),
        // Wheel and Atelier are playable through the selected capability 148.
        (148, &[161, 308, 309, 424], &[]),
        (150, &[174, 175, 176], &[]),
        // Live balances use their own selected capability without changing Atelier V1.
        (272, &[484], &[]),
        // pkg4 additions: raid server messages arrive on domain 57 under 92 zones_nologout;
        // animus mastery (168) and loyalty skills (274) query through their gated commands.
        (92, &[], &[57]),
        (168, &[232], &[]),
        (274, &[488], &[]),
        (276, &[492], &[]),
        (277, &[493], &[]),
        (278, &[494], &[]),
        (279, &[495], &[]),
        (280, &[496], &[]),
        // NX-5: item-only visuals use a separate transient stream.
        (
            350,
            &[],
            &[oteryn_protocol_oteryn::presentation::STATE_DOMAIN_ITEM_EFFECTS],
        ),
        (370, &[640], &[]),
        (381, &[662], &[]),
        (382, &[664], &[]),
        (383, &[], &[]),
        (384, &[669], &[]),
        (207, &[402], &[]),
        (445, &[730], &[]),
        (446, &[], &[]),
        // Native cue basis and object appearance add no command or domain owner.
        (385, &[], &[]),
        (386, &[], &[]),
        // CL25J: native feature adapters now use these existing gated wire routes.
        (151, &[177], &[]),
        (152, &[181, 668], &[]),
        (153, &[182], &[]),
        (154, &[183, 184, 185, 186], &[]),
        (195, &[378, 379, 380, 392, 393, 394, 395, 641, 660], &[]),
        (197, &[383, 384], &[98]),
        (199, &[387, 388], &[]),
        (225, &[431], &[]),
        (230, &[440], &[]),
        (233, &[447], &[]),
        // P-C01: native Bestiary/Charms (command 84) and PvP expert (command 103,
        // confirmed-mode domain 52) panels use their existing gated routes.
        (67, &[84], &[]),
        (83, &[], &[48]),
        (87, &[103], &[52]),
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
fn gap_cl1_advertises_existing_character_tab_capability() {
    assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&199));
    assert!(gated_routes().any(|(id, commands, _)| id == 199 && commands.contains(&388)));
}

use oteryn_protocol_oteryn::charm::{
    COMMAND_TYPE_CHARM_ASSIGN_INTENT, STATE_DOMAIN_CHARACTER_CHARMS,
};

#[test]
fn derived_gated_routes_match_previous_table() {
    // Frozen contents of the former GATED_ROUTES table. Compare sets because the
    // protocol registry generates its entries in capability order.
    let expected: &[(u32, &[u32], &[u32])] = &[
        (18, &[], &[17]),
        (37, &[36], &[]),
        (31, &[], &[23]),
        (25, &[25], &[21]),
        (1, &[4, 5], &[4, 5]),
        (8, &[], &[13]),
        (10, &[], &[15]),
        (14, &[21], &[14]),
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
            payload: &overlay_payload,
        },
    ];
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

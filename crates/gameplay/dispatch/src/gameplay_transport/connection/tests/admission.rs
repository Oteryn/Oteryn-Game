#[test]
fn pending_spell_repair_keeps_liveness_and_promotes_on_the_same_connection()
-> Result<(), Box<dyn Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async {
                struct RepairAuthority(tokio::sync::Notify);
                impl FreshAdmissionAuthority for RepairAuthority {
                    async fn admit(
                        &self,
                        _: FreshAdmissionAttempt<'_>,
                    ) -> Result<AdmittedSession, AdmissionRefusal> {
                        Err(AdmissionRefusal::Rejected)
                    }
                    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
                        Some(at(0))
                    }
                    async fn repair_pending_spells(&self, _: AdmittedSession) -> bool {
                        self.0.notified().await;
                        true
                    }
                }
                tokio::time::pause();
                let authority = RepairAuthority(tokio::sync::Notify::new());
                let mut admitted = positioned()?;
                admitted.first_entry = FirstEntryOutcome::AwaitingSpellRepair;
                let (mut server, mut client) = tokio::io::duplex(1 << 16);
                let serving = serve_admitted(&mut server, admitted, &authority, FAST);
                let reading = async {
                    let mut probes = 0;
                    loop {
                        let length = client.read_u32().await? as usize;
                        let mut frame = vec![0; length];
                        client.read_exact(&mut frame).await?;
                        let envelope = decode_wire_envelope(&frame)?;
                        match envelope.message_type() {
                            MessageType::LivenessProbe => {
                                probes += 1;
                                let id = oteryn_protocol_oteryn::decode_liveness_probe(
                                    envelope.payload(),
                                )?;
                                client
                                    .write_all(&framed(&ack(ADMITTED_GENERATION, id)))
                                    .await?;
                                if probes == 2 {
                                    authority.0.notify_one();
                                }
                            }
                            MessageType::SnapshotBegin => assert!(probes >= 2),
                            MessageType::SnapshotCommit => break,
                            MessageType::SnapshotChunk => {}
                            kind => return Err(format!("unexpected repair frame: {kind:?}").into()),
                        }
                    }
                    // An ACK already in flight when repair completes remains valid after setup.
                    client
                        .write_all(&framed(&ack(ADMITTED_GENERATION, 2)))
                        .await?;
                    client.shutdown().await?;
                    Ok::<(), Box<dyn Error>>(())
                };
                let (end, read) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
                    tokio::join!(serving, reading)
                })
                .await
                .map_err(|_| "pending spell repair did not complete its connection")?;
                read?;
                assert!(
                    matches!(end, ConnectionEnd::AdmittedThenDisconnected(session)
            if session.first_entry == FirstEntryOutcome::Reconciled)
                );
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "spell repair connection thread panicked")?
        .map_err(Into::into)
}

#[test]
fn refused_entry_participants_never_invoke_spell_repair() -> Result<(), Box<dyn Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async {
                struct RepairAuthority(Cell<usize>);
                impl FreshAdmissionAuthority for RepairAuthority {
                    async fn admit(
                        &self,
                        _: FreshAdmissionAttempt<'_>,
                    ) -> Result<AdmittedSession, AdmissionRefusal> {
                        Err(AdmissionRefusal::Rejected)
                    }
                    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
                        Some(at(0))
                    }
                    async fn repair_pending_spells(&self, _: AdmittedSession) -> bool {
                        self.0.set(self.0.get() + 1);
                        true
                    }
                }
                let authority = RepairAuthority(Cell::new(0));
                for outcome in [
                    FirstEntryOutcome::NotApplicable,
                    FirstEntryOutcome::RefusedByChannel,
                    FirstEntryOutcome::RefusedStaleAuthority,
                    FirstEntryOutcome::RefusedUnavailable,
                ] {
                    let mut admitted = positioned()?;
                    admitted.first_entry = outcome;
                    let (_, frames) = drive_session(&authority, admitted, &[]).await?;
                    assert!(frames.is_empty());
                }
                assert_eq!(authority.0.get(), 0);
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "spell repair connection thread panicked")?
        .map_err(Into::into)
}

#[test]
fn admitted_session_closes_on_gap_foreign_message_and_stale_generation()
-> Result<(), Box<dyn Error>> {
    run(async {
        let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
        for (frame, error, expected) in [
            (
                command(1, 3, step, StepDirection::East),
                FoundationProtocolError::CommandSequenceGap,
                encode_command_protocol_error(
                    FoundationProtocolError::CommandSequenceGap,
                    1,
                    3,
                    1,
                )?,
            ),
            (
                envelope(99, 1, &[]),
                FoundationProtocolError::UnknownMessageType,
                error_frame(FoundationProtocolError::UnknownMessageType, 1),
            ),
            (
                command(2, 1, step, StepDirection::East),
                FoundationProtocolError::StaleConnectionGeneration,
                error_frame(FoundationProtocolError::StaleConnectionGeneration, 1),
            ),
        ] {
            let authority = StepAuthority {
                steps: RefCell::new(Vec::new()),
                tutorial_notices: Default::default(),
            };
            let (end, frames) = drive_admitted(&authority, &[frame]).await?;
            let mut all = baseline();
            all.push(expected);
            assert_eq!(frames, all);
            assert!(matches!(end, ConnectionEnd::AdmittedThenClosed(_, e) if e == error));
            assert!(authority.steps.borrow().is_empty());
        }
        Ok(())
    })
}

#[test]
fn admission_accepts_only_after_the_owning_authority_commits() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = Authority::new(Ok(()));
        let (end, frames) = drive(&authority, &[bootstrap(1, 1, b"grant")]).await?;
        let admitted = AdmittedSession {
            game_session_id: GameSessionId::decode(&SESSION)?,
            world_id: WorldId::decode(&WORLD)?,
            channel_id: ChannelId::decode(&CHANNEL)?,
            runtime_actor: None,
            first_entry: FirstEntryOutcome::NotApplicable,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        };
        assert_eq!(end, ConnectionEnd::AdmittedThenDisconnected(admitted));
        assert_eq!(authority.calls.get(), 1);
        assert_eq!(
            authority.seen.borrow().as_slice(),
            &[(CharacterId::decode(&CHARACTER)?, b"grant".to_vec())]
        );
        let accepted = decode_wire_envelope(&frames[0])?;
        assert_eq!(accepted.message_type(), MessageType::ServerAccepted);
        assert_eq!(accepted.connection_generation(), 0);
        assert_eq!(frames.len(), 1);
        Ok(())
    })
}

#[test]
fn admission_refusal_writes_no_acceptance() -> Result<(), Box<dyn Error>> {
    run(async {
        for refusal in [
            AdmissionRefusal::Rejected,
            AdmissionRefusal::Unavailable,
            AdmissionRefusal::PremiumRequired,
        ] {
            let authority = Authority::new(Err(refusal));
            let (end, frames) = drive(&authority, &[bootstrap(1, 1, b"grant")]).await?;
            assert_eq!(end, ConnectionEnd::AdmissionRefused(refusal));
            assert_eq!(authority.calls.get(), 1);
            assert!(frames.is_empty());
        }
        Ok(())
    })
}

#[test]
fn join_progress_distinguishes_refused_baseline_and_flushed_snapshot()
-> Result<(), Box<dyn Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            struct JoinAuthority(bool);
            impl FreshAdmissionAuthority for JoinAuthority {
                async fn admit(
                    &self,
                    _: FreshAdmissionAttempt<'_>,
                ) -> Result<AdmittedSession, AdmissionRefusal> {
                    Err(AdmissionRefusal::Rejected)
                }

                async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
                    self.0.then(|| at(0))
                }
            }
            for (entry, observed, expected) in [
                (
                    FirstEntryOutcome::RefusedUnavailable,
                    true,
                    "held_unplayable",
                ),
                (FirstEntryOutcome::Positioned, false, "held_no_baseline"),
                (FirstEntryOutcome::Positioned, true, "snapshot_flushed"),
            ] {
                let mut result = Ok(());
                let lines = oteryn_error_codes::capture(|| {
                    result = run(async {
                        let mut admitted = positioned()?;
                        admitted.first_entry = entry;
                        let (_, frames) =
                            drive_session(&JoinAuthority(observed), admitted, &[]).await?;
                        assert_eq!(frames.is_empty(), expected != "snapshot_flushed");
                        if expected == "snapshot_flushed" {
                            assert_eq!(
                                decode_wire_envelope(frames.last().ok_or("snapshot commit")?)?
                                    .message_type(),
                                MessageType::SnapshotCommit,
                            );
                        }
                        Ok(())
                    })
                    .map_err(|error| error.to_string());
                });
                result?;
                assert!(
                    lines.iter().any(|line| {
                        line.contains("level=info module=gameplay_transport event=join_progress")
                            && line.contains(&format!("phase={expected}"))
                            && line.contains(&format!("first_entry={entry:?}"))
                    }),
                    "missing {expected}: {lines:?}"
                );
            }
            Ok(())
        })?
        .join()
        .map_err(|_| "join progress test thread panicked")?
        .map_err(Into::into)
}

#[test]
fn post_snapshot_wait_reports_phase_before_socket_polling()
-> Result<(), Box<dyn Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            struct SlowLiveSetup;
            impl FreshAdmissionAuthority for SlowLiveSetup {
                async fn admit(
                    &self,
                    _: FreshAdmissionAttempt<'_>,
                ) -> Result<AdmittedSession, AdmissionRefusal> {
                    Err(AdmissionRefusal::Rejected)
                }
                async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
                    Some(at(0))
                }
                async fn gameplay_idle_exemptions(
                    &self,
                    _: AdmittedSession,
                ) -> Option<(bool, bool)> {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    Some((false, false))
                }
            }
            let mut result = Ok(());
            let lines = oteryn_error_codes::capture(|| {
                result = run(async {
                    tokio::time::pause();
                    let (end, frames) = drive_session(&SlowLiveSetup, positioned()?, &[]).await?;
                    assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
                    assert_eq!(
                        decode_wire_envelope(frames.last().ok_or("snapshot commit")?)?
                            .message_type(),
                        MessageType::SnapshotCommit
                    );
                    Ok(())
                })
                .map_err(|error| error.to_string());
            });
            result?;
            let flushed = lines
                .iter()
                .position(|line| line.contains("phase=snapshot_flushed"))
                .ok_or("flushed")?;
            let waiting = lines
                .iter()
                .position(|line| {
                    line.contains("event=connection_phase_wait")
                        && line.contains("phase=idle_exemptions")
                        && line.contains("state=pending")
                })
                .ok_or("missing pending idle_exemptions diagnostic")?;
            let completed = lines
                .iter()
                .position(|line| {
                    line.contains("event=connection_phase_wait")
                        && line.contains("phase=idle_exemptions")
                        && line.contains("state=completed")
                })
                .ok_or("completed")?;
            let ready = lines
                .iter()
                .position(|line| line.contains("phase=live_ready"))
                .ok_or("live ready")?;
            assert!(flushed < waiting && waiting < completed && completed < ready);
            assert!(
                lines
                    .iter()
                    .any(|line| line.contains("event=connection_live_started")
                        && line.contains("phase=frame_error"))
            );
            Ok(())
        })?
        .join()
        .map_err(|_| "post snapshot diagnostic thread panicked")?
        .map_err(Into::into)
}

#[test]
fn premchan_served_channel_list_is_capability_gated_and_keeps_the_flag()
-> Result<(), Box<dyn Error>> {
    struct ChannelAuthority(bool);
    impl FreshAdmissionAuthority for ChannelAuthority {
        async fn admit(
            &self,
            _: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }
        async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(at(0))
        }
        fn channel_list(&self) -> Option<Vec<u8>> {
            Some(oteryn_protocol_oteryn::channel_list::encode_channel_list(
                &WORLD, &CHANNEL, self.0,
            ))
        }
    }
    run(async {
        for premium in [false, true] {
            for selected in [false, true] {
                let mut admitted = positioned()?;
                admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                    PRODUCTION_OFFERED_CAPABILITIES,
                    if selected { &[83] } else { &[] },
                )
                .ok_or("selection")?;
                let (end, frames) =
                    drive_session(&ChannelAuthority(premium), admitted, &[]).await?;
                assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
                let mut lists = Vec::new();
                for frame in frames {
                    let envelope = decode_wire_envelope(&frame)?;
                    if envelope.message_type() == MessageType::SnapshotChunk {
                        for domain in
                            oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?.1
                        {
                            if domain.domain_id == 48 {
                                assert_eq!((domain.revision, domain.snapshot_type), (1, 1));
                                lists.push(domain.payload.to_vec());
                            }
                        }
                    }
                }
                if selected {
                    let mut expected = vec![0x0a, 38, 0x0a, 16];
                    expected.extend_from_slice(&WORLD);
                    expected.extend_from_slice(&[0x12, 16]);
                    expected.extend_from_slice(&CHANNEL);
                    expected.extend_from_slice(&[0x18, u8::from(premium)]);
                    assert_eq!(lists, vec![expected]);
                } else {
                    assert!(lists.is_empty());
                }
            }
        }
        Ok(())
    })
}

/// N8: a classified refusal writes exactly one `ProtocolError` (generation 0,
/// `TRANSPORT_FATAL`, the code, nothing else) and no acceptance.
#[test]
fn classified_admission_refusal_writes_one_protocol_error() -> Result<(), Box<dyn Error>> {
    run(async {
        use FoundationProtocolError as E;
        for error in [
            E::AdmissionGrantMalformed,
            E::AdmissionGrantAuthenticationFailed,
            E::AdmissionGrantBindingMismatch,
            E::AdmissionGrantNotYetValid,
            E::AdmissionGrantExpired,
            E::AdmissionGrantReplayed,
            E::AdmissionAttemptReconciliationRequired,
            E::AdmissionGrantSecurityStateRevoked,
            E::AdmissionGrantSecurityEvidenceStale,
            E::AdmissionGrantRouteStale,
            E::AdmissionGrantRuntimeGenerationStale,
            E::AdmissionGrantWorldStale,
            E::AdmissionGrantRevisionUnsupported,
            E::AdmissionAccountCharacterConflict,
            E::AdmissionIncumbentProtected,
            E::AdmissionCapacityExceeded,
            E::AdmissionCapabilityRequired,
        ] {
            let code = error.code();
            let refusal = AdmissionRefusal::Classified(error);
            let authority = Authority::new(Err(refusal));
            let (end, frames) = drive(&authority, &[bootstrap(1, 1, b"grant")]).await?;
            assert_eq!(end, ConnectionEnd::AdmissionRefused(refusal));
            assert_eq!(authority.calls.get(), 1);
            assert_eq!(frames, vec![error_frame(error, 0)]);
            let envelope = decode_wire_envelope(&frames[0])?;
            assert_eq!(envelope.message_type(), MessageType::ProtocolError);
            assert_eq!(envelope.connection_generation(), 0);
            let view = oteryn_protocol_oteryn::decode_protocol_error(envelope.payload())?;
            assert_eq!(view.error_code, code);
            assert_eq!(
                view.disposition,
                crate::foundation::ProtocolDisposition::TransportFatal
            );
            assert_eq!(
                (
                    view.related_command_id,
                    view.expected_command_id,
                    view.expected_server_sequence
                ),
                (0, 0, 0)
            );
        }
        Ok(())
    })
}

#[test]
fn admission_version_and_profile_rejected_before_authority() -> Result<(), Box<dyn Error>> {
    run(async {
        for (frame, error) in [
            (
                bootstrap(2, 1, b"grant"),
                FoundationProtocolError::ProtocolMajorMismatch,
            ),
            (
                bootstrap(1, 2, b"grant"),
                FoundationProtocolError::TransportProfileMismatch,
            ),
        ] {
            let authority = Authority::new(Ok(()));
            let (end, frames) = drive(&authority, &[frame]).await?;
            assert_eq!(end, ConnectionEnd::ProtocolViolation(error));
            assert_eq!(authority.calls.get(), 0);
            assert_eq!(frames, vec![error_frame(error, 0)]);
        }
        Ok(())
    })
}

#[test]
fn admission_phase_direction_and_malformed_input_rejected_before_authority()
-> Result<(), Box<dyn Error>> {
    run(async {
        let command = envelope(7, 0, &[]);
        let server_direction = envelope(2, 0, &[]);
        let unknown = envelope(99, 0, &[]);
        let generation = {
            let mut payload = Vec::new();
            scalar(&mut payload, 1, 1);
            scalar(&mut payload, 2, 1);
            scalar(&mut payload, 3, 1);
            bytes(&mut payload, 5, b"grant");
            bytes(&mut payload, 6, &CHARACTER);
            bytes(&mut payload, 7, b"seam-test");
            envelope(1, 3, &payload)
        };
        for frame in [command, server_direction, unknown, generation, vec![0xff]] {
            let authority = Authority::new(Ok(()));
            let (end, frames) = drive(&authority, &[frame]).await?;
            assert!(
                matches!(end, ConnectionEnd::ProtocolViolation(_)),
                "{end:?}"
            );
            assert_eq!(authority.calls.get(), 0);
            assert_eq!(frames.len(), 1);
            assert_eq!(
                decode_wire_envelope(&frames[0])?.message_type(),
                MessageType::ProtocolError
            );
        }
        Ok(())
    })
}

#[test]
fn admission_resume_is_not_served_and_admits_nothing() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = Authority::new(Ok(()));
        let (end, frames) = drive(&authority, &[resume()]).await?;
        assert_eq!(end, ConnectionEnd::ResumeUnavailable);
        assert_eq!(authority.calls.get(), 0);
        assert!(frames.is_empty());
        Ok(())
    })
}

#[test]
fn admission_post_admission_input_closes_without_gameplay() -> Result<(), Box<dyn Error>> {
    run(async {
        for (frame, error) in [
            (
                envelope(7, 1, &[]),
                FoundationProtocolError::UnknownMessageType,
            ),
            (
                envelope(7, 2, &[]),
                FoundationProtocolError::StaleConnectionGeneration,
            ),
        ] {
            let authority = Authority::new(Ok(()));
            let (end, frames) = drive(&authority, &[bootstrap(1, 1, b"grant"), frame]).await?;
            let ConnectionEnd::AdmittedThenClosed(_, reported) = end else {
                return Err(format!("{end:?}").into());
            };
            assert_eq!(reported, error);
            assert_eq!(authority.calls.get(), 1);
            assert_eq!(frames.len(), 2);
            assert_eq!(frames[1], error_frame(error, ADMITTED_GENERATION));
        }
        Ok(())
    })
}

#[test]
fn authenticated_queue_refusal_crosses_the_real_connection_driver() -> Result<(), Box<dyn Error>> {
    run(async {
        use oteryn_protocol_oteryn::admission_queue::{AdmissionQueue, decode_queue, encode_queue_refusal};
        let queue = AdmissionQueue { position: 3, retry_after_ms: 2_000 };
        let refusal = AdmissionRefusal::Queued(queue);
        let authority = Authority::new(Err(refusal));
        let (end, frames) = drive(&authority, &[bootstrap(1, 1, b"grant")]).await?;
        assert_eq!(end, ConnectionEnd::AdmissionRefused(refusal));
        assert_eq!(authority.calls.get(), 1);
        assert_eq!(frames, vec![encode_queue_refusal(queue)?]);
        let envelope = decode_wire_envelope(&frames[0])?;
        assert_eq!(decode_queue(envelope.payload())?, Some(queue));
        Ok(())
    })
}

#[test]
fn operator_publication_reloads_through_the_authenticated_connection_writer() -> Result<(), Box<dyn Error>> {
    run(async {
        use crate::gameplay_transport::bulletin::BulletinSource;
        struct Published { inner: Authority, source: std::sync::Arc<BulletinSource>, reads: Cell<u32> }
        impl FreshAdmissionAuthority for Published {
            async fn admit(&self, attempt: FreshAdmissionAttempt<'_>) -> Result<AdmittedSession, AdmissionRefusal> { self.inner.admit(attempt).await }
            async fn client_bulletin(&self) -> Option<String> { self.reads.set(self.reads.get() + 1); self.source.load().await }
        }
        let path = std::env::temp_dir().join(format!("oteryn-wire-bulletin-{}.json", std::process::id()));
        let source = std::sync::Arc::new(BulletinSource::new(path.clone()));
        let authority = Published { inner: Authority::new(Ok(())), source: source.clone(), reads: Cell::new(0) };
        for title in ["First publication", "Operator replacement"] {
            let document = format!(r#"{{"news":[{{"title":"{title}","body":"Live notice"}}],"events":[{{"year":2027,"month":1,"day":12,"title":"Gathering","body":"Town square"}}]}}"#);
            std::fs::write(&path, &document)?;
            let (_, frames) = drive(&authority, &[bootstrap(1, 1, b"grant")]).await?;
            let envelope = decode_wire_envelope(frames.first().ok_or("missing acceptance")?)?;
            assert_eq!(envelope.message_type(), MessageType::ServerAccepted);
            assert_eq!(oteryn_protocol_oteryn::bulletin::document_from_acceptance(envelope.payload())?, Some(document.as_str()));
        }
        assert_eq!(authority.reads.get(), 2);
        let refused = Published { inner: Authority::new(Err(AdmissionRefusal::Rejected)), source, reads: Cell::new(0) };
        let (_, frames) = drive(&refused, &[bootstrap(1, 1, b"invalid")]).await?;
        assert!(frames.is_empty());
        assert_eq!(refused.reads.get(), 0);
        std::fs::remove_file(path)?;
        Ok(())
    })
}

#[test]
fn fresh_refusal_write_uses_only_the_remaining_entry_budget() -> Result<(), Box<dyn Error>> {
    run(async {
        tokio::time::pause();
        let (mut server, _client) = tokio::io::duplex(1);
        let refusal = AdmissionRefusal::Classified(FoundationProtocolError::SessionJoinUnavailable);
        let authority = Authority::new(Err(refusal));
        let start = tokio::time::Instant::now();
        let until = start + std::time::Duration::from_millis(50);
        let end = admit_frame_until(&mut server, &bootstrap(1, 1, b"grant"), &authority, &Identifiers, until).await;
        assert_eq!(end.err(), Some(ConnectionEnd::AdmissionRefused(refusal)));
        let finished = tokio::time::Instant::now();
        assert!(finished >= until);
        assert!(finished < until + std::time::Duration::from_millis(10));
        Ok(())
    })
}

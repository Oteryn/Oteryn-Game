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
        for refusal in [AdmissionRefusal::Rejected, AdmissionRefusal::Unavailable] {
            let authority = Authority::new(Err(refusal));
            let (end, frames) = drive(&authority, &[bootstrap(1, 1, b"grant")]).await?;
            assert_eq!(end, ConnectionEnd::AdmissionRefused(refusal));
            assert_eq!(authority.calls.get(), 1);
            assert!(frames.is_empty());
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

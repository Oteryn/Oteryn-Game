// Real transport-loop coverage; this fixture grants no House gameplay authority.
use super::*;

#[test]
fn bed_sleep_results_require_world_interaction_in_the_real_dispatch() -> Result<(), Box<dyn Error>>
{
    run(async {
        for disposition in [
            UseDisposition::BedChooseSkill,
            UseDisposition::BedOccupied,
            UseDisposition::BedWoken,
            UseDisposition::OfflineTraining,
        ] {
            for selected in [false, true] {
                let authority = UseAuthority {
                    uses: RefCell::new(Vec::new()),
                    commands: RefCell::new(Vec::new()),
                    overlay: None,
                    watermark: None,
                    outcome: UseOutcome {
                        disposition,
                        ..UseOutcome::rejected()
                    },
                };
                let capabilities = if selected {
                    SelectedCapabilities::select(
                        &[OfferedCapability {
                            id: CAPABILITY_WORLD_INTERACTION_V1,
                            requires: &[],
                        }],
                        &[CAPABILITY_WORLD_INTERACTION_V1],
                    )
                    .ok_or("selection")?
                } else {
                    SessionContinuity::FRESH.selected_capabilities
                };
                let admitted = AdmittedSession {
                    continuity: SessionContinuity {
                        selected_capabilities: capabilities,
                        ..SessionContinuity::FRESH
                    },
                    ..positioned()?
                };
                let frame = use_command(1, 1, u64::from(COMMAND_TYPE_USE_INTENT), b"bed-test", 0);
                let (_, frames) = drive_session(&authority, admitted, &[frame]).await?;
                let (status, result) = if selected {
                    (CommandStatus::Accepted, disposition)
                } else {
                    (CommandStatus::Rejected, UseDisposition::Rejected)
                };
                let payload = encode_use_result(result).map_err(|error| format!("{error:?}"))?;
                assert!((0..64).any(|sequence| {
                    encode_command_result(1, 1, sequence, status, &payload)
                        .is_ok_and(|result| frames.contains(&result))
                }));
                assert_eq!(authority.commands.borrow().len(), 1);
            }
        }
        Ok(())
    })
}

#[test]
fn bed_sleep_dispatch_requires_capability_and_refuses_malformed_or_stale_targets()
-> Result<(), Box<dyn Error>> {
    run(async {
        let authority = UseAuthority {
            uses: RefCell::new(Vec::new()),
            commands: RefCell::new(Vec::new()),
            overlay: None,
            watermark: None,
            outcome: UseOutcome::rejected(),
        };
        for (selected, payload, expected_status, expected_payload) in [
            (
                false,
                vec![10, 2, 8, 1, 16, 1],
                CommandStatus::Rejected,
                vec![],
            ),
            (
                true,
                vec![],
                CommandStatus::Rejected,
                encode_use_result(UseDisposition::Rejected)
                    .map_err(|error| format!("{error:?}"))?,
            ),
            (
                true,
                vec![10, 2, 8, 1, 16, 7],
                CommandStatus::Rejected,
                encode_use_result(UseDisposition::Rejected)
                    .map_err(|error| format!("{error:?}"))?,
            ),
            (
                true,
                vec![10, 2, 8, 1, 16, 1],
                CommandStatus::Accepted,
                encode_use_result(UseDisposition::StaleState)
                    .map_err(|error| format!("{error:?}"))?,
            ),
        ] {
            let capabilities = if selected {
                SelectedCapabilities::select(
                    &[OfferedCapability {
                        id: CAPABILITY_WORLD_INTERACTION_V1,
                        requires: &[],
                    }],
                    &[CAPABILITY_WORLD_INTERACTION_V1],
                )
                .ok_or("selection")?
            } else {
                SessionContinuity::FRESH.selected_capabilities
            };
            let admitted = AdmittedSession {
                continuity: SessionContinuity {
                    selected_capabilities: capabilities,
                    ..SessionContinuity::FRESH
                },
                ..positioned()?
            };
            let frame = oteryn_protocol_oteryn::encode_client_command(
                1,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id: 1,
                    command_type: COMMAND_TYPE_BED_SLEEP_INTENT,
                    payload: &payload,
                },
            )?;
            let (_, frames) = drive_session(&authority, admitted, &[frame]).await?;
            assert!((0..64).any(|sequence| {
                encode_command_result(1, 1, sequence, expected_status, &expected_payload)
                    .is_ok_and(|result| frames.contains(&result))
            }));
        }
        assert!(authority.commands.borrow().is_empty());
        Ok(())
    })
}

use super::*;

/// WORLDINT-WIRE-1: a `SEALED` authority outcome carries its gate message id through the
/// server to the result under capability 11, and is refused as plain `Rejected` without it.
#[test]
fn a_sealed_use_carries_its_gate_message_id_only_under_capability_11() -> Result<(), Box<dyn Error>>
{
    run(async {
        let authority = UseAuthority {
            uses: RefCell::new(Vec::new()),
            commands: RefCell::new(Vec::new()),
            overlay: None,
            watermark: None,
            outcome: UseOutcome {
                disposition: UseDisposition::Sealed,
                committed: None,
                earned: EarnedNotice::NoneEarned,
                gate_message_id: Some(42),
                refusal_detail: None,
            },
        };
        let use_type = u64::from(COMMAND_TYPE_USE_INTENT);
        let frames_with = |capabilities: &'static [u32]| {
            let offered = [
                OfferedCapability {
                    id: CAPABILITY_ITEM_USE_V1,
                    requires: &[],
                },
                OfferedCapability {
                    id: 18,
                    requires: &[],
                },
                OfferedCapability {
                    id: CAPABILITY_WORLD_INTERACTION_V1,
                    requires: &[
                        CAPABILITY_ITEM_USE_V1, // ITEM_USE_V1
                        18,                     // WORLD_MAP_VIEW_V1
                    ],
                },
            ];
            let selected = SelectedCapabilities::select(&offered, capabilities).expect("selection");
            let admitted = AdmittedSession {
                continuity: SessionContinuity {
                    selected_capabilities: selected,
                    ..SessionContinuity::FRESH
                },
                ..positioned().expect("session")
            };
            (
                admitted,
                use_command(1, 1, use_type, b"oteryn:cell/entry-door", 0),
            )
        };
        let result_present = |frames: &[Vec<u8>], status, payload: &[u8]| {
            (0..64u64).any(|sequence| {
                encode_command_result(1, 1, sequence, status, payload)
                    .is_ok_and(|expected| frames.contains(&expected))
            })
        };
        let (admitted, command) = frames_with(&[
            CAPABILITY_ITEM_USE_V1,          // ITEM_USE_V1
            18,                              // WORLD_MAP_VIEW_V1
            CAPABILITY_WORLD_INTERACTION_V1, // WORLD_INTERACTION_V1
        ]);
        let (_end, frames) = drive_session(&authority, admitted, &[command]).await?;
        let sealed = encode_use_result_with_gate(UseDisposition::Sealed, Some(42)).expect("sealed");
        assert!(result_present(&frames, CommandStatus::Accepted, &sealed));

        let (admitted, command) = frames_with(&[CAPABILITY_ITEM_USE_V1]);
        let (_end, frames) = drive_session(&authority, admitted, &[command]).await?;
        assert!(result_present(
            &frames,
            CommandStatus::Rejected,
            &encode_use_result(UseDisposition::Rejected).unwrap_or_default()
        ));
        Ok(())
    })
}

#[test]
fn prey_unoffered_command_is_refused_by_real_dispatch() -> Result<(), Box<dyn Error>> {
    use oteryn_protocol_oteryn::prey::{self, CAPABILITY_PREY_V1, COMMAND_TYPE_PREY};
    run(async {
        for offered in [
            super::super::super::capabilities::PRODUCTION_OFFERED_CAPABILITIES,
            super::super::super::capabilities::ITEM_VIEW_OFFERED_CAPABILITIES,
            super::super::super::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
        ] {
            let selected =
                SelectedCapabilities::select(offered, &[CAPABILITY_PREY_V1]).ok_or("selection")?;
            assert!(!selected.as_slice().contains(&CAPABILITY_PREY_V1));
            assert!(!selected.command_selected(COMMAND_TYPE_PREY));
            let authority = UseAuthority {
                uses: RefCell::new(Vec::new()),
                commands: RefCell::new(Vec::new()),
                overlay: None,
                watermark: None,
                outcome: UseOutcome {
                    disposition: UseDisposition::NothingToUse,
                    committed: None,
                    earned: EarnedNotice::NoneEarned,
                    gate_message_id: None,
                    refusal_detail: None,
                },
            };
            let admitted = AdmittedSession {
                continuity: SessionContinuity {
                    selected_capabilities: selected,
                    ..SessionContinuity::FRESH
                },
                ..positioned()?
            };
            for payload in [
                prey::encode_request(&prey::Request::Query).map_err(|e| format!("{e:?}"))?,
                vec![255],
            ] {
                let mut command = Vec::new();
                scalar(&mut command, 1, 1);
                scalar(&mut command, 2, u64::from(COMMAND_TYPE_PREY));
                bytes(&mut command, 4, &payload);
                let command = envelope(7, ADMITTED_GENERATION, &command);
                let (_, frames) = drive_session(&authority, admitted, &[command]).await?;
                assert!(frames.contains(&encode_command_result(
                    1,
                    1,
                    1,
                    CommandStatus::Rejected,
                    &[]
                )?));
                assert!(authority.uses.borrow().is_empty());
                assert!(authority.commands.borrow().is_empty());
            }
        }
        Ok(())
    })
}

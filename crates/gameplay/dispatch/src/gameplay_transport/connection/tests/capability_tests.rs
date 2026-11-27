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
                reward_granted: None,
                gate_message_id: Some(42),
                cooldown_clearance: None,
                potion_utterance: None,
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
                    reward_granted: None,
                    gate_message_id: None,
                    cooldown_clearance: None,
                    potion_utterance: None,
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

#[test]
fn depot_search_without_negotiated_capability_is_refused_by_real_dispatch()
-> Result<(), Box<dyn Error>> {
    use oteryn_protocol_oteryn::depot_search::{
        self, CAPABILITY_DEPOT_SEARCH_V1, COMMAND_TYPE_DEPOT_SEARCH,
    };
    run(async {
        use super::super::super::capabilities::{
            BUNDLE_WORLD_OFFERED_CAPABILITIES, ITEM_VIEW_OFFERED_CAPABILITIES,
            PRODUCTION_OFFERED_CAPABILITIES,
        };
        for (offered, supported) in [
            (PRODUCTION_OFFERED_CAPABILITIES, &[153][..]),
            (ITEM_VIEW_OFFERED_CAPABILITIES, &[153][..]),
            (BUNDLE_WORLD_OFFERED_CAPABILITIES, &[][..]),
            (BUNDLE_WORLD_OFFERED_CAPABILITIES, &[153][..]),
        ] {
            let selected = SelectedCapabilities::select(offered, supported).ok_or("selection")?;
            assert!(!selected.as_slice().contains(&CAPABILITY_DEPOT_SEARCH_V1));
            assert!(!selected.command_selected(COMMAND_TYPE_DEPOT_SEARCH));
            let authority = UseAuthority {
                uses: RefCell::new(Vec::new()),
                commands: RefCell::new(Vec::new()),
                overlay: None,
                watermark: None,
                outcome: UseOutcome {
                    disposition: UseDisposition::NothingToUse,
                    committed: None,
                    earned: EarnedNotice::NoneEarned,
                    reward_granted: None,
                    gate_message_id: None,
                    cooldown_clearance: None,
                    potion_utterance: None,
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
                depot_search::encode_request(depot_search::Request::Search { client_id: 3031 })
                    .map_err(|e| format!("{e:?}"))?,
                vec![255],
            ] {
                let mut command = Vec::new();
                scalar(&mut command, 1, 1);
                scalar(&mut command, 2, u64::from(COMMAND_TYPE_DEPOT_SEARCH));
                bytes(&mut command, 4, &payload);
                let command = envelope(7, ADMITTED_GENERATION, &command);
                let (_, frames) = drive_session(&authority, admitted, &[command]).await?;
                let mut results = 0;
                for frame in &frames {
                    let envelope = oteryn_protocol_oteryn::decode_wire_envelope(frame)?;
                    if envelope.message_type() == oteryn_protocol_oteryn::MessageType::CommandResult
                    {
                        let result =
                            oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                        assert_eq!(result.command_id, 1);
                        assert_eq!(result.status, CommandStatus::Rejected);
                        assert!(result.payload.is_empty());
                        results += 1;
                    }
                }
                assert_eq!(results, 1);
                assert!(authority.uses.borrow().is_empty());
                assert!(authority.commands.borrow().is_empty());
            }
        }
        Ok(())
    })
}

#[test]
fn entry_room_dev_client_selects_guild_and_party_without_bundle_mechanics()
-> Result<(), Box<dyn Error>> {
    // qualification::seam_flow has no bundle, active native gameplay or NPC book.
    // The real node's default entry-room generation also has no native gameplay.
    let authority = Authority::new(Err(AdmissionRefusal::Rejected));
    let base =
        crate::gameplay_transport::world_required_capabilities::offered_for_scope(false, None);
    let offered = mechanic_offers(&authority, base.to_vec());
    let selected =
        SelectedCapabilities::select(&offered, oteryn_dev_client::CLIENT_SUPPORTED_CAPABILITIES)
            .ok_or("selection capacity")?;
    assert!(
        selected.contains(40),
        "guild has no prerequisite and is offered on the entry room"
    );
    assert!(
        selected.contains(42),
        "party requires only spatial entities (6)"
    );
    for id in [4, 37, 43, 60, 62, 63, 68, 72, 81, 140, 165, 286] {
        assert!(
            !selected.contains(id),
            "unexpected entry-room mechanic {id}"
        );
    }
    Ok(())
}

#[test]
fn potion_utterance_only_reaches_the_negotiated_use_receipt() -> Result<(), Box<dyn Error>> {
    use oteryn_protocol_oteryn::world_object::{
        CAPABILITY_POTION_UTTERANCE_V1, PotionUtterance, decode_use_result_with_utterance,
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async {
                let authority = UseAuthority {
                    uses: RefCell::new(Vec::new()),
                    commands: RefCell::new(Vec::new()),
                    overlay: None,
                    watermark: None,
                    outcome: UseOutcome {
                        disposition: UseDisposition::Committed,
                        potion_utterance: Some(PotionUtterance::Restore),
                        ..UseOutcome::rejected()
                    },
                };
                let offered = [
                    OfferedCapability {
                        id: CAPABILITY_ITEM_USE_V1,
                        requires: &[],
                    },
                    OfferedCapability {
                        id: CAPABILITY_POTION_UTTERANCE_V1,
                        requires: &[CAPABILITY_ITEM_USE_V1],
                    },
                ];
                for (supported, expected) in [
                    (&[CAPABILITY_ITEM_USE_V1][..], None),
                    (
                        &[CAPABILITY_ITEM_USE_V1, CAPABILITY_POTION_UTTERANCE_V1][..],
                        Some(PotionUtterance::Restore),
                    ),
                ] {
                    let mut admitted = positioned()?;
                    admitted.continuity.selected_capabilities =
                        SelectedCapabilities::select(&offered, supported)
                            .ok_or("potion capability selection")?;
                    let command = use_command(
                        1,
                        1,
                        u64::from(COMMAND_TYPE_USE_INTENT),
                        b"oteryn:cell/entry-door",
                        0,
                    );
                    let (_, frames) = drive_session(&authority, admitted, &[command]).await?;
                    let mut replies = 0;
                    for frame in frames {
                        let envelope = decode_wire_envelope(&frame)?;
                        if envelope.message_type() == MessageType::CommandResult {
                            let result =
                                oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                            assert_eq!(result.status, CommandStatus::Accepted);
                            if expected.is_none() {
                                assert_eq!(result.payload, [0x08, 1]);
                            }
                            assert_eq!(
                                decode_use_result_with_utterance(result.payload)
                                    .map_err(|error| format!("{error:?}"))?,
                                (UseDisposition::Committed, None, expected),
                            );
                            replies += 1;
                        }
                    }
                    assert_eq!(replies, 1);
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "potion receipt connection panicked")??;
    Ok(())
}

#[test]
fn stash_row_edit_uncertainty_does_not_publish_a_terminal_refusal() -> Result<(), Box<dyn Error>> {
    use oteryn_protocol_oteryn::stash::{
        CAPABILITY_STASH_ACTIONS_V1, COMMAND_TYPE_STASH_ROW_ACTION, StashRowAction, StashRowIntent,
        encode_stash_row_intent,
    };
    struct UncertainEdit(Cell<bool>);
    impl FreshAdmissionAuthority for UncertainEdit {
        async fn admit(
            &self,
            _: FreshAdmissionAttempt<'_>,
        ) -> Result<AdmittedSession, AdmissionRefusal> {
            Err(AdmissionRefusal::Rejected)
        }

        async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
            Some(at(0))
        }

        async fn observe_character_inventory(
            &self,
            _: ExactActorRef,
            _: GameSessionId,
        ) -> Option<InventoryItems> {
            Some(InventoryItems::default())
        }

        async fn stash_row_action(
            &self,
            _: ExactActorRef,
            _: UseCommand,
            intent: StashRowIntent,
            _: bool,
        ) -> Result<Option<std::num::NonZeroU32>, crate::durability::DurabilityError> {
            assert_eq!(intent.action, StashRowAction::LootAdd);
            self.0.set(true);
            Err(crate::durability::DurabilityError::Unavailable)
        }
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async {
                let authority = UncertainEdit(Cell::new(false));
                let mut admitted = positioned()?;
                let offered = super::super::super::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES;
                admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                    offered,
                    &[4, 6, 37, 62, 98, CAPABILITY_STASH_ACTIONS_V1],
                )
                .ok_or("stash actions selection")?;
                let payload = encode_stash_row_intent(StashRowIntent {
                    row: uuid_v7(23),
                    action: StashRowAction::LootAdd,
                })
                .map_err(|error| format!("{error:?}"))?;
                let command = oteryn_protocol_oteryn::encode_client_command(
                    1,
                    &oteryn_protocol_oteryn::ClientCommandValue {
                        command_id: 1,
                        command_type: COMMAND_TYPE_STASH_ROW_ACTION,
                        payload: &payload,
                    },
                )?;
                let (end, frames) = drive_session(&authority, admitted, &[command]).await?;
                assert!(authority.0.get());
                assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
                for frame in frames {
                    assert_ne!(
                        decode_wire_envelope(&frame)?.message_type(),
                        MessageType::CommandResult
                    );
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "stash row edit connection panicked")??;
    Ok(())
}

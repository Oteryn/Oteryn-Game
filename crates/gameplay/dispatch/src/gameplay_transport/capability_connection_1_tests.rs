#[test]
fn a_bundle_world_admits_only_a_client_that_selects_18() -> Result<(), Box<dyn Error>> {
    run(async {
        let mut authority = NegotiatingAuthority::new(Some(BUNDLE_WORLD_OFFERED_CAPABILITIES));
        // ADMIT0: a bundle World's floor is 11 and 18, including the cooking USE dependency.
        authority.required = Some(
            super::super::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[11, CAPABILITY_WORLD_MAP_VIEW_V1],
                crate::foundation::RuntimeScopeRefV1::channel(
                    WorldId::decode(&WORLD)?,
                    ChannelId::decode(&CHANNEL)?,
                ),
                1,
                "capability-transport-fixture",
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
            )
            .map_err(|error| format!("{error:?}"))?,
        );
        let required =
            encode_protocol_error(FoundationProtocolError::AdmissionCapabilityRequired, 0)?;
        // 4, 6 and 18 select 18 with the rest of the production set the client supports.
        let (admitted, frames) =
            admit(&authority, &bootstrap(&[4, 6, 11, 13, 15, 17, 18])?).await?;
        assert_eq!(accepted_selection(&frames)?, [4, 6, 11, 13, 15, 17, 18]);
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        assert!(
            admitted
                .continuity
                .selected_capabilities
                .contains(CAPABILITY_WORLD_MAP_VIEW_V1)
        );
        // A client lacking any member of the closure is refused by the owner, after its
        // authentication, with exactly one ADMISSION_CAPABILITY_REQUIRED (1116).
        for supported in [
            &[
                4,  // ITEM_VIEW_MOVE_V1
                6,  // WORLD_SPATIAL_ENTITIES
                13, // PACED_MOVEMENT_V1
                17, // ATTACK_V1
            ][..],
            &[
                6,  // WORLD_SPATIAL_ENTITIES
                13, // PACED_MOVEMENT_V1
                17, // ATTACK_V1
                18, // WORLD_MAP_VIEW_V1
            ],
            &[
                4,  // ITEM_VIEW_MOVE_V1
                13, // PACED_MOVEMENT_V1
                17, // ATTACK_V1
                18, // WORLD_MAP_VIEW_V1
            ],
            &[],
        ] {
            let (refused, frames) = admit(&authority, &bootstrap(supported)?).await?;
            assert!(matches!(
                refused,
                Err(ConnectionEnd::AdmissionRefused(
                    AdmissionRefusal::Classified(
                        FoundationProtocolError::AdmissionCapabilityRequired
                    )
                ))
            ));
            assert_eq!(frames, std::slice::from_ref(&required));
        }
        Ok(())
    })
}

#[test]
fn a_bundle_world_recovery_lacking_18_is_refused_with_1118_and_one_frame()
-> Result<(), Box<dyn Error>> {
    run(async {
        let mut authority = NegotiatingAuthority::new(Some(BUNDLE_WORLD_OFFERED_CAPABILITIES));
        authority.required = Some(
            super::super::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[11, CAPABILITY_WORLD_MAP_VIEW_V1],
                crate::foundation::RuntimeScopeRefV1::channel(
                    WorldId::decode(&WORLD)?,
                    ChannelId::decode(&CHANNEL)?,
                ),
                1,
                "capability-transport-fixture",
                BUNDLE_WORLD_OFFERED_CAPABILITIES,
            )
            .map_err(|error| format!("{error:?}"))?,
        );
        let (admitted, _) = admit(&authority, &bootstrap(&[4, 6, 11, 13, 15, 17, 18])?).await?;
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        authority.lost.set(Some(admitted));
        // ADMIT0 §5 / D1187: a recovery lacking a required member is answered with exactly
        // one RECOVERY_CAPABILITY_REQUIRED (1118, OPERATION_TERMINAL), not silently dropped.
        let required =
            encode_protocol_error(FoundationProtocolError::RecoveryCapabilityRequired, 0)?;
        for supported in [
            &[
                4,  // ITEM_VIEW_MOVE_V1
                6,  // WORLD_SPATIAL_ENTITIES
                13, // PACED_MOVEMENT_V1
                17, // ATTACK_V1
            ][..],
            &[
                6,  // WORLD_SPATIAL_ENTITIES
                13, // PACED_MOVEMENT_V1
                17, // ATTACK_V1
                18, // WORLD_MAP_VIEW_V1
            ],
            &[],
        ] {
            let (refused, frames) = admit(&authority, &resume(supported)).await?;
            assert_eq!(
                refused.err(),
                Some(ConnectionEnd::AdmissionRefused(
                    AdmissionRefusal::RecoveryCapabilityRequired
                ))
            );
            assert_eq!(frames, std::slice::from_ref(&required));
        }
        assert_eq!(
            FoundationProtocolError::RecoveryCapabilityRequired.disposition(),
            crate::foundation::ProtocolDisposition::OperationTerminal
        );
        // The same session, supporting the closure, resumes with its selection.
        let (resumed, frames) = admit(&authority, &resume(&[4, 6, 11, 13, 15, 17, 18])).await?;
        let resumed = resumed.map_err(|end| format!("{end:?}"))?;
        assert!(
            resumed
                .continuity
                .selected_capabilities
                .contains(CAPABILITY_WORLD_MAP_VIEW_V1)
        );
        assert_eq!(frames.len(), 1);
        Ok(())
    })
}

#[test]
fn a_fixture_world_offers_exactly_the_production_set_and_never_18() -> Result<(), Box<dyn Error>> {
    run(async {
        // MAP-CUTOVER-1b: a World without a bundle keeps the production set and domain 2.
        let authority = NegotiatingAuthority::new(None);
        assert_eq!(
            authority.offered_capabilities(),
            PRODUCTION_OFFERED_CAPABILITIES
        );
        assert!(authority.required.is_none());
        let (admitted, frames) =
            admit(&authority, &bootstrap(&[4, 6, 11, 13, 15, 17, 18])?).await?;
        let selected = accepted_selection(&frames)?;
        assert!(!selected.contains(&CAPABILITY_WORLD_MAP_VIEW_V1));
        admitted.map_err(|end| format!("{end:?}"))?;
        Ok(())
    })
}

/// Tibian noon (minute 720) of the default epoch: day light 250, color 215.
const NOON_UTC_MS: u64 = 720 * 2_500;

#[test]
fn admission_selecting_19_sends_the_domain_18_snapshot_at_the_carried_revision()
-> Result<(), Box<dyn Error>> {
    run(async {
        // CLOCK-1 (§11.3): the snapshot reads the World clock at the authority's time fact.
        use oteryn_protocol_oteryn::world_clock::{WorldClock, encode_world_clock};
        let noon = encode_world_clock(WorldClock {
            tibian_minute: 720,
            light_level: 250,
            light_color: 215,
        })
        .map_err(|error| format!("{error:?}"))?;
        let carries_noon = |frames: &[Vec<u8>]| {
            frames
                .concat()
                .windows(noon.len())
                .any(|w| w == noon.as_slice())
        };
        let authority = NegotiatingAuthority::new(None);
        let (admitted, frames) = admit(
            &authority,
            &bootstrap(&[
                6,  // WORLD_SPATIAL_ENTITIES
                19, // WORLD_LIGHT_V1
            ])?,
        )
        .await?;
        let expected = [
            6,  // WORLD_SPATIAL_ENTITIES
            19, // WORLD_LIGHT_V1
        ];
        assert_eq!(accepted_selection(&frames)?, expected);
        let mut admitted = admitted.map_err(|end| format!("{end:?}"))?;
        assert_eq!(admitted.continuity.world_clock_revision, 0);
        admitted.continuity.world_clock_revision = 7;
        let (end, frames) = serve(&authority, admitted, &[]).await?;
        assert!(carries_noon(&frames));
        // A resume or transfer never winds the revision back.
        let ConnectionEnd::AdmittedThenDisconnected(served) = end else {
            return Err(format!("{end:?}").into());
        };
        assert_eq!(served.continuity.world_clock_revision, 7);
        let (admitted, _) = admit(&authority, &bootstrap(&[6])?).await?;
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        let (_, frames) = serve(&authority, admitted, &[]).await?;
        assert!(!carries_noon(&frames));
        Ok(())
    })
}

#[test]
fn item_view_admission_selecting_4_sends_domain_9_and_without_4_none() -> Result<(), Box<dyn Error>>
{
    run(async {
        // MAP-ITEM-REF-1: the Item view offered set selects 4 with its required 6, and the
        // admitted session is sent domain 9; a session admitted without 4 is sent none. The
        // fixture serves no combat state, so the client here does not support 17.
        let authority = NegotiatingAuthority::new(Some(ITEM_VIEW_OFFERED_CAPABILITIES));
        let inventory = authority
            .observe_character_inventory(
                ExactActorRef::transport_fixture(
                    WorldId::decode(&WORLD)?,
                    ChannelId::decode(&CHANNEL)?,
                ),
                GameSessionId::decode(&SESSION)?,
            )
            .await
            .ok_or("inventory")?;
        let [nine, _] = SessionItemView::resume(ItemViewContinuity::default())
            .snapshot(inventory, None)
            .map_err(|error| format!("{error:?}"))?;
        let carries_nine = |frames: &[Vec<u8>]| {
            frames
                .concat()
                .windows(nine.payload.len())
                .any(|w| w == nine.payload)
        };
        let (admitted, frames) = admit(
            &authority,
            &bootstrap(&[
                4,  // ITEM_VIEW_MOVE_V1
                6,  // WORLD_SPATIAL_ENTITIES
                13, // PACED_MOVEMENT_V1
            ])?,
        )
        .await?;
        let expected = [
            4,  // ITEM_VIEW_MOVE_V1
            6,  // WORLD_SPATIAL_ENTITIES
            13, // PACED_MOVEMENT_V1
        ];
        assert_eq!(accepted_selection(&frames)?, expected);
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        let (_, frames) = serve(&authority, admitted, &[]).await?;
        assert!(carries_nine(&frames));
        let (admitted, frames) = admit(
            &authority,
            &bootstrap(&[
                6,  // WORLD_SPATIAL_ENTITIES
                13, // PACED_MOVEMENT_V1
            ])?,
        )
        .await?;
        let expected = [
            6,  // WORLD_SPATIAL_ENTITIES
            13, // PACED_MOVEMENT_V1
        ];
        assert_eq!(accepted_selection(&frames)?, expected);
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        let (_, frames) = serve(&authority, admitted, &[]).await?;
        assert!(!frames.is_empty());
        assert!(!carries_nine(&frames));
        Ok(())
    })
}

#[test]
fn resume_keeps_the_original_selection_and_a_downgrade_falls_back_to_fresh_admission()
-> Result<(), Box<dyn Error>> {
    run(async {
        let authority = NegotiatingAuthority::new(Some(INJECTED));
        let (admitted, _) = admit(
            &authority,
            &bootstrap(&[
                1, // BESTIARY_CHARMS_V1
                8, // ACHIEVEMENT_NOTICES_V1
            ])?,
        )
        .await?;
        let admitted = admitted.map_err(|end| format!("{end:?}"))?;
        authority.lost.set(Some(admitted));

        // A resume that supports more keeps exactly the original selection.
        let (resumed, frames) = admit(
            &authority,
            &resume(&[
                1,  // BESTIARY_CHARMS_V1
                7,  // CHAT_V1
                8,  // ACHIEVEMENT_NOTICES_V1
                10, // ANALYSER_V1
            ]),
        )
        .await?;
        let resumed = resumed.map_err(|end| format!("{end:?}"))?;
        let expected = [
            1, // BESTIARY_CHARMS_V1
            8, // ACHIEVEMENT_NOTICES_V1
        ];
        assert_eq!(
            resumed.continuity.selected_capabilities.as_slice(),
            expected
        );
        let expected = encode_server_resume_accepted(&ServerResumeAcceptedValue {
            game_session_id: resumed.game_session_id,
            connection_generation: resumed.continuity.connection_generation,
            current_server_sequence: resumed.continuity.server_sequence,
            next_command_id: resumed.continuity.next_command_id,
            schema_revision: 1,
            selected_capabilities: &[
                1, // BESTIARY_CHARMS_V1
                8, // ACHIEVEMENT_NOTICES_V1
            ],
        })?;
        assert_eq!(frames, [expected]);

        // A resume that lacks selected capability 8 is not acknowledged: the client falls back
        // to fresh admission, which negotiates again from what it now supports.
        let (refused, frames) = admit(&authority, &resume(&[1])).await?;
        assert!(frames.is_empty());
        assert!(matches!(
            refused,
            Err(ConnectionEnd::AdmittedThenDisconnected(_))
        ));
        let (fresh, frames) = admit(&authority, &bootstrap(&[1])?).await?;
        assert!(fresh.is_ok());
        assert_eq!(accepted_selection(&frames)?, [1]);
        Ok(())
    })
}

#[test]
fn an_unselected_capability_command_is_refused_and_a_core_one_dispatched()
-> Result<(), Box<dyn Error>> {
    run(async {
        let authority = NegotiatingAuthority::new(Some(INJECTED));
        let step = oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::North);
        let frames = [
            command(1, 13, b"chat"),
            command(2, 4, b"charm"),
            command(3, 1, &step),
            command(
                4,
                oteryn_protocol_oteryn::depot::COMMAND_TYPE_DEPOT_INTENT,
                &[8, 1, 16, 1],
            ),
        ];
        let (_, written) = serve(&authority, session(SessionContinuity::FRESH)?, &frames).await?;
        let rejected = |sequence, id| {
            crate::foundation::encode_command_result(
                1,
                sequence,
                id,
                crate::foundation::CommandStatus::Rejected,
                &[],
            )
        };
        assert!(written.contains(&rejected(1, 1)?));
        assert!(written.contains(&rejected(2, 2)?));
        assert!(written.contains(&rejected(4, 4)?));
        assert_eq!(authority.steps.get(), 1);
        Ok(())
    })
}

#[test]
fn an_unselected_domain_is_never_sent() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = NegotiatingAuthority::new(Some(INJECTED));
        let spatial = encode_world_spatial(&WorldSpatialObservation {
            content_generation: [0x5c; 32],
            actor_position: ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
        });
        let notices = encode_achievement_notices_snapshot(AchievementWatermark {
            fact_count: 1,
            total_points: 3,
        });
        let mut domains = vec![DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            revision: 1,
            snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
            payload: &spatial,
        }];
        // The registered overlay domain is always sent, empty at revision 0.
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            revision: 0,
            snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
            payload: &[],
        });
        let core: Vec<Vec<u8>> = encode_single_chunk_snapshot(1, 1, 0, &domains)?.into();
        domains.push(DomainSnapshot {
            domain_id: STATE_DOMAIN_ACCOUNT_ACHIEVEMENT_NOTICES,
            revision: 0,
            snapshot_type: SNAPSHOT_TYPE_ACHIEVEMENT_NOTICES_V1,
            payload: &notices,
        });
        let with_notices: Vec<Vec<u8>> = encode_single_chunk_snapshot(1, 1, 0, &domains)?.into();

        // A notice revision without capability 8 selected sends nothing of domain 13.
        let unselected = SessionContinuity {
            achievement_notice_revision: Some(0),
            ..SessionContinuity::FRESH
        };
        let (_, frames) = serve(&authority, session(unselected)?, &[]).await?;
        assert_eq!(frames, core);
        let selected = SessionContinuity {
            achievement_notice_revision: Some(0),
            selected_capabilities: selection(&[8]),
            ..SessionContinuity::FRESH
        };
        let (_, frames) = serve(&authority, session(selected)?, &[]).await?;
        assert_eq!(frames, with_notices);
        Ok(())
    })
}

// ITEM-USE-WIRE-1: capability 15 ITEM_USE_V1 is registered and gated, not offered, and
// requires 4; fields 4 and 5 of USE_INTENT are REJECTED without it.

/// An injected offered set: 4 alone and 15 requiring 4.
const ITEM_USE_OFFERED: &[OfferedCapability] = &[
    OfferedCapability {
        id: CAPABILITY_ITEM_VIEW_MOVE_V1,
        requires: &[],
    },
    OfferedCapability {
        id: CAPABILITY_ITEM_USE_V1,
        requires: &[CAPABILITY_ITEM_VIEW_MOVE_V1],
    },
];

/// One use result as written: command ID, status and payload.
type UseResult = (u64, crate::foundation::CommandStatus, Vec<u8>);

#[test]
fn item_use_wire_fields_4_and_5_are_rejected_without_capability_15() -> Result<(), Box<dyn Error>> {
    use super::super::item_view::UseItemTarget;
    use crate::foundation::CommandStatus;
    use oteryn_protocol_oteryn::decode_command_result;
    use oteryn_protocol_oteryn::item_view::ItemHandle;
    use oteryn_protocol_oteryn::world_object::{
        COMMAND_TYPE_USE_INTENT, UseDisposition, UseIntent, UseTarget, encode_item_use_intent,
        encode_use_result,
    };
    use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
    use std::num::NonZeroU32;
    run(async {
        let authority = NegotiatingAuthority::new(Some(ITEM_USE_OFFERED));
        let creature = Some(EntityRef {
            identity: [0x5c; 16],
            generation: 1,
        });
        let with = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(ItemHandle::MIN),
            use_with: creature,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(|error| format!("{error:?}"))?;
        let by_definition = encode_item_use_intent(&UseIntent {
            target: UseTarget::ItemByDefinition(NonZeroU32::MIN),
            use_with: None,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(|error| format!("{error:?}"))?;
        let by_definition_with = encode_item_use_intent(&UseIntent {
            target: UseTarget::ItemByDefinition(NonZeroU32::MIN),
            use_with: creature,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(|error| format!("{error:?}"))?;
        let frames = [
            command(1, COMMAND_TYPE_USE_INTENT, &with),
            command(2, COMMAND_TYPE_USE_INTENT, &by_definition),
            command(3, COMMAND_TYPE_USE_INTENT, &by_definition_with),
        ];
        // The use results written, by command ID.
        let results = |written: &[Vec<u8>]| -> Result<Vec<UseResult>, Box<dyn Error>> {
            let mut results = Vec::new();
            for frame in written {
                let envelope = decode_wire_envelope(frame)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload())?;
                    results.push((result.command_id, result.status, result.payload.to_vec()));
                }
            }
            Ok(results)
        };
        let rejected = (
            CommandStatus::Rejected,
            encode_use_result(UseDisposition::Rejected).unwrap_or_default(),
        );
        for selected in [&[][..], &[CAPABILITY_ITEM_VIEW_MOVE_V1]] {
            let continuity = SessionContinuity {
                selected_capabilities: selection(selected),
                ..SessionContinuity::FRESH
            };
            let (_, written) = serve(&authority, session(continuity)?, &frames).await?;
            let expected: Vec<_> = (1..=3)
                .map(|id| (id, rejected.0, rejected.1.clone()))
                .collect();
            assert_eq!(results(&written)?, expected, "{selected:?}");
        }
        // Under capability 15, before ITEM-USE-1: a use on a creature fails closed, a use by
        // definition finds nothing to use, a corpse handle still opens and any other item handle
        // still finds nothing to use.
        let backpack = ItemHandle::MIN;
        let use_handle = |id| {
            command(
                id,
                COMMAND_TYPE_USE_INTENT,
                &oteryn_protocol_oteryn::world_object::encode_use_item_intent(backpack),
            )
        };
        let with_backpack = encode_item_use_intent(&UseIntent {
            target: UseTarget::Item(backpack),
            use_with: creature,
            use_with_map_item: None,
            use_with_position: None,
        })
        .map_err(|error| format!("{error:?}"))?;
        let item_use = selection(&[
            CAPABILITY_ITEM_VIEW_MOVE_V1, // ITEM_VIEW_MOVE_V1
            CAPABILITY_ITEM_USE_V1,       // ITEM_USE_V1
        ]);
        let here = ActorPosition {
            x: 0,
            y: 0,
            floor: 0,
        };
        for (target, disposition) in [
            (UseItemTarget::NotACorpse, UseDisposition::NothingToUse),
            (
                UseItemTarget::Corpse {
                    position: ActorPosition {
                        x: 1,
                        y: 0,
                        floor: 0,
                    },
                    contents: Vec::new(),
                },
                UseDisposition::Committed,
            ),
        ] {
            *authority.item_target.borrow_mut() = Some(ItemTargetObservation {
                actor: here,
                target,
            });
            let continuity = SessionContinuity {
                selected_capabilities: item_use,
                ..SessionContinuity::FRESH
            };
            let frames = [
                command(1, COMMAND_TYPE_USE_INTENT, &with_backpack),
                command(2, COMMAND_TYPE_USE_INTENT, &by_definition),
                command(3, COMMAND_TYPE_USE_INTENT, &by_definition_with),
                use_handle(4),
            ];
            let (_, written) = serve(&authority, session(continuity)?, &frames).await?;
            assert_eq!(
                results(&written)?,
                [
                    (1, rejected.0, rejected.1.clone()),
                    (
                        2,
                        CommandStatus::Accepted,
                        encode_use_result(UseDisposition::NothingToUse).unwrap_or_default()
                    ),
                    (3, rejected.0, rejected.1.clone()),
                    (
                        4,
                        CommandStatus::Accepted,
                        encode_use_result(disposition).unwrap_or_default()
                    ),
                ],
                "{disposition:?}"
            );
        }
        Ok(())
    })
}

// RUNE-WIRE-1 (RUNE-USE-0 §4): capability 22 RUNE_USE_V1 is registered, not offered before
// RUNE-CAST-1, and requires 15; an unqualified USE position source is REJECTED.

/// An injected offered set: 4, 6, 15 requiring 4 and 22 requiring 15.
pub(in crate::gameplay_transport) const RUNE_USE_OFFERED: &[OfferedCapability] = &[
    OfferedCapability {
        id: CAPABILITY_WORLD_SPATIAL_ENTITIES,
        requires: &[],
    },
    OfferedCapability {
        id: CAPABILITY_ITEM_VIEW_MOVE_V1,
        requires: &[],
    },
    OfferedCapability {
        id: CAPABILITY_ITEM_USE_V1,
        requires: &[CAPABILITY_ITEM_VIEW_MOVE_V1],
    },
    OfferedCapability {
        id: oteryn_protocol_oteryn::world_object::CAPABILITY_RUNE_USE_V1,
        requires: &[CAPABILITY_ITEM_USE_V1],
    },
];

#[test]
fn rune_wire_capability_22_is_registered_not_offered_and_requires_15() -> Result<(), Box<dyn Error>>
{
    use oteryn_protocol_oteryn::world_object::CAPABILITY_RUNE_USE_V1;
    let entry = registry_capabilities()?
        .into_iter()
        .find(|capability| capability["id"] == CAPABILITY_RUNE_USE_V1)
        .ok_or("capability 22")?;
    assert_eq!(entry["offered"], false);
    assert_eq!(ids(&entry["requires"])?, [CAPABILITY_ITEM_USE_V1]);
    assert!(
        PRODUCTION_OFFERED_CAPABILITIES
            .iter()
            .all(|offered| offered.id != CAPABILITY_RUNE_USE_V1)
    );
    // CAP-NEG-1: 22 without 15 (or 15 without 4) is never selected.
    let select = |supported: &[u32]| {
        SelectedCapabilities::select(RUNE_USE_OFFERED, supported)
            .expect("bounded")
            .as_slice()
            .to_vec()
    };
    assert_eq!(select(&[22]), Vec::<u32>::new());
    assert_eq!(
        select(&[
            4,  // ITEM_VIEW_MOVE_V1
            22, // RUNE_USE_V1
        ]),
        [4]
    );
    assert_eq!(
        select(&[
            15, // ITEM_USE_V1
            22, // RUNE_USE_V1
        ]),
        Vec::<u32>::new()
    );
    let expected = [
        4,  // ITEM_VIEW_MOVE_V1
        15, // ITEM_USE_V1
        22, // RUNE_USE_V1
    ];
    assert_eq!(
        select(&[
            4,  // ITEM_VIEW_MOVE_V1
            15, // ITEM_USE_V1
            22, // RUNE_USE_V1
        ]),
        expected
    );
    // It owns no command type or domain.
    let selected = selection(&[22]);
    assert!(!selected.command_selected(COMMAND_TYPE_ITEM_MOVE_INTENT));
    assert!(!selected.domain_selected(STATE_DOMAIN_CHARACTER_INVENTORY));
    Ok(())
}

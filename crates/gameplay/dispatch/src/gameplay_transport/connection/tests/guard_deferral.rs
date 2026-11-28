//! NBREL3: a refused guard consumes only its reserved command, never the session.
use super::*;
use crate::durability::DurabilityError;
use crate::gameplay_transport::world_spatial::decode_step_result;
use oteryn_protocol_oteryn::actor_spell::{
    COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT, decode_spell_cast_result,
};
use oteryn_protocol_oteryn::chat::{
    CAPABILITY_CHAT_V1, COMMAND_TYPE_CHAT_INTENT, ChatDisposition, ChatSpeechMode,
    decode_chat_intent_result, encode_chat_intent,
};
use oteryn_protocol_oteryn::cyclopedia_character::{
    CAPABILITY_CYCLOPEDIA_CHARACTER_V1, COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB, CharacterTab,
};

struct GuardAuthority {
    movement: StepAuthority,
    failure: Cell<Option<fn() -> DurabilityError>>,
    activity: bool,
    commands: crate::gameplay_transport::session_ingress::SessionCommands,
}

impl GuardAuthority {
    fn guard(&self) -> Result<(), DurabilityError> {
        self.failure.take().map_or(Ok(()), |failure| Err(failure()))
    }
}

impl FreshAdmissionAuthority for GuardAuthority {
    fn session_commands(
        &self,
    ) -> Option<&crate::gameplay_transport::session_ingress::SessionCommands> {
        Some(&self.commands)
    }

    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(0))
    }

    async fn house_scope_ack(&self, _: AdmittedSession) -> Result<Option<(u64, Vec<u8>)>, ()> {
        Ok(Some((1, vec![1])))
    }

    async fn house_transfer_input(&self, _: AdmittedSession) -> Result<(), DurabilityError> {
        if self.activity { Ok(()) } else { self.guard() }
    }

    async fn activity_transfer_input(&self, _: AdmittedSession) -> Result<(), DurabilityError> {
        if self.activity { self.guard() } else { Ok(()) }
    }

    async fn paced_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: StepDirection,
    ) -> (StepOutcome, Option<std::time::Duration>) {
        self.movement.paced_step(actor, session, direction).await
    }

    async fn observe_combat_state(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<ActorCombatState> {
        Some(ActorCombatState {
            target: None,
            modes: FightModes::DEFAULT,
            in_fight: false,
        })
    }

    async fn observe_expert_modes(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<oteryn_protocol_oteryn::pvp_expert::ExpertFightModes> {
        use oteryn_protocol_oteryn::pvp_expert::{ExpertFightModes, ExpertMode};
        Some(ExpertFightModes {
            modes: FightModes::DEFAULT,
            expert: ExpertMode::Dove,
        })
    }

    async fn attack_target(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
        _: u64,
        _: Option<WireEntityRef>,
    ) -> AttackIntentDisposition {
        AttackIntentDisposition::Ok
    }

    async fn cyclopedia_character_command(
        &self,
        _: AdmittedSession,
        _: u64,
        _: u32,
        payload: &[u8],
    ) -> Result<Vec<u8>, ()> {
        use oteryn_protocol_oteryn::cyclopedia_character::{CharacterTab, GeneralStats, TabView};
        assert_eq!(payload, [CharacterTab::GeneralStats as u8]);
        TabView::GeneralStats(GeneralStats::default())
            .encode()
            .map_err(|_| ())
    }

    async fn logout(&self, _: AdmittedSession) -> LogoutReply {
        LogoutReply::Result(LogoutResult::Accepted)
    }

    async fn chat_intent_completion(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
        _: u64,
        _: ChatIntent,
    ) -> Option<(ChatIntentResult, Option<SpellCastOutcome>)> {
        // An owner call would leave the command pending instead of terminalizing it.
        None
    }
}

fn guard_case(
    failure: fn() -> DurabilityError,
    transient: bool,
    command_type: u32,
    capabilities: &'static [u32],
    house: bool,
) -> Result<(), Box<dyn Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            run(async {
                let refused = if command_type == COMMAND_TYPE_CHAT_INTENT {
                    let payload = encode_chat_intent(&ChatIntent::Say {
                        mode: ChatSpeechMode::Say,
                        text: "hello".into(),
                    })
                    .map_err(|error| format!("{error:?}"))?;
                    oteryn_protocol_oteryn::encode_client_command(
                        1,
                        &oteryn_protocol_oteryn::ClientCommandValue {
                            command_id: 1,
                            command_type,
                            payload: &payload,
                        },
                    )?
                } else if command_type == oteryn_protocol_oteryn::pvp_expert::COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT {
                    use oteryn_protocol_oteryn::pvp_expert::{ExpertFightModes, ExpertMode, encode_expert_fight_modes};
                    let payload = encode_expert_fight_modes(&ExpertFightModes {
                        modes: FightModes::DEFAULT,
                        expert: ExpertMode::Dove,
                    });
                    oteryn_protocol_oteryn::encode_client_command(
                        1,
                        &oteryn_protocol_oteryn::ClientCommandValue { command_id: 1, command_type, payload: &payload },
                    )?
                } else if command_type == COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB {
                    oteryn_protocol_oteryn::encode_client_command(
                        1,
                        &oteryn_protocol_oteryn::ClientCommandValue {
                            command_id: 1,
                            command_type,
                            payload: &[CharacterTab::Defence as u8, 0, 0, 20],
                        },
                    )?
                } else if command_type == oteryn_protocol_oteryn::pvp_expert::COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT {
                    use oteryn_protocol_oteryn::pvp_expert::{ExpertFightModes, ExpertMode, encode_expert_fight_modes};
                    let payload = encode_expert_fight_modes(&ExpertFightModes {
                        modes: FightModes::DEFAULT,
                        expert: ExpertMode::Dove,
                    });
                    oteryn_protocol_oteryn::encode_client_command(
                        1,
                        &oteryn_protocol_oteryn::ClientCommandValue { command_id: 1, command_type, payload: &payload },
                    )?
                } else if command_type == COMMAND_TYPE_USE_INTENT {
                    use_command(1, 1, u64::from(command_type), b"oteryn:cell/entry-door", 0)
                } else {
                    command(1, 1, u64::from(command_type), StepDirection::East)
                };
                let mut admitted = positioned()?;
                let expert = command_type
                    == oteryn_protocol_oteryn::pvp_expert::COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT;
                let offered = if expert {
                    crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES
                } else {
                    crate::gameplay_transport::capabilities::PRODUCTION_OFFERED_CAPABILITIES
                };
                admitted.continuity.selected_capabilities =
                    SelectedCapabilities::select(offered, capabilities).ok_or("capability selection")?;
                if expert {
                    assert!(admitted.continuity.selected_capabilities.command_selected(command_type));
                }
                if house {
                    use crate::foundation::{
                        ConnectionGeneration, RuntimeScopeRefV1, ScopeOwnershipGeneration,
                    };
                    admitted.item_fence = Some(CurrentCharacterItemFence {
                        character_id: crate::domain::CharacterId::from_bytes(CHARACTER)
                            .map_err(|error| format!("{error:?}"))?,
                        game_session_id: admitted.game_session_id,
                        connection_generation: ConnectionGeneration::new(1)
                            .map_err(|error| format!("{error:?}"))?,
                        character_lease_generation: 1,
                        runtime_scope: RuntimeScopeRefV1::instance(
                            admitted.world_id,
                            uuid_v7(0x66),
                        )
                        .map_err(|error| format!("{error:?}"))?,
                        scope_ownership_generation: ScopeOwnershipGeneration::new(1)
                            .map_err(|error| format!("{error:?}"))?,
                    });
                }
                let step = u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT);
                for activity in [false, true] {
                    let authority = GuardAuthority {
                        movement: StepAuthority {
                            steps: RefCell::new(Vec::new()),
                            tutorial_notices: Default::default(),
                        },
                        failure: Cell::new(Some(failure)),
                        activity,
                        commands: Default::default(),
                    };
                    let (end, frames) = drive_session(
                        &authority,
                        admitted,
                        &[
                            refused.clone(),
                            refused.clone(),
                            command(1, 2, step, StepDirection::North),
                        ],
                    )
                    .await?;
                    assert!(
                        matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)),
                        "{end:?}"
                    );
                    let results: Vec<_> = frames
                        .iter()
                        .map(|frame| decode_wire_envelope(frame))
                        .collect::<Result<Vec<_>, _>>()?
                        .into_iter()
                        .filter(|envelope| envelope.message_type() == MessageType::CommandResult)
                        .map(|envelope| {
                            oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    if transient {
                        assert_eq!(
                            results.len(),
                            3,
                            "busy guard must keep the command stream open"
                        );
                        assert_eq!(results[0].command_id, 1);
                        assert_eq!(results[0].status, CommandStatus::Rejected);
                        match command_type {
                            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT => {
                                assert_eq!(
                                    decode_step_result(results[0].payload),
                                    Ok(StepDisposition::Rejected)
                                );
                                assert_eq!(
                                    results[0].payload,
                                    &[0x08, 0x03],
                                    "STEP refusal receipt in House={house}"
                                );
                            }
                            COMMAND_TYPE_CHAT_INTENT
                                if capabilities.contains(&CAPABILITY_CHAT_V1) =>
                            {
                                assert_eq!(
                                    decode_chat_intent_result(results[0].payload),
                                    Ok(ChatIntentResult {
                                        disposition: ChatDisposition::Rejected,
                                        wait_seconds: 0,
                                    })
                                );
                            }
                            oteryn_protocol_oteryn::pvp_expert::COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT => {
                                assert_eq!(results[0].payload, &[0x08, 0x06]);
                                assert_eq!(
                                    oteryn_protocol_oteryn::attack::decode_attack_intent_result(results[0].payload),
                                    Ok(oteryn_protocol_oteryn::attack::AttackIntentDisposition::Rejected)
                                );
                            }
                            COMMAND_TYPE_USE_INTENT => assert_eq!(
                                oteryn_protocol_oteryn::world_object::decode_use_result(
                                    results[0].payload
                                ),
                                Ok(UseDisposition::Rejected)
                            ),
                            COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT => {
                                assert_eq!(
                                    decode_spell_cast_result(results[0].payload),
                                    Ok(SpellCastDisposition::Rejected)
                                );
                            }
                            COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB
                                if capabilities.contains(&CAPABILITY_CYCLOPEDIA_CHARACTER_V1) =>
                            {
                                assert_eq!(
                                    results[0].error_code,
                                    FoundationProtocolError::CommandRefused.code(),
                                );
                            }
                            _ => {
                                assert!(results[0].payload.is_empty());
                                assert_eq!(results[0].error_code, 0);
                            }
                        }
                        assert_eq!(results[1].command_id, 1);
                        assert_eq!(results[1].status, CommandStatus::DuplicateReplay);
                        assert_eq!(results[1].payload, results[0].payload);
                        assert_eq!(results[1].error_code, results[0].error_code);
                        assert_eq!(results[2].command_id, 2);
                        assert_eq!(results[2].status, CommandStatus::Accepted);
                        assert_eq!(
                            decode_step_result(results[2].payload),
                            Ok(StepDisposition::Blocked)
                        );
                        assert_eq!(*authority.movement.steps.borrow(), [StepDirection::North]);
                    } else {
                        assert!(
                            results.is_empty(),
                            "fence mismatch must disconnect before a result"
                        );
                        assert!(authority.movement.steps.borrow().is_empty());
                    }
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "guard test thread panicked")?
        .map_err(Into::into)
}

#[test]
fn house_transfer_guard_deadline_defers_and_replay_never_executes() -> Result<(), Box<dyn Error>> {
    guard_case(
        || DurabilityError::RootPassDeadlineExceeded,
        true,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        &[],
        false,
    )
}

#[test]
fn house_transfer_guard_unavailable_defers_and_replay_never_executes() -> Result<(), Box<dyn Error>>
{
    guard_case(
        || DurabilityError::RootUnavailable,
        true,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        &[],
        false,
    )
}

#[test]
fn house_transfer_guard_fence_mismatch_disconnects() -> Result<(), Box<dyn Error>> {
    guard_case(
        || DurabilityError::Unavailable,
        false,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        &[],
        false,
    )
}

#[test]
fn selected_chat_guard_deferral_returns_typed_rejection() -> Result<(), Box<dyn Error>> {
    guard_case(
        || DurabilityError::RootUnavailable,
        true,
        COMMAND_TYPE_CHAT_INTENT,
        &[CAPABILITY_CHAT_V1],
        false,
    )
}

#[test]
fn selected_defence_tab_guard_refusal_is_terminal_and_replayable() -> Result<(), Box<dyn Error>> {
    for capabilities in [&[CAPABILITY_CYCLOPEDIA_CHARACTER_V1][..], &[]] {
        guard_case(
            || DurabilityError::RootUnavailable,
            true,
            COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
            capabilities,
            false,
        )?;
    }
    Ok(())
}

#[test]
fn deferred_native_cast_keeps_its_typed_disposition() -> Result<(), Box<dyn Error>> {
    guard_case(
        || DurabilityError::RootUnavailable,
        true,
        COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT,
        &[],
        false,
    )
}

#[test]
fn use_guard_deferral_returns_typed_rejection() -> Result<(), Box<dyn Error>> {
    guard_case(
        || DurabilityError::RootUnavailable,
        true,
        COMMAND_TYPE_USE_INTENT,
        &[],
        false,
    )
}

#[test]
fn unselected_chat_and_unknown_command_keep_empty_guard_rejection() -> Result<(), Box<dyn Error>> {
    guard_case(
        || DurabilityError::RootUnavailable,
        true,
        COMMAND_TYPE_CHAT_INTENT,
        &[],
        false,
    )?;
    guard_case(
        || DurabilityError::RootUnavailable,
        true,
        999,
        &[CAPABILITY_CHAT_V1],
        false,
    )
}

#[test]
fn house_scope_step_guard_defers_and_replay_never_executes() -> Result<(), Box<dyn Error>> {
    guard_case(
        || DurabilityError::RootPassDeadlineExceeded,
        true,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        &[],
        true,
    )?;
    guard_case(
        || DurabilityError::RootUnavailable,
        true,
        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
        &[],
        true,
    )
}

#[test]
fn ordinary_commands_ignore_busy_house_input_holder() -> Result<(), Box<dyn Error>> {
    use oteryn_protocol_oteryn::{
        ClientCommandValue, attack, cyclopedia_character as cyclopedia,
        house_transfer::CAPABILITY_HOUSE_TRANSFER_V1, logout, world_spatial::encode_step_intent,
    };

    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async {
                let mut admitted = positioned()?;
                let caps = [
                    CAPABILITY_WORLD_SPATIAL_ENTITIES,
                    attack::CAPABILITY_ATTACK_V1,
                    logout::CAPABILITY_LOGOUT_V1,
                    CAPABILITY_HOUSE_TRANSFER_V1,
                    cyclopedia::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
                ];
                admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                    crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                    &caps,
                )
                .ok_or("capability selection")?;
                assert_eq!(admitted.continuity.selected_capabilities.as_slice(), caps);
                admitted.continuity.house_input_pending = Some(false);
                let authority = GuardAuthority {
                    movement: StepAuthority {
                        steps: RefCell::new(Vec::new()),
                        tutorial_notices: Default::default(),
                    },
                    failure: Cell::new(Some(|| DurabilityError::RootUnavailable)),
                    activity: false,
                    commands: Default::default(),
                };
                let commands = [
                    (
                        COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                        encode_step_intent(StepDirection::East),
                    ),
                    (
                        attack::COMMAND_TYPE_ATTACK_TARGET_INTENT,
                        attack::encode_attack_target_intent(None).map_err(|e| format!("{e:?}"))?,
                    ),
                    (
                        cyclopedia::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
                        vec![cyclopedia::CharacterTab::GeneralStats as u8],
                    ),
                    (
                        logout::COMMAND_TYPE_LOGOUT_INTENT,
                        logout::encode_logout_intent(),
                    ),
                ];
                let frames = commands
                    .iter()
                    .zip(1..)
                    .map(|((command_type, payload), command_id)| {
                        oteryn_protocol_oteryn::encode_client_command(
                            admitted.continuity.connection_generation,
                            &ClientCommandValue {
                                command_id,
                                command_type: *command_type,
                                payload,
                            },
                        )
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                let (_, replies) = drive_session(&authority, admitted, &frames).await?;
                let mut results = Vec::new();
                for reply in &replies {
                    let envelope = decode_wire_envelope(reply)?;
                    if envelope.message_type() == MessageType::CommandResult {
                        let result =
                            oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                        results.push((result.command_id, result.status));
                    }
                }
                assert_eq!(
                    results,
                    [
                        (1, CommandStatus::Accepted),
                        (2, CommandStatus::Accepted),
                        (3, CommandStatus::Accepted),
                        (4, CommandStatus::Accepted),
                    ]
                );
                assert_eq!(*authority.movement.steps.borrow(), [StepDirection::East]);
                assert!(authority.failure.get().is_some(), "busy guard was called");
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "guard test thread panicked")?
        .map_err(Into::into)
}

#[test]
fn selected_expert_mode_guard_deferral_returns_retryable_refusal() -> Result<(), Box<dyn Error>> {
    use oteryn_protocol_oteryn::{attack::CAPABILITY_ATTACK_V1, pvp_expert::*};
    guard_case(
        || DurabilityError::RootUnavailable,
        true,
        COMMAND_TYPE_EXPERT_FIGHT_MODES_INTENT,
        &[
            CAPABILITY_WORLD_SPATIAL_ENTITIES,
            CAPABILITY_ATTACK_V1,
            CAPABILITY_PVP_EXPERT_V1,
        ],
        false,
    )
}

/// A monk actor whose first periodic Serene evaluation changes Serene (SPELL-D8 §8.2).
struct SereneAuthority {
    ticks: Cell<usize>,
}

impl FreshAdmissionAuthority for SereneAuthority {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _actor: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(0))
    }

    async fn observe_vitals(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> Option<(u64, ActorVitals)> {
        Some((1, SERENE_VITALS))
    }

    async fn tick_vitals(
        &self,
        _actor: ExactActorRef,
        _game_session_id: GameSessionId,
    ) -> Option<(u64, ActorVitals)> {
        self.ticks.set(self.ticks.get() + 1);
        Some((
            2,
            ActorVitals {
                serene: false,
                ..SERENE_VITALS
            },
        ))
    }
}

/// The loop runs the Serene evaluation every 1000 ms while the actor has `ACTOR_VITALS`, and
/// publishes a change as the next `ACTOR_VITALS` delta without any command.
#[test]
fn the_serene_cadence_publishes_a_changed_vitals_delta() -> Result<(), Box<dyn Error>> {
    use super::super::actor_spell::tests as spell;
    run(async {
        let (_runtime, actor, session) = spell::runtime_with_player(0x71);
        let authority = SereneAuthority {
            ticks: Cell::new(0),
        };
        let admitted = AdmittedSession {
            game_session_id: session,
            world_id: WorldId::decode(&uuid_v7(0x60))?,
            channel_id: ChannelId::decode(&uuid_v7(0x61))?,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        };
        let vitals_payload = encode_actor_vitals(&SERENE_VITALS).map_err(|_| "vitals")?;
        let mut expected: Vec<Vec<u8>> = encode_single_chunk_snapshot(
            ADMITTED_GENERATION,
            1,
            0,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 1,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &encode_world_spatial(&at(0)),
                },
                empty_overlay_snapshot(),
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_ACTOR_VITALS,
                    revision: 1,
                    snapshot_type: SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                    payload: &vitals_payload,
                },
            ],
        )?
        .into();
        expected.push(encode_state_delta(
            ADMITTED_GENERATION,
            1,
            STATE_DOMAIN_ACTOR_VITALS,
            1,
            2,
            DELTA_TYPE_ACTOR_VITALS_V1,
            &encode_actor_vitals(&ActorVitals {
                serene: false,
                ..SERENE_VITALS
            })
            .map_err(|_| "vitals")?,
        )?);
        let (mut server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 16);
        let wanted = expected.len();
        let read = async move {
            let mut frames = Vec::new();
            while frames.len() < wanted {
                let mut length = [0_u8; 4];
                client.read_exact(&mut length).await?;
                let mut frame = vec![0_u8; u32::from_be_bytes(length) as usize];
                client.read_exact(&mut frame).await?;
                frames.push(frame);
            }
            // Closing the client ends the served connection.
            drop(client);
            Ok::<_, std::io::Error>(frames)
        };
        let reader = tokio::spawn(read);
        let end = serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS).await;
        let frames = reader.await?;
        assert_eq!(frames?, expected);
        assert_eq!(authority.ticks.get(), 1);
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
        Ok(())
    })
}

#[test]
fn admitted_cast_pays_once_publishes_vitals_and_a_retry_expires() -> Result<(), Box<dyn Error>>
{
    use super::super::actor_spell::{encode_spell_cast_intent, tests as spell};
    run(async {
        let (runtime, actor, session) = spell::runtime_with_player(0x70);
        let mut states = super::super::actor_spell::ChannelSpellStates::default();
        states
            .initialize(
                &runtime,
                actor,
                session,
                spell::FACTS,
                (0, 0),
                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(0),
            )
            .ok_or("initialize")?;
        let authority = SpellAuthority {
            runtime,
            states: RefCell::new(states),
            book: crate::spell::cast::v1_spell_book()?,
            casts: Cell::new(0),
            hide_vitals: false,
        };
        let exura = encode_spell_cast_intent(&spell::exura());
        let unknown = encode_spell_cast_intent(&SpellCastIntent {
            spell: std::num::NonZeroU32::new(9).ok_or("index")?,
            ..spell::exura()
        });
        let admitted = AdmittedSession {
            game_session_id: session,
            world_id: WorldId::decode(&uuid_v7(0x60))?,
            channel_id: ChannelId::decode(&uuid_v7(0x61))?,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        };
        let (end, frames) = drive_session(
            &authority,
            admitted,
            &[
                cast_command(1, &exura),
                cast_command(2, &exura),
                // Spell index 0 does not decode: REJECTED before the Channel owner.
                cast_command(3, &[0x08, 0x00, 0x10, 0x01]),
                cast_command(4, &unknown),
                // The retry of the cast that paid: never executed again.
                cast_command(1, &exura),
            ],
        )
        .await?;
        let full = ActorVitals {
            health: 185,
            max_health: 185,
            mana: 90,
            max_mana: 90,
            soul: 100,
            harmony: 0,
            serene: false,
        };
        let paid = ActorVitals { mana: 70, ..full };
        let result = |sequence, id, status, disposition| {
            encode_command_result(
                ADMITTED_GENERATION,
                sequence,
                id,
                status,
                &encode_spell_cast_result(disposition),
            )
        };
        let full_payload = encode_actor_vitals(&full).map_err(|_| "vitals")?;
        let mut expected: Vec<Vec<u8>> = encode_single_chunk_snapshot(
            ADMITTED_GENERATION,
            1,
            0,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 1,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &encode_world_spatial(&at(0)),
                },
                empty_overlay_snapshot(),
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_ACTOR_VITALS,
                    revision: 1,
                    snapshot_type: SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                    payload: &full_payload,
                },
            ],
        )?
        .into();
        expected.extend([
            result(1, 1, CommandStatus::Accepted, SpellCastDisposition::Cast)?,
            encode_state_delta(
                ADMITTED_GENERATION,
                2,
                STATE_DOMAIN_ACTOR_VITALS,
                1,
                2,
                DELTA_TYPE_ACTOR_VITALS_V1,
                &encode_actor_vitals(&paid).map_err(|_| "vitals")?,
            )?,
            result(
                3,
                2,
                CommandStatus::Accepted,
                SpellCastDisposition::CoolingDown,
            )?,
            result(
                4,
                3,
                CommandStatus::Rejected,
                SpellCastDisposition::Rejected,
            )?,
            result(
                5,
                4,
                CommandStatus::Rejected,
                SpellCastDisposition::Rejected,
            )?,
            encode_command_protocol_error(
                FoundationProtocolError::CommandOutcomeExpired,
                ADMITTED_GENERATION,
                1,
                0,
            )?,
        ]);
        assert_eq!(frames, expected);
        assert!(matches!(
            end,
            ConnectionEnd::AdmittedThenClosed(
                _,
                FoundationProtocolError::CommandOutcomeExpired
            )
        ));
        // The malformed intent and the retry never reached the Channel owner; mana was paid
        // exactly once.
        assert_eq!(authority.casts.get(), 3);
        assert_eq!(
            authority.observe_vitals(actor, session).await,
            Some((2, paid))
        );
        Ok(())
    })
}

struct FieldVitalsAuthority {
    runtime: RefCell<crate::gameplay_transport::ChannelRuntimeV1>,
    states: RefCell<crate::gameplay_transport::actor_spell::ChannelSpellStates>,
    room: crate::content::QualifiedNativeEntryRoom,
    chased: Cell<bool>,
}

impl FreshAdmissionAuthority for FieldVitalsAuthority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(0))
    }

    async fn observe_vitals(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(u64, ActorVitals)> {
        crate::gameplay_transport::actor_spell::observe_vitals(
            &self.runtime.borrow(),
            &self.states.borrow(),
            actor,
            session,
        )
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

    async fn tick_combat(&self, _: ExactActorRef, _: GameSessionId) -> Option<ActorCombatState> {
        Some(ActorCombatState {
            target: Some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: [0x77; 16],
                generation: 1,
            }),
            modes: FightModes {
                chase: ChaseMode::Chase,
                ..FightModes::DEFAULT
            },
            in_fight: true,
        })
    }

    async fn chase_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> Option<(StepOutcome, Option<std::time::Duration>)> {
        if self.chased.replace(true) {
            return None;
        }
        Some(self.paced_step(actor, session, StepDirection::East).await)
    }

    async fn paced_step(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
        direction: StepDirection,
    ) -> (StepOutcome, Option<std::time::Duration>) {
        use crate::gameplay_transport::actor_spell::actor_movement::creature_field_step_tests;
        assert_eq!(direction, StepDirection::East);
        let mut runtime = self.runtime.borrow_mut();
        let contact = creature_field_step_tests::contact(&runtime);
        let (_, duration) = self
            .states
            .borrow_mut()
            .step_with_non_player_field_contact(
                &mut runtime,
                self.room.movement_cells(),
                actor,
                session,
                0,
                crate::movement::CardinalStep::East,
                &std::collections::BTreeSet::new(),
                None,
                None,
                Some(contact),
            )
            .expect("field step commits");
        (
            StepOutcome {
                disposition: StepDisposition::Moved,
                moved_to: Some(at(1)),
            },
            Some(duration),
        )
    }
}

#[test]
fn field_step_dispatch_publishes_committed_vitals_before_cadence() -> Result<(), Box<dyn Error>> {
    use crate::gameplay_transport::actor_spell::actor_movement::creature_field_step_tests;
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async {
                let (runtime, mut states, room, actor, session) =
                    creature_field_step_tests::ready();
                states
                    .get_mut(&runtime, actor, session)
                    .ok_or("field victim state")?
                    .install_equipped_mana_shield(true);
                let authority = FieldVitalsAuthority {
                    runtime: RefCell::new(runtime),
                    states: RefCell::new(states),
                    room,
                    chased: Cell::new(false),
                };
                let mut admitted = positioned()?;
                admitted.game_session_id = session;
                admitted.runtime_actor = Some(actor);
                let (before_revision, before) = authority
                    .observe_vitals(actor, session)
                    .await
                    .ok_or("initial vitals")?;
                let (_, frames) = drive_session(
                    &authority,
                    admitted,
                    &[command(
                        1,
                        1,
                        u64::from(COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT),
                        StepDirection::East,
                    )],
                )
                .await?;
                let (after_revision, after) = authority
                    .observe_vitals(actor, session)
                    .await
                    .ok_or("committed vitals")?;
                assert_eq!(
                    after.health, before.health,
                    "the shield absorbed the field hit"
                );
                assert!(
                    after.mana < before.mana,
                    "the real field entry consumed shield mana"
                );
                let mut actual = Vec::new();
                for frame in &frames {
                    let envelope = oteryn_protocol_oteryn::decode_wire_envelope(frame)?;
                    if envelope.message_type() == oteryn_protocol_oteryn::MessageType::StateDelta {
                        let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
                        if delta.domain_id == STATE_DOMAIN_ACTOR_VITALS {
                            actual.push((delta.base_revision, delta.new_revision, delta.payload));
                        }
                    }
                }
                let expected = encode_actor_vitals(&after).map_err(|_| "vitals encoding")?;
                assert_eq!(
                    actual,
                    [(before_revision, after_revision, expected.as_slice())],
                    "the STEP response must publish the committed field damage immediately"
                );
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "field STEP dispatch thread panicked")?
        .map_err(Into::into)
}

#[test]
fn chase_field_step_publishes_committed_health_and_mana() -> Result<(), Box<dyn Error>> {
    use crate::gameplay_transport::actor_spell::actor_movement::creature_field_step_tests;
    use oteryn_protocol_oteryn::{actor_spell::decode_actor_vitals, decode_state_delta};
    use std::time::Duration;

    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async {
                tokio::time::pause();
                let (runtime, mut states, room, actor, session) =
                    creature_field_step_tests::ready();
                let victim = states
                    .get_mut(&runtime, actor, session)
                    .ok_or("field victim")?;
                victim.restore_saved_vitals(victim.vitals().health, 2);
                victim.install_equipped_mana_shield(true);
                let authority = FieldVitalsAuthority {
                    runtime: RefCell::new(runtime),
                    states: RefCell::new(states),
                    room,
                    chased: Cell::new(false),
                };
                let mut admitted = positioned()?;
                admitted.runtime_actor = Some(actor);
                admitted.game_session_id = session;
                admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                    crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                    &[6, 17],
                )
                .ok_or("chase capabilities")?;
                let (revision, before) = authority
                    .observe_vitals(actor, session)
                    .await
                    .ok_or("initial vitals")?;
                let (mut server, client) = tokio::io::duplex(1 << 16);
                let serving = serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS);
                let driving = async {
                    let mut client = client;
                    // Let the first combat tick schedule the real chase. Stop before the
                    // Serene cadence could send the missing delta on the old code.
                    let update =
                        tokio::time::timeout(COMBAT_REFRESH + Duration::from_millis(100), async {
                            loop {
                                let frame =
                                    crate::gameplay_transport::tcp_tls::read_frame(&mut client)
                                        .await?;
                                let envelope = decode_wire_envelope(&frame)?;
                                if envelope.message_type() == MessageType::StateDelta {
                                    let delta = decode_state_delta(envelope.payload())?;
                                    if delta.domain_id == STATE_DOMAIN_ACTOR_VITALS {
                                        let value = decode_actor_vitals(delta.payload)
                                            .map_err(|_| "vitals payload")?;
                                        return Ok::<_, Box<dyn Error>>((
                                            delta.base_revision,
                                            delta.new_revision,
                                            value,
                                        ));
                                    }
                                }
                            }
                        })
                        .await;
                    let (after_revision, after) = authority
                        .observe_vitals(actor, session)
                        .await
                        .ok_or("committed vitals")?;
                    assert!(authority.chased.get(), "combat scheduled a chase step");
                    assert!(
                        after.health < before.health && after.mana < before.mana,
                        "the field hit altered both pools"
                    );
                    assert_eq!(
                        update.map_err(
                            |_| "chase omitted the field-entry ActorVitals delta before cadence"
                        )??,
                        (revision, after_revision, after),
                    );
                    client.shutdown().await?;
                    Ok::<_, Box<dyn Error>>(())
                };
                let (_, result) = tokio::join!(serving, driving);
                result
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "chase field regression panicked")?
        .map_err(Into::into)
}

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

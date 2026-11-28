#[test]
fn vitals_publication_coalesces_real_changes_from_last_transmitted_revision()
-> Result<(), Box<dyn Error>> {
    let (sequence, wire) = vitals_delta(2, 11, 40, 43, &SERENE_VITALS).ok_or("delta")?;
    let envelope = oteryn_protocol_oteryn::decode_wire_envelope(&wire)?;
    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())?;
    assert_eq!(
        (sequence, delta.base_revision, delta.new_revision),
        (12, 40, 43)
    );
    assert_eq!(
        delta.payload,
        encode_actor_vitals(&SERENE_VITALS).map_err(|_| "vitals")?
    );
    assert!(vitals_delta(2, 11, 0, 43, &SERENE_VITALS).is_none());
    assert!(vitals_delta(2, 11, 43, 43, &SERENE_VITALS).is_none());
    assert!(vitals_delta(2, 11, 44, 43, &SERENE_VITALS).is_none());
    Ok(())
}

struct PendingSpellAuthority;
impl FreshAdmissionAuthority for PendingSpellAuthority {
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
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<(u64, ActorVitals)> {
        Some((1, SERENE_VITALS))
    }
    async fn cast_spell_completion(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
        _: u64,
        _: SpellCastIntent,
    ) -> Option<SpellCastOutcome> {
        None
    }

    async fn chat_intent_completion(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
        _: u64,
        _: oteryn_protocol_oteryn::chat::ChatIntent,
    ) -> Option<(ChatIntentResult, Option<SpellCastOutcome>)> {
        None
    }
}

#[test]
fn unknown_spoken_spell_commit_keeps_command_and_requires_chat_capability()
-> Result<(), Box<dyn Error>> {
    run(async {
        for selected in [false, true] {
            let (_, actor, session) = super::super::actor_spell::tests::runtime_with_player(0x74);
            let mut continuity = SessionContinuity::FRESH;
            if selected {
                continuity.selected_capabilities = SelectedCapabilities::select(
                    super::super::capabilities::PRODUCTION_OFFERED_CAPABILITIES,
                    &[7],
                )
                .ok_or("chat selection")?;
            }
            let admitted = AdmittedSession {
                game_session_id: session,
                world_id: WorldId::decode(&uuid_v7(0x60))?,
                channel_id: ChannelId::decode(&uuid_v7(0x61))?,
                runtime_actor: Some(actor),
                first_entry: FirstEntryOutcome::Positioned,
                controller: None,
                continuity,
                item_fence: None,
            };
            let payload = oteryn_protocol_oteryn::chat::encode_chat_intent(
                &oteryn_protocol_oteryn::chat::ChatIntent::Say {
                    mode: oteryn_protocol_oteryn::chat::ChatSpeechMode::Say,
                    text: "exiva Bob".into(),
                },
            )
            .map_err(|error| format!("{error:?}"))?;
            let command = oteryn_protocol_oteryn::encode_client_command(
                1,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id: 1,
                    command_type: 13,
                    payload: &payload,
                },
            )?;
            let (mut server, mut client) = tokio::io::duplex(1 << 16);
            write_frame(&mut client, &command).await?;
            client.shutdown().await?;
            let end =
                serve_admitted(&mut server, admitted, &PendingSpellAuthority, IDLE_LIVENESS).await;
            let ConnectionEnd::AdmittedThenDisconnected(retained) = end else {
                return Err("unexpected end".into());
            };
            assert_eq!(
                retained.continuity.next_command_id,
                if selected { 1 } else { 2 }
            );
            drop(server);
            let mut bytes = Vec::new();
            client.read_to_end(&mut bytes).await?;
            let mut remaining = bytes.as_slice();
            let mut results = 0;
            while !remaining.is_empty() {
                let prefix: [u8; 4] = remaining.get(..4).ok_or("prefix")?.try_into()?;
                let length = usize::try_from(u32::from_be_bytes(prefix))?;
                let wire = remaining.get(4..4 + length).ok_or("frame")?;
                let envelope = oteryn_protocol_oteryn::decode_wire_envelope(wire)?;
                if envelope.message_type() == MessageType::CommandResult {
                    results += 1;
                }
                remaining = remaining.get(4 + length..).ok_or("tail")?;
            }
            assert_eq!(results, usize::from(!selected));
        }
        Ok(())
    })
}

#[test]
fn unknown_spell_commit_preserves_original_command_and_emits_no_terminal_result()
-> Result<(), Box<dyn Error>> {
    use super::super::actor_spell::tests as spell;
    run(async {
        let (_, actor, session) = spell::runtime_with_player(0x73);
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
        let intent = SpellCastIntent {
            spell: std::num::NonZeroU32::new(1).ok_or("index")?,
            target: oteryn_protocol_oteryn::actor_spell::SpellTarget::None,
            aim_at_target: false,
        };
        let payload = super::super::actor_spell::encode_spell_cast_intent(&intent);
        let (mut server, mut client) = tokio::io::duplex(1 << 16);
        write_frame(&mut client, &cast_command(1, &payload)).await?;
        let end =
            serve_admitted(&mut server, admitted, &PendingSpellAuthority, IDLE_LIVENESS).await;
        let ConnectionEnd::AdmittedThenDisconnected(retained) = end else {
            return Err("unexpected end".into());
        };
        assert_eq!(retained.continuity.next_command_id, 1);
        assert_eq!(retained.continuity.server_sequence, 0);
        assert_eq!(retained.continuity.vitals_revision, 1);
        drop(server);
        let mut bytes = Vec::new();
        client.read_to_end(&mut bytes).await?;
        let mut remaining = bytes.as_slice();
        while !remaining.is_empty() {
            let prefix: [u8; 4] = remaining.get(..4).ok_or("prefix")?.try_into()?;
            let length = usize::try_from(u32::from_be_bytes(prefix))?;
            let wire = remaining.get(4..4 + length).ok_or("frame")?;
            let envelope = oteryn_protocol_oteryn::decode_wire_envelope(wire)?;
            assert_ne!(
                envelope.message_type(),
                oteryn_protocol_oteryn::MessageType::CommandResult
            );
            remaining = remaining.get(4 + length..).ok_or("tail")?;
        }
        Ok(())
    })
}

struct RescuedCastAuthority {
    receipt: crate::gameplay_transport::spell_cooldowns::TestReceipt,
}

impl FreshAdmissionAuthority for RescuedCastAuthority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(0))
    }

    async fn observe_join_vitals(
        &self,
        _: ExactActorRef,
        admitted: AdmittedSession,
        _: bool,
    ) -> crate::gameplay_transport::spell_cooldowns::JoinVitals {
        (Some((1, SERENE_VITALS)), Some(self.receipt.feed(admitted)))
    }

    fn cooldown_clock_us(&self) -> u64 {
        7_000_000
    }

    async fn observe_vitals(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
    ) -> Option<(u64, ActorVitals)> {
        self.receipt.rescued()
    }

    async fn cast_spell_completion(
        &self,
        _: ExactActorRef,
        _: GameSessionId,
        _: u64,
        _: SpellCastIntent,
    ) -> Option<SpellCastOutcome> {
        let vitals = ActorVitals {
            health: 30,
            mana: 1,
            ..SERENE_VITALS
        };
        self.receipt.rescue((3, vitals))?;
        Some(SpellCastOutcome {
            refresh_inventory: false,
            disposition: oteryn_protocol_oteryn::actor_spell::SpellCastDisposition::Cast,
            vitals: Some((2, SERENE_VITALS)),
        })
    }
}

#[test]
fn accepted_cast_result_precedes_the_rescue_pair() -> Result<(), Box<dyn Error>> {
    assert_rescue_cast_frames(true)
}

#[test]
fn accepted_cast_result_precedes_legacy_cooldowns_and_rescued_vitals() -> Result<(), Box<dyn Error>>
{
    assert_rescue_cast_frames(false)
}

fn assert_rescue_cast_frames(sparse: bool) -> Result<(), Box<dyn Error>> {
    use oteryn_protocol_oteryn::{
        MessageType, decode_state_delta, decode_wire_envelope, presentation,
    };
    run(async move {
        let (_, actor, game_session_id) =
            super::super::actor_spell::tests::runtime_with_player(0x73);
        let mut continuity = SessionContinuity::FRESH;
        let offered = super::super::native_cue_offer::offered(
            super::super::capabilities::PRODUCTION_OFFERED_CAPABILITIES,
            true,
        );
        let mut supported = vec![6, presentation::CAPABILITY_PRESENTATION_V1];
        if sparse {
            supported.push(presentation::CAPABILITY_ACTOR_COOLDOWN_DELTAS_V1);
        }
        continuity.selected_capabilities =
            SelectedCapabilities::select(&offered, &supported).ok_or("presentation selection")?;
        let admitted = AdmittedSession {
            game_session_id,
            world_id: WorldId::decode(&uuid_v7(0x60))?,
            channel_id: ChannelId::decode(&uuid_v7(0x61))?,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity,
            item_fence: None,
        };
        let authority = RescuedCastAuthority {
            receipt: Default::default(),
        };
        let payload = super::super::actor_spell::encode_spell_cast_intent(&SpellCastIntent {
            spell: std::num::NonZeroU32::MIN,
            target: oteryn_protocol_oteryn::actor_spell::SpellTarget::None,
            aim_at_target: false,
        });
        let (mut server, mut client) = tokio::io::duplex(1 << 16);
        write_frame(&mut client, &cast_command(1, &payload)).await?;
        client.shutdown().await?;
        serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS).await;
        drop(server);
        let mut bytes = Vec::new();
        client.read_to_end(&mut bytes).await?;
        let mut frames = Vec::new();
        let mut bytes = bytes.as_slice();
        while !bytes.is_empty() {
            let prefix: [u8; 4] = bytes.get(..4).ok_or("prefix")?.try_into()?;
            let length = usize::try_from(u32::from_be_bytes(prefix))?;
            let envelope = decode_wire_envelope(bytes.get(4..4 + length).ok_or("frame")?)?;
            match envelope.message_type() {
                MessageType::CommandResult => {
                    let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                    assert_eq!(
                        result.status,
                        oteryn_protocol_oteryn::CommandStatus::Accepted
                    );
                    frames.push((0, 0));
                }
                MessageType::StateDelta => {
                    let delta = decode_state_delta(envelope.payload())?;
                    if delta.domain_id == presentation::STATE_DOMAIN_ACTOR_COOLDOWNS {
                        assert_eq!(
                            delta.delta_type,
                            if sparse {
                                presentation::DELTA_TYPE_ACTOR_COOLDOWNS_WITH_VITALS_V1
                            } else {
                                presentation::DELTA_TYPE_ACTOR_COOLDOWNS_V1
                            },
                        );
                        let update = presentation::decode_actor_cooldowns(delta.payload)
                            .map_err(|error| format!("{error:?}"))?;
                        assert_eq!(update.server_now_ms, if sparse { 0 } else { 7_000 });
                        assert_eq!(
                            update.entries,
                            [presentation::ActorCooldown {
                                kind: presentation::ActorCooldownKind::Spell,
                                id: 1,
                                expires_at_ms: if sparse { 23_000 } else { 30_000 },
                            }],
                        );
                    }
                    frames.push((delta.domain_id, delta.new_revision));
                }
                _ => {}
            }
            bytes = bytes.get(4 + length..).ok_or("tail")?;
        }
        assert_eq!(
            frames,
            [
                (0, 0),
                (presentation::STATE_DOMAIN_ACTOR_COOLDOWNS, 1),
                (
                    oteryn_protocol_oteryn::actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                    3
                ),
            ]
        );
        Ok(())
    })
}

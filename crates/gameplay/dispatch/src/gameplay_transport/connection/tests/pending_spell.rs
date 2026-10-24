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

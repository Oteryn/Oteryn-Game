use super::*;
use oteryn_protocol_oteryn::actor_spell_catalog as wire;

#[test]
fn spell_catalog_join_indices_drive_real_cast_command() -> Result<(), Box<dyn Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async {
                use crate::gameplay_transport::actor_spell::tests as spell;
                let (runtime, actor, session) = spell::runtime_with_player(0x70);
                let mut states =
                    crate::gameplay_transport::actor_spell::ChannelSpellStates::default();
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
                let mut admitted = positioned()?;
                admitted.runtime_actor = Some(actor);
                admitted.game_session_id = session;
                admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                    &[OfferedCapability {
                        id: wire::CAPABILITY_ACTOR_SPELL_CATALOG_V1,
                        requires: &[],
                    }],
                    &[wire::CAPABILITY_ACTOR_SPELL_CATALOG_V1],
                )
                .ok_or("selection")?;
                let (_, frames) = drive_session(&authority, admitted, &[]).await?;
                let mut received = None;
                for frame in frames {
                    let envelope = decode_wire_envelope(&frame)?;
                    if envelope.message_type() == MessageType::SnapshotChunk {
                        for domain in
                            oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())?.1
                        {
                            if domain.domain_id == wire::STATE_DOMAIN_ACTOR_SPELL_CATALOG {
                                received = Some(
                                    wire::decode_catalog(domain.payload).map_err(|_| "catalog")?,
                                );
                            }
                        }
                    }
                }
                let catalog = received.ok_or("joined actor received no catalog")?;
                for entry in &catalog {
                    let index = std::num::NonZeroU32::new(entry.cast_index).ok_or("index")?;
                    assert_eq!(
                        authority.book.indexed(index).ok_or("real cast index")?.name,
                        entry.name
                    );
                }
                let heal = catalog
                    .iter()
                    .find(|entry| entry.words == "exura")
                    .ok_or("healing spell")?;
                let intent = SpellCastIntent {
                    spell: std::num::NonZeroU32::new(heal.cast_index).ok_or("index")?,
                    target: oteryn_protocol_oteryn::actor_spell::SpellTarget::None,
                    aim_at_target: false,
                };
                let (_, frames) = drive_session(
                    &authority,
                    admitted,
                    &[cast_command(
                        1,
                        &oteryn_protocol_oteryn::actor_spell::encode_spell_cast_intent(&intent),
                    )],
                )
                .await?;
                assert_eq!(authority.casts.get(), 1);
                let expected = encode_spell_cast_result(
                    oteryn_protocol_oteryn::actor_spell::SpellCastDisposition::Cast,
                );
                assert!(
                    frames
                        .iter()
                        .any(|frame| decode_wire_envelope(frame).is_ok_and(|env| {
                            env.message_type() == MessageType::CommandResult
                                && oteryn_protocol_oteryn::decode_command_result(env.payload())
                                    .is_ok_and(|result| result.payload == expected)
                        }))
                );
                Ok(())
            })
            .map_err(|error| error.to_string())
        })
        .map_err(|error| error.to_string())?
        .join()
        .map_err(|_| "scenario thread panicked")?
        .map_err(Into::into)
}

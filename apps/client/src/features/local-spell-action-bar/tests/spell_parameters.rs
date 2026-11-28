use super::play_channel;
use crate::play::BarAction;
use oteryn_protocol_oteryn as wire;
use oteryn_session::{Admission, CLIENT_SUPPORTED_CAPABILITIES, Session, read_frame, write_frame};
use std::{num::NonZeroU32, time::Duration};
use wire::actor_spell::SpellCastDisposition;
use wire::actor_spell_v2::{
    CAPABILITY_ACTOR_SPELL_PARAMETERS_V2, COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_PARAMETERS_V2,
    ParameterSpellCastResult, PrivateSpellEffect, PrivateSpellFeedback,
    decode_parameter_spell_cast_intent, encode_parameter_spell_cast_result,
};

type Error = Box<dyn std::error::Error + Send + Sync>;
const WAIT: Duration = Duration::from_secs(5);
const WORDS: &str = "exiva \"Alice Smith\"";
const FEEDBACK: &str = "Alice Smith is far to the north.";

fn id(marker: u8) -> [u8; 16] {
    let mut id = [marker; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}

async fn join(peer: &mut tokio::io::DuplexStream, selected: &[u32]) -> Result<(), Error> {
    read_frame(peer).await?;
    write_frame(
        peer,
        &wire::encode_server_accepted(&wire::ServerAcceptedValue {
            game_session_id: wire::GameSessionId::decode(&id(1))?,
            world_id: wire::WorldId::decode(&id(2))?,
            channel_id: wire::ChannelId::decode(&id(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 1,
            schema_revision: 1,
            selected_capabilities: selected,
        })?,
    )
    .await?;
    let position = wire::world_spatial::ActorPosition {
        x: 100,
        y: 200,
        floor: 7,
    };
    let (spatial_type, spatial) = if selected.contains(&6) {
        use wire::world_spatial_entities as entities;
        (
            2,
            entities::encode_world_spatial_entities_snapshot(
                &entities::WorldSpatialEntitiesSnapshot {
                    content_generation: [1; 32],
                    actor_position: position,
                    own_identity: [5; 16],
                    entities: vec![entities::WorldSpatialEntity {
                        motion: None,
                        entity: entities::EntityRef {
                            identity: [5; 16],
                            generation: 1,
                        },
                        kind: entities::EntityKind::Player,
                        display_name: None,
                        position,
                        detail: entities::EntityDetail::Actor {
                            direction: wire::world_spatial::StepDirection::South,
                            appearance_ref: 1,
                            health_percent: 100,
                        },
                    }],
                },
            )
            .map_err(|error| format!("{error:?}"))?,
        )
    } else {
        (
            1,
            wire::world_spatial::encode_world_spatial(
                &wire::world_spatial::WorldSpatialObservation {
                    content_generation: [1; 32],
                    actor_position: position,
                },
            ),
        )
    };
    let overlay = wire::world_object::encode_world_object_overlay_snapshot(&[])
        .map_err(|error| format!("{error:?}"))?;
    let catalog = wire::actor_spell_catalog::encode_catalog(&[
        wire::actor_spell_catalog::SpellCatalogEntry {
            cast_index: 71,
            name: "Find Person".into(),
            words: "exiva".into(),
            level: 8,
            mana: 20,
            mana_percent: false,
            soul: 0,
            learned: true,
            available: true,
            cooldown_groups: vec![],
            rune_item: None,
        },
    ])
    .map_err(|error| format!("{error:?}"))?;
    let chat = wire::chat::encode_chat_rooms(wire::chat::ChatRoomSet::default());
    let domains = [
        (1, spatial_type, spatial.as_slice()),
        (2, 1, overlay.as_slice()),
        (12, 1, chat.as_slice()),
        (100, 1, catalog.as_slice()),
    ];
    let domains = domains.map(|(domain_id, snapshot_type, payload)| wire::DomainSnapshot {
        domain_id,
        revision: 1,
        snapshot_type,
        payload,
    });
    for frame in wire::encode_single_chunk_snapshot(1, 1, 0, &domains)? {
        write_frame(peer, &frame).await?;
    }
    Ok(())
}

fn scenario(negotiated: bool) -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    tokio::time::timeout(WAIT, async {
                        let selected: &[u32] = if negotiated {
                            &[4, 6, 7, 287, CAPABILITY_ACTOR_SPELL_PARAMETERS_V2]
                        } else {
                            &[7, 287]
                        };
                        let (stream, mut peer) = tokio::io::duplex(64 * 1024);
                        let (session, joined) = tokio::join!(
                            Session::admit(
                                stream,
                                Admission {
                                    appearance_items: None,
                                    schema_revision: 1,
                                    character_id: wire::CharacterId::decode(&id(4))?,
                                    admission_material: b"grant",
                                    client_build_id: "spell-parameter-test",
                                    supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
                                    deadline: WAIT,
                                }
                            ),
                            join(&mut peer, selected)
                        );
                        joined?;
                        let (link, commands, outbox) = play_channel();
                        let task =
                            tokio::spawn(crate::play::run_session(session?, commands, outbox));
                        while link.spell_catalog_panel().catalog.entries().is_empty() {
                            tokio::task::yield_now().await;
                        }
                        assert!(link.assign_bar_action(
                            0,
                            BarAction::Text {
                                text: WORDS.into(),
                                send_automatically: true,
                            }
                        ));
                        link.cast_spell_slot(NonZeroU32::MIN);
                        let bytes = read_frame(&mut peer).await?;
                        let command = wire::decode_wire_envelope(&bytes)?.client_command(1)?;
                        if negotiated {
                            assert_eq!(
                                command.command_type,
                                COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_PARAMETERS_V2
                            );
                            let cast = decode_parameter_spell_cast_intent(command.payload)
                                .map_err(|error| format!("{error:?}"))?;
                            assert_eq!(cast.intent.spell.get(), 71);
                            assert_eq!(cast.parameter.as_deref(), Some("Alice Smith"));
                            let reply =
                                encode_parameter_spell_cast_result(&ParameterSpellCastResult {
                                    disposition: SpellCastDisposition::Cast,
                                    feedback: Some(PrivateSpellFeedback {
                                        text: FEEDBACK.into(),
                                        effect: PrivateSpellEffect::MagicBlue,
                                    }),
                                    editor: None,
                                })
                                .map_err(|error| format!("{error:?}"))?;
                            write_frame(
                                &mut peer,
                                &wire::encode_command_result(
                                    1,
                                    1,
                                    command.command_id,
                                    wire::CommandStatus::Accepted,
                                    &reply,
                                )?,
                            )
                            .await?;
                            while link.spell_parameter_feedback().as_deref() != Some(FEEDBACK) {
                                tokio::task::yield_now().await;
                            }
                            assert!(matches!(
                                crate::play::mailbox::lock(&link.mailbox)
                                    .pushed
                                    .visuals
                                    .as_slice(),
                                [wire::presentation::PresentationEventKind::MagicEffect {
                                    position: wire::world_spatial::ActorPosition {
                                        x: 100,
                                        y: 200,
                                        floor: 7
                                    },
                                    effect_id: 13,
                                    source: wire::presentation::PresentationSource::Own,
                                }]
                            ));
                        } else {
                            assert_eq!(command.command_type, wire::chat::COMMAND_TYPE_CHAT_INTENT);
                            assert_eq!(
                                command.payload,
                                b"\x0a\x17\x08\x01\x12\x13exiva \"Alice Smith\""
                            );
                        }
                        task.abort();
                        Ok::<(), Error>(())
                    })
                    .await?
                })
        })?
        .join()
        .map_err(|_| "0608 play scenario panicked")?
}

#[test]
fn gap_0608_actor_spell_parameters_v2() -> Result<(), Error> {
    scenario(true)
}

#[test]
fn gap_0608_legacy_spoken_parameter_bytes() -> Result<(), Error> {
    scenario(false)
}

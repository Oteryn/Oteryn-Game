//! C18a replay owner: `oteryn_session::Session::apply_domain_delta` applies the
//! ACTOR_COMBAT_STATE payload, then `protocol::run_cast` reads that state and
//! `protocol::cast_selected` applies the spell result to `SpellFeedback`.

use oteryn_client_runtime::combat::{CombatCommand, CombatReport, run_combat};
use oteryn_client_runtime::spell::SpellFeedback;
use oteryn_protocol_oteryn::actor_spell::{
    self, SpellCastDisposition, SpellCastIntent, SpellTarget,
};
use oteryn_protocol_oteryn::attack::{self, ActorCombatState, FightModes};
use oteryn_protocol_oteryn::{
    DomainSnapshot, ServerAcceptedValue, encode_command_result, encode_server_accepted,
    encode_single_chunk_snapshot,
};
use oteryn_session::{Admission, CharacterId, CommandStatus, Session, read_frame, write_frame};
use std::num::NonZeroU32;
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;

fn fixture() -> Result<(Vec<u8>, Vec<u8>, String), Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok((
            std::fs::read(root.join("c18a-combat.bin"))?,
            std::fs::read(root.join("c18a-result.bin"))?,
            std::fs::read_to_string(root.join("c18a-expected.txt"))?,
        ));
    }
    let target = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: [7; 16],
        generation: 3,
    };
    Ok((
        attack::encode_actor_combat_state(&ActorCombatState {
            target: Some(target),
            modes: FightModes::DEFAULT,
            in_fight: true,
        })
        .map_err(|e| format!("{e:?}"))?,
        actor_spell::encode_spell_cast_result(oteryn_session::SpellCastDisposition::TargetIllegal),
        "TargetIllegal\n".into(),
    ))
}

fn id(marker: u8) -> [u8; 16] {
    [
        1, 0x93, 0x4f, 0x10, 0x7c, 0, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        marker,
    ]
}

#[test]
fn replays_server_payload_and_rejects_invalid_combat_state() -> Result<(), Error> {
    let (combat, result, expected) = fixture()?;
    replay(combat.clone(), result, expected)?;
    replay(
        combat,
        actor_spell::encode_spell_cast_result(oteryn_session::SpellCastDisposition::Rejected),
        "Rejected\n".into(),
    )
}

fn replay(combat: Vec<u8>, result: Vec<u8>, expected: String) -> Result<(), Error> {
    assert!(attack::decode_actor_combat_state(&[0xff]).is_err());
    let disposition =
        actor_spell::decode_spell_cast_result(&result).map_err(|error| format!("{error:?}"))?;
    let result_status = if disposition == SpellCastDisposition::Rejected {
        CommandStatus::Rejected
    } else {
        CommandStatus::Accepted
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), Error> {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async move {
                    let (client, mut peer) = tokio::io::duplex(64 * 1024);
                    let server = tokio::spawn(async move {
                        read_frame(&mut peer).await?;
                        write_frame(
                            &mut peer,
                            &encode_server_accepted(&ServerAcceptedValue {
                                game_session_id: oteryn_protocol_oteryn::GameSessionId::decode(
                                    &id(1),
                                )?,
                                world_id: oteryn_protocol_oteryn::WorldId::decode(&id(2))?,
                                channel_id: oteryn_protocol_oteryn::ChannelId::decode(&id(3))?,
                                connection_generation: 1,
                                current_server_sequence: 0,
                                next_command_id: 1,
                                schema_revision: 1,
                                selected_capabilities: &[6, 17],
                            })?,
                        )
                        .await?;
                        let position = oteryn_protocol_oteryn::world_spatial::ActorPosition {
                            x: 100,
                            y: 200,
                            floor: 7,
                        };
                        let spatial = oteryn_protocol_oteryn::world_spatial_entities::encode_world_spatial_entities_snapshot(
                            &oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntitiesSnapshot {
                                content_generation: [1; 32], actor_position: position,
                                own_identity: [5; 16],
                                entities: vec![oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntity {
                                    entity: oteryn_protocol_oteryn::world_spatial_entities::EntityRef { identity: [5; 16], generation: 1 },
                                    kind: oteryn_protocol_oteryn::world_spatial_entities::EntityKind::Player,
                                    display_name: None, position,
                                    detail: oteryn_protocol_oteryn::world_spatial_entities::EntityDetail::Actor {
                                        direction: oteryn_protocol_oteryn::world_spatial::StepDirection::South,
                                        appearance_ref: 1, health_percent: 100,
                                    },
                                }],
                            },
                        ).map_err(|error| format!("{error:?}"))?;
                        let overlay = oteryn_protocol_oteryn::world_object::encode_world_object_overlay_snapshot(&[])
                            .map_err(|error| format!("{error:?}"))?;
                        for frame in encode_single_chunk_snapshot(
                            1,
                            1,
                            1,
                            &[
                                DomainSnapshot { domain_id: oteryn_protocol_oteryn::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, revision: 1, snapshot_type: oteryn_protocol_oteryn::world_spatial_entities::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, payload: &spatial },
                                DomainSnapshot { domain_id: oteryn_protocol_oteryn::world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY, revision: 1, snapshot_type: oteryn_protocol_oteryn::world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, payload: &overlay },
                                DomainSnapshot { domain_id: attack::STATE_DOMAIN_ACTOR_COMBAT_STATE, revision: 1, snapshot_type: attack::SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1, payload: &combat },
                            ],
                        )? {
                            write_frame(&mut peer, &frame).await?;
                        }
                        let frame = read_frame(&mut peer).await?;
                        let command = oteryn_protocol_oteryn::decode_wire_envelope(&frame)?
                            .client_command(1)?;
                        let intent = actor_spell::decode_spell_cast_intent(command.payload)
                            .map_err(|e| format!("{e:?}"))?;
                        assert_eq!(
                            intent,
                            SpellCastIntent {
                                spell: NonZeroU32::MIN,
                                target: SpellTarget::AttackTarget,
                                aim_at_target: true
                            }
                        );
                        write_frame(
                            &mut peer,
                            &encode_command_result(
                                1,
                                2,
                                command.command_id,
                                result_status,
                                &result,
                            )?,
                        )
                        .await?;
                        Ok::<_, Error>(())
                    });
                    let mut session = Session::admit(
                        client,
                        Admission {
                            appearance_items: None,
                            schema_revision: 1,
                            character_id: CharacterId::decode(&id(4))?,
                            admission_material: b"grant",
                            client_build_id: "c18a-replay",
                            supported_capabilities: &[6, 17],
                            deadline: Duration::from_secs(5),
                        },
                    )
                    .await?;
                    let before = session.combat_state().cloned();
                    let mut feedback = SpellFeedback::default();
                    let report = run_combat(
                        &mut session,
                        &mut feedback,
                        CombatCommand::Cast(NonZeroU32::MIN),
                    )
                    .await?;
                    assert!(matches!(report, CombatReport::Cast(_)));
                    let CombatReport::Cast(disposition) = report else { return Err("cast returned a non-cast report".into()); };
                    assert_eq!(format!("{disposition:?}\n"), expected);
                    let expected_feedback = match disposition {
                        SpellCastDisposition::Cast => "Spell cast",
                        SpellCastDisposition::CoolingDown => "Spell is cooling down",
                        SpellCastDisposition::LevelTooLow => "Level too low",
                        SpellCastDisposition::MagicLevelTooLow => "Magic level too low",
                        SpellCastDisposition::NotEnoughMana => "Not enough mana",
                        SpellCastDisposition::NotEnoughSoul => "Not enough soul",
                        SpellCastDisposition::NotAvailable => "Spell not available",
                        SpellCastDisposition::TargetRequired => "Select a target first",
                        SpellCastDisposition::TargetIllegal => "Illegal target",
                        SpellCastDisposition::Rejected => "Spells unavailable",
                    };
                    assert_eq!(feedback.text(), Some(expected_feedback));
                    assert_eq!(session.combat_state(), before.as_ref());
                    server.await??;
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "C18a replay thread panicked")?
}

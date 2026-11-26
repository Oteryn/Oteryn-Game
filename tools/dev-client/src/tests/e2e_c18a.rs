//! C18a replay owner: `oteryn_session::Session::apply_domain_delta` applies the
//! ACTOR_COMBAT_STATE payload, then `protocol::run_cast` reads that state and
//! `protocol::cast_selected` applies the spell result to `SpellFeedback`.

use oteryn_client_runtime::combat::{CombatCommand, CombatReport, run_combat};
use oteryn_client_runtime::spell::SpellFeedback;
use oteryn_protocol_oteryn::actor_spell::{self, SpellCastIntent, SpellTarget};
use oteryn_protocol_oteryn::attack::{self, ActorCombatState, FightModes};
use oteryn_protocol_oteryn::{
    DomainSnapshot, ServerAcceptedValue, encode_command_result, encode_server_accepted,
    encode_single_chunk_snapshot,
};
use oteryn_session::{Admission, CharacterId, CommandStatus, Session, read_frame, write_frame};
use std::num::NonZeroU32;
use std::time::Duration;

type Error = Box<dyn std::error::Error + Send + Sync>;

struct Fixture {
    combat: Vec<u8>,
    result: Vec<u8>,
    spell: NonZeroU32,
    expected: oteryn_session::SpellCastDisposition,
    feedback: &'static str,
}

fn fixture() -> Result<Fixture, Error> {
    use oteryn_session::SpellCastDisposition;
    let target = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: [7; 16],
        generation: 3,
    };
    Ok(Fixture {
        combat: attack::encode_actor_combat_state(&ActorCombatState {
            target: Some(target),
            modes: FightModes::DEFAULT,
            in_fight: true,
        })
        .map_err(|e| format!("{e:?}"))?,
        result: actor_spell::encode_spell_cast_result(SpellCastDisposition::TargetIllegal),
        spell: NonZeroU32::MIN,
        expected: SpellCastDisposition::TargetIllegal,
        feedback: "Illegal target",
    })
}

fn id(marker: u8) -> [u8; 16] {
    [
        1, 0x93, 0x4f, 0x10, 0x7c, 0, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        marker,
    ]
}

#[test]
fn e2e_c18a_replays_captured_damage_or_unit_refusal() -> Result<(), Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        return replay_capture(std::path::Path::new(&root));
    }
    let Fixture {
        combat,
        result,
        spell,
        expected,
        feedback: expected_feedback,
    } = fixture()?;
    assert!(attack::decode_actor_combat_state(&[0xff]).is_err());
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
                                    motion: None,
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
                        let domains = [
                                DomainSnapshot { domain_id: oteryn_protocol_oteryn::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, revision: 1, snapshot_type: oteryn_protocol_oteryn::world_spatial_entities::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2, payload: &spatial },
                                DomainSnapshot { domain_id: oteryn_protocol_oteryn::world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY, revision: 1, snapshot_type: oteryn_protocol_oteryn::world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, payload: &overlay },
                                DomainSnapshot { domain_id: attack::STATE_DOMAIN_ACTOR_COMBAT_STATE, revision: 1, snapshot_type: attack::SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1, payload: &combat },
                        ];
                        for frame in encode_single_chunk_snapshot(1, 1, 1, &domains)? {
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
                                spell,
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
                                CommandStatus::Accepted,
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
                        CombatCommand::Cast(spell),
                    )
                    .await?;
                    assert!(matches!(report, CombatReport::Cast(_)));
                    let CombatReport::Cast(disposition) = report else { return Err("cast returned a non-cast report".into()); };
                    assert_eq!(disposition, expected);
                    assert_eq!(feedback.text(), Some(expected_feedback));
                    assert_eq!(session.combat_state(), before.as_ref());
                    server.await??;
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "C18a replay thread panicked")?
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    command_id: u64,
    game_session_id: String,
    payload: Vec<u8>,
    server_sequence: u64,
    status: u32,
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Proof {
    game_session_id: String,
    relog_session_id: String,
    command_id: u64,
    cast_sequence: u64,
    health_sequence: u64,
    identity: [u8; 16],
    generation: u64,
    before: u8,
    after: u8,
    capabilities: Vec<u32>,
}

fn session_id(text: &str) -> Result<oteryn_protocol_oteryn::GameSessionId, Error> {
    let uuid = uuid::Uuid::parse_str(text)?;
    if uuid.to_string() != text {
        return Err("C18a session must use canonical UUIDv7 text".into());
    }
    Ok(oteryn_protocol_oteryn::GameSessionId::decode(
        uuid.as_bytes(),
    )?)
}

fn replay_capture(root: &std::path::Path) -> Result<(), Error> {
    use oteryn_protocol_oteryn::world_spatial_entities as wire;
    let cast: Envelope = serde_json::from_slice(&std::fs::read(root.join("c18a-cast.json"))?)?;
    let health: Envelope = serde_json::from_slice(&std::fs::read(root.join("c18a-health.json"))?)?;
    let proof: Proof = serde_json::from_slice(&std::fs::read(root.join("c18a-proof.json"))?)?;
    let session = session_id(&proof.game_session_id)?;
    if session_id(&cast.game_session_id)? != session
        || session_id(&health.game_session_id)? != session
        || session_id(&proof.relog_session_id)? == session
        || proof.command_id == 0
        || cast.command_id != proof.command_id
        || health.command_id != proof.command_id
        || cast.server_sequence == 0
        || cast.server_sequence != proof.cast_sequence
        || health.server_sequence != proof.health_sequence
        || health.server_sequence <= cast.server_sequence
        || cast.status != CommandStatus::Accepted as u32
        || health.status != cast.status
        || proof.generation == 0
        || proof.before > 100
        || proof.after >= proof.before
    {
        return Err("C18a capture provenance, sequence, or health proof is invalid".into());
    }
    let disposition = actor_spell::decode_spell_cast_result(&cast.payload)
        .map_err(|error| format!("{error:?}"))?;
    let mut feedback = SpellFeedback::default();
    feedback.record(disposition);
    assert_eq!(disposition, oteryn_session::SpellCastDisposition::Cast);
    assert_eq!(feedback.text(), Some("Spell cast"));

    let fields = wire::EntityFields::selected(&proof.capabilities);
    let (delta, _, _) =
        wire::decode_world_spatial_entities_delta_with_sidecar(&health.payload, fields)
            .map_err(|error| format!("{error:?}"))?;
    let target = wire::EntityRef {
        identity: proof.identity,
        generation: proof.generation,
    };
    let entry = delta
        .update
        .iter()
        .find(|entry| entry.entity == target)
        .ok_or("C18a health capture lacks the exact Rat generation")?;
    assert_eq!(entry.kind, wire::EntityKind::Creature);
    assert!(
        matches!(entry.detail, wire::EntityDetail::Actor { health_percent, .. }
        | wire::EntityDetail::ActorWithIcons { health_percent, .. } if health_percent == proof.after)
    );
    Ok(())
}

#[test]
fn strict_envelope_rejects_unknown_duplicate_and_missing_keys() {
    let base =
        r#"{"command_id":1,"game_session_id":"x","payload":[1],"server_sequence":2,"status":1}"#;
    assert!(serde_json::from_str::<Envelope>(base).is_ok());
    for extra in [r#","status":1}"#, r#","unexpected":0}"#] {
        let invalid = format!("{}{extra}", base.strip_suffix('}').unwrap());
        assert!(serde_json::from_str::<Envelope>(&invalid).is_err());
    }
    assert!(serde_json::from_str::<Envelope>(r#"{"payload":[1]}"#).is_err());
}

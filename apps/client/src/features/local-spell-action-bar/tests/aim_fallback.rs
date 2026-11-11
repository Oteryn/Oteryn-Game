//! Capability 480: TARGET and CURSOR aim modes are refused by the server for a spell that is not a
//! wave or beam. The client keeps the plain cast (with the held target) for those spells.
use oteryn_client_runtime::combat::{CombatCommand, CombatReport, run_combat};
use oteryn_client_runtime::spell::SpellFeedback;
use oteryn_protocol_oteryn::actor_spell::{
    self, AimedSpellCastIntent, SpellAimMode, SpellCastIntent, SpellTarget,
};
use oteryn_protocol_oteryn::attack::{self, ActorCombatState, FightModes};
use oteryn_protocol_oteryn::world_object::{self, encode_world_object_overlay_snapshot};
use oteryn_protocol_oteryn::world_spatial::{self, ActorPosition};
use oteryn_protocol_oteryn::world_spatial_entities::{
    self, EntityRef, WorldSpatialEntitiesSnapshot,
};
use oteryn_protocol_oteryn::{
    ChannelId, DomainSnapshot, GameSessionId, ServerAcceptedValue, WorldId, decode_wire_envelope,
    encode_command_result, encode_server_accepted, encode_single_chunk_snapshot,
};
use oteryn_session::{
    Admission, CharacterId, CommandStatus, Session, SpellCastDisposition, SpellTargetPosition,
    read_frame, write_frame,
};
use std::num::NonZeroU32;
use std::time::Duration;
use tokio::io::DuplexStream;

type Error = Box<dyn std::error::Error + Send + Sync>;

const HELD: EntityRef = EntityRef {
    identity: [7; 16],
    generation: 3,
};

fn id(marker: u8) -> [u8; 16] {
    [
        0x01, 0x8f, 0x00, 0x00, 0x00, 0x00, 0x70, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        marker,
    ]
}

/// Serves the join, then answers each cast with the next scripted disposition and returns the
/// decoded intents it saw.
async fn peer(
    mut stream: DuplexStream,
    replies: Vec<SpellCastDisposition>,
) -> Result<Vec<AimedSpellCastIntent>, Error> {
    read_frame(&mut stream).await?;
    write_frame(
        &mut stream,
        &encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&id(1))?,
            world_id: WorldId::decode(&id(2))?,
            channel_id: ChannelId::decode(&id(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 1,
            schema_revision: 1,
            selected_capabilities: &[6, 17, 480],
        })?,
    )
    .await?;
    let position = ActorPosition {
        x: 100,
        y: 200,
        floor: 7,
    };
    let spatial = world_spatial_entities::encode_world_spatial_entities_snapshot(
        &WorldSpatialEntitiesSnapshot {
            content_generation: [1; 32],
            actor_position: position,
            own_identity: [5; 16],
            entities: vec![world_spatial_entities::WorldSpatialEntity {
                entity: EntityRef {
                    identity: [5; 16],
                    generation: 1,
                },
                kind: world_spatial_entities::EntityKind::Player,
                display_name: None,
                position,
                detail: world_spatial_entities::EntityDetail::Actor {
                    direction: world_spatial::StepDirection::South,
                    appearance_ref: 1,
                    health_percent: 100,
                },
            }],
        },
    )
    .map_err(|e| format!("spatial: {e:?}"))?;
    let overlay =
        encode_world_object_overlay_snapshot(&[]).map_err(|e| format!("overlay: {e:?}"))?;
    let combat = attack::encode_actor_combat_state(&ActorCombatState {
        target: Some(HELD),
        modes: FightModes::DEFAULT,
        in_fight: true,
    })
    .map_err(|e| format!("combat: {e:?}"))?;
    for frame in encode_single_chunk_snapshot(
        1,
        1,
        10,
        &[
            DomainSnapshot {
                domain_id: world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 1,
                snapshot_type: world_spatial_entities::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                payload: &spatial,
            },
            DomainSnapshot {
                domain_id: world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 1,
                snapshot_type: world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload: &overlay,
            },
            DomainSnapshot {
                domain_id: attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
                revision: 1,
                snapshot_type: attack::SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1,
                payload: &combat,
            },
        ],
    )? {
        write_frame(&mut stream, &frame).await?;
    }
    let mut seen = Vec::new();
    for (index, disposition) in replies.into_iter().enumerate() {
        let bytes = read_frame(&mut stream).await?;
        let command = decode_wire_envelope(&bytes)?.client_command(1)?;
        seen.push(
            actor_spell::decode_aimed_spell_cast_intent(command.payload)
                .map_err(|e| format!("spell: {e:?}"))?,
        );
        write_frame(
            &mut stream,
            &encode_command_result(
                1,
                11 + index as u64,
                command.command_id,
                if disposition == SpellCastDisposition::Rejected {
                    CommandStatus::Rejected
                } else {
                    CommandStatus::Accepted
                },
                &actor_spell::encode_spell_cast_result(disposition),
            )?,
        )
        .await?;
    }
    Ok(seen)
}

/// Runs `command` against a server that answers with `replies`; returns the report and the casts
/// the server received.
fn run(
    command: CombatCommand,
    replies: Vec<SpellCastDisposition>,
) -> Result<(CombatReport, Vec<AimedSpellCastIntent>), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<_, Error> {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let (client, server) = tokio::io::duplex(64 * 1024);
                    let peer = tokio::spawn(peer(server, replies));
                    let mut session = Session::admit(
                        client,
                        Admission {
                            appearance_items: None,
                            schema_revision: 1,
                            character_id: CharacterId::decode(&id(4))?,
                            admission_material: b"grant",
                            client_build_id: "spell-aim-fallback-test",
                            supported_capabilities: &[6, 17, 480],
                            deadline: Duration::from_secs(5),
                        },
                    )
                    .await?;
                    let mut feedback = SpellFeedback::default();
                    let report = run_combat(&mut session, &mut feedback, command).await?;
                    Ok((report, peer.await??))
                })
        })?
        .join()
        .map_err(|_| "spell aim fallback thread panicked")?
}

fn spell() -> Result<NonZeroU32, Error> {
    NonZeroU32::new(9).ok_or_else(|| "spell".into())
}

fn plain_cast_with_held_target(intent: &AimedSpellCastIntent) -> Result<bool, Error> {
    Ok(intent.aim_mode.is_none()
        && intent.intent
            == SpellCastIntent {
                spell: spell()?,
                target: SpellTarget::AttackTarget,
                aim_at_target: true,
            })
}

#[test]
fn a_refused_target_aim_falls_back_to_the_plain_cast_with_the_held_target() -> Result<(), Error> {
    let (report, seen) = run(
        CombatCommand::Cast(spell()?),
        vec![
            SpellCastDisposition::Rejected,
            SpellCastDisposition::TargetIllegal,
        ],
    )?;
    assert_eq!(
        report,
        CombatReport::Cast(SpellCastDisposition::TargetIllegal)
    );
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].aim_mode, Some(SpellAimMode::Target));
    assert!(plain_cast_with_held_target(&seen[1])?);
    Ok(())
}

#[test]
fn an_accepted_target_aim_is_sent_once() -> Result<(), Error> {
    let (report, seen) = run(
        CombatCommand::Cast(spell()?),
        vec![SpellCastDisposition::TargetIllegal],
    )?;
    assert_eq!(
        report,
        CombatReport::Cast(SpellCastDisposition::TargetIllegal)
    );
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].aim_mode, Some(SpellAimMode::Target));
    Ok(())
}

#[test]
fn a_refused_cursor_aim_falls_back_to_the_facing_cast() -> Result<(), Error> {
    let command = CombatCommand::CastCursor {
        spell: spell()?,
        position: SpellTargetPosition {
            x: 101,
            y: 200,
            floor: 7,
        },
        direction: None,
    };
    let (report, seen) = run(
        command,
        vec![
            SpellCastDisposition::Rejected,
            SpellCastDisposition::TargetIllegal,
        ],
    )?;
    assert_eq!(
        report,
        CombatReport::Cast(SpellCastDisposition::TargetIllegal)
    );
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0].aim_mode, Some(SpellAimMode::Cursor));
    assert_eq!(seen[1].aim_mode, Some(SpellAimMode::Facing));
    let (_, seen) = run(command, vec![SpellCastDisposition::TargetIllegal])?;
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].aim_mode, Some(SpellAimMode::Cursor));
    Ok(())
}

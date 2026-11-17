//! Production combat routing through a real admitted session and decoded wire intents.
use oteryn_client_runtime::combat::{CombatCommand, CombatReport, run_combat};
use oteryn_client_runtime::spell::SpellFeedback;
use oteryn_protocol_oteryn::actor_spell::{self, SpellCastIntent, SpellTargetPosition};
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
    Admission, CharacterId, CommandStatus, Session, SpellCastDisposition, SpellTarget, read_frame,
    write_frame,
};
use std::num::NonZeroU32;
use std::time::Duration;
use tokio::io::DuplexStream;

type Error = Box<dyn std::error::Error + Send + Sync>;

fn id(marker: u8) -> [u8; 16] {
    [
        1, 0x93, 0x4f, 0x10, 0x7c, 0, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        marker,
    ]
}

async fn peer(
    mut stream: DuplexStream,
    targeted: bool,
    expected: SpellCastIntent,
) -> Result<(), Error> {
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
            selected_capabilities: &[6, 17],
        })?,
    )
    .await?;
    let spatial = world_spatial_entities::encode_world_spatial_entities_snapshot(
        &WorldSpatialEntitiesSnapshot {
            content_generation: [1; 32],
            actor_position: ActorPosition {
                x: 100,
                y: 200,
                floor: 7,
            },
            own_identity: [5; 16],
            entities: vec![world_spatial_entities::WorldSpatialEntity {
                entity: EntityRef {
                    identity: [5; 16],
                    generation: 1,
                },
                kind: world_spatial_entities::EntityKind::Player,
                display_name: None,
                position: ActorPosition {
                    x: 100,
                    y: 200,
                    floor: 7,
                },
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
        target: targeted.then_some(EntityRef {
            identity: [7; 16],
            generation: 3,
        }),
        modes: FightModes::DEFAULT,
        in_fight: targeted,
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
    let bytes = read_frame(&mut stream).await?;
    let command = decode_wire_envelope(&bytes)?.client_command(1)?;
    assert_eq!(
        command.command_type,
        actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT
    );
    assert_eq!(
        actor_spell::decode_spell_cast_intent(command.payload)
            .map_err(|e| format!("spell: {e:?}"))?,
        expected
    );
    // A normal refusal needs no vitals delta; it must reach the native feedback unchanged.
    write_frame(
        &mut stream,
        &encode_command_result(
            1,
            11,
            command.command_id,
            CommandStatus::Accepted,
            &actor_spell::encode_spell_cast_result(SpellCastDisposition::TargetIllegal),
        )?,
    )
    .await?;
    Ok(())
}

fn check(targeted: bool, command: CombatCommand, expected: SpellCastIntent) -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), Error> {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let (client, server) = tokio::io::duplex(64 * 1024);
                    let peer = tokio::spawn(peer(server, targeted, expected));
                    let mut session = Session::admit(
                        client,
                        Admission {
                            appearance_items: None,
                            schema_revision: 1,
                            character_id: CharacterId::decode(&id(4))?,
                            admission_material: b"grant",
                            client_build_id: "gap-cl2-test",
                            supported_capabilities: &[6, 17],
                            deadline: Duration::from_secs(5),
                        },
                    )
                    .await?;
                    let mut feedback = SpellFeedback::default();
                    assert_eq!(
                        run_combat(&mut session, &mut feedback, command).await?,
                        CombatReport::Cast(SpellCastDisposition::TargetIllegal)
                    );
                    assert_eq!(feedback.text(), Some("Illegal target"));
                    peer.await??;
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "spell targeting thread panicked")?
}

#[test]
fn native_hotkey_aims_at_the_session_attack_target() -> Result<(), Error> {
    check(
        true,
        CombatCommand::Cast(NonZeroU32::MIN),
        SpellCastIntent {
            spell: NonZeroU32::MIN,
            target: SpellTarget::AttackTarget,
            aim_at_target: true,
        },
    )
}

#[test]
fn native_hotkey_without_opponent_keeps_directional_fallback() -> Result<(), Error> {
    check(
        false,
        CombatCommand::Cast(NonZeroU32::MIN),
        SpellCastIntent {
            spell: NonZeroU32::MIN,
            target: SpellTarget::None,
            aim_at_target: false,
        },
    )
}

#[test]
fn cursor_cast_uses_ordinary_spell_wire_and_does_not_aim_at_attack_target() -> Result<(), Error> {
    let position = SpellTargetPosition {
        x: 102,
        y: 199,
        floor: 7,
    };
    check(
        true,
        CombatCommand::CastAt {
            spell: NonZeroU32::new(42).ok_or("spell")?,
            position,
        },
        SpellCastIntent {
            spell: NonZeroU32::new(42).ok_or("spell")?,
            target: SpellTarget::Position(position),
            aim_at_target: false,
        },
    )
}

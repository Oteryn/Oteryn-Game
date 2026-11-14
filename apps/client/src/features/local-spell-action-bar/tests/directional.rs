//! Native action-bar controls through admission, ordinary cast wire and HUD feedback.
//! Crystal 201189f src/creatures/combat/spells.cpp:1238-1261: selected target
//! or direction fallback. Direction stays server-owned; no target means SpellTarget::None.
use oteryn_client_runtime::combat::{CombatCommand, CombatReport, run_combat};
use oteryn_client_runtime::spell::SpellFeedback;
use oteryn_protocol_oteryn::actor_spell::{self, SpellCastIntent};
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

// SPELL-D1 source order: Energy Wave, exevo vis hur. The pinned catalogue
// records Crystal ff7ede59 data/scripts/spells/attack/energy_wave.lua:87 and
// Canary 99902524 same path:48 as the needDirection(true) donor evidence.
const ENERGY_WAVE_INDEX: u32 = 71;

fn directional_spell() -> Result<NonZeroU32, Error> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../content/test-packs/spells/r25/catalog.json");
    let catalog: serde_json::Value = serde_json::from_slice(&std::fs::read(path)?)?;
    let mut spells = catalog["bundles"]
        .as_array()
        .ok_or("spell bundles")?
        .iter()
        .map(|entry| &entry["bundle"]["spell"])
        .collect::<Vec<_>>();
    spells.sort_by(|left, right| {
        left["identity"]["key"]
            .as_str()
            .cmp(&right["identity"]["key"].as_str())
    });
    let spell = spells
        .get(ENERGY_WAVE_INDEX as usize - 1)
        .ok_or("Energy Wave index")?;
    assert_eq!(spell["identity"]["key"], "candidate:spell/energy_wave");
    assert_eq!(spell["words"], "exevo vis hur");
    assert_eq!(spell["targeting"]["needs_direction"], true);
    assert_eq!(spell["targeting"]["needs_target"], false);
    NonZeroU32::new(ENERGY_WAVE_INDEX).ok_or_else(|| "spell index".into())
}

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

fn check(targeted: bool, expected: SpellCastIntent, facing: bool) -> Result<(), Error> {
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
                            client_build_id: "clhud-native-bar-test",
                            supported_capabilities: &[6, 17],
                            deadline: Duration::from_secs(5),
                        },
                    )
                    .await?;
                    let energy_wave = directional_spell()?;
                    assert_eq!(expected.spell, energy_wave);
                    // These are the same Apply/Cast callbacks used by the native form.
                    let (link, mut commands, outbox) = crate::play::play_channel();
                    link.use_shared_frontend();
                    {
                        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
                        crate::play::feature_local_spell_action_bar::input::submit(
                            &mut mailbox,
                            &link.commands,
                            0,
                            &[energy_wave.to_string(), "".into(), "".into(), "".into()],
                        );
                        crate::play::feature_local_spell_action_bar::input::submit(
                            &mut mailbox,
                            &link.commands,
                            1,
                            &[],
                        );
                    }
                    let crate::combat_input::PlayCommand::Combat(command) = commands.try_recv()?
                    else {
                        return Err("native action bar did not queue ordinary combat".into());
                    };
                    assert_eq!(command, CombatCommand::Cast(energy_wave));
                    let command = if facing {
                        CombatCommand::CastFacing {
                            spell: energy_wave,
                            direction: None,
                        }
                    } else {
                        command
                    };
                    let mut feedback = SpellFeedback::default();
                    let report = run_combat(&mut session, &mut feedback, command).await?;
                    assert_eq!(
                        report,
                        CombatReport::Cast(SpellCastDisposition::TargetIllegal)
                    );
                    assert_eq!(feedback.text(), Some("Illegal target"));
                    outbox.battle_result(report, session.combat_state().copied(), None);
                    let mut view = crate::play::tests::view().map_err(|_| "view")?;
                    view.tick(&link).map_err(|_| "tick")?;
                    assert!(view.combat_status().contains("Illegal target"));
                    assert!(commands.try_recv().is_err());
                    peer.await??;
                    Ok(())
                })
        })?
        .join()
        .map_err(|_| "spell targeting thread panicked")?
}

#[test]
fn clhud_native_bar_aims_at_the_admitted_attack_target() -> Result<(), Error> {
    check(
        true,
        SpellCastIntent {
            spell: NonZeroU32::new(ENERGY_WAVE_INDEX).ok_or("spell")?,
            target: SpellTarget::AttackTarget,
            aim_at_target: true,
        },
        false,
    )
}

#[test]
fn clhud_native_bar_without_opponent_keeps_directional_fallback() -> Result<(), Error> {
    check(
        false,
        SpellCastIntent {
            spell: NonZeroU32::new(ENERGY_WAVE_INDEX).ok_or("spell")?,
            target: SpellTarget::None,
            aim_at_target: false,
        },
        false,
    )
}

#[test]
fn explicit_facing_cast_ignores_attack_target_on_real_wire() -> Result<(), Error> {
    check(
        true,
        SpellCastIntent {
            spell: NonZeroU32::new(ENERGY_WAVE_INDEX).ok_or("spell")?,
            target: SpellTarget::None,
            aim_at_target: false,
        },
        true,
    )
}

//! C17: admitted Session -> production battle panel -> attack and combat modes.
use oteryn_client::play::{PlayLink, PlayView, play_channel, run_session};
use oteryn_client_ui::battle::Action;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_protocol_oteryn::{
    ChannelId, CharacterId, CommandStatus, DomainSnapshot, GameSessionId, ServerAcceptedValue,
    WorldId,
    attack::{self, ActorCombatState, AttackIntentDisposition, ChaseMode, FightMode, FightModes},
    decode_wire_envelope, encode_command_result, encode_server_accepted,
    encode_single_chunk_snapshot, encode_state_delta,
    world_spatial::{ActorPosition, StepDirection},
    world_spatial_entities::{
        EntityDetail, EntityKind, EntityRef, WorldSpatialEntitiesDelta,
        WorldSpatialEntitiesSnapshot, WorldSpatialEntity, encode_world_spatial_entities_delta,
        encode_world_spatial_entities_snapshot,
    },
};
use oteryn_session::{Admission, CLIENT_SUPPORTED_CAPABILITIES, Session, read_frame, write_frame};
use std::time::Duration;
use tokio::io::{AsyncReadExt, DuplexStream};

type Error = Box<dyn std::error::Error + Send + Sync>;
const WAIT: Duration = Duration::from_secs(5);

#[test]
fn native_battle_scenario() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(scenario())
        })?
        .join()
        .map_err(|_| std::io::Error::other("battle scenario panicked"))?
}
async fn scenario() -> Result<(), Error> {
    let (client, server) = tokio::io::duplex(64 * 1024);
    let peer = tokio::spawn(peer(server));
    let session = Session::admit(
        client,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: CharacterId::decode(&identity(4))?,
            admission_material: b"grant",
            client_build_id: "c17-native-scenario",
            supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
            deadline: WAIT,
        },
    )
    .await?;
    let mut view = PlayView::from_join(session.join_snapshot())?;
    let (link, commands, outbox) = play_channel();
    let task = tokio::spawn(run_session(session, commands, outbox));
    pump(&mut view, &link, |_, link| {
        link.battle_panel().combat.is_some()
    })
    .await?;
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(5)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Alt),
        repeat: false,
    };
    assert!(link.battle_input(std::slice::from_ref(&key)));
    assert!(link.battle_panel().open);
    assert_eq!(link.battle_panel().rows, vec![creature()]);
    link.battle_request(Action::Attack(creature().entity));
    pump(&mut view, &link, |_, link| {
        link.battle_panel()
            .combat
            .is_some_and(|s| s.target == Some(creature().entity))
    })
    .await?;
    link.battle_request(Action::Fight(FightMode::Defensive));
    pump(&mut view, &link, |_, link| {
        link.battle_panel()
            .combat
            .is_some_and(|s| s.modes.fight_mode == FightMode::Defensive)
    })
    .await?;
    link.battle_request(Action::Chase(ChaseMode::Chase));
    pump(&mut view, &link, |_, link| {
        link.battle_panel()
            .combat
            .is_some_and(|s| s.modes.chase == ChaseMode::Chase)
    })
    .await?;
    link.battle_request(Action::Stop);
    pump(&mut view, &link, |_, link| {
        link.battle_panel()
            .combat
            .is_some_and(|s| s.target.is_none())
            && link.battle_panel().rows.is_empty()
    })
    .await?;
    link.battle_request(Action::Attack(creature().entity)); // stale row must send nothing
    assert!(link.battle_input(std::slice::from_ref(&key)));
    assert!(!link.battle_panel().open);
    drop(link);
    tokio::time::timeout(WAIT, task).await??;
    tokio::time::timeout(WAIT, peer).await???;
    Ok(())
}
async fn pump(
    view: &mut PlayView,
    link: &PlayLink,
    ready: impl Fn(&PlayView, &PlayLink) -> bool,
) -> Result<(), Error> {
    tokio::time::timeout(WAIT, async {
        loop {
            view.tick(link).map_err(|e| format!("tick: {e:?}"))?;
            if ready(view, link) {
                return Ok::<_, Error>(());
            }
            tokio::task::yield_now().await;
        }
    })
    .await?
}

async fn peer(mut stream: DuplexStream) -> Result<(), Error> {
    let bootstrap = read_frame(&mut stream).await?;
    assert!(
        decode_wire_envelope(&bootstrap)?
            .client_bootstrap()?
            .supported_capabilities
            .contains(&17)
    );
    write_frame(
        &mut stream,
        &encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&identity(1))?,
            world_id: WorldId::decode(&identity(2))?,
            channel_id: ChannelId::decode(&identity(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 7,
            schema_revision: 1,
            selected_capabilities: &[6, 13, 17],
        })?,
    )
    .await?;
    let spatial = encode_world_spatial_entities_snapshot(&WorldSpatialEntitiesSnapshot {
        content_generation: [1; 32],
        actor_position: position(100),
        own_identity: [7; 16],
        entities: vec![
            actor(100),
            creature(),
            WorldSpatialEntity {
                entity: EntityRef {
                    identity: [9; 16],
                    generation: 1,
                },
                position: ActorPosition {
                    floor: -1,
                    ..position(101)
                },
                ..creature()
            },
        ],
    })
    .map_err(|e| format!("snapshot: {e:?}"))?;
    let combat = attack::encode_actor_combat_state(&state(None, FightModes::DEFAULT))
        .map_err(|e| format!("combat: {e:?}"))?;
    for frame in encode_single_chunk_snapshot(
        1,
        1,
        40,
        &[
            DomainSnapshot {
                domain_id: 10,
                revision: 1,
                snapshot_type: 1,
                payload: &combat,
            },
            DomainSnapshot {
                domain_id: 1,
                revision: 5,
                snapshot_type: 2,
                payload: &spatial,
            },
            DomainSnapshot {
                domain_id: 2,
                revision: 2,
                snapshot_type: 1,
                payload: &[],
            },
        ],
    )? {
        write_frame(&mut stream, &frame).await?;
    }
    let mut modes = FightModes::DEFAULT;
    for i in 0..4_u64 {
        let bytes = read_frame(&mut stream).await?;
        let command = decode_wire_envelope(&bytes)?.client_command(1)?;
        assert_eq!(command.command_id, 7 + i);
        let target = match i {
            0 => {
                assert_eq!(command.command_type, 11);
                assert_eq!(
                    attack::decode_attack_target_intent(command.payload),
                    Ok(Some(creature().entity))
                );
                Some(creature().entity)
            }
            1 | 2 => {
                assert_eq!(command.command_type, 12);
                if i == 1 {
                    modes.fight_mode = FightMode::Defensive;
                } else {
                    modes.chase = ChaseMode::Chase;
                }
                assert_eq!(
                    attack::decode_fight_modes_intent(command.payload),
                    Ok(modes)
                );
                Some(creature().entity)
            }
            _ => {
                assert_eq!(command.command_type, 11);
                assert_eq!(
                    attack::decode_attack_target_intent(command.payload),
                    Ok(None)
                );
                None
            }
        };
        let result = attack::encode_attack_intent_result(AttackIntentDisposition::Ok);
        write_frame(
            &mut stream,
            &encode_command_result(1, 41 + i * 2, 7 + i, CommandStatus::Accepted, &result)?,
        )
        .await?;
        let payload = attack::encode_actor_combat_state(&state(target, modes))
            .map_err(|e| format!("combat delta: {e:?}"))?;
        write_frame(
            &mut stream,
            &encode_state_delta(1, 42 + i * 2, 10, 1 + i, 2 + i, 1, &payload)?,
        )
        .await?;
    }
    let payload = encode_world_spatial_entities_delta(&WorldSpatialEntitiesDelta {
        content_generation: [1; 32],
        actor_position: position(100),
        enter: vec![],
        update: vec![],
        leave: vec![creature().entity],
    })
    .map_err(|e| format!("leave: {e:?}"))?;
    write_frame(
        &mut stream,
        &encode_state_delta(1, 49, 1, 5, 6, 2, &payload)?,
    )
    .await?;
    let mut extra = [0];
    assert_eq!(stream.read(&mut extra).await?, 0);
    Ok(())
}
fn state(target: Option<EntityRef>, modes: FightModes) -> ActorCombatState {
    ActorCombatState {
        target,
        modes,
        in_fight: target.is_some(),
    }
}
fn creature() -> WorldSpatialEntity {
    WorldSpatialEntity {
        kind: EntityKind::Creature,
        entity: EntityRef {
            identity: [8; 16],
            generation: 3,
        },
        position: position(101),
        ..actor(101)
    }
}
fn position(x: i32) -> ActorPosition {
    ActorPosition {
        x,
        y: 200,
        floor: -2,
    }
}
fn actor(x: i32) -> WorldSpatialEntity {
    WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Player,
        entity: EntityRef {
            identity: [7; 16],
            generation: 1,
        },
        position: position(x),
        detail: EntityDetail::Actor {
            direction: StepDirection::East,
            appearance_ref: 1,
            health_percent: 100,
        },
    }
}
fn identity(marker: u8) -> [u8; 16] {
    [
        1, 0x93, 0x4f, 0x10, 0x7c, 0, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        marker,
    ]
}

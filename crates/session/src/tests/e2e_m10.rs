//! Replay for SCENARIO_MATRIX M10's real drowning payload.
//!
//! This exercises `snapshot_state.rs::SnapshotState::decode`,
//! `domain_dispatch.rs::Session::apply_delta`, and
//! `session_access.rs::Session::actor_vitals`, the three session functions that turn the
//! server's ActorVitals snapshot/delta into client-visible health.
use super::*;
use oteryn_protocol_oteryn::actor_spell;
use oteryn_protocol_oteryn::world_object::{
    SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
    encode_world_object_overlay_snapshot,
};
use oteryn_protocol_oteryn::world_spatial::{
    ActorPosition, SNAPSHOT_TYPE_WORLD_SPATIAL_V1, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
    WorldSpatialObservation, encode_world_spatial,
};
use oteryn_protocol_oteryn::{
    ChannelId, DomainSnapshot, ServerAcceptedValue, WorldId, encode_server_accepted,
    encode_single_chunk_snapshot, encode_state_delta,
};
use std::error::Error;
use tokio::io::DuplexStream;

type TestError = Box<dyn Error + Send + Sync>;

fn id(marker: u8) -> [u8; 16] {
    [
        0x01, 0x93, 0x4f, 0x10, 0x7c, 0, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        marker,
    ]
}

async fn peer(
    mut stream: DuplexStream,
    snapshot: Vec<u8>,
    delta: Vec<u8>,
    base: u64,
    new: u64,
    sequence: u64,
) -> Result<(), TestError> {
    let _bootstrap = read_frame(&mut stream).await?;
    write_frame(
        &mut stream,
        &encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&id(1))?,
            world_id: WorldId::decode(&id(2))?,
            channel_id: ChannelId::decode(&id(3))?,
            connection_generation: 1,
            current_server_sequence: sequence - 1,
            next_command_id: 1,
            schema_revision: 1,
            selected_capabilities: &[],
        })?,
    )
    .await?;
    let spatial = encode_world_spatial(&WorldSpatialObservation {
        content_generation: [7; 32],
        actor_position: ActorPosition {
            x: 1,
            y: 2,
            floor: -7,
        },
    });
    let overlay =
        encode_world_object_overlay_snapshot(&[]).map_err(|error| format!("overlay: {error:?}"))?;
    for frame in encode_single_chunk_snapshot(
        1,
        1,
        sequence - 1,
        &[
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 1,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &spatial,
            },
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 1,
                snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload: &overlay,
            },
            DomainSnapshot {
                domain_id: actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                revision: base,
                snapshot_type: actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                payload: &snapshot,
            },
        ],
    )? {
        write_frame(&mut stream, &frame).await?;
    }
    write_frame(
        &mut stream,
        &encode_state_delta(
            1,
            sequence,
            actor_spell::STATE_DOMAIN_ACTOR_VITALS,
            base,
            new,
            actor_spell::DELTA_TYPE_ACTOR_VITALS_V1,
            &delta,
        )?,
    )
    .await?;
    tokio::time::sleep(Duration::from_millis(200)).await;
    Ok(())
}

#[test]
fn replays_drowning_vitals_and_rejects_invalid_payload_without_mutation() -> Result<(), TestError> {
    let (snapshot, delta, expected) = if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        (
            std::fs::read(path.join("m10-snapshot.bin"))?,
            std::fs::read(path.join("m10-drown.bin"))?,
            std::fs::read_to_string(path.join("m10-expected.txt"))?,
        )
    } else {
        let initial = actor_spell::ActorVitals {
            health: 150,
            max_health: 150,
            mana: 55,
            max_mana: 55,
            ..actor_spell::ActorVitals::default()
        };
        let drowned = actor_spell::ActorVitals {
            health: 130,
            ..initial
        };
        (
            actor_spell::encode_actor_vitals(&initial)
                .map_err(|error| format!("snapshot: {error:?}"))?,
            actor_spell::encode_actor_vitals(&drowned)
                .map_err(|error| format!("delta: {error:?}"))?,
            "150 130 55 3 4 41\n".into(),
        )
    };
    let values = expected
        .split_whitespace()
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()?;
    let [initial_health, health, mana, base, new, sequence] = values.as_slice() else {
        return Err("invalid M10 expected metadata".into());
    };
    let run = |payload: Vec<u8>| {
        let snapshot = snapshot.clone();
        async move {
            let (client, server) = tokio::io::duplex(64 * 1024);
            let serving = tokio::spawn(peer(server, snapshot, payload, *base, *new, *sequence));
            let mut session = Session::admit(
                client,
                Admission {
                    appearance_items: None,
                    schema_revision: 1,
                    character_id: CharacterId::decode(&id(4))?,
                    admission_material: b"grant",
                    client_build_id: "e2e-m10",
                    supported_capabilities: &[],
                    deadline: Duration::from_secs(2),
                },
            )
            .await?;
            assert_eq!(
                u64::from(
                    session
                        .actor_vitals()
                        .ok_or("initial vitals absent")?
                        .health
                ),
                *initial_health
            );
            let result = session.service_liveness(Duration::from_millis(100)).await;
            serving.await??;
            Ok::<_, TestError>((session, result))
        }
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let (mut applied, result) = run(delta).await?;
        result?;
        let visible = applied.actor_vitals().ok_or("drowned vitals absent")?;
        assert_eq!((u64::from(visible.health), u64::from(visible.mana)), (*health, *mana));
        assert!(matches!(applied.take_events().as_slice(), [SessionEvent::ActorVitals(value)] if value.base_revision == *base && value.new_revision == *new));

        let (rejected, result) = run(vec![8, 255]).await?;
        assert!(result.is_err());
        assert_eq!(u64::from(rejected.actor_vitals().ok_or("vitals changed to absent")?.health), *initial_health);
        Ok::<(), TestError>(())
    })
}

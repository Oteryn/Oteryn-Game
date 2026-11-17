//! M04 client replay capture: push an adjacent player into a free adjacent cell and retain the
//! exact WORLD_SPATIAL_ENTITIES payload delivered to the pushing client.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind};
use oteryn_protocol_oteryn::creature_push::*;
use oteryn_protocol_oteryn::world_spatial_entities::{
    DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2, decode_world_spatial_entities_delta,
};

const DIRECTIONS: [StepDirection; 4] = [
    StepDirection::North,
    StepDirection::East,
    StepDirection::South,
    StepDirection::West,
];

fn distance(a: TilePos, b: TilePos) -> u16 {
    a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
}

fn position(at: TilePos) -> ActorPosition {
    ActorPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    }
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await;
    let result = match result {
        Ok(result) => result,
        Err(_) => Err("M04.client capture timed out".into()),
    };
    if let Err(error) = &result {
        evidence(&format!("M04_CLIENT_GAP error={error}"));
    }
    evidence(&format!(
        "e2e_all scenario=M04.client status={} seconds={:.3}",
        if result.is_ok() { "PASS" } else { "FAIL" },
        started.elapsed().as_secs_f64()
    ));
}

fn own(client: &DevClientSession) -> TestResult<oteryn_dev_client::WorldSpatialEntity> {
    let entities = client.world_entities().ok_or("M04 entity view missing")?;
    entities
        .iter()
        .find(|entity| {
            entity.kind == EntityKind::Player && &entity.entity.identity == entities.own_identity()
        })
        .copied()
        .ok_or_else(|| "M04 own entity missing".into())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin: serde_json::Value =
        serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let mut pusher = connect_character(clients, 0x94, 0).await?;
    let mut target = connect_character(clients, 0x95, 0).await?;
    let pusher_at = tile(pusher.world_spatial().actor_position)?;
    let path = route(world, tile(target.world_spatial().actor_position)?, |at| {
        distance(at, pusher_at) == 1
            && DIRECTIONS.into_iter().any(|direction| {
                destination(world, at, direction)
                    .is_some_and(|next| next != pusher_at && distance(next, pusher_at) == 1)
            })
    })
    .ok_or("M04 no adjacent player push lane")?;
    walk(&mut target, world, &path).await?;
    let before = tile(target.world_spatial().actor_position)?;
    let reference = own(&target)?.entity;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !pusher
        .world_entities()
        .and_then(|entities| entities.get(&reference))
        .is_some_and(|entity| entity.position == position(before))
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("M04 pushing client did not observe adjacent player".into());
        }
        pusher.service_liveness(Duration::from_millis(25)).await?;
        target.service_liveness(Duration::from_millis(25)).await?;
    }
    pusher.take_events();
    let blocked_intent = encode_creature_push_intent(&CreaturePushIntent {
        target: reference,
        destination: position(pusher_at),
    })
    .map_err(|error| format!("M04 blocked push encode: {error:?}"))?;
    let blocked = pusher
        .session_mut()
        .gated_command(
            CAPABILITY_CREATURE_PUSH_V1,
            COMMAND_TYPE_CREATURE_PUSH_INTENT,
            &blocked_intent,
        )
        .await?;
    if blocked.status != CommandStatus::Accepted || blocked.payload != [8, 5] {
        return Err(format!("M04 occupied destination reply: {blocked:?}").into());
    }
    if pusher
        .world_entities()
        .and_then(|entities| entities.get(&reference))
        .is_none_or(|entity| entity.position != position(before))
    {
        return Err("M04 occupied push changed the visible player".into());
    }
    let landing = DIRECTIONS
        .into_iter()
        .filter_map(|direction| destination(world, before, direction))
        .find(|at| *at != pusher_at && distance(*at, pusher_at) == 1)
        .ok_or("M04 no free push landing")?;
    let intent = encode_creature_push_intent(&CreaturePushIntent {
        target: reference,
        destination: position(landing),
    })
    .map_err(|error| format!("M04 push encode: {error:?}"))?;
    let reply = pusher
        .session_mut()
        .gated_command(
            CAPABILITY_CREATURE_PUSH_V1,
            COMMAND_TYPE_CREATURE_PUSH_INTENT,
            &intent,
        )
        .await?;
    if reply.status != CommandStatus::Accepted || reply.payload != [8, 1] {
        return Err(format!("M04 push reply: {reply:?}").into());
    }
    let delta = pusher
        .session_mut()
        .read_gated_delta(
            oteryn_protocol_oteryn::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
        )
        .await?;
    let decoded = decode_world_spatial_entities_delta(&delta.payload)
        .map_err(|error| format!("M04 spatial payload: {error:?}"))?;
    let moved = decoded
        .update
        .iter()
        .find(|entity| entity.entity == reference)
        .ok_or("M04 spatial payload omitted pushed player")?;
    if moved.position != position(landing) {
        return Err("M04 pushed player delta has wrong landing".into());
    }
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let directory = Path::new(&directory);
        let expected = format!(
            "{} {} {} {} {} {} {} {}\n",
            delta.base_revision,
            delta.new_revision,
            delta.server_sequence,
            reference
                .identity
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            reference.generation,
            moved.position.x,
            moved.position.y,
            moved.position.floor
        );
        for (name, contents) in [
            ("m04-spatial.bin", delta.payload.as_slice()),
            ("m04-expected.txt", expected.as_bytes()),
        ] {
            if let Err(error) = std::fs::write(directory.join(name), contents) {
                evidence(&format!("M04_CAPTURE_WRITE_ERROR file={name} error={error}"));
            }
        }
    }
    Ok(())
}

//! M04 client replay capture: push an adjacent player into a free adjacent cell and retain the
//! exact WORLD_SPATIAL_ENTITIES payload delivered to the pushing client.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind};
use oteryn_protocol_oteryn::creature_push::*;
use oteryn_protocol_oteryn::logout::LogoutResult;
use oteryn_protocol_oteryn::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY;
use oteryn_protocol_oteryn::world_spatial_entities::{
    DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2, EntityFields, SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
    decode_world_spatial_entities_delta_with_fields,
};
use std::io::Write as _;

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
    let result =
        verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
            .await;
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
    let capture = oteryn_dev_client::InboundCapture::default();
    let mut pusher = capture.connect(connect_world(clients, 0x94)).await?;
    let baseline = pusher
        .session_mut()
        .admission_snapshot(STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY)
        .filter(|snapshot| snapshot.snapshot_type == SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2)
        .ok_or("M04 admission world entity snapshot missing")?
        .revision;
    let mut target =
        provisioning::with_peer_liveness(&mut pusher, connect_character(clients, 0x95, 0)).await?;
    let pusher_at = tile(pusher.world_spatial().actor_position)?;
    let start = tile(target.world_spatial().actor_position)?;
    let path = provisioning::with_peer_liveness(
        &mut pusher,
        route_work(&mut target, move || {
            Ok(route(world, start, |at| {
                at.floor == pusher_at.floor
                    && distance(at, pusher_at) == 1
                    && DIRECTIONS.into_iter().any(|direction| {
                        destination(world, at, direction).is_some_and(|next| {
                            next.floor == at.floor
                                && next != pusher_at
                                && distance(next, pusher_at) == 1
                        })
                    })
            }))
        }),
    )
    .await?
    .ok_or("M04 no adjacent player push lane")?;
    provisioning::with_peer_liveness(&mut pusher, async {
        for direction in path {
            let from = tile(target.world_spatial().actor_position)?;
            let expected =
                destination(world, from, direction).ok_or("M04 approach landing missing")?;
            let outcome = target.step(direction).await?;
            if outcome.disposition != StepDisposition::Moved
                || outcome.world_spatial_delta.is_none()
                || tile(target.world_spatial().actor_position)? != expected
            {
                return Err(
                    format!("M04 approach step {direction:?} from {from:?}: {outcome:?}").into(),
                );
            }
            verdict::Harness::global().progress("walk");
        }
        Ok(())
    })
    .await?;
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
    target.service_liveness(Duration::from_millis(25)).await?;
    if target.world_spatial().actor_position != position(before) {
        return Err("M04 occupied push moved the target session".into());
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
        .find(|at| at.floor == before.floor && *at != pusher_at && distance(*at, pusher_at) == 1)
        .ok_or("M04 no free push landing")?;
    if landing == before {
        return Err("M04 push landing equals the target's original position".into());
    }
    let intent = encode_creature_push_intent(&CreaturePushIntent {
        target: reference,
        destination: position(landing),
    })
    .map_err(|error| format!("M04 push encode: {error:?}"))?;
    let before_push = pusher.last_server_sequence();
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
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let delta = loop {
        if let Some(delta) = pusher
            .session_mut()
            .raw_world_entity_deltas()
            .iter()
            .find(|delta| delta.server_sequence > before_push)
        {
            break delta.clone();
        }
        let remaining = deadline
            .checked_duration_since(tokio::time::Instant::now())
            .ok_or("M04 push did not publish a spatial delta")?;
        pusher.wait_for_events(remaining).await?;
    };
    if delta.delta_type != DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2 || delta.base_revision < baseline {
        return Err("M04 unexpected spatial delta type or revision".into());
    }
    let fields = EntityFields::selected(pusher.session_mut().selected_capabilities());
    let (decoded, _) = decode_world_spatial_entities_delta_with_fields(&delta.payload, fields)
        .map_err(|error| format!("M04 spatial payload: {error:?}"))?;
    let moved = decoded
        .update
        .iter()
        .find(|entity| entity.entity == reference)
        .ok_or("M04 spatial payload omitted pushed player")?;
    if moved.position != position(landing) {
        return Err("M04 pushed player delta has wrong landing".into());
    }
    provisioning::with_peer_liveness(&mut pusher, async {
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        while target.world_spatial().actor_position == position(before) {
            let remaining = deadline
                .checked_duration_since(tokio::time::Instant::now())
                .ok_or("M04 target did not receive the push movement")?;
            target.wait_for_events(remaining).await?;
        }
        if target.world_spatial().actor_position != position(landing) {
            return Err("M04 target session received the wrong landing".into());
        }
        let logout = target.session_mut().logout().await?;
        if logout.result != Some(LogoutResult::Accepted) {
            return Err(format!("M04 target logout refused: {logout:?}").into());
        }
        Ok(())
    })
    .await?;
    drop(target);
    let relogged =
        provisioning::with_peer_liveness(&mut pusher, connect_character(clients, 0x95, 0)).await?;
    if relogged.world_spatial().actor_position != position(landing) {
        return Err("M04 relog did not restore the pushed landing".into());
    }
    evidence(&format!("M04_RELOG_PASS before={before:?} landing={landing:?}"));
    drop(relogged);
    let directory = required("E2E_ALL_CAPTURE_DIR")?;
    let path = Path::new(&directory).join("m04-inbound.bin");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    file.write_all(&capture.bytes()?)?;
    drop(file);

    oteryn_dev_client::replay_player_push(
        &std::fs::read(&path)?,
        crate::foundation::CharacterId::decode(&clients.characters[1])?,
        reference,
        position(before),
        position(landing),
        delta.server_sequence,
    )
    .await?;
    evidence(&format!(
        "M04_REPLAY_PASS server_sequence={}",
        delta.server_sequence
    ));
    Ok(())
}

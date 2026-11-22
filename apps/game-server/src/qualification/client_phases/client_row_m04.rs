//! M04 client replay capture: push an adjacent player into a free adjacent cell and retain
//! a canonical projection of validated WORLD_SPATIAL_ENTITIES events. Entity fields retain their
//! negotiated values; light and party-icon sidecars are outside this movement replay.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind};
use oteryn_protocol_oteryn::creature_push::*;
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityFields, EntityLights, encode_world_spatial_entities_delta_with_fields,
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

async fn admit_push_pair<F, Fut>(mut connect: F) -> TestResult<(DevClientSession, DevClientSession)>
where
    F: FnMut(u8, usize) -> Fut,
    Fut: std::future::Future<Output = TestResult<DevClientSession>>,
{
    let pusher = connect(0x94, 0).await?;
    let target = connect(0x95, 1).await?;
    Ok((pusher, target))
}

// This canonical entity projection retains all fields represented by WorldSpatialEntity.
// Light and party-icon sidecars and noncore frames are outside this movement replay.
fn capture_capabilities(selected: &[u32]) -> Vec<u32> {
    [4, 6, 286, 372, 386]
        .into_iter()
        .filter(|capability| selected.contains(capability))
        .collect()
}

#[derive(Debug)]
struct CanonicalSpatialTransition {
    delta: oteryn_dev_client::AppliedDelta<oteryn_dev_client::WorldSpatialEntitiesDelta>,
    payload: Vec<u8>,
    icons_only: bool,
}

#[derive(Debug)]
struct CanonicalSpatialCapture {
    baseline: oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntitiesSnapshot,
    baseline_revision: u64,
    baseline_sequence: u64,
    capabilities: Vec<u32>,
    transitions: Vec<CanonicalSpatialTransition>,
}

// Pusher commands in this row do not claim domain 1. Account for every queued core revision
// before copying the current state, including the typed projection of a replacement snapshot.
fn drain_spatial_revision(pusher: &mut DevClientSession, mut revision: u64) -> TestResult<u64> {
    let mut replacement = None;
    for event in pusher.take_events() {
        let (sequence, base, new) = match event {
            oteryn_dev_client::SessionEvent::WorldSpatialEntities(delta) =>
                (delta.server_sequence, delta.base_revision, delta.new_revision),
            oteryn_dev_client::SessionEvent::PartyIcons(delta) =>
                (delta.server_sequence, delta.base_revision, delta.new_revision),
            oteryn_dev_client::SessionEvent::GatedSnapshotReplaced(delta)
                if delta.value.domain_id == STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY => {
                    replacement = Some((delta.server_sequence, delta.base_revision, delta.new_revision));
                    (delta.server_sequence, delta.base_revision, delta.new_revision)
                }
            oteryn_dev_client::SessionEvent::WorldSpatial(delta)
                if replacement == Some((delta.server_sequence, delta.base_revision, delta.new_revision)) => continue,
            event if event.domain_id() == STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY =>
                return Err("M04 unexpected core event before capture".into()),
            _ => continue,
        };
        if base != revision {
            return Err(format!("M04 incomplete pre-push spatial history at sequence {sequence}: {base} != {revision}").into());
        }
        revision = new;
    }
    Ok(revision)
}

fn capture_baseline(pusher: &DevClientSession, revision: u64) -> TestResult<CanonicalSpatialCapture> {
    let entities = pusher.world_entities().ok_or("M04 baseline entity view missing")?;
    let spatial = pusher.world_spatial();
    Ok(CanonicalSpatialCapture {
        baseline: oteryn_protocol_oteryn::world_spatial_entities::WorldSpatialEntitiesSnapshot {
            content_generation: spatial.content_generation,
            actor_position: spatial.actor_position,
            own_identity: *entities.own_identity(),
            entities: entities.iter().copied().collect(),
        },
        baseline_revision: revision,
        baseline_sequence: pusher.last_server_sequence(),
        capabilities: capture_capabilities(pusher.selected_capabilities()),
        transitions: Vec::new(),
    })
}

async fn capture_push_delta(
    pusher: &mut DevClientSession,
    mut capture: CanonicalSpatialCapture,
    reference: oteryn_dev_client::EntityRef,
    landing: ActorPosition,
    timeout: Duration,
) -> TestResult<CanonicalSpatialCapture> {
    let deadline = tokio::time::Instant::now() + timeout;
    let mut revision = capture.baseline_revision;
    let mut sequence = capture.baseline_sequence;
    let mut actor_position = capture.baseline.actor_position;
    loop {
        let remaining = deadline
            .checked_duration_since(tokio::time::Instant::now())
            .ok_or("M04 pushed player delta absent")?;
        // This drains events already applied while reading the command result before waiting.
        let events = pusher.wait_for_events(remaining).await
            .map_err(|error| format!("M04 pushed player delta absent: {error}"))?;
        for event in events {
            let (delta, icons_only) = match event {
                oteryn_dev_client::SessionEvent::WorldSpatialEntities(delta) => (delta, false),
                oteryn_dev_client::SessionEvent::PartyIcons(delta) => (
                    oteryn_dev_client::AppliedDelta {
                        server_sequence: delta.server_sequence,
                        base_revision: delta.base_revision,
                        new_revision: delta.new_revision,
                        value: oteryn_dev_client::WorldSpatialEntitiesDelta {
                            content_generation: capture.baseline.content_generation,
                            actor_position,
                            enter: vec![], update: vec![], leave: vec![],
                        },
                    },
                    true,
                ),
                event if event.domain_id() == STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY =>
                    return Err("M04 unexpected core replacement during push capture".into()),
                _ => continue,
            };
            if delta.server_sequence <= capture.baseline_sequence {
                continue;
            }
            if delta.server_sequence <= sequence || delta.base_revision != revision {
                return Err("M04 incomplete ordered spatial capture".into());
            }
            let matches_push = !icons_only && delta.value.update.iter().any(|entity| {
                entity.entity == reference && entity.position == landing
            });
            // The native decoder guarantees PartyIcons is an icons-only update. Its empty
            // entity projection retains the real core revision but cannot satisfy the push.
            let payload = encode_world_spatial_entities_delta_with_fields(
                &delta.value,
                EntityFields::selected(&capture.capabilities),
                &EntityLights::new(),
            ).map_err(|error| format!("M04 canonical spatial encode: {error:?}"))?;
            revision = delta.new_revision;
            sequence = delta.server_sequence;
            actor_position = delta.value.actor_position;
            capture.transitions.push(CanonicalSpatialTransition { delta, payload, icons_only });
            if matches_push {
                return Ok(capture);
            }
        }
    }
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
    let (mut pusher, mut target) =
        admit_push_pair(|tag, index| connect_character(clients, tag, index)).await?;
    let mut spatial_revision = pusher.join_snapshot().world_spatial_revision;
    // No background reader exists. These are only admission events already represented by the
    // join baseline; track every subsequently applied domain-1 update from this revision.
    pusher.take_events();
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
    spatial_revision = drain_spatial_revision(&mut pusher, spatial_revision)?;
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
    spatial_revision = drain_spatial_revision(&mut pusher, spatial_revision)?;
    let baseline = capture_baseline(&pusher, spatial_revision)?;
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
    let captured = capture_push_delta(
        &mut pusher,
        baseline,
        reference,
        position(landing),
        Duration::from_secs(5),
    )
    .await?;
    let transition = captured.transitions.last().ok_or("M04 capture has no push delta")?;
    let delta = &transition.delta;
    let moved = delta
        .value
        .update
        .iter()
        .find(|entity| entity.entity == reference)
        .ok_or("M04 spatial payload omitted pushed player")?;
    if moved.position != position(landing) {
        return Err("M04 pushed player delta has wrong landing".into());
    }
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let directory = Path::new(&directory);
        let baseline_payload = oteryn_protocol_oteryn::world_spatial_entities::encode_world_spatial_entities_snapshot_with_fields(
            &captured.baseline,
            EntityFields::selected(&captured.capabilities),
            &EntityLights::new(),
        ).map_err(|error| format!("M04 canonical baseline encode: {error:?}"))?;
        let baseline = format!("{} {}\n", captured.baseline_revision, captured.baseline_sequence);
        let expected = format!("{} {} {} {} {}\n",
            reference.identity.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
            reference.generation, moved.position.x, moved.position.y, moved.position.floor,
        );
        let capabilities = captured.capabilities.iter().map(u32::to_string).collect::<Vec<_>>().join(" ");
        let mut history = String::new();
        for (index, transition) in captured.transitions.iter().enumerate() {
            let delta = &transition.delta;
            history.push_str(&format!("{} {} {} {}\n", delta.server_sequence, delta.base_revision,
                delta.new_revision, if transition.icons_only { "icons-projection" } else { "spatial" }));
            std::fs::write(directory.join(format!("m04-spatial-{index}.bin")), &transition.payload)?;
        }
        for (name, contents) in [
            ("m04-baseline.bin", baseline_payload.as_slice()),
            ("m04-baseline.txt", baseline.as_bytes()),
            ("m04-history.txt", history.as_bytes()),
            ("m04-expected.txt", expected.as_bytes()),
            ("m04-capabilities.txt", capabilities.as_bytes()),
        ] {
            std::fs::write(directory.join(name), contents)?;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "core_capture_tests.rs"]
mod core_capture_tests;

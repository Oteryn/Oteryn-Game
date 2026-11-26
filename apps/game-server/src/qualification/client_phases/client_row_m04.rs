//! M04 client replay capture: push an adjacent player into a free adjacent cell and retain the
//! exact WORLD_SPATIAL_ENTITIES payload delivered to the pushing client.

use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind};
use oteryn_protocol_oteryn::creature_push::*;
use oteryn_protocol_oteryn::world_spatial_entities::{
    WorldSpatialEntitiesSnapshot, decode_world_spatial_entities_delta,
    encode_world_spatial_entities_snapshot,
};

use std::pin::Pin;
use std::task::Context;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

type ObservedWire = Arc<std::sync::Mutex<Vec<u8>>>;

struct PushWire {
    tls: tokio_rustls::client::TlsStream<TcpStream>,
    received: ObservedWire,
}

impl AsyncRead for PushWire {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        let this = self.get_mut();
        let before = buf.filled().len();
        let result = Pin::new(&mut this.tls).poll_read(cx, buf);
        if let Poll::Ready(Ok(())) = result {
            let mut received = this
                .received
                .lock()
                .map_err(|_| io::Error::other("M04 wire capture lock"))?;
            let bytes = &buf.filled()[before..];
            if received.len() + bytes.len() > 16 * 1024 * 1024 {
                return Poll::Ready(Err(io::Error::other("M04 wire capture exceeded its limit")));
            }
            received.extend_from_slice(bytes);
        }
        result
    }
}

impl AsyncWrite for PushWire {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.get_mut().tls).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().tls).poll_flush(cx)
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.get_mut().tls).poll_shutdown(cx)
    }
}

fn observed_envelope(mut wire: &[u8], sequence: u64) -> TestResult<&[u8]> {
    while let Some((prefix, rest)) = wire.split_first_chunk::<4>() {
        let length = oteryn_protocol_oteryn::FrameLength::from_prefix(prefix)?.get() as usize;
        let (frame, next) = rest
            .split_at_checked(length)
            .ok_or("M04 truncated wire frame")?;
        let envelope = decode_wire_envelope(frame)?;
        if envelope.message_type() == MessageType::StateDelta
            && envelope.server_sequence() == sequence
        {
            return Ok(frame);
        }
        wire = next;
    }
    Err("M04 push envelope absent from wire capture".into())
}

const PUSH_CAPABILITIES: &[u32] = &[
    oteryn_protocol_oteryn::item_view::CAPABILITY_ITEM_VIEW_MOVE_V1,
    oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES,
    oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1,
    oteryn_protocol_oteryn::world_spatial::CAPABILITY_PACED_MOVEMENT_V1,
    oteryn_protocol_oteryn::world_object::CAPABILITY_ITEM_USE_V1,
    oteryn_protocol_oteryn::world_map::CAPABILITY_WORLD_MAP_VIEW_V1,
    oteryn_protocol_oteryn::logout::CAPABILITY_LOGOUT_V1,
    oteryn_protocol_oteryn::capability_tables::CAPABILITY_WORLD_SPATIAL_FIELDS,
    CAPABILITY_CREATURE_PUSH_V1,
];

fn east(world: &BundleWorld, from: TilePos) -> Option<TilePos> {
    destination(world, from, StepDirection::East).filter(|next| {
        next.x.checked_sub(1) == Some(from.x) && next.y == from.y && next.floor == from.floor
    })
}

fn east_landing(world: &BundleWorld, from: TilePos, pusher: TilePos) -> Option<TilePos> {
    east(world, from).filter(|next| distance(*next, pusher) == 1)
}

fn distance(a: TilePos, b: TilePos) -> u16 {
    if a.floor == b.floor {
        a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
    } else {
        u16::MAX
    }
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
        "SEAM_EVIDENCE e2e_all scenario=M04.client status={} seconds={:.3}",
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
    let received = ObservedWire::default();
    let mut pusher =
        client_row_i12::connect_for_capture(clients, 0x94, 0, PUSH_CAPABILITIES, |tls| PushWire {
            tls,
            received: received.clone(),
        })
        .await?;
    let mut target = connect_character(clients, 0x95, 1).await?;
    let pusher_at = tile(pusher.world_spatial().actor_position)?;
    let path = route_avoiding(
        world,
        tile(target.world_spatial().actor_position)?,
        |at| {
            east(world, at).is_some_and(|before| {
                distance(before, pusher_at) == 1 && east_landing(world, before, pusher_at).is_some()
            })
        },
        |at| at != pusher_at,
    )
    .ok_or("M04 no adjacent player push lane")?;
    walk(&mut target, world, &path).await?;
    // Face east before the forced east step. The replay's predecessor retains
    // the pushed player's fields and places it one tile west of the landing.
    let step = target.step_retrying(StepDirection::East).await?;
    if step.disposition != StepDisposition::Moved {
        return Err(format!("M04 target approach step: {step:?}").into());
    }
    target
        .service_liveness(Duration::from_millis(u64::from(step.step_wait_ms)))
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
        pusher
            .service_liveness_discarding_events(Duration::from_millis(25))
            .await?;
        target.service_liveness(Duration::from_millis(25)).await?;
    }
    pusher.take_events();
    let blocked_intent = encode_creature_push_intent(&CreaturePushIntent {
        target: reference,
        destination: position(pusher_at),
    })
    .map_err(|error| format!("M04 blocked push encode: {error:?}"))?;
    let blocked = pusher
        .gated_command(
            CAPABILITY_CREATURE_PUSH_V1,
            COMMAND_TYPE_CREATURE_PUSH_INTENT,
            &blocked_intent,
        )
        .await?;
    if blocked.status != CommandStatus::Accepted || blocked.payload != [8, 5] {
        return Err(format!("M04 occupied destination reply: {blocked:?}").into());
    }
    let entities = pusher.world_entities().ok_or("M04 entity view missing")?;
    let predecessor = *entities
        .get(&reference)
        .filter(|entity| entity.position == position(before))
        .ok_or("M04 occupied push changed the visible player")?;
    let pusher_entity = *entities
        .iter()
        .find(|entity| &entity.entity.identity == entities.own_identity())
        .ok_or("M04 pusher entity missing")?;
    let predecessor_sequence = pusher.last_server_sequence();
    let landing = east_landing(world, before, pusher_at).ok_or("M04 no free east landing")?;
    let intent = encode_creature_push_intent(&CreaturePushIntent {
        target: reference,
        destination: position(landing),
    })
    .map_err(|error| format!("M04 push encode: {error:?}"))?;
    let reply = pusher
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
    let (delta, decoded) = loop {
        if let Some(found) = pusher
            .raw_world_entity_deltas()
            .iter()
            .filter(|delta| delta.server_sequence > predecessor_sequence)
            .find_map(|delta| {
                let decoded = decode_world_spatial_entities_delta(&delta.payload).ok()?;
                decoded
                    .update
                    .iter()
                    .any(|entity| {
                        entity.entity == reference && entity.position == position(landing)
                    })
                    .then_some((delta.clone(), decoded))
            })
        {
            break found;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("M04 no cap-6 spatial delta for the pushed player".into());
        }
        pusher
            .service_liveness_discarding_events(Duration::from_millis(25))
            .await?;
    };
    if !decoded.enter.is_empty() || !decoded.leave.is_empty() || decoded.update.len() != 1 {
        return Err("M04 push delta contains changes outside the replay's two-player view".into());
    }
    let moved = decoded.update[0];
    let mut expected_predecessor = moved;
    expected_predecessor.position.x -= 1;
    if predecessor != expected_predecessor {
        return Err("M04 live predecessor differs from the replay predecessor".into());
    }
    if let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let directory = Path::new(&directory);
        let predecessor_payload =
            encode_world_spatial_entities_snapshot(&WorldSpatialEntitiesSnapshot {
                content_generation: decoded.content_generation,
                actor_position: pusher_entity.position,
                own_identity: pusher_entity.entity.identity,
                entities: vec![pusher_entity, predecessor],
            })
            .map_err(|error| format!("M04 predecessor encode: {error:?}"))?;
        let wire = received.lock().map_err(|_| "M04 wire capture lock")?;
        let envelope = observed_envelope(&wire, delta.server_sequence)?;
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
            ("m04-envelope.bin", envelope),
            ("m04-spatial.bin", delta.payload.as_ref()),
            ("m04-predecessor.bin", predecessor_payload.as_slice()),
            ("m04-expected.txt", expected.as_bytes()),
        ] {
            std::fs::write(directory.join(name), contents)?;
        }
    }
    Ok(())
}

//! M02 client capture: a far door use makes the server autowalk and retry the use.

use super::*;
use crate::map::door::AuthoredClosedDoor;
use oteryn_dev_client::SessionEvent;
use oteryn_protocol_oteryn::world_map::{MapOrigin, MapTile};
use oteryn_protocol_oteryn::world_object::UseDisposition;

const GATE_POSITION: ActorPosition = ActorPosition {
    x: 32402,
    y: 32235,
    floor: 7,
};
#[path = "client_row_m02_replay.rs"]
mod replay;

const CLOSED_GATE: u16 = 2177;
const OPEN_GATE: u16 = 2178;

const GATE: TilePos = TilePos {
    x: 32402,
    y: 32235,
    floor: -7,
};
const APPROACH: TilePos = TilePos {
    x: 32399,
    y: 32235,
    floor: -7,
};

const EXPECTED_ARRIVAL: ActorPosition = ActorPosition {
    x: 32401,
    y: 32235,
    floor: 7,
};

fn distance(a: TilePos, b: TilePos) -> u16 {
    if a.floor == b.floor {
        a.x.abs_diff(b.x).max(a.y.abs_diff(b.y))
    } else {
        u16::MAX
    }
}

fn plain_gate(world: &BundleWorld) -> TestResult<u64> {
    let doors = world.facts().doors();
    let mut keys = doors.keys().filter(|&key| {
        doors.authored_closed(key) == Some(AuthoredClosedDoor::Plain)
            && crate::map::view::placement_of(key).is_some_and(|(at, _)| at == GATE)
    });
    let key = keys.next().ok_or("M02 authored gate is missing")?;
    if keys.next().is_some() {
        return Err("M02 authored gate is ambiguous".into());
    }
    Ok(key)
}

async fn walk_to_approach(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
) -> TestResult<Option<MapTile>> {
    let snapshot = client
        .session_mut()
        .gated_snapshot(oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or("M02 initial map snapshot is missing")?;
    let mut gate = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
        .map_err(|error| format!("M02 map snapshot: {error:?}"))?
        .tiles
        .into_iter()
        .find(|tile| tile.position == GATE_POSITION);
    let from = tile(client.world_spatial().actor_position)?;
    let path = route_live(client, world, |at| at == APPROACH)
        .await?
        .ok_or_else(|| format!("M02 authored gate approach is unreachable from {from:?}"))?;
    for direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, before, direction).ok_or("M02 missing planned landing")?;
        let outcome = client.step(direction).await?;
        let after = tile(client.world_spatial().actor_position)?;
        if outcome.disposition != StepDisposition::Moved
            || outcome.world_spatial_delta.is_none()
            || after != expected
        {
            return Err(format!(
                "M02 approach step {direction:?} from={before:?} expected={expected:?} actual={after:?}: {outcome:?}"
            ).into());
        }
        update_gate(&mut gate, &client.take_events());
        verdict::Harness::global().progress("walk-to-gate");
    }
    Ok(gate)
}

fn is_expected_walk(steps: &[ActorPosition]) -> bool {
    steps
        == [
            ActorPosition {
                x: 32400,
                ..EXPECTED_ARRIVAL
            },
            EXPECTED_ARRIVAL,
        ]
}

fn gate_matches(tile: &MapTile, ordinal: u8, appearance: u16, revision: u64) -> bool {
    tile.position == GATE_POSITION
        && tile.items.iter().any(|item| {
            item.appearance_id == appearance
                && item.origin
                    == (MapOrigin::BaseOrdinal {
                        ordinal,
                        object_revision: revision,
                    })
        })
}

fn update_gate(gate: &mut Option<MapTile>, events: &[SessionEvent]) {
    for event in events {
        if let SessionEvent::WorldMap(delta) = event {
            if delta.value.cleared.contains(&GATE_POSITION) {
                *gate = None;
            }
            if let Some(tile) = delta
                .value
                .tiles
                .iter()
                .find(|tile| tile.position == GATE_POSITION)
            {
                *gate = Some(tile.clone());
            }
        }
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let capture_dir =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("M02 requires a fresh capture directory")?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let key = plain_gate(world)?;
    let mut client = connect_world(clients, 0xa2).await?;
    if client.world_spatial().actor_position == EXPECTED_ARRIVAL {
        return Err("M02 first admission already starts at the expected relog position".into());
    }
    let (_, ordinal) = crate::map::view::placement_of(key).ok_or("M02 invalid gate placement")?;
    let mut gate = walk_to_approach(&mut client, world).await?;
    if !gate
        .as_ref()
        .is_some_and(|tile| gate_matches(tile, ordinal, CLOSED_GATE, 0))
    {
        return Err(
            "M02 approach did not observe the authored closed gate at revision zero".into(),
        );
    }
    let stand = tile(client.world_spatial().actor_position)?;
    if stand != APPROACH || distance(stand, GATE) != 3 {
        return Err("M02 route missed its planned approach".into());
    }
    update_gate(&mut gate, &client.take_events());
    if !gate
        .as_ref()
        .is_some_and(|tile| gate_matches(tile, ordinal, CLOSED_GATE, 0))
    {
        return Err("M02 gate changed before far use".into());
    }
    let map = client
        .session_mut()
        .world_map_snapshot()
        .ok_or("M02 missing map binding")?
        .header;
    let session = client.join_snapshot().game_session_id;
    let first_sequence = client
        .last_server_sequence()
        .checked_add(1)
        .ok_or("M02 sequence overflow")?;
    let capabilities = client.selected_capabilities().to_vec();
    client.session_mut().begin_qualification_capture();
    verdict::Harness::global().progress("far-use");
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let result = client.use_object(&placement, 0).await?;
    require_mechanic_acceptance(result.status, "M02 far use")?;
    if result.disposition != UseDisposition::Committed {
        return Err(format!("M02 far use did not commit: {result:?}").into());
    }
    let events = client.take_events();
    let mut previous = ActorPosition {
        x: i32::from(stand.x),
        y: i32::from(stand.y),
        floor: 7,
    };
    let mut steps = Vec::new();
    for event in &events {
        if let SessionEvent::WorldSpatialEntities(delta) = event
            && delta.server_sequence < result.result_server_sequence
            && delta.value.actor_position != previous
        {
            previous = delta.value.actor_position;
            steps.push(previous);
        }
    }
    if !is_expected_walk(&steps) {
        return Err(
            format!("M02 expected two consecutive eastward autowalk steps, got {steps:?}").into(),
        );
    }
    update_gate(&mut gate, &events);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    while !gate
        .as_ref()
        .is_some_and(|tile| gate_matches(tile, ordinal, OPEN_GATE, 1))
    {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("M02 far use did not publish the open gate at revision one".into());
        }
        client
            .service_liveness(remaining.min(Duration::from_millis(100)))
            .await?;
        update_gate(&mut gate, &client.take_events());
    }
    let arrived = client.world_spatial().actor_position;
    if arrived != EXPECTED_ARRIVAL || distance(tile(arrived)?, GATE) > 1 {
        return Err(format!(
            "M02 far use did not reach {EXPECTED_ARRIVAL:?} adjacent to the gate: {arrived:?}"
        )
        .into());
    }
    let last_sequence = client.last_server_sequence();
    let frames = client.session_mut().take_qualification_capture();
    let (logout, detail, _, _) = logout_once(&mut client).await?;
    if logout != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("M02 first logout refused: {detail}").into());
    }
    drop(client);
    let relog = connect_world(clients, 0xa3).await?;
    if relog.world_spatial().actor_position != EXPECTED_ARRIVAL {
        return Err(format!(
            "M02 relog did not restore the landing: {:?}",
            relog.world_spatial()
        )
        .into());
    }
    evidence("M02_GAMEPLAY_VERIFIED steps=2 gate=open revision=1 relog_position=32401,32235,7");
    let binding = replay::Capture {
        game_session_id: uuid_text(session.as_bytes()),
        command_id: result.command_id,
        first_sequence,
        result_sequence: result.result_server_sequence,
        last_sequence,
        capabilities,
        content_generation: map.content_generation,
        bundle_digest: map.bundle_digest,
        reset_epoch: map.reset_epoch,
        first_visible_floor: map.first_visible_floor,
        ordinal,
    };
    replay::write_capture(Path::new(&capture_dir), &binding, frames)?;
    evidence(&format!(
        "e2e_capture scenario=M02.client status=CAPTURED binding={}",
        binding.identity()
    ));
    Ok(())
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    if let Err(error) = verdict::observe(journey(clients)).await {
        evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=M02.client status=FAIL evidence={error}"
        ));
    }
}

#[test]
fn m02_requires_intermediate_step_in_order() {
    let intermediate = ActorPosition {
        x: 32400,
        ..EXPECTED_ARRIVAL
    };
    assert!(is_expected_walk(&[intermediate, EXPECTED_ARRIVAL]));
    assert!(!is_expected_walk(&[EXPECTED_ARRIVAL]));
    assert!(!is_expected_walk(&[EXPECTED_ARRIVAL, intermediate]));
}

#[test]
fn m02_requires_open_appearance_at_next_revision() -> TestResult {
    use oteryn_protocol_oteryn::world_map::{MapDefinition, MapItem};
    let mut gate = MapTile {
        position: GATE_POSITION,
        items: vec![MapItem {
            definition: MapDefinition::Item(std::num::NonZeroU32::new(1).ok_or("item id")?),
            count: 1,
            sub_type: 0,
            origin: MapOrigin::BaseOrdinal {
                ordinal: 1,
                object_revision: 0,
            },
            appearance_id: CLOSED_GATE,
        }],
        more: false,
        ground_speed: 150,
    };
    assert!(gate_matches(&gate, 1, CLOSED_GATE, 0));
    assert!(!gate_matches(&gate, 1, OPEN_GATE, 1));
    gate.items[0].appearance_id = OPEN_GATE;
    assert!(!gate_matches(&gate, 1, OPEN_GATE, 1));
    gate.items[0].origin = MapOrigin::BaseOrdinal {
        ordinal: 1,
        object_revision: 1,
    };
    assert!(gate_matches(&gate, 1, OPEN_GATE, 1));
    Ok(())
}

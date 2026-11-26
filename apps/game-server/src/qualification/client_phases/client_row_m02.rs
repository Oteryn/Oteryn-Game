//! M02 client capture: a far door use makes the server autowalk and retry the use.

use super::*;
use crate::map::door::AuthoredClosedDoor;
use oteryn_protocol_oteryn::world_object::UseDisposition;

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

fn capture(payload: &[u8], expected: ActorPosition) -> TestResult {
    use std::io::Write;

    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok(());
    };
    let root = Path::new(&root);
    std::fs::File::create_new(root.join("m02-world-spatial.bin"))?.write_all(payload)?;
    writeln!(
        std::fs::File::create_new(root.join("m02-expected.txt"))?,
        "{} {} {}",
        expected.x,
        expected.y,
        expected.floor,
    )?;
    Ok(())
}

async fn walk_to_approach(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
) -> TestResult {
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
        client.take_events();
        verdict::Harness::global().progress("walk-to-gate");
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
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
    walk_to_approach(&mut client, world).await?;
    let stand = tile(client.world_spatial().actor_position)?;
    if stand != APPROACH || distance(stand, GATE) != 3 {
        return Err("M02 route missed its planned approach".into());
    }
    client.take_events();
    verdict::Harness::global().progress("far-use");
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let result = client.use_object(&placement, 0).await?;
    if result.disposition != UseDisposition::Committed {
        return Err(format!("M02 far use did not commit: {result:?}").into());
    }
    let arrived = client.world_spatial().actor_position;
    if arrived != EXPECTED_ARRIVAL || distance(tile(arrived)?, GATE) > 1 {
        return Err(format!(
            "M02 far use did not reach {EXPECTED_ARRIVAL:?} adjacent to the gate: {arrived:?}"
        )
        .into());
    }
    let payload =
        oteryn_protocol_oteryn::world_spatial::encode_world_spatial(client.world_spatial());
    capture(&payload, EXPECTED_ARRIVAL)?;
    evidence(&format!(
        "SEAM_EVIDENCE e2e_all scenario=M02.client status=PASS position={arrived:?}"
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

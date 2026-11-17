//! P19 client capture of a negotiated forge marker on an exact visible creature.
use super::*;
use oteryn_protocol_oteryn::appearance::CAPABILITY_FORGE_CREATURE_V1;

fn observed_forge_marker(creature: bool, stacks: Option<u8>) -> bool {
    creature && matches!(stacks, Some(1..=5 | 15))
}

#[test]
fn forge_marker_evidence_requires_a_marked_creature() {
    assert!(!observed_forge_marker(true, None));
    assert!(!observed_forge_marker(false, Some(5)));
    assert!(observed_forge_marker(true, Some(5)));
    assert!(observed_forge_marker(true, Some(15)));
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_character(clients, 0x53, 0).await?;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_FORGE_CREATURE_V1)
    {
        return Err("forge creature capability 104 was not selected".into());
    }
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes,
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    let origin = tile(client.world_spatial().actor_position)?;
    let mut points: BTreeSet<_> = metadata
        .spawns
        .sources
        .iter()
        .flat_map(|source| {
            source.points.iter().map(move |point| TilePos {
                x: point.x,
                y: point.y,
                floor: source.floor,
            })
        })
        .filter(|at| {
            at.floor == origin.floor && at.x.abs_diff(origin.x).max(at.y.abs_diff(origin.y)) <= 160
        })
        .collect();
    let mut visits = 0;
    let observed = loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
        if let Some(entry) = client
            .session_mut()
            .visible_appearances()
            .into_iter()
            .find(|entry| {
                observed_forge_marker(
                    entry.entity.kind == oteryn_dev_client::EntityKind::Creature,
                    entry.forge_stacks,
                )
            })
        {
            break format!(
                "entity={:?};forge_stacks={:?};spawn_visits={visits};capture_kind=typed-visible-appearance",
                entry.entity.entity, entry.forge_stacks
            );
        }
        let adjacent = |here: TilePos, point: TilePos| {
            here.floor == point.floor && here.x.abs_diff(point.x).max(here.y.abs_diff(point.y)) <= 1
        };
        let here = tile(client.world_spatial().actor_position)?;
        points.retain(|point| !adjacent(here, *point));
        let path = route(world, here, |at| {
            points.iter().any(|point| adjacent(at, *point))
        })
        .ok_or_else(|| {
            format!("step=forge-marker no marker after {visits} reachable spawn visits")
        })?;
        walk(&mut client, world, &path).await?;
        visits += 1;
    };
    let _ = client.session_mut().logout().await?;
    Ok(observed)
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    match verdict::indexed_timeout(Duration::from_secs(330), verdict::observe(journey(clients)))
        .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=P19.client status=CAPTURED evidence={observed}"
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P19.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P19.client status=FAIL evidence=timeout"),
    }
}

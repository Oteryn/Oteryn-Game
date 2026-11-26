//! Capture the real appearance binding and spatial history of a visible forge creature.
use super::*;
use oteryn_dev_client::EntityKind;
use oteryn_protocol_oteryn::appearance::CAPABILITY_FORGE_CREATURE_V1;

async fn journey(clients: &SeamClients<'_>, directory: &Path) -> TestResult {
    let world = full_e2ef7::qualification_world(clients)?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let client = connect_character(clients, 0x53, 0).await?;
    let mut client = provisioning::provision_fixture(client, clients, 0, 100).await?;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_FORGE_CREATURE_V1)
    {
        return Err("P19 forge creature capability was not selected".into());
    }
    let start = tile(client.world_spatial().actor_position)?;
    let sites: std::collections::BTreeSet<_> = metadata
        .spawns
        .sources
        .iter()
        .flat_map(|source| {
            source
                .points
                .iter()
                .map(move |point| (source.floor, point.x, point.y))
        })
        .filter(|&(floor, x, y)| {
            start.floor.abs_diff(floor) <= 2 && start.x.abs_diff(x).max(start.y.abs_diff(y)) <= 161
        })
        .collect();
    let approach = |at: TilePos, (floor, x, y)| {
        at.floor == floor && at.x.abs_diff(x).max(at.y.abs_diff(y)) <= 5
    };
    // The marker is visible at a distance. Observe spawn areas from outside
    // melee range and repeat the sweep as the population owner adds markers.
    let mut points = sites.clone();
    let mut blocked = std::collections::BTreeSet::new();
    let files = loop {
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
        let marker = client
            .session_mut()
            .visible_appearances()
            .into_iter()
            .find(|entry| {
                entry.entity.kind == EntityKind::Creature && entry.forge_stacks.is_some()
            });
        if let Some(marker) = marker {
            if let Some(files) = client.session_mut().p19_capture_files(marker.entity.entity) {
                break files;
            }
            // Re-admit in the same area to get a new common world/appearance baseline
            // if the walk to this spawn exceeded the retained delta history.
            full_e2ef5::player_logout(&mut client).await?;
            drop(client);
            client = connect_character(clients, 0x54, 0).await?;
            if let Some(files) = client
                .session_mut()
                .visible_appearances()
                .into_iter()
                .filter(|entry| {
                    entry.entity.kind == EntityKind::Creature && entry.forge_stacks.is_some()
                })
                .find_map(|entry| client.session_mut().p19_capture_files(entry.entity.entity))
            {
                break files;
            }
        }
        let before = client.world_spatial().actor_position;
        let here = tile(before)?;
        points.retain(|&point| !approach(here, point));
        let path = route_avoiding(
            world,
            here,
            |at| points.iter().any(|&point| approach(at, point)),
            |at| !blocked.contains(&at),
        );
        let Some(direction) = path.and_then(|path| path.first().copied()) else {
            if points == sites && blocked.is_empty() {
                return Err("P19 no reachable compiled spawn approaches".into());
            }
            let tick = Duration::from_secs(10);
            verdict::Harness::global()
                .bounded_wait(tokio::time::Instant::now() + tick, "forge-population-tick");
            client.service_liveness(tick).await?;
            points.clone_from(&sites);
            blocked.clear();
            continue;
        };
        let expected = destination(world, here, direction).ok_or("P19 route lost its landing")?;
        let step = client.step_retrying(direction).await?;
        let after = client.world_spatial().actor_position;
        match step.disposition {
            StepDisposition::Moved
                if step.world_spatial_delta.is_some() && tile(after)? == expected =>
            {
                verdict::Harness::global().progress("walk");
            }
            // The compiled route cannot predict live collision. Replan from
            // the observed position without retrying this cell in the sweep.
            StepDisposition::Blocked if before == after && step.world_spatial_delta.is_none() => {
                blocked.insert(expected);
            }
            _ => return Err(format!("P19 unexpected step from {before:?}: {step:?}").into()),
        }
    };
    for (name, bytes) in files {
        std::fs::write(directory.join(name), bytes)?;
    }
    full_e2ef5::player_logout(&mut client).await
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return;
    };
    match verdict::indexed_timeout(
        Duration::from_secs(330),
        verdict::observe(journey(clients, Path::new(&directory))),
    )
    .await
    {
        Ok(Ok(())) => evidence(
            "SEAM_EVIDENCE e2e_all scenario=P19.client status=PASS evidence=forge-marker-replay-captured",
        ),
        Ok(Err(error)) => evidence(&format!(
            "SEAM_EVIDENCE e2e_all scenario=P19.client status=FAIL evidence={error}"
        )),
        Err(_) => {
            evidence("SEAM_EVIDENCE e2e_all scenario=P19.client status=FAIL evidence=timeout")
        }
    }
}

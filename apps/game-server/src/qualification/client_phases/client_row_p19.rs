//! Capture the real appearance binding and spatial history of a visible forge creature.
use super::*;
use oteryn_dev_client::EntityKind;
use oteryn_protocol_oteryn::appearance::CAPABILITY_FORGE_CREATURE_V1;

async fn place_observer(clients: &SeamClients<'_>, at: TilePos, spawn: TilePos) -> TestResult {
    wait_character_released(clients, 0).await?;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let updated = sqlx::query(
        "UPDATE game_character_last_positions
         SET tile_x=$1, tile_y=$2, tile_floor=$3, write_sequence=write_sequence+1
         WHERE character_id=encode($4,'hex')::uuid AND world_id=encode($5,'hex')::uuid",
    )
    .bind(i32::from(at.x))
    .bind(i32::from(at.y))
    .bind(-i16::from(at.floor))
    .bind(clients.characters[0].as_slice())
    .bind(clients.world.as_slice())
    .execute(&mut db)
    .await?;
    if updated.rows_affected() != 1 {
        return Err("P19 fixture needs a saved position for the released observer".into());
    }
    evidence(&format!(
        "FIXTURE_PLACEMENT row=P19 tile={at:?} spawn={spawn:?}"
    ));
    Ok(())
}

fn observer_tile(
    world: &BundleWorld,
    spawn: TilePos,
    occupied: &std::collections::BTreeSet<TilePos>,
) -> Option<TilePos> {
    // Keep the creature's home well inside the interest window and leave room
    // for it to move as the live monster owner starts processing the observer.
    (3..=5).find_map(|distance| {
        (-distance..=distance).find_map(|dy: i16| {
            (-distance..=distance).find_map(|dx: i16| {
                let at = TilePos {
                    x: spawn.x.checked_add_signed(dx)?,
                    y: spawn.y.checked_add_signed(dy)?,
                    floor: spawn.floor,
                };
                (dx.abs().max(dy.abs()) == distance
                    && !occupied.contains(&at)
                    && world.enterable(at))
                .then_some(at)
            })
        })
    })
}

async fn journey(clients: &SeamClients<'_>, directory: &Path) -> TestResult {
    let world = full_e2ef7::qualification_world(clients)?;
    let bytes = std::fs::read(required("NODE_BOOT_WORLD_BUNDLE")?)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    const ELIGIBLE_CREATURE: &str = "oteryn:creature.dragon";

    let points = metadata.spawns.sources.iter().flat_map(|source| {
        source.points.iter().map(move |point| {
            (
                point.creature,
                TilePos {
                    x: point.x,
                    y: point.y,
                    floor: source.floor,
                },
            )
        })
    });
    let occupied = points.clone().map(|(_, at)| at).collect();
    let sites: Vec<_> = points
        .filter(|&(creature, _)| {
            metadata.spawns.creatures[creature as usize].key == ELIGIBLE_CREATURE
        })
        .map(|(_, at)| at)
        .collect();
    let (spawn, at) = sites
        .iter()
        .filter_map(|&spawn| Some((spawn, observer_tile(world, spawn, &occupied)?)))
        .max_by_key(|&(_, at)| {
            sites
                .iter()
                .filter(|p| {
                    p.floor == at.floor && p.x.abs_diff(at.x) <= 5 && p.y.abs_diff(at.y) <= 5
                })
                .count()
        })
        .ok_or("P19 compiled world lacks an enterable observation tile by a Dragon spawn")?;

    let client = connect_character(clients, 0x53, 0).await?;
    let mut client = provisioning::provision_fixture(client, clients, 0, 100).await?;
    full_e2ef5::player_logout(&mut client).await?;
    drop(client);
    place_observer(clients, at, spawn).await?;
    let mut client = connect_character(clients, 0x54, 0).await?;
    if tile(client.world_spatial().actor_position)? != at {
        return Err("P19 observer was admitted away from its fixture tile".into());
    }
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_FORGE_CREATURE_V1)
    {
        return Err("P19 forge creature capability was not selected".into());
    }

    let mut previous = None;
    let files = loop {
        let creatures = client.world_entities().map_or(0, |entities| {
            entities
                .iter()
                .filter(|entity| entity.kind == EntityKind::Creature)
                .count()
        });
        let marker = client
            .session_mut()
            .visible_appearances()
            .into_iter()
            .find(|entry| {
                entry.entity.kind == EntityKind::Creature && entry.forge_stacks.is_some()
            });
        let observed = (creatures, marker.is_some());
        if previous != Some(observed) {
            evidence(&format!(
                "P19_FORGE_OBSERVATION visible_creatures={creatures} visible_mapping={}",
                observed.1
            ));
            previous = Some(observed);
        }
        if let Some(marker) = marker {
            break client
                .session_mut()
                .p19_capture_files(marker.entity.entity)
                .ok_or("P19 received Forge mapping but could not retain a complete replay")?;
        }
        let tick = Duration::from_secs(1);
        verdict::Harness::global()
            .bounded_wait(tokio::time::Instant::now() + tick, "forge-population-tick");
        client.service_liveness(tick).await?;
        client.take_events();
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

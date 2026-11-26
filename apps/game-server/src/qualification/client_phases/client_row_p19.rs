//! Capture the real appearance binding and spatial history of a visible forge creature.
use super::*;
use oteryn_dev_client::{DevClientSession, EntityKind};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(serde::Deserialize)]
struct MarkedCreature {
    identity: [u8; 16],
    generation: u64,
    x: u16,
    y: u16,
    floor: i8,
    stacks: u8,
}

impl MarkedCreature {
    fn entity(&self) -> EntityRef {
        EntityRef {
            identity: self.identity,
            generation: self.generation,
        }
    }

    fn tile(&self) -> TilePos {
        TilePos {
            x: self.x,
            y: self.y,
            floor: -self.floor,
        }
    }

    fn in_view(&self, client: &DevClientSession) -> bool {
        client.world_entities().is_some_and(|entities| {
            entities
                .iter()
                .any(|entity| entity.kind == EntityKind::Creature && entity.entity == self.entity())
        })
    }
}

async fn census(client: &DevClientSession) -> TestResult<Vec<MarkedCreature>> {
    let mut socket = tokio::net::UnixStream::connect(required("NODE_BOOT_CONTROL_SOCKET")?).await?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    socket
        .write_all(format!("inspect-forge {session}\n").as_bytes())
        .await?;
    socket.shutdown().await?;
    let mut response = Vec::new();
    socket.take(128 * 1024).read_to_end(&mut response).await?;
    serde_json::from_slice(&response)
        .map_err(|error| format!("P19 Forge census could not be read: {error}").into())
}

fn log_census(client: &DevClientSession, marked: &[MarkedCreature]) {
    evidence(&format!(
        "DIAGNOSTIC row=P19 forge_marked={} marked_in_view={}",
        marked.len(),
        marked.iter().filter(|mark| mark.in_view(client)).count(),
    ));
    for mark in marked {
        let identity: String = mark
            .identity
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        evidence(&format!(
            "DIAGNOSTIC row=P19 forge_id={identity} generation={} position={:?} stacks={} in_view={}",
            mark.generation,
            mark.tile(),
            mark.stacks,
            mark.in_view(client),
        ));
    }
}

async fn wait_tick(client: &mut DevClientSession) -> TestResult {
    let tick = Duration::from_secs(1);
    verdict::Harness::global()
        .bounded_wait(tokio::time::Instant::now() + tick, "forge-population-tick");
    client.service_liveness(tick).await?;
    client.take_events();
    Ok(())
}

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
    let occupied = metadata
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
        .collect();

    let client = connect_character(clients, 0x53, 0).await?;
    let mut client = provisioning::provision_fixture(client, clients, 0, 100).await?;
    let mut marked = census(&client).await?;
    log_census(&client, &marked);
    while marked.is_empty() {
        wait_tick(&mut client).await?;
        marked = census(&client).await?;
    }
    if !marked.iter().any(|mark| mark.in_view(&client)) {
        let (target, at) = marked
            .iter()
            .find_map(|mark| observer_tile(world, mark.tile(), &occupied).map(|at| (mark, at)))
            .ok_or("P19 compiled world lacks an enterable tile near any marked creature")?;
        full_e2ef5::player_logout(&mut client).await?;
        drop(client);
        place_observer(clients, at, target.tile()).await?;
        client = connect_character(clients, 0x54, 0).await?;
        if tile(client.world_spatial().actor_position)? != at {
            return Err("P19 observer was admitted away from its fixture tile".into());
        }
        marked = census(&client).await?;
        log_census(&client, &marked);
    }
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_FORGE_CREATURE_V1)
    {
        return Err("P19 forge creature capability was not selected".into());
    }

    let mut previously_visible = Vec::new();
    let files = loop {
        let marker = client
            .session_mut()
            .visible_appearances()
            .into_iter()
            .find(|entry| {
                entry.entity.kind == EntityKind::Creature && entry.forge_stacks.is_some()
            });
        let visible: Vec<_> = marked
            .iter()
            .filter(|mark| mark.in_view(&client))
            .map(MarkedCreature::entity)
            .collect();
        if visible != previously_visible {
            log_census(&client, &marked);
            previously_visible = visible;
        }
        if let Some(marker) = marker {
            evidence(&format!(
                "DIAGNOSTIC row=P19 mapping_entity={:?} stacks={:?}",
                marker.entity.entity, marker.forge_stacks
            ));
            break client
                .session_mut()
                .p19_capture_files(marker.entity.entity)
                .ok_or("P19 received Forge mapping but could not retain a complete replay")?;
        }
        wait_tick(&mut client).await?;
        marked = census(&client).await?;
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

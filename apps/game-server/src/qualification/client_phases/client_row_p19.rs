//! Verify the first visible Forge binding for each exact fixture creature.
use super::*;
use oteryn_dev_client::native_session::AppearanceUpdate;
use oteryn_dev_client::{DevClientSession, EntityKind, SessionEvent};
use oteryn_protocol_oteryn::appearance::CAPABILITY_FORGE_CREATURE_V1;
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

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    let result = verdict::indexed_timeout(
        Duration::from_secs(330),
        verdict::observe(prove(clients, world)),
    )
    .await;
    match result {
        Ok(Ok(())) => evidence(
            "e2e_all scenario=P19.client status=PASS evidence=first-appearance-influenced-3-and-fiendish-15",
        ),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=P19.client status=FAIL evidence={error}"
        )),
        Err(_) => evidence("e2e_all scenario=P19.client status=FAIL evidence=timeout"),
    }
}

async fn prove(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult {
    let capture = required("E2E_ALL_CAPTURE_DIR")?;
    let mut client = connect_character(clients, 0x19, 0).await?;
    log_census(&client, &census(&client).await?);
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_FORGE_CREATURE_V1)
    {
        return Err("P19 Forge appearance capability absent".into());
    }
    use crate::qualification_wild_spawn::adjacent_cells;
    let cells = |at: TilePos| {
        adjacent_cells(crate::foundation::MovementLocalPosition {
            x: i32::from(at.x),
            y: i32::from(at.y),
            floor: -i16::from(at.floor),
        })
        .into_iter()
        .filter(|candidate| {
            let (Ok(x), Ok(y)) = (u16::try_from(candidate.x), u16::try_from(candidate.y)) else {
                return false;
            };
            let target = TilePos {
                x,
                y,
                floor: at.floor,
            };
            world
                .base()
                .tile(x, y, at.floor)
                .is_some_and(|tile| tile.walkable() && !tile.protection_zone())
                && world.enterable(target)
                && !world.floors().contains(target)
                && world.floors().teleport(target).is_none()
        })
        .take(2)
        .collect::<Vec<_>>()
    };
    let start = tile(client.world_spatial().actor_position)?;
    let path = route(world, start, |at| cells(at).len() == 2)
        .ok_or("P19 no approach with two open spawn tiles")?;
    classa3_observation::walk(&mut client, world, &path).await?;
    let at = client.world_spatial().actor_position;
    let positions = cells(tile(at)?);
    if positions.len() != 2 {
        return Err("P19 reached an approach without two open spawn tiles".into());
    }
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let refused = request_fixture_control(
        session.clone(),
        "--spawn-forge",
        format!(
            "oteryn:creature.druid_familiar {} {} {} fiendish",
            positions[0].x, positions[0].y, positions[0].floor
        ),
    )
    .await;
    if refused.as_ref().err().map(String::as_str) != Some("ineligible_forge_creature") {
        return Err(format!("P19 expected ineligible_forge_creature, got {refused:?}").into());
    }
    let mut observed = Vec::new();
    for ((classification, stacks, folder), position) in
        [("influenced 3", 3, ""), ("fiendish", 15, "fiendish")]
            .into_iter()
            .zip(positions)
    {
        let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
        client.take_events();
        let reply = request_fixture_control(
            session.clone(),
            "--spawn-forge",
            format!(
                "oteryn:creature.rat {} {} {} {classification}",
                position.x, position.y, position.floor
            ),
        )
        .await?;
        let exact = parse_spawned_entity(&reply)?;
        evidence(&format!(
            "FIXTURE_PLACEMENT row=P19 entity={exact:?} position={position:?} classification={classification}"
        ));
        let mut reported_missing = false;
        let first = 'mapping: loop {
            for event in client.take_events() {
                if let SessionEvent::Appearance(delta) = event
                    && let AppearanceUpdate::Mappings(mappings) = delta.value
                    && let Some(binding) = mappings
                        .upserts
                        .iter()
                        .find(|binding| binding.entity == exact)
                {
                    break 'mapping (binding.forge_stacks, delta.server_sequence);
                }
            }
            if !reported_missing {
                let marked = census(&client).await?;
                if marked
                    .iter()
                    .any(|mark| mark.entity() == exact && mark.in_view(&client))
                {
                    log_census(&client, &marked);
                    evidence(&format!(
                        "DIAGNOSTIC row=P19 marked_entity={exact:?} in_view=true mapping_received=false"
                    ));
                    reported_missing = true;
                }
            }
            let tick = Duration::from_secs(1);
            verdict::Harness::global().bounded_wait(
                tokio::time::Instant::now() + tick,
                "forge-appearance-mapping",
            );
            client.service_liveness(tick).await?;
        };
        log_census(&client, &census(&client).await?);
        if first.0 != Some(stacks) {
            return Err(
                format!("P19 {classification} first wire binding was {:?}", first.0).into(),
            );
        }
        let files = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if let Some(files) = client.session_mut().p19_capture_files(exact) {
                    return TestResult::Ok(files);
                }
                client.service_liveness(Duration::from_millis(50)).await?;
            }
        })
        .await
        .map_err(|_| format!("P19 {classification} exact creature replay capture missing"))??;
        let directory = Path::new(&capture).join(folder);
        std::fs::create_dir_all(&directory)?;
        for (name, bytes) in files {
            std::fs::write(directory.join(name), bytes)?;
        }
        observed.push((exact, stacks));
        evidence(&format!(
            "P19_FORGE entity={exact:?} forge_stacks={stacks} sequence={}",
            first.1
        ));
        let previous = client.join_snapshot().game_session_id;
        classa3_observation::logout(&mut client).await?;
        drop(client);
        client = connect_character(clients, 0x19, 0).await?;
        if client.join_snapshot().game_session_id == previous
            || client.world_spatial().actor_position != at
        {
            return Err("P19 fresh admission did not resume the saved observation position".into());
        }
        log_census(&client, &census(&client).await?);
        let appearances = client.session_mut().visible_appearances();
        for &(entity, stacks) in &observed {
            if !appearances
                .iter()
                .any(|entry| entry.entity.entity == entity && entry.forge_stacks == Some(stacks))
            {
                return Err(
                    format!("P19 fresh admission lacks {entity:?} with stacks={stacks}").into(),
                );
            }
        }
    }
    classa3_observation::logout(&mut client).await
}

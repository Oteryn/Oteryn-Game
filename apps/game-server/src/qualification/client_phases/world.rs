//! Real-world journeys use the same pinned bundle and native content as `serve`.
use super::*;
use crate::map::boot::BundleWorld;
use crate::map::overlay::TilePos;
use std::collections::{BTreeMap, VecDeque};
use std::path::Path;

include!("character_core.rs");
include!("character_release.rs");
include!("floor_journey.rs");
include!("full_e2ef7_mount.rs");
include!("full_e2ef5_mount.rs");
#[path = "map_journey.rs"]
mod map_journey;
include!("native_content.rs");
#[path = "full_e2ef1.rs"]
mod provisioning;
include!("quest_journal.rs");
#[path = "starter_inventory.rs"]
mod starter_inventory;

pub(crate) const TOWN: TilePos = TilePos {
    x: 32369,
    y: 32238,
    floor: -7,
};

pub(crate) fn load_world(
    data: Vec<u8>,
    pin: &serde_json::Value,
    world: WorldId,
    channel: ChannelId,
) -> TestResult<BundleWorld> {
    let digest = hex32(pin["digest"].as_str().ok_or("World digest")?)?;
    let checked = crate::map::boot::check(
        data,
        crate::map::boot::BootPins {
            bundle: crate::map::BundlePins {
                digest,
                project_format_version: pin["project_format_version"]
                    .as_str()
                    .ok_or("World project")?
                    .into(),
                world_schema_version: pin["world_schema_version"]
                    .as_str()
                    .ok_or("World schema")?
                    .into(),
                content_revision: pin["content_revision"]
                    .as_str()
                    .ok_or("World content")?
                    .into(),
                production: false,
            },
            map_revision: format!("sha256:{}", pin["digest"].as_str().ok_or("World digest")?),
            start: TOWN,
        },
    )?;
    let input = native_input()?;
    // Prewarm before the world pass admits its first client.
    native_spell_book()?;
    let room = crate::content::qualify_selected_native_gameplay_room(world, input)?;
    let issuance = crate::content::NativeEntryActivationIssuance {
        world_id: world,
        activation_sequence: 1,
        server_artifact_digest: room.compiled().server_digest(),
        client_artifact_digest: room.compiled().client_digest(),
        frame_binding_digest: room.frame_binding().digest(),
    };
    let mut controller = crate::content::ContentActivationController::new();
    crate::content::activate_native_entry_room_with_gameplay(
        &mut controller,
        &crate::content::NodeBootQuiescence::before_channel_runtime(),
        world,
        &issuance,
        input,
    )?;
    let native = controller
        .active()
        .and_then(|active| active.native_gameplay())
        .ok_or("world has no native generation")?;
    let rat = native
        .reward_table()
        .row("oteryn:creature.rat")
        .map_err(|reason| format!("rat reward unavailable: {reason:?}"))?;
    if rat.loot_table.entries.is_empty() {
        return Err("town hunt has no production loot table".into());
    }
    crate::node::serve::boot_bundle_world(
        checked,
        world,
        channel,
        controller
            .active()
            .and_then(|active| active.native_gameplay()),
    )
    .map_err(|error| format!("production world loader: {error:?}").into())
}

fn tile(position: ActorPosition) -> TestResult<TilePos> {
    Ok(TilePos {
        x: u16::try_from(position.x)?,
        y: u16::try_from(position.y)?,
        floor: -i8::try_from(position.floor)?,
    })
}

/// Plan from immutable production map facts; only replies to real STEP commands prove travel.
fn route(
    world: &BundleWorld,
    from: TilePos,
    goal: impl Fn(TilePos) -> bool,
) -> Option<Vec<StepDirection>> {
    let mut queue = VecDeque::from([from]);
    let mut parents = BTreeMap::from([(from, None)]);
    while let Some(at) = queue.pop_front() {
        if goal(at) {
            let mut path = Vec::new();
            let mut cursor = at;
            while let Some(Some((previous, direction))) = parents.get(&cursor) {
                path.push(*direction);
                cursor = *previous;
            }
            path.reverse();
            return Some(path);
        }
        for direction in [
            StepDirection::North,
            StepDirection::East,
            StepDirection::South,
            StepDirection::West,
        ] {
            let Some(next) = destination(world, at, direction) else {
                continue;
            };
            if next.x.abs_diff(from.x) > 160
                || next.y.abs_diff(from.y) > 160
                || next.floor.abs_diff(from.floor) > 2
                || parents.contains_key(&next)
            {
                continue;
            }
            parents.insert(next, Some((at, direction)));
            queue.push_back(next);
        }
    }
    None
}

/// Use the production floor resolver over the production bundle's collision facts.
fn destination(world: &BundleWorld, from: TilePos, direction: StepDirection) -> Option<TilePos> {
    use crate::map::floor::{Ground, resolve_landing};
    use crate::movement::CardinalStep;
    let direction = match direction {
        StepDirection::North => CardinalStep::North,
        StepDirection::East => CardinalStep::East,
        StepDirection::South => CardinalStep::South,
        StepDirection::West => CardinalStep::West,
    };
    let position = |(x, y, z): (i32, i32, i32)| {
        Some(TilePos {
            x: u16::try_from(x).ok()?,
            y: u16::try_from(y).ok()?,
            floor: -i8::try_from(z).ok().filter(|z| (0..=15).contains(z))?,
        })
    };
    let ground = |cell| match position(cell) {
        Some(pos) => match world.base().tile(pos.x, pos.y, pos.floor) {
            None => Ground::Absent,
            Some(tile) if tile.walkable() && world.enterable(pos) => Ground::Walkable,
            Some(_) => Ground::Blocked,
        },
        None => Ground::Absent,
    };
    let next = resolve_landing(
        world.floors(),
        ground,
        |_| true,
        (i32::from(from.x), i32::from(from.y), -i32::from(from.floor)),
        direction,
    )
    .ok()?;
    position(next)
}

fn near_locker(world: &BundleWorld, at: TilePos) -> bool {
    (-1..=1).any(|dy| {
        (-1..=1).any(|dx| {
            let Some((x, y)) = at.x.checked_add_signed(dx).zip(at.y.checked_add_signed(dy)) else {
                return false;
            };
            let pos = TilePos {
                x,
                y,
                floor: at.floor,
            };
            world.base().tile(x, y, at.floor).is_some_and(|tile| {
                (0..tile.depths().iter().filter(|depth| **depth == 0).count()).any(|ordinal| {
                    u8::try_from(ordinal)
                        .ok()
                        .is_some_and(|ordinal| world.facts().depot_at(pos, ordinal).is_some())
                })
            })
        })
    })
}

fn town_route_retry_delay(
    disposition: StepDisposition,
    stationary: bool,
    attempt: u8,
) -> Option<Duration> {
    (disposition == StepDisposition::Rejected && stationary && attempt <= 5)
        .then_some(Duration::from_millis(500))
}

async fn walk(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let before = client.world_spatial().actor_position;
        let mut outcome = client.step_retrying(direction).await?;
        for attempt in 1..=6 {
            if outcome.disposition != StepDisposition::Rejected
                || client.world_spatial().actor_position != before
            {
                break;
            }
            evidence(&format!(
                "town_route step_rejected attempt={attempt} direction={direction:?} from={before:?} expected={:?} actual={:?}",
                destination(world, tile(before)?, direction),
                client.world_spatial().actor_position,
            ));
            let Some(delay) = town_route_retry_delay(
                outcome.disposition,
                client.world_spatial().actor_position == before,
                attempt,
            ) else {
                break;
            };
            tokio::time::sleep(delay).await;
            outcome = client.step_retrying(direction).await?;
        }
        if outcome.disposition != StepDisposition::Moved || outcome.world_spatial_delta.is_none() {
            return Err(format!(
                "town route step {direction:?} from={before:?} expected={:?} actual={:?}: {outcome:?}",
                destination(world, tile(before)?, direction),
                client.world_spatial().actor_position,
            )
            .into());
        }
        let after = client.world_spatial().actor_position;
        if Some(tile(after)?) != destination(world, tile(before)?, direction) {
            return Err(format!(
                "town step changed position unexpectedly: {before:?} -> {after:?}"
            )
            .into());
        }
        client.take_events();
    }
    Ok(())
}

pub(super) async fn logout_retrying_busy(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<(
    Option<oteryn_protocol_oteryn::logout::LogoutResult>,
    String,
    usize,
    bool,
)> {
    use oteryn_protocol_oteryn::logout::LogoutResult;

    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    let mut attempts = 0;
    loop {
        attempts += 1;
        let outcome = client
            .session_mut()
            .logout()
            .await
            .map_err(|error| format!("logout attempt {attempts} failed: {error}"))?;
        if outcome.result != Some(LogoutResult::Busy) {
            return Ok((outcome.result, format!("{outcome:?}"), attempts, false));
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok((outcome.result, format!("{outcome:?}"), attempts, true));
        }
        client.service_liveness(Duration::from_millis(250)).await
            .map_err(|error| format!("logout Busy retry liveness failed after attempt {attempts}: {outcome:?}; error={error}"))?;
    }
}

pub(super) async fn journeys(
    clients: &SeamClients<'_>,
    packet_clients: &SeamClients<'_>,
    path: &Path,
) -> TestResult {
    let started = std::time::Instant::now();
    let bytes = std::fs::read(path)?;
    let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))?;
    let pin = serde_json::from_slice(&std::fs::read(required("NODE_BOOT_WORLD_PIN")?)?)?;
    let world = load_world(
        bytes.clone(),
        &pin,
        WorldId::decode(&clients.world)?,
        ChannelId::decode(&clients.channel)?,
    )?;
    evidence(&format!(
        "world_loader state=booted duration_ms={} start={TOWN:?}",
        started.elapsed().as_millis()
    ));
    // The required core keeps its ordinary pair; packet journeys own a separate pair.
    let mut client = connect_world(clients, 0x70).await?;
    let entered_at = tile(client.world_spatial().actor_position)?;
    evidence(&format!(
        "world_fixture role=core phase=first_entry position={entered_at:?}"
    ));
    if entered_at != TOWN {
        return Err(format!(
            "Platform-created character did not enter the configured town: expected={TOWN:?} actual={entered_at:?}"
        )
        .into());
    }
    let (result, logout, attempts, busy_timed_out) = logout_retrying_busy(&mut client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("first town entry logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    drop(client);
    {
        let clients = packet_clients;
        include!("full_e2ef6_mount.rs");
        full_e2ef5::run(clients, &world).await;
        for index in 0..clients.accounts.len() {
            wait_character_released(clients, index).await?;
        }
        include!("full_e2ef4_mount.rs");
        // F4 can drop its socket; require terminal release before the core starts.
        for index in 0..clients.accounts.len() {
            wait_character_released(clients, index).await?;
        }
        evidence(&format!(
            "world_fixture role=packets phase=terminal_release accounts={}",
            clients.accounts.len()
        ));
    }
    // The core/profile pair has not inherited packet movement, loot or death.
    let profile_result = profile_journey(clients, &world, &bytes, &metadata.spawns).await;
    if let Err(error) = &profile_result {
        evidence(&format!("world_profile state=failed error={error}"));
    }
    let mut client = connect_world(clients, 0x71).await?;
    for capability in [17, 18, 151, 152, 153, 179, 230] {
        if !client.selected_capabilities().contains(&capability) {
            return Err(
                format!("qualified World did not offer journey capability {capability}").into(),
            );
        }
    }
    let snapshot = client
        .session_mut()
        .gated_snapshot(oteryn_protocol_oteryn::world_map::STATE_DOMAIN_WORLD_MAP_VIEW)
        .ok_or("town join has no production bundle map snapshot")?;
    let map = oteryn_protocol_oteryn::world_map::decode_world_map_snapshot(&snapshot.payload)
        .map_err(|error| format!("town map snapshot: {error:?}"))?;
    if map.header.bundle_digest != world.base().digest() || map.tiles.is_empty() {
        return Err("town join served a different or empty World bundle".into());
    }
    // Resume from the profile journey's saved cell and travel normally to town.
    let from = tile(client.world_spatial().actor_position)?;
    evidence(&format!(
        "world_fixture role=core phase=return_to_town from={from:?} to={TOWN:?}"
    ));
    let back = route(&world, from, |at| at == TOWN).ok_or_else(|| {
        format!("required core has no return route to town: from={from:?} to={TOWN:?}")
    })?;
    walk(&mut client, &world, &back).await?;
    client = character_core(client, clients, &world).await?;
    client = include!("full_e2ef3_registration.rs");
    client = floor_journey(client, clients, &world).await?;
    client_guild::guild(&mut client).await?;
    client_party::party(&mut client).await?;
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let buddy: String = sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(clients.characters[0].as_slice())
    .fetch_one(&mut connection)
    .await?;
    client_vip::add(&mut client, &buddy).await?;
    client_player_trade::pair(&mut client, clients).await?;
    client_prey::prey(&mut client).await?;
    client_task_board::task_board(&mut client).await?;
    client_imbue::imbue(&mut client).await?;
    client_hireling::hireling(&mut client).await?;
    client_browse_field::browse_field(&mut client).await?;
    client_daily_boost::daily_boost(&mut client).await?;
    let path = route(&world, TOWN, |at| near_locker(&world, at))
        .ok_or("town has no reachable authored depot locker")?;
    walk(&mut client, &world, &path).await?;
    client_depot_search::depot_search(&mut client).await?;
    client_stash::stash(&mut client).await?;
    client_market::market(&mut client).await?;
    shop_journey(&mut client, &world, &metadata.npcs, clients).await?;
    client_hotkey_equip::hotkey_equip(&mut client).await?;
    client_item_forge::item_forge(&mut client).await?;
    client_store::store(&mut client).await?;
    let after = destination(
        &world,
        tile(client.world_spatial().actor_position)?,
        StepDirection::West,
    )
    .map(|at| ActorPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    });
    client = client_disconnect_resume::recover(client, clients, after).await?;
    travel_to_spawn(&mut client, &world, &metadata.spawns, "oteryn:creature.rat").await?;
    client_hunt::hunt(&mut client).await?;
    travel_to_spawn(
        &mut client,
        &world,
        &metadata.spawns,
        "oteryn:creature.poison_spider",
    )
    .await?;
    client_death::death(&mut client, clients).await?;
    let respawn = client.world_spatial().actor_position;
    let old_session = client.join_snapshot().game_session_id;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let logout = client.session_mut().logout().await?;
        if logout.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("death relog logout was not accepted: {logout:?}").into());
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
    drop(client);
    client = connect_world(clients, 0x73).await?;
    if client.join_snapshot().game_session_id == old_session
        || client.world_spatial().actor_position != respawn
        || client
            .actor_vitals()
            .is_none_or(|vitals| vitals.health != vitals.max_health)
    {
        return Err("death relog did not retain the saved respawn position and full vitals".into());
    }
    evidence(&format!(
        "mechanic=death state=relogged position={respawn:?} health={}",
        client.actor_vitals().ok_or("death relog vitals")?.health
    ));

    drop(client);
    profile_result
}

async fn profile_journey(
    clients: &SeamClients<'_>,
    world: &BundleWorld,
    bytes: &[u8],
    spawns: &oteryn_world_bundle::spawn::Table,
) -> TestResult {
    // Catalogue classification takes several seconds in a debug build. Prepare it
    // before admission so the peer can keep acknowledging probes during the journey.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let model = map_journey::MapModel::load(world, bytes, &root)?;
    let client = connect_character(clients, 0x6e, 0).await?;
    let mut client = provisioning::provision(client, clients, 0).await?;
    let pz = if world
        .base()
        .tile(TOWN.x, TOWN.y, TOWN.floor)
        .is_some_and(|tile| tile.protection_zone())
    {
        client_spell_journey::protection_zone_probe(&mut client, true).await
    } else {
        Err("configured town start is not a bundle PZ".into())
    };
    let outside: TestResult = async {
        let path = route(world, TOWN, |at| {
            world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| !tile.protection_zone())
        })
        .ok_or("no reachable outside-PZ comparison tile")?;
        walk(&mut client, world, &path).await?;
        client_spell_journey::protection_zone_probe(&mut client, false).await
    }
    .await;
    let pz = pz.and(outside);
    let back = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == TOWN
    })
    .ok_or("PZ comparison has no return route")?;
    walk(&mut client, world, &back).await?;
    let map = map_journey::run(&mut client, world, &model).await;
    let combat = match travel_to_spawn(&mut client, world, spawns, "oteryn:creature.rat").await {
        Ok(()) => client_combat_journey::combat_journey(&mut client).await,
        Err(error) => Err(error),
    };
    let spells = client_spell_journey::spell_journey(&mut client, pz).await;
    // Combat may still hold the logout/PZ lock. Settle it outside the temple
    // before restoring the peer that main's trade/party phases use.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(70);
    loop {
        let logout = client.session_mut().logout().await?;
        if logout.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
            break;
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                format!("profile combat lock did not settle before logout: {logout:?}").into(),
            );
        }
        client.service_liveness(Duration::from_millis(100)).await?;
        client.take_events();
    }
    drop(client);
    client = connect_character(clients, 0x6f, 0).await?;
    let back = route(world, tile(client.world_spatial().actor_position)?, |at| {
        at == TOWN
    })
    .ok_or("profile journey has no return route")?;
    walk(&mut client, world, &back).await?;
    let (result, logout, attempts, busy_timed_out) = logout_retrying_busy(&mut client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) {
        return Err(format!("profile journey logout refused: {logout}; attempts={attempts}; busy_timed_out={busy_timed_out}").into());
    }
    map?;
    combat?;
    spells
}

async fn connect_world(
    clients: &SeamClients<'_>,
    tag: u8,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    connect_character(clients, tag, 1).await
}

fn fresh_grant_material(tag: u8) -> TestResult<([u8; 32], [u8; 16])> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT_CONNECTION: AtomicU64 = AtomicU64::new(1);
    let sequence = NEXT_CONNECTION
        .try_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            value.checked_add(1)
        })
        .map_err(|_| "qualification connection counter exhausted")?;
    let mut nonce = [tag; 32];
    nonce[24..].copy_from_slice(&sequence.to_be_bytes());
    let mut attempt = v7(tag, 0x05);
    attempt[8..].copy_from_slice(&sequence.to_be_bytes());
    // Preserve all 64 counter bits around the UUIDv7 version and RFC variant.
    attempt[6] |= (sequence >> 62) as u8;
    attempt[8] = (attempt[8] & 0x3f) | 0x80;
    Ok((nonce, attempt))
}

pub(super) async fn connect_character(
    clients: &SeamClients<'_>,
    tag: u8,
    index: usize,
) -> TestResult<oteryn_dev_client::DevClientSession> {
    wait_character_released(clients, index).await?;
    let generation = platform_generation(clients.descriptor, &clients.accounts[index]).await?;
    let (nonce, attempt) = fresh_grant_material(tag)?;
    let grant = Grant {
        signing: clients.signing,
        key_id: clients.key_id,
        account_id: &clients.accounts[index],
        character_id: clients.characters[index],
        world_id: clients.world,
        channel_id: clients.channel,
        security_generation: generation,
        scope_generation: clients.scope_generation,
        nonce,
        attempt,
    };
    let token = sign_grant(&grant, now_seconds()?);
    Ok(
        oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
            address: clients.address,
            server_name: "localhost",
            root_certificate: clients.certificate,
            schema_revision: 1,
            character_id: crate::foundation::CharacterId::decode(&clients.characters[index])?,
            admission_material: token.as_bytes(),
            client_build_id: "oteryn-dev-client/world-qualification",
            deadline: Duration::from_secs(20),
        })
        .await?,
    )
}

async fn travel_to_spawn(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    spawns: &oteryn_world_bundle::spawn::Table,
    creature: &str,
) -> TestResult {
    let from = tile(client.world_spatial().actor_position)?;
    let points: Vec<_> = spawns
        .sources
        .iter()
        .flat_map(|source| source.points.iter().map(move |point| (source.floor, point)))
        .filter(|(_, point)| {
            spawns
                .creatures
                .get(point.creature as usize)
                .is_some_and(|entry| entry.key == creature)
        })
        .collect();
    let path = route(world, from, |at| {
        !world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_none_or(|tile| tile.protection_zone())
            && points.iter().any(|(floor, point)| {
                at.floor == *floor && at.x.abs_diff(point.x) <= 1 && at.y.abs_diff(point.y) <= 1
            })
    })
    .ok_or_else(|| format!("no reachable {creature} spawn within town route budget"))?;
    walk(client, world, &path).await?;
    for _ in 0..32 {
        let here = client.world_spatial().actor_position;
        let target = client
            .world_entities()
            .and_then(|entities| {
                entities
                    .iter()
                    .filter(|entry| {
                        entry.kind == oteryn_dev_client::EntityKind::Creature
                            && entry.position.floor == here.floor
                    })
                    .min_by_key(|entry| {
                        entry
                            .position
                            .x
                            .abs_diff(here.x)
                            .max(entry.position.y.abs_diff(here.y))
                    })
            })
            .copied()
            .ok_or("spawn route has no visible creature")?;
        let target = tile(target.position)?;
        let path = route(world, tile(here)?, |at| {
            at.floor == target.floor && at.x.abs_diff(target.x).max(at.y.abs_diff(target.y)) <= 1
        })
        .ok_or("visible creature unreachable")?;
        let Some(direction) = path.first() else {
            return Ok(());
        };
        walk(client, world, &[*direction]).await?;
    }
    Err("could not approach a live creature within the route budget".into())
}

pub(crate) fn smoke() -> TestResult {
    use oteryn_world_bundle_compiler::{bundle::BuildClass, compile, source};
    let started = std::time::Instant::now();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let project = source::project_regions(&root, &[(126, 125), (126, 126), (127, 127)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let identity = serde_json::from_slice(&std::fs::read(
        root.join("content/world/pins/oteryn.identity.json"),
    )?)?;
    let input = compile::Input {
        regions: &project.regions,
        palette: &palette,
        identity,
        world: source::world_extent(&root)?,
        build_class: BuildClass::NonProduction,
        draft_areas: oteryn_world_bundle_compiler::project::draft_areas(&project.index)?,
        families: &project.families,
    };
    let compiled = compile::compile(&input, &registry)?;
    compile::equivalence(&input, &registry, &compiled.bytes)?;
    let compiled_ms = started.elapsed().as_millis();
    let boot_started = std::time::Instant::now();
    let mut pin: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("content/world/pins/oteryn.json"))?)?;
    pin["digest"] = compiled
        .digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>()
        .into();
    let world = load_world(
        compiled.bytes,
        &pin,
        WorldId::decode(&v7(1, 2))?,
        ChannelId::decode(&v7(1, 3))?,
    )?;
    assert!(
        world
            .base()
            .tile(TOWN.x, TOWN.y, TOWN.floor)
            .is_some_and(|tile| tile.protection_zone()),
        "town temple must be a protection zone"
    );
    assert!(
        route(&world, TOWN, |at| near_locker(&world, at)).is_some(),
        "town depot must be reachable"
    );
    let floor_path = route(&world, TOWN, |at| at.floor != TOWN.floor)
        .ok_or("town floor-change route must be reachable")?;
    let mut landing = TOWN;
    for direction in floor_path {
        landing = destination(&world, landing, direction).ok_or("floor route landing")?;
    }
    assert_ne!(landing.floor, TOWN.floor);
    assert!(
        route(&world, landing, |at| at == TOWN).is_some(),
        "town floor-change route must have a return path"
    );
    let rats: Vec<_> = project
        .families
        .spawns
        .iter()
        .flat_map(|source| &source.points)
        .filter(|point| point.creature == "oteryn:creature.rat")
        .map(|point| point.cell)
        .collect();
    assert!(
        route(&world, TOWN, |at| rats.iter().any(|&(x, y, z)| i16::from(
            at.floor
        )
            == -i16::from(z)
            && at.x.abs_diff(x).max(at.y.abs_diff(y)) <= 1))
        .is_some(),
        "town hunt must reach an authored rat with an admitted corpse and loot table"
    );
    evidence(&format!(
        "world_loader_smoke state=booted compile_ms={compiled_ms} boot_ms={} duration_ms={}",
        boot_started.elapsed().as_millis(),
        started.elapsed().as_millis()
    ));
    Ok(())
}

async fn shop_journey(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    npcs: &oteryn_world_bundle::npc::Table,
    clients: &SeamClients<'_>,
) -> TestResult<()> {
    let Some(npc) = npcs.npcs.iter().find(|npc| npc.key == "oteryn:npc.sam") else {
        evidence(
            "mechanic=npc_shop skipped=authored_shop_npc_missing content_id=oteryn:npc.sam source=content/npcs/definitions/index.json:1",
        );
        return Ok(());
    };
    let from = tile(client.world_spatial().actor_position)?;
    let Some(path) = route(world, from, |at| {
        npc.placements.iter().any(|placement| {
            placement.floor == at.floor
                && at.x.abs_diff(placement.x).max(at.y.abs_diff(placement.y)) <= 3
        })
    }) else {
        evidence(
            "mechanic=npc_shop skipped=no_route_to_authored_shop_within_town_budget content_id=oteryn:npc.sam source=apps/game-server/src/qualification/client_phases/world.rs:126",
        );
        return Ok(());
    };
    walk(client, world, &path).await?;
    let target = client
        .world_entities()
        .and_then(|entities| {
            entities.iter().find(|entity| {
                entity.kind == oteryn_dev_client::EntityKind::Npc
                    && npc.placements.iter().any(|placement| {
                        entity.position.x == i32::from(placement.x)
                            && entity.position.y == i32::from(placement.y)
                            && entity.position.floor == -i16::from(placement.floor)
                    })
            })
        })
        .copied();
    let Some(target) = target else {
        evidence(
            "mechanic=npc_shop skipped=authored_shop_has_no_visible_actor_at_placement content_id=oteryn:npc.sam source=crates/protocol-oteryn/src/world_spatial_entities.rs:105",
        );
        return Ok(());
    };
    client_npc_shop::transact(client, target.entity, clients).await
}

#[cfg(test)]
#[test]
fn qualification_nbtown_checks_first_entry_before_mutating_packets() -> TestResult {
    let source = include_str!("world.rs");
    let (_, journey) = source
        .split_once("pub(super) async fn journeys(")
        .ok_or("missing world journey")?;
    let (journey, _) = journey
        .split_once("async fn profile_journey(")
        .ok_or("missing profile journey boundary")?;
    let entry_check = journey
        .find("if entered_at != TOWN {")
        .ok_or("missing strict Platform-created town assertion")?;
    for packet in [
        "include!(\"full_e2ef6_mount.rs\")",
        "full_e2ef5::run(clients",
        "include!(\"full_e2ef4_mount.rs\")",
        "profile_journey(clients",
    ] {
        let packet_start = journey.find(packet).ok_or("missing mutating packet")?;
        assert!(
            entry_check < packet_start,
            "town entry checked after {packet}"
        );
    }
    let mount = include_str!("full_e2ef6_mount.rs");
    let starter = mount
        .find("full_e2ef6::run_starter(clients)")
        .ok_or("missing pristine starter row")?;
    let packet = mount
        .find("full_e2ef7::run_packet(clients)")
        .ok_or("missing F7 packet")?;
    let remaining = mount
        .find("full_e2ef6::run(clients")
        .ok_or("missing remaining F6 rows")?;
    assert!(starter < packet && packet < remaining);
    Ok(())
}

#[cfg(test)]
#[test]
fn qualification_nbfix_fresh_grants_for_reconnects_and_colliding_tags() -> TestResult {
    let mut nonces = std::collections::BTreeSet::new();
    let mut attempts = std::collections::BTreeSet::new();
    for tag in [0xe0, 0xa1, 0xa0, 0xe0, 0xa1, 0xa0] {
        let (nonce, attempt) = fresh_grant_material(tag)?;
        assert!(nonces.insert(nonce), "reused grant nonce for tag {tag}");
        assert!(attempts.insert(attempt), "reused attempt for tag {tag}");
        assert_eq!(nonce[0], tag);
        assert_eq!(attempt[7], tag);
        assert_eq!(attempt[6] >> 4, 7);
        assert_eq!(attempt[8] >> 6, 2);
    }
    Ok(())
}

#[cfg(test)]
#[test]
fn qualification_nbroute_retries_only_stationary_rejections_with_a_bound() {
    for attempt in 1..=6 {
        assert_eq!(
            town_route_retry_delay(StepDisposition::Rejected, true, attempt),
            (attempt <= 5).then_some(Duration::from_millis(500)),
        );
        for disposition in [StepDisposition::Moved, StepDisposition::Blocked] {
            assert_eq!(town_route_retry_delay(disposition, true, attempt), None);
        }
        for disposition in [
            StepDisposition::Moved,
            StepDisposition::Blocked,
            StepDisposition::Rejected,
        ] {
            assert_eq!(town_route_retry_delay(disposition, false, attempt), None);
        }
    }
}

// Route over the loaded qualification crop. A route may need to leave the
// normal local town window to get around a wall or use a return staircase.
pub(super) async fn qualification_hunt_route(
    client: &mut DevClientSession,
    world: &BundleWorld,
    target: Option<TilePos>,
    return_to: Option<TilePos>,
    deadline: tokio::time::Instant,
) -> TestResult<TilePos> {
    if tokio::time::Instant::now() >= deadline {
        return Err("qualification_hunt_route expired before travel".into());
    }
    let start = tile(client.world_spatial().actor_position)?;
    let adjacent = |at: TilePos, other: TilePos| {
        at.floor == other.floor && at.x.abs_diff(other.x).max(at.y.abs_diff(other.y)) <= 1
    };
    let mut live_rats = Vec::new();
    let mut corpses = Vec::new();
    for entity in client
        .world_entities()
        .into_iter()
        .flat_map(|view| view.iter())
    {
        let at = tile(entity.position)?;
        match entity.kind {
            oteryn_dev_client::EntityKind::Creature
                if entity
                    .display_name
                    .is_some_and(|name| name.as_str() == Ok("Rat")) =>
            {
                live_rats.push(at);
            }
            oteryn_dev_client::EntityKind::Corpse => corpses.push(at),
            _ => {}
        }
    }
    let snapshot = world.clone();
    let (path, endpoint) = route_work(client, move || {
        let world = &snapshot;
        let rat_approaches = if target.is_none() {
            let bundle = required("NODE_BOOT_WORLD_BUNDLE").map_err(|error| error.to_string())?;
            let bytes = std::fs::read(bundle).map_err(|error| error.to_string())?;
            let metadata = oteryn_world_bundle::bundle::visit(&bytes, |_, _| Ok(()))
                .map_err(|error| error.to_string())?;
            let creatures = &metadata.spawns.creatures;
            metadata
                .spawns
                .sources
                .iter()
                .flat_map(|source| {
                    source.points.iter().filter_map(move |point| {
                        creatures
                            .get(point.creature as usize)
                            .filter(|creature| creature.key == "oteryn:creature.rat")
                            .map(|_| TilePos {
                                x: point.x,
                                y: point.y,
                                floor: source.floor,
                            })
                    })
                })
                // The authored first floor of the Thais sewer has rats, cave rats
                // and a few bats. Stay in that site to avoid deeper mixed spawns.
                .filter(|rat| {
                    rat.floor == TOWN.floor - 1
                        && rat.x.abs_diff(TOWN.x).max(rat.y.abs_diff(TOWN.y)) <= 26
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
            live_rats.retain(|&at| rat_approaches.iter().any(|&rat| adjacent(at, rat)));
        let landing = |at, direction| {
            (tokio::time::Instant::now() < deadline)
                .then(|| destination(world, at, direction))
                .flatten()
        };
        let returnable = |endpoint| {
            return_to.is_none_or(|origin| find_route(endpoint, |at| at == origin, landing).is_ok())
        };
        let rat_tile = |at: TilePos| {
            world
                .base()
                .tile(at.x, at.y, at.floor)
                .is_some_and(|tile| tile.walkable() && !tile.protection_zone())
                && !rat_approaches.contains(&at)
        };
        let path = if let Some(target) = target {
            find_route(start, |at| at == target && returnable(at), landing)
        } else {
            let approached_live = (!live_rats.is_empty()).then(|| find_route(start, |at| {
                rat_tile(at)
                    && live_rats.iter().any(|&rat| at != rat && adjacent(at, rat))
                    && returnable(at)
            }, landing));
            if let Some(Ok(path)) = approached_live {
                Ok(path)
            } else {
                find_route(start, |at| {
                    rat_tile(at)
                        && rat_approaches.iter().any(|&rat| adjacent(at, rat))
                        && !corpses.iter().any(|&corpse| adjacent(at, corpse))
                        && returnable(at)
                }, landing)
            }
        }
        .map_err(|visited| {
            format!(
                "qualification_hunt_route from {start:?} has no approach and return after {visited} tiles"
            )
        })?;
        let endpoint = path
            .iter()
            .try_fold(start, |at, &direction| destination(world, at, direction))
            .ok_or("qualification_hunt_route endpoint disappeared")?;
        Ok((path, endpoint))
    })
    .await?;
    let steps = path.len();
    for direction in path {
        if tokio::time::Instant::now() >= deadline {
            return Err("qualification_hunt_route exhausted its walking budget".into());
        }
        walk(client, world, &[direction]).await?;
    }
    if tile(client.world_spatial().actor_position)? != endpoint {
        return Err(format!("qualification_hunt_route did not reach {endpoint:?}").into());
    }
    if tokio::time::Instant::now() >= deadline {
        return Err("qualification_hunt_route expired during travel".into());
    }
    evidence(&format!(
        "acquisition route=qualification_hunt_route start={start:?} end={endpoint:?} steps={steps}"
    ));
    Ok(endpoint)
}

// Walk to a live authored encounter for each hunt and return to the served
// start only after the caller has observed its actual XP or loot requirement.
#[allow(
    dead_code,
    reason = "A single trip or return needs only the route function."
)]
pub(super) async fn qualification_acquire(
    client: &mut DevClientSession,
    world: &BundleWorld,
    deadline: tokio::time::Instant,
    mut observed: impl AsyncFnMut(&mut DevClientSession) -> TestResult<bool>,
    mut hunt: impl AsyncFnMut(&mut DevClientSession, tokio::time::Instant) -> TestResult,
) -> TestResult {
    let entry = tile(client.world_spatial().actor_position)?;
    while !observed(client).await? {
        if tokio::time::Instant::now() >= deadline {
            return Err("qualification hunt exhausted the 150 s acquisition budget".into());
        }
        qualification_hunt_route(client, world, None, Some(entry), deadline).await?;
        hunt(client, deadline).await?;
    }
    qualification_hunt_route(client, world, Some(entry), None, deadline).await?;
    Ok(())
}

// Reuse the node-boot Platform intent and control socket so each row starts
// with its own character on the disposable account, with normal birth custody.
#[allow(
    dead_code,
    reason = "The Open PvP row uses the arena accounts prepared by node boot."
)]
pub(super) async fn qualification_character(
    clients: &SeamClients<'_>,
    index: usize,
    tag: u8,
    name: &str,
    vocation: &str,
) -> TestResult<[u8; 16]> {
    let account = clients.accounts.get(index).ok_or("row account absent")?;
    let world = uuid_text(&clients.world);
    let operation = uuid_text(&v7(tag, 0xed));
    let work = required("NODE_BOOT_PLATFORM_GRANT_DIR")?;
    let socket = Path::new(&required("NODE_BOOT_CONTROL_SOCKET")?).to_path_buf();
    let base = socket
        .parent()
        .and_then(Path::parent)
        .ok_or("row control socket has no deployment directory")?;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let revisions: (String, String, String, String) = sqlx::query_as(
        "SELECT profile_revision, ruleset_revision, content_revision, starter_template_revision
         FROM game_character_roots WHERE account_id=$1::uuid AND world_id=$2::uuid
           AND character_id=$3::uuid",
    )
    .bind(account)
    .bind(&world)
    .bind(uuid_text(&clients.characters[index]))
    .fetch_one(&mut database)
    .await?;
    let script = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tools/qualification/game_platform/composition.py");
    let account_for_intent = account.clone();
    let world_for_intent = world.clone();
    let operation_for_intent = operation.clone();
    let name_for_intent = name.to_owned();
    let vocation = vocation.to_owned();
    let issued = tokio::task::spawn_blocking(move || {
        std::process::Command::new("timeout")
            .args(["25s", "python3"])
            .arg(script)
            .args([
                "bootstrap",
                &work,
                &account_for_intent,
                &world_for_intent,
                &operation_for_intent,
                &revisions.0,
                &revisions.1,
                &revisions.2,
                &revisions.3,
                &name_for_intent,
                &vocation,
            ])
            .output()
    })
    .await??;
    if !issued.status.success() {
        return Err(format!("Platform bootstrap intent refused for {name}").into());
    }
    let binary = base.join("bin/oteryn-game-ops");
    let config = base.join("ops/ops.toml");
    let (created, attempts) = tokio::task::spawn_blocking(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(20);
        let mut attempts = 0;
        loop {
            attempts += 1;
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            let created = std::process::Command::new("timeout")
                .arg(format!("{:.3}s", remaining.as_secs_f64().max(0.001)))
                .arg(&binary)
                .arg("--config")
                .arg(&config)
                .args(["character", "bootstrap", "--socket"])
                .arg(&socket)
                .args(["--operation-id", &operation])
                .output()?;
            // game-ops exit 7 is an ambiguous bootstrap result. Reconcile the
            // same intent while the original deadline still permits a retry.
            let backoff = Duration::from_millis(250);
            if created.status.code() != Some(7)
                || deadline.saturating_duration_since(std::time::Instant::now()) <= backoff
            {
                return Ok::<_, std::io::Error>((created.status, attempts));
            }
            std::thread::sleep(backoff);
        }
    })
    .await??;
    if !created.success() {
        return Err(format!(
            "Game bootstrap refused for {name} after {attempts} attempts: {created}"
        )
        .into());
    }
    let ids: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(character_id) FROM game_character_roots
         WHERE account_id=$1::uuid AND world_id=$2::uuid AND name=$3 LIMIT 2",
    )
    .bind(account)
    .bind(&world)
    .bind(name)
    .fetch_all(&mut database)
    .await?;
    database.close().await?;
    let [id] = ids.as_slice() else {
        return Err(format!("expected one dedicated character named {name}").into());
    };
    let id = id.as_slice().try_into()?;
    if clients.characters.contains(&id) {
        return Err(format!("dedicated character {name} overlaps the shared fixture").into());
    }
    Ok(id)
}

async fn qualification_hunt_route(
    client: &mut DevClientSession,
    world: &BundleWorld,
    target: Option<TilePos>,
    return_to: Option<TilePos>,
    deadline: tokio::time::Instant,
) -> TestResult<TilePos> {
    let start = tile(client.world_spatial().actor_position)?;
    let path = qualification_hunt_plan(client, world, target, return_to, deadline)?;
    let endpoint = path
        .iter()
        .try_fold(start, |at, &direction| destination(world, at, direction))
        .ok_or("qualification_hunt_route endpoint disappeared")?;
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
async fn qualification_acquire(
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
async fn qualification_character(
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
    let created = tokio::task::spawn_blocking(move || {
        std::process::Command::new("timeout")
            .arg("20s")
            .arg(binary)
            .arg("--config")
            .arg(config)
            .args(["character", "bootstrap", "--socket"])
            .arg(socket)
            .args(["--operation-id", &operation])
            .output()
    })
    .await??;
    if !created.status.success() {
        return Err(format!("Game bootstrap refused for {name}").into());
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

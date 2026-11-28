//! S09 live protection-zone logout, persisted position, release and original reply replay.
use super::*;
use oteryn_protocol_oteryn::logout::LogoutResult;
use std::io::Write;

type SavedPosition = (i32, i32, i16, String, String);

async fn saved_position(
    clients: &SeamClients<'_>,
    db: &mut sqlx::PgConnection,
) -> TestResult<Option<SavedPosition>> {
    Ok(sqlx::query_as(
        "SELECT tile_x,tile_y,tile_floor,character_lease_generation::text,write_sequence::text \
         FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid \
         AND world_id=encode($2,'hex')::uuid",
    )
    .bind(clients.characters[0].as_slice())
    .bind(clients.world.as_slice())
    .fetch_optional(db)
    .await?)
}

async fn journey(clients: &SeamClients<'_>, world: &BundleWorld) -> TestResult<String> {
    let root =
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("S09 requires a fresh capture directory")?;
    let head = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()?;
    if !head.status.success() {
        return Err("S09 could not bind capture to the current commit".into());
    }
    let commit = std::str::from_utf8(&head.stdout)?.trim();
    let path = Path::new(&root).join("s09-logout.bin");
    let mut capture = std::fs::File::create_new(&path)?;
    let mut db = sqlx::PgConnection::connect(clients.url).await?;
    let mut client = connect_character(clients, 0x43, 0).await?;
    let start = tile(client.world_spatial().actor_position)?;
    let default = TilePos {
        x: 32369,
        y: 32238,
        floor: -7,
    };
    let plan = route(world, start, |at| {
        at != start && at != default && is_logout_rest_tile(world, at)
    })
    .ok_or("S09 has no route to a distinct authored logout tile")?;
    let mut target = start;
    for direction in plan {
        target = destination(world, target, direction).ok_or("S09 planned step has no landing")?;
        let step = client.step(direction).await?;
        if step.disposition != StepDisposition::Moved
            || tile(client.world_spatial().actor_position)? != target
        {
            return Err(format!("step=move expected={target:?} observed={step:?}").into());
        }
    }
    if !is_logout_rest_tile(world, target) {
        return Err(format!("step=pz tile={target:?}").into());
    }
    let expected = (
        i32::from(target.x),
        i32::from(target.y),
        i16::from(target.floor),
    );
    let before = saved_position(clients, &mut db).await?;
    if before
        .as_ref()
        .is_some_and(|row| (row.0, row.1, row.2) == expected)
    {
        return Err(format!("step=pre-save already persisted {expected:?}").into());
    }
    let session = client.join_snapshot().game_session_id;
    let command = client.next_command_id();
    let (outcome, frame) = client.session_mut().logout_with_frame().await?;
    if outcome.status != CommandStatus::Accepted || outcome.result != Some(LogoutResult::Accepted) {
        return Err(format!("step=logout observed {outcome:?}").into());
    }
    let released: Option<(bool, String)> = sqlx::query_as(
        "SELECT session_state=3 AND current_transport_ref IS NULL AND control_loss_epoch IS NULL, \
         character_lease_generation::text FROM game_durability_reconnect_sessions \
         WHERE game_session_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid",
    )
    .bind(session.as_bytes().as_slice())
    .bind(clients.characters[0].as_slice())
    .fetch_optional(&mut db)
    .await?;
    let Some((true, lease)) = released else {
        return Err(format!("step=logout-release exact session observed={released:?}").into());
    };
    let saved = saved_position(clients, &mut db).await?;
    if !saved
        .as_ref()
        .is_some_and(|row| (row.0, row.1, row.2) == expected && row.3 == lease)
        || saved == before
    {
        return Err(format!("step=logout-save expected={expected:?} lease={lease} before={before:?} observed={saved:?}").into());
    }
    drop(client);
    let mut fresh = connect_character(clients, 0x46, 0).await?;
    if tile(fresh.world_spatial().actor_position)? != target
        || fresh.join_snapshot().game_session_id == session
        || saved_position(clients, &mut db).await? != saved
    {
        return Err(format!(
            "step=relog expected={target:?} actual={:?}",
            fresh.world_spatial().actor_position
        )
        .into());
    }
    let cleanup = fresh.session_mut().logout().await?;
    if cleanup.status != CommandStatus::Accepted || cleanup.result != Some(LogoutResult::Accepted) {
        return Err(format!("step=relog-logout observed {cleanup:?}").into());
    }

    let binding = format!(
        "{}\t{}\t{}\t{}\t{}\n",
        commit,
        std::env::var("OTERYN_E2E_ROW_BATCH")?,
        uuid_text(session.as_bytes()),
        command,
        outcome.result_server_sequence,
    );
    capture.write_all(binding.as_bytes())?;
    capture.write_all(&frame)?;
    drop(capture);
    let contents = std::fs::read(&path)?;
    let replay_frame = contents
        .strip_prefix(binding.as_bytes())
        .ok_or("S09 replay capture binding mismatch")?;
    let decoded = oteryn_dev_client::DevClientSession::replay_logout_frame(replay_frame)?;
    if replay_frame != frame || decoded != outcome || decoded.command_id != command {
        return Err(format!("step=replay original={outcome:?} decoded={decoded:?}").into());
    }
    Ok(format!(
        "logout=Accepted release=terminal saved={target:?} replay=PASS"
    ))
}

pub(super) async fn run_packet(clients: &SeamClients<'_>, world: &BundleWorld) {
    let started = std::time::Instant::now();
    match verdict::indexed_timeout(
        Duration::from_secs(90),
        verdict::observe(journey(clients, world)),
    )
    .await
    {
        Ok(Ok(observed)) => evidence(&format!(
            "e2e_all scenario=S09.client status=PASS seconds={:.3} evidence={observed}",
            started.elapsed().as_secs_f64()
        )),
        Ok(Err(error)) => evidence(&format!(
            "e2e_all scenario=S09.client status=FAIL evidence={}",
            error.to_string().replace(' ', "_")
        )),
        Err(_) => evidence("e2e_all scenario=S09.client status=FAIL evidence=timeout"),
    }
}

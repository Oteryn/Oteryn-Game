//! S09 live protection-zone logout, persisted position, release and original reply replay.
use super::*;
use oteryn_protocol_oteryn::{attack::AttackIntentDisposition, logout::LogoutResult};
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

async fn terminal_lease(
    clients: &SeamClients<'_>,
    db: &mut sqlx::PgConnection,
    session: &[u8; 16],
) -> TestResult<String> {
    let released: Option<(bool, String)> = sqlx::query_as(
        "SELECT session_state=3 AND current_transport_ref IS NULL AND control_loss_epoch IS NULL, \
         character_lease_generation::text FROM game_durability_reconnect_sessions \
         WHERE game_session_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid",
    )
    .bind(session.as_slice())
    .bind(clients.characters[0].as_slice())
    .fetch_optional(db)
    .await?;
    require_terminal_lease(released)
}

fn require_terminal_lease(released: Option<(bool, String)>) -> TestResult<String> {
    match released {
        Some((true, lease)) => Ok(lease),
        _ => Err(format!("step=logout-release exact session observed={released:?}").into()),
    }
}

fn require_logout_save(
    expected: (i32, i32, i16),
    lease: &str,
    before: &Option<SavedPosition>,
    saved: &Option<SavedPosition>,
) -> TestResult {
    if !saved
        .as_ref()
        .is_some_and(|row| (row.0, row.1, row.2) == expected && row.3 == lease)
        || saved == before
    {
        return Err(format!("step=logout-save expected={expected:?} lease={lease} before={before:?} observed={saved:?}").into());
    }
    Ok(())
}

fn bound_frame<'a>(contents: &'a [u8], binding: &[u8], original: &[u8]) -> TestResult<&'a [u8]> {
    contents
        .strip_prefix(binding)
        .filter(|frame| *frame == original)
        .ok_or_else(|| "S09 replay capture binding or frame mismatch".into())
}

async fn walk_s09(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
    path: &[StepDirection],
) -> TestResult {
    for &direction in path {
        let before = tile(client.world_spatial().actor_position)?;
        let expected =
            destination(world, before, direction).ok_or("S09 planned step has no landing")?;
        let step = client.step_retrying(direction).await?;
        if step.disposition != StepDisposition::Moved
            || tile(client.world_spatial().actor_position)? != expected
        {
            return Err(format!(
                "step=move from={before:?} expected={expected:?} observed={step:?}"
            )
            .into());
        }
    }
    Ok(())
}

async fn establish_combat(
    client: &mut oteryn_dev_client::DevClientSession,
    world: &BundleWorld,
) -> TestResult {
    let outside = route(world, tile(client.world_spatial().actor_position)?, |at| {
        world
            .base()
            .tile(at.x, at.y, at.floor)
            .is_some_and(|tile| !tile.protection_zone())
    })
    .ok_or("S09 has no route outside protection")?;
    walk_s09(client, world, &outside).await?;
    let session = uuid_text(client.join_snapshot().game_session_id.as_bytes());
    let rat = parse_spawned_entity(&request_spawn(session, "oteryn:creature.rat".into()).await?)?;
    let visible_by = tokio::time::Instant::now() + Duration::from_secs(5);
    let position = loop {
        if let Some(entity) = client.world_entities().and_then(|view| view.get(&rat)) {
            if entity.kind != oteryn_dev_client::EntityKind::Creature {
                return Err("S09 spawned Rat was not a creature".into());
            }
            break tile(entity.position)?;
        }
        if tokio::time::Instant::now() >= visible_by {
            return Err("S09 spawned Rat never entered the entity view".into());
        }
        client.service_liveness(Duration::from_millis(25)).await?;
    };
    let approach = route_avoiding(
        world,
        tile(client.world_spatial().actor_position)?,
        |at| {
            at.floor == position.floor
                && u32::from(at.x.abs_diff(position.x)) + u32::from(at.y.abs_diff(position.y)) == 1
                && world
                    .base()
                    .tile(at.x, at.y, at.floor)
                    .is_some_and(|tile| !tile.protection_zone())
        },
        |at| at != position,
    )
    .ok_or("S09 has no unprotected approach to the Rat")?;
    walk_s09(client, world, &approach).await?;
    let attack = client.session_mut().attack_target(Some(&rat)).await?;
    if attack.disposition != AttackIntentDisposition::Ok {
        return Err(format!("step=attack observed={attack:?}").into());
    }
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    while !client
        .session_mut()
        .combat_state()
        .is_some_and(|state| state.in_fight && state.target == Some(rat))
    {
        if tokio::time::Instant::now() >= deadline {
            return Err("S09 Rat fight did not establish a combat lock".into());
        }
        client.service_liveness(Duration::from_millis(50)).await?;
    }
    let clear = client.session_mut().attack_target(None).await?;
    if clear.disposition != AttackIntentDisposition::Ok {
        return Err(format!("step=clear-rat observed={clear:?}").into());
    }
    Ok(())
}

fn require_combat_lock(in_fight: bool) -> TestResult {
    if !in_fight {
        return Err("S09 protection-zone logout requires an active combat lock".into());
    }
    Ok(())
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
    establish_combat(&mut client, world).await?;
    let from = tile(client.world_spatial().actor_position)?;
    let plan = route(world, from, |at| {
        at != start && at != default && is_logout_rest_tile(world, at)
    })
    .ok_or("S09 has no route to a distinct authored logout tile")?;
    walk_s09(&mut client, world, &plan).await?;
    let target = tile(client.world_spatial().actor_position)?;
    if target == start || target == default || !is_logout_rest_tile(world, target) {
        return Err(format!("step=pz tile={target:?}").into());
    }
    let expected = (
        i32::from(target.x),
        i32::from(target.y),
        -i16::from(target.floor),
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
    require_combat_lock(
        client
            .session_mut()
            .combat_state()
            .is_some_and(|state| state.in_fight),
    )?;
    let (outcome, frame) = client.session_mut().logout_with_frame().await?;
    if outcome.status != CommandStatus::Accepted || outcome.result != Some(LogoutResult::Accepted) {
        return Err(format!("step=logout observed {outcome:?}").into());
    }
    client.session_mut().observe_logout_close().await?;
    let lease = terminal_lease(clients, &mut db, session.as_bytes()).await?;
    let saved = saved_position(clients, &mut db).await?;
    require_logout_save(expected, &lease, &before, &saved)?;
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
    let fresh_session = fresh.join_snapshot().game_session_id;
    let cleanup = fresh.session_mut().logout().await?;
    if cleanup.status != CommandStatus::Accepted || cleanup.result != Some(LogoutResult::Accepted) {
        return Err(format!("step=relog-logout observed {cleanup:?}").into());
    }

    fresh.session_mut().observe_logout_close().await?;
    terminal_lease(clients, &mut db, fresh_session.as_bytes()).await?;

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
    let replay_frame = bound_frame(&contents, binding.as_bytes(), &frame)?;
    let decoded = oteryn_dev_client::DevClientSession::replay_logout_frame(replay_frame)?;
    if decoded != outcome || decoded.command_id != command {
        return Err(format!("step=replay original={outcome:?} decoded={decoded:?}").into());
    }
    Ok(format!(
        "logout=Accepted combat_lock=active server_close=clean release=terminal saved={target:?} replay=PASS"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_logout_without_an_active_combat_lock() -> TestResult {
        assert!(require_combat_lock(false).is_err());
        require_combat_lock(true)
    }

    #[test]
    fn rejects_unsaved_default_unchanged_and_prior_lease_positions() -> TestResult {
        let expected = (32369, 32237, 7);
        let saved = Some((32369, 32237, 7, "2".into(), "42".into()));
        require_logout_save(expected, "2", &None, &saved)?;
        assert!(require_logout_save(expected, "2", &None, &None).is_err());
        let default = Some((32369, 32238, 7, "2".into(), "42".into()));
        assert!(require_logout_save(expected, "2", &None, &default).is_err());
        assert!(require_logout_save(expected, "2", &saved, &saved).is_err());
        assert!(require_logout_save(expected, "3", &None, &saved).is_err());
        Ok(())
    }

    #[test]
    fn rejects_a_missing_or_unreleased_session() -> TestResult {
        assert!(require_terminal_lease(None).is_err());
        assert!(require_terminal_lease(Some((false, "2".into()))).is_err());
        assert_eq!(require_terminal_lease(Some((true, "2".into())))?, "2");
        Ok(())
    }

    #[test]
    fn rejects_a_missing_stale_or_substituted_capture() -> TestResult {
        assert_eq!(
            bound_frame(b"this-run\nreply", b"this-run\n", b"reply")?,
            b"reply"
        );
        assert!(bound_frame(b"", b"this-run\n", b"reply").is_err());
        assert!(bound_frame(b"prior-run\nreply", b"this-run\n", b"reply").is_err());
        assert!(bound_frame(b"this-run\nreplacement", b"this-run\n", b"reply").is_err());
        Ok(())
    }
}

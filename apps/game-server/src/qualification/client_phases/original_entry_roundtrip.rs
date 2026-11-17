//! Original Oteryn entry-room journey through the running node's real TLS listener.
use super::*;
use oteryn_dev_client::{DevClientSession, SessionEvent};
use oteryn_protocol_oteryn::world_object::UseDisposition;

const SELECTOR: &str = "oteryn-entry-roundtrip";

pub(super) fn selected(selector: Option<&str>, bundle_world: bool) -> TestResult<bool> {
    if selector != Some(SELECTOR) {
        return Ok(false);
    }
    if bundle_world {
        return Err("original entry roundtrip requires NODE_BOOT_WORLD=0".into());
    }
    Ok(true)
}

pub(super) async fn run(clients: &SeamClients<'_>) -> TestResult {
    let started = tokio::time::Instant::now();
    let mut active = None;
    let observed = tokio::time::timeout(Duration::from_secs(120), journey(clients, &mut active))
        .await
        .map_err(|_| -> Box<dyn std::error::Error> { "original entry journey deadline".into() })
        .and_then(|result| result);
    // Even a failed assertion or cancelled journey must release its actual server session.
    let cleanup = tokio::time::timeout(Duration::from_secs(180), async {
        let logout = if let Some(mut client) = active.take() {
            let result = accepted_logout(&mut client).await;
            drop(client);
            result
        } else {
            Ok(())
        };
        let released = wait_released(clients).await;
        logout.and(released)
    })
    .await
    .map_err(|_| -> Box<dyn std::error::Error> { "original entry cleanup deadline".into() })
    .and_then(|result| result);
    let result = observed.and(cleanup);
    let (status, proof) = if result.is_ok() {
        (
            "PASS",
            "original-native-room;fresh-login;door;logout-save;fresh-relogin;terminal-cleanup",
        )
    } else {
        ("FAIL", "original-native-room;incomplete-journey-or-cleanup")
    };
    evidence(&format!(
        "e2e_all scenario=OTERYN.entry-roundtrip status={status} seconds={:.3} evidence={proof}",
        started.elapsed().as_secs_f64(),
    ));
    result
}

async fn journey(clients: &SeamClients<'_>, active: &mut Option<DevClientSession>) -> TestResult {
    let admissions = committed_admissions(clients.url).await?;
    let content = crate::content::qualify_native_entry_room(WorldId::decode(&clients.world)?)?
        .compiled()
        .client_digest();
    *active = Some(client_world::connect_character(clients, 0xD1, 0).await?);
    let client = active.as_mut().ok_or("original entry client missing")?;
    let joined = client.join_snapshot().clone();
    if joined.world_spatial.content_generation != content
        || joined.world_spatial.actor_position
            != (ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            })
    {
        return Err("original entry first snapshot content/position mismatch".into());
    }
    let door = joined
        .world_object_overlay
        .iter()
        .find(|entry| entry.placement == crate::content::accepted::DOOR_CELL.0.as_bytes())
        .ok_or("original entry door absent from actual join")?;
    if door.content_generation != content
        || door.state != crate::content::accepted::DOOR_CLOSED_STATE.as_bytes()
    {
        return Err("original entry door is not the expected closed original object".into());
    }
    evidence(&format!(
        "original_entry phase=joined session={} position=0,0,0 door_revision={}",
        uuid_text(joined.game_session_id.as_bytes()),
        door.revision,
    ));
    client.take_events();
    moved(
        client,
        StepDirection::East,
        ActorPosition {
            x: 1,
            y: 0,
            floor: 0,
        },
    )
    .await?;
    let open = door_use(
        client,
        door.revision,
        crate::content::accepted::DOOR_OPEN_STATE,
        content,
    )
    .await?;
    moved(
        client,
        StepDirection::North,
        ActorPosition {
            x: 1,
            y: -1,
            floor: 0,
        },
    )
    .await?;
    moved(
        client,
        StepDirection::South,
        ActorPosition {
            x: 1,
            y: 0,
            floor: 0,
        },
    )
    .await?;
    door_use(
        client,
        open,
        crate::content::accepted::DOOR_CLOSED_STATE,
        content,
    )
    .await?;
    let final_position = client.world_spatial().actor_position;
    accepted_logout(client).await?;
    drop(active.take());
    wait_released(clients).await?;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let saved: (String, i32, i32, i16, String, String) = sqlx::query_as(
        "SELECT world_id::text,tile_x,tile_y,tile_floor,character_lease_generation::text,write_sequence::text \
         FROM game_character_last_positions WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(clients.characters[0].as_slice())
    .fetch_one(&mut database)
    .await?;
    database.close().await?;
    if saved.0 != uuid_text(&clients.world)
        || (saved.1, saved.2, saved.3) != (final_position.x, final_position.y, final_position.floor)
    {
        return Err("logout did not persist the actual final original-room position".into());
    }
    evidence(&format!(
        "original_entry phase=durable-readback world={} position={},{},{} lease_generation={} write_sequence={}",
        saved.0, saved.1, saved.2, saved.3, saved.4, saved.5,
    ));
    *active = Some(client_world::connect_character(clients, 0xD2, 0).await?);
    let client = active
        .as_ref()
        .ok_or("original entry fresh client missing")?;
    if client.join_snapshot().game_session_id == joined.game_session_id
        || client.world_spatial().actor_position != final_position
        || client.world_spatial().content_generation != content
        || committed_admissions(clients.url).await? != admissions + 2
    {
        return Err(
            "fresh original entry admission did not preserve saved position in a new session"
                .into(),
        );
    }
    evidence(&format!(
        "original_entry phase=fresh-relogin old_session={} new_session={} position={},{},{} admissions_delta=2",
        uuid_text(joined.game_session_id.as_bytes()),
        uuid_text(client.join_snapshot().game_session_id.as_bytes()),
        final_position.x,
        final_position.y,
        final_position.floor,
    ));
    Ok(())
}

async fn moved(
    client: &mut DevClientSession,
    direction: StepDirection,
    expected: ActorPosition,
) -> TestResult {
    let before = client.world_spatial().actor_position;
    let outcome = client.step_retrying(direction).await?;
    let delta = outcome
        .world_spatial_delta
        .as_ref()
        .ok_or("original entry step omitted delta")?;
    if outcome.status != CommandStatus::Accepted
        || outcome.disposition != StepDisposition::Moved
        || client.world_spatial().actor_position != expected
        || delta.value.actor_position != expected
        || delta.new_revision <= delta.base_revision
    {
        return Err(format!(
            "original entry step did not move {before:?} to {expected:?}: {outcome:?}"
        )
        .into());
    }
    evidence(&format!(
        "original_entry phase=step command={} sequence={} base={} revision={} position={},{},{} disposition=Moved",
        outcome.command_id,
        outcome.result_server_sequence,
        delta.base_revision,
        delta.new_revision,
        expected.x,
        expected.y,
        expected.floor,
    ));
    client.take_events();
    Ok(())
}

async fn door_use(
    client: &mut DevClientSession,
    revision: u64,
    state: &str,
    content: [u8; 32],
) -> TestResult<u64> {
    let placement = crate::content::accepted::DOOR_CELL.0.as_bytes();
    let outcome = client.use_object(placement, revision).await?;
    if outcome.status != CommandStatus::Accepted || outcome.disposition != UseDisposition::Committed
    {
        return Err(format!("original entry USE failed: {outcome:?}").into());
    }
    let next = revision
        .checked_add(1)
        .ok_or("original entry door revision overflow")?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(20);
    loop {
        let remaining = deadline
            .checked_duration_since(tokio::time::Instant::now())
            .ok_or("original entry door delta deadline")?;
        for event in client.wait_for_events(remaining).await? {
            if let SessionEvent::WorldObjectOverlay(delta) = event {
                if delta.value.placement == placement {
                    if delta.value.content_generation != content
                        || delta.value.state != state.as_bytes()
                        || delta.value.revision != next
                        || delta.base_revision != revision
                        || delta.new_revision != next
                    {
                        return Err("original entry door pushed unexpected state/revision".into());
                    }
                    evidence(&format!(
                        "original_entry phase=use command={} result_sequence={} delta_sequence={} base={} revision={} state={} disposition=Committed",
                        outcome.command_id,
                        outcome.result_server_sequence,
                        delta.server_sequence,
                        delta.base_revision,
                        delta.new_revision,
                        state,
                    ));
                    return Ok(next);
                }
            }
        }
    }
}

async fn accepted_logout(client: &mut DevClientSession) -> TestResult {
    let (result, outcome, attempts, timed_out) = client_world::logout_retrying_busy(client).await?;
    if result != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted) || timed_out {
        return Err(
            format!("original entry logout refused: {outcome}; attempts={attempts}").into(),
        );
    }
    evidence(&format!(
        "original_entry phase=logout result=Accepted attempts={attempts}"
    ));
    Ok(())
}

async fn wait_released(clients: &SeamClients<'_>) -> TestResult {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(160);
    loop {
        let live: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions \
             WHERE world_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND session_state<>3)",
        )
        .bind(clients.world.as_slice()).bind(clients.characters[0].as_slice())
        .fetch_one(&mut database).await?;
        if !live {
            database.close().await?;
            evidence("original_entry phase=release terminal=true");
            return Ok(());
        }
        if tokio::time::Instant::now() >= deadline {
            return Err("original entry character not terminally released".into());
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

#[test]
fn selector_is_exact_and_refuses_imported_bundle() -> TestResult {
    assert!(!selected(None, false)?);
    assert!(!selected(Some("f6"), true)?);
    assert!(!selected(Some("oteryn-entry-roundtrip-extra"), false)?);
    assert!(selected(Some(SELECTOR), false)?);
    assert!(selected(Some(SELECTOR), true).is_err());
    Ok(())
}

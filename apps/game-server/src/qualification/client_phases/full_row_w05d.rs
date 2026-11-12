//! W05d seasonal events and the event scheduler, derived from the world-event owner
//! (`durability/world_event.rs`, `gameplay_transport/world_event.rs`).
//!
//! 1. Admit the node-boot character and send the real world-event query (capability 53).
//! 2. Compare the reply with the durable reset/event-epoch row (read-only SQL): same activity and
//!    same announcement, and a second query with a new command id returns the same state.
//! 3. Log out, obtain a fresh admission and query again; the state must be unchanged.
//! 4. PASS only when an event is active (the announcement is then the scheduler's published effect);
//!    with no event active on the run date the row reports FAIL naming that precondition.
use super::*;
use oteryn_protocol_oteryn::world_event as wire;
use sqlx::{Connection, Row};

const NEEDED: &str = "needed:seasonal-event-active-in-the-calendar-for-the-run-date";

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(state)) if state.active => (
            "PASS",
            compact(&format!(
                "announcement-matches-durable-epoch-row,epoch={},text={}",
                state.reset_epoch, state.announcement
            )),
        ),
        Ok(Ok(state)) => (
            "FAIL",
            format!("{NEEDED};observed=inactive-epoch={}", state.reset_epoch),
        ),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=W05d.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let token: String = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(400)
        .collect();
    if token.is_empty() {
        "unknown".to_owned()
    } else {
        token
    }
}

/// The reply must agree with the durable epoch row about activity and announcement.
fn agrees(
    state: &wire::WorldEventState,
    row_active: Option<bool>,
    row_announcement: Option<&str>,
    row_epoch: &str,
) -> bool {
    state.reset_epoch.to_string() == row_epoch
        && state.active == row_active.unwrap_or(false)
        && state.announcement == row_announcement.unwrap_or_default()
}

async fn query(
    client: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<wire::WorldEventState> {
    let reply = client
        .session_mut()
        .gated_command(
            wire::CAPABILITY_WORLD_EVENT_V1,
            wire::COMMAND_TYPE_WORLD_EVENT_QUERY,
            &[],
        )
        .await?;
    if reply.status != CommandStatus::Accepted {
        return Err(format!("step=query observed=status-{:?}", reply.status).into());
    }
    wire::decode_world_event_state(&reply.payload)
        .map_err(|error| format!("step=decode observed={error:?}").into())
}

async fn durable(clients: &SeamClients<'_>) -> TestResult<(Option<bool>, Option<String>, String)> {
    let world = WorldId::decode(&clients.world)?;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let row = sqlx::query("SELECT a.reset_epoch::text AS epoch, e.active, e.announcement FROM game_world_reset_active a LEFT JOIN game_world_event_epochs e USING(world_id,reset_epoch) WHERE a.world_id = encode($1,'hex')::uuid")
        .bind(world.as_bytes().as_slice())
        .fetch_optional(&mut database)
        .await?
        .ok_or("step=durable observed=no-active-reset-row")?;
    Ok((
        row.try_get("active")?,
        row.try_get("announcement")?,
        row.try_get("epoch")?,
    ))
}

async fn logout(client: &mut oteryn_dev_client::DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<wire::WorldEventState> {
    let mut client = connect_world(clients, 0xe2).await?;
    let first = query(&mut client).await?;
    let (active, announcement, epoch) = durable(clients).await?;
    if !agrees(&first, active, announcement.as_deref(), &epoch) {
        return Err(format!(
            "step=durable-compare observed=reply-active:{}-row-active:{active:?}-reply-epoch:{}-row-epoch:{epoch}",
            first.active, first.reset_epoch
        )
        .into());
    }
    let second = query(&mut client).await?;
    if second != first {
        return Err("step=repeat-query observed=state-changed".into());
    }
    logout(&mut client).await?;
    let mut client = connect_world(clients, 0xe2).await?;
    let fresh = query(&mut client).await?;
    if fresh != first {
        return Err("step=fresh-admission observed=state-changed".into());
    }
    logout(&mut client).await?;
    Ok(first)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(active: bool, text: &str) -> wire::WorldEventState {
        wire::WorldEventState {
            reset_epoch: 3,
            active,
            announcement: text.to_owned(),
        }
    }

    #[test]
    fn reply_must_agree_with_the_durable_row() {
        assert!(agrees(&state(false, ""), None, None, "3"));
        assert!(agrees(&state(true, "Fair"), Some(true), Some("Fair"), "3"));
        assert!(!agrees(
            &state(true, "Fair"),
            Some(false),
            Some("Fair"),
            "3"
        ));
        assert!(!agrees(
            &state(true, "Fair"),
            Some(true),
            Some("Other"),
            "3"
        ));
        assert!(!agrees(&state(true, "Fair"), Some(true), Some("Fair"), "4"));
    }

    #[test]
    fn compact_is_one_token() {
        assert_eq!(compact("a  b"), "a-b");
        assert_eq!(compact(" "), "unknown");
    }
}

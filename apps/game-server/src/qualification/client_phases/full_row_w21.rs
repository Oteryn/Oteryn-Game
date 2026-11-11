//! W21 boss difficulty selection, derived from the capability-225 owners
//! (`crates/gameplay/dispatch/.../boss_difficulty.rs`, durable `boss_difficulty.rs`).
//! 1. Admit the node-boot character and require capability 225; read (read-only SQL) one enabled
//!    boss key of the room catalogue, the only key the server accepts.
//! 2. Drive the native client panel: Open expects a server `Window` with a fresh context, the
//!    requested boss key and its current opening difficulty (any value the server chose).
//! 3. Preview a different difficulty (5, or 4 when it opened at 5); expect a `Window` with it and
//!    the durable context row to hold it (read-only SQL).
//! 4. Start it; a started fight needs the server's accepted profile. The server answers
//!    `NoAcceptedProfile` for every Start, so the selection can never begin a fight: the row ends
//!    FAIL citing the refusal and the needed accepted profile.
//! 5. Cancel with the context the server returned (the panel drops it on a refusal); expect
//!    `Closed` and the durable context row closed. Log out.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::{
    boss_difficulty::{Refusal, Reply},
    raw_feature,
};
use oteryn_dev_client::client_ui::boss_difficulty::{
    BossDifficultyPanel, Intent, apply_result, request,
};
use oteryn_protocol_oteryn::boss_difficulty::CAPABILITY_BOSS_DIFFICULTY_V1;

const START_REFUSAL: &str =
    "source:crates/gameplay/durability/src/durability/boss_difficulty.rs:184";

enum Verdict {
    Fail(String),
}

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(90), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(Verdict::Fail(detail))) => ("FAIL", detail),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=W21.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let token: String = value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(300)
        .collect();
    if token.is_empty() {
        "unknown".to_owned()
    } else {
        token
    }
}

/// Send one panel intent through the native client path and return the decoded server reply.
async fn act(
    client: &mut DevClientSession,
    panel: &mut BossDifficultyPanel,
    intent: Intent,
    step: &str,
) -> TestResult<Reply> {
    let command = request(panel, intent)
        .ok_or_else(|| format!("step={step} observed=client-panel-built-no-command"))?;
    let result = raw_feature::send(client.session_mut(), &command).await?;
    if !apply_result(panel, &result) {
        return Err(format!(
            "step={step} observed=status:{:?},payload-bytes:{}",
            result.status,
            result.payload.len()
        )
        .into());
    }
    panel
        .reply
        .clone()
        .ok_or_else(|| format!("step={step} observed=no-reply").into())
}

async fn context_row(
    clients: &SeamClients<'_>,
    context: [u8; 16],
) -> TestResult<Option<(Option<i32>, bool)>> {
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let row = sqlx::query_as("SELECT selected_difficulty::int4, closed FROM game_boss_difficulty_contexts WHERE context_id = encode($1,'hex')::uuid")
        .bind(context.as_slice())
        .fetch_optional(&mut database)
        .await?;
    database.close().await?;
    Ok(row)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    let mut client = row_connect_world(clients, 0xd1).await?;
    if !client
        .selected_capabilities()
        .contains(&CAPABILITY_BOSS_DIFFICULTY_V1)
    {
        return Err("step=admission observed=capability-225-not-selected".into());
    }
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let boss: Option<String> = sqlx::query_scalar(
        "SELECT boss_key FROM game_boss_room_catalogue WHERE enabled ORDER BY boss_key LIMIT 1",
    )
    .fetch_optional(&mut database)
    .await?;
    database.close().await?;
    let boss = boss.ok_or("step=boss-key observed=none needed=enabled-boss-room-catalogue-row")?;

    let mut panel = BossDifficultyPanel {
        selected: true,
        boss_key: boss.clone(),
        ..Default::default()
    };
    let (context, opening) = match act(&mut client, &mut panel, Intent::Open, "open").await? {
        Reply::Window {
            context,
            boss_key,
            selected_difficulty,
            ..
        } if boss_key == boss => (context, selected_difficulty),
        other => return Err(format!("step=open observed={other:?}").into()),
    };

    let chosen: u16 = if opening == 5 { 4 } else { 5 };
    panel.difficulty = chosen;
    match act(&mut client, &mut panel, Intent::Preview, "preview").await? {
        Reply::Window {
            context: replied,
            selected_difficulty,
            ..
        } if replied == context && selected_difficulty == chosen => {}
        other => return Err(format!("step=preview observed={other:?}").into()),
    }
    if context_row(clients, context).await? != Some((Some(i32::from(chosen)), false)) {
        return Err(
            format!("step=preview observed=durable-context-not-difficulty-{chosen}-open").into(),
        );
    }

    let start = act(&mut client, &mut panel, Intent::Start, "start").await?;
    let refused = matches!(
        start,
        Reply::Refused {
            reason: Refusal::NoAcceptedProfile,
            ..
        }
    );
    if !refused {
        return Err(format!("step=start observed={start:?}").into());
    }

    // The refusal leaves the context usable, but the panel dropped it: restore the returned
    // context, cancel it and require the durable close.
    panel.pending = false;
    panel.context = Some(context);
    match act(&mut client, &mut panel, Intent::Cancel, "cancel").await? {
        Reply::Closed { context: closed } if closed == context => {}
        other => return Err(format!("step=cancel observed={other:?}").into()),
    }
    if context_row(clients, context).await?.map(|row| row.1) != Some(true) {
        return Err("step=cancel observed=durable-context-not-closed".into());
    }
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(Verdict::Fail(format!(
        "step=start;observed=NoAcceptedProfile;{START_REFUSAL};needed=accepted-profile-for-{boss}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_refusal_cites_the_durable_owner_line() {
        assert!(START_REFUSAL.starts_with("source:crates/gameplay/durability/"));
        assert!(!START_REFUSAL.contains(char::is_whitespace));
    }

    #[test]
    fn compact_is_one_token() {
        assert!(!compact("a b\nc").contains(char::is_whitespace));
    }
}

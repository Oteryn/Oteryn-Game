//! W05b daily world changes, derived from the world-change owner (`world-change/src/daily.rs`).
//!
//! The server rolls the day's changes at the daily save and stores them immutably in
//! `game_world_change_days`; no wire frame publishes them, so the row reads that durable decision
//! (read-only SQL) around real admissions:
//! 1. Admit the character, read the active reset's decision and check it names that reset epoch
//!    and carries every boolean flag of the day.
//! 2. Log out, obtain a fresh admission and read it again; an immutable decision must be identical.
//! 3. The day's player-visible effects (thawing ice flowers, trader, flower-month NPC) sit at fixed
//!    map positions outside this run's walk budget, so no effect can be observed and the row
//!    reports FAIL naming that precondition; it never edits state.
use super::*;
use sqlx::{Connection, Row};

const FLAGS: [&str; 6] = [
    "thawing",
    "their_masters_voice",
    "grimvale_full_moon",
    "flower_month",
    "stampede",
    "fire_from_the_earth",
];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(90), journey(clients)).await;
    let detail = match result {
        Ok(Ok(flags)) => format!(
            "needed:daily-change-effect-observable-by-a-player-in-walk-range(decision-durable-and-immutable,flags={flags})"
        ),
        Ok(Err(error)) => compact(&error.to_string()),
        Err(_) => "step=journey-observed=timeout".to_owned(),
    };
    evidence(&format!(
        "e2e_all scenario=W05b.full status=FAIL seconds={:.3} evidence={detail}",
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

/// One token naming which of the day's boolean flags are set; a missing flag is a malformed decision.
fn flags(decision: &serde_json::Value, epoch: &str) -> TestResult<String> {
    if decision["reset_epoch"]
        .as_i64()
        .map(|v| v.to_string())
        .as_deref()
        != Some(epoch)
    {
        return Err(format!(
            "step=decision observed=epoch-{}-not-{epoch}",
            decision["reset_epoch"]
        )
        .into());
    }
    let mut set = Vec::new();
    for name in FLAGS {
        match decision.get(name).and_then(serde_json::Value::as_bool) {
            Some(true) => set.push(name),
            Some(false) => {}
            // Rows written before a flag existed omit the defaulted ones.
            None if matches!(name, "stampede" | "fire_from_the_earth") => {}
            None => return Err(format!("step=decision observed=missing-flag-{name}").into()),
        }
    }
    Ok(if set.is_empty() {
        "none".to_owned()
    } else {
        set.join("+")
    })
}

async fn decision(clients: &SeamClients<'_>) -> TestResult<(String, String)> {
    let world = WorldId::decode(&clients.world)?;
    let mut database = sqlx::PgConnection::connect(clients.url).await?;
    let row = sqlx::query("SELECT d.reset_epoch::text AS epoch, d.decision::text AS decision FROM game_world_change_days d JOIN game_world_reset_active a ON a.world_id = d.world_id AND a.reset_epoch = d.reset_epoch WHERE d.world_id = encode($1,'hex')::uuid")
        .bind(world.as_bytes().as_slice())
        .fetch_optional(&mut database)
        .await?
        .ok_or("step=decision observed=no-change-day-row-for-the-active-reset")?;
    Ok((row.try_get("epoch")?, row.try_get("decision")?))
}

async fn logout(client: &mut RowClient) -> TestResult {
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = row_connect_world(clients, 0xe1).await?;
    client.service_liveness(Duration::from_millis(200)).await?;
    let (epoch, text) = decision(clients).await?;
    let parsed: serde_json::Value = serde_json::from_str(&text)?;
    let observed = flags(&parsed, &epoch)?;
    logout(&mut client).await?;
    let mut client = row_connect_world(clients, 0xe1).await?;
    if decision(clients).await? != (epoch, text) {
        return Err("step=fresh-admission observed=decision-changed".into());
    }
    logout(&mut client).await?;
    Ok(observed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_require_the_epoch_and_every_flag() -> TestResult {
        let day = serde_json::json!({"reset_epoch": 7, "thawing": true, "their_masters_voice": false,
            "grimvale_full_moon": false, "flower_month": true});
        assert_eq!(flags(&day, "7")?, "thawing+flower_month");
        assert!(flags(&day, "8").is_err());
        assert!(flags(&serde_json::json!({"reset_epoch": 7}), "7").is_err());
        Ok(())
    }

    #[test]
    fn compact_is_one_token() {
        assert_eq!(compact("a  b"), "a-b");
    }
}

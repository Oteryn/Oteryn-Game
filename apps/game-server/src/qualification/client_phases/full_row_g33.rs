//! G33: seasonal Cake and Santa (W05d). Both producers are gated by committed annual windows
//! in `content/encounters/event-schedule/schedule.json`: piece_of_cake Feb 21 to Feb 27 and
//! christmas Dec 12 to Dec 31 (end exclusive). The server evaluates them on the reset epoch's
//! calendar (`crates/gameplay/foundation/src/scheduled_events.rs`), i.e. the wall clock.
//! 1. Admit the node-boot character and log out (proves the run can reach the world).
//! 2. Outside both windows the mechanic is closed by content: SKIPPED with `content:`.
//!    Inside a window the Cake extraction spot and the Santa NPC dialogue need scripted
//!    placement that no player route provides in this run: FAIL with `needed:`.
use super::*;

/// Annual windows (month, day) start inclusive, end exclusive, from the committed schedule.
const WINDOWS: [((u32, u32), (u32, u32)); 2] = [((2, 21), (2, 27)), ((12, 12), (12, 31))];

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(60), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => detail,
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G33.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

/// Civil (month, day) of a unix time in UTC (days-from-civil inverse).
fn month_day(unix_secs: u64) -> (u32, u32) {
    let z = i64::try_from(unix_secs / 86_400).unwrap_or(0) + 719_468;
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = u32::try_from(doy - (153 * mp + 2) / 5 + 1).unwrap_or(1);
    let month = u32::try_from(if mp < 10 { mp + 3 } else { mp - 9 }).unwrap_or(1);
    (month, day)
}

fn in_window(date: (u32, u32)) -> bool {
    WINDOWS
        .iter()
        .any(|(start, end)| date >= *start && date < *end)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<(&'static str, String)> {
    let mut client = row_connect_world(clients, 0xe1).await?;
    client.service_liveness(Duration::from_millis(50)).await?;
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout;observed=refused".into());
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    let (month, day) = month_day(now);
    if in_window((month, day)) {
        Ok((
            "FAIL",
            "needed:scripted-cake-extraction-spot-and-santa-npc-placement".to_owned(),
        ))
    } else {
        Ok((
            "SKIPPED",
            format!("content:annual-windows-feb21-27-and-dec12-31-closed-at-{month:02}-{day:02}"),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn month_day_matches_known_dates() {
        assert_eq!(month_day(0), (1, 1));
        assert_eq!(month_day(1_773_000_000), (3, 8));
        assert_eq!(month_day(1_794_268_800), (11, 10));
    }

    #[test]
    fn windows_are_start_inclusive_end_exclusive() {
        assert!(in_window((2, 21)) && in_window((2, 26)) && !in_window((2, 27)));
        assert!(in_window((12, 12)) && in_window((12, 30)) && !in_window((12, 31)));
        assert!(!in_window((11, 10)));
    }
}

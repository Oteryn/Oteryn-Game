//! G33: seasonal Cake and Santa (W05d). Both producers are gated by committed annual windows
//! in `content/encounters/event-schedule/schedule.json`: piece_of_cake Feb 21 to Feb 27 and
//! christmas Dec 12 to Dec 31 (end exclusive). The server evaluates them on the reset epoch's
//! immutable Europe/Berlin activated_at decision, retained across restart and clock passage.
//! 1. Admit the node-boot character and log out (proves the run can reach the world).
//! 2. Read the active epoch's persisted seasonal keys through read-only evidence SQL.
//! 3. Only a verified closed server-active decision is content SKIPPED. Missing/unknown
//!    decisions and active seasons without scripted Cake/Santa placements remain FAIL.
use super::*;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(60), journey(clients)).await;
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

fn seasonal_verdict(events: &[String]) -> (&'static str, String) {
    if events
        .iter()
        .any(|key| matches!(key.as_str(), "piece_of_cake" | "christmas"))
    {
        (
            "FAIL",
            "needed:scripted-cake-extraction-spot-and-santa-npc-placement".to_owned(),
        )
    } else {
        (
            "SKIPPED",
            "content:server-active-seasonal-window-closed".to_owned(),
        )
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<(&'static str, String)> {
    let mut client = connect_world(clients, 0xe1).await?;
    client.service_liveness(Duration::from_millis(50)).await?;
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout;observed=refused".into());
    }
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let decision: Option<(String, i16, Vec<String>, i16, i16)> = sqlx::query_as(
        "SELECT a.reset_epoch::text,e.seasonal_calendar_revision,e.seasonal_event_keys, \
         extract(month FROM r.activated_at AT TIME ZONE 'Europe/Berlin')::smallint, \
         extract(day FROM r.activated_at AT TIME ZONE 'Europe/Berlin')::smallint \
         FROM game_world_reset_active a JOIN game_world_reset_epochs r USING(world_id,reset_epoch) \
         JOIN game_world_event_epochs e USING(world_id,reset_epoch) \
         WHERE a.world_id=encode($1,'hex')::uuid",
    )
    .bind(clients.world.as_slice())
    .fetch_optional(&mut connection)
    .await?;
    let (epoch, revision, events, month, day) =
        decision.ok_or("step=seasonal observed=missing-active-seasonal-decision")?;
    if revision != 1 {
        return Err(format!("step=seasonal observed=epoch-{epoch}-calendar-revision-{revision} needed=committed-seasonal-calendar-v1").into());
    }
    let month = u8::try_from(month)?;
    let day = u8::try_from(day)?;
    let expected =
        oteryn_gameplay_foundation::scheduled_events::content_schedule().seasonal_at(month, day);
    if events.iter().map(String::as_str).collect::<Vec<_>>() != expected {
        return Err(
            format!("step=seasonal observed=epoch-{epoch}-decision-calendar-mismatch").into(),
        );
    }
    let (status, detail) = seasonal_verdict(&events);
    Ok((
        status,
        format!("{detail};epoch={epoch};activated-date={month:02}-{day:02}"),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_epoch_seasonal_keys_control_verdict_instead_of_process_date() {
        for season in ["piece_of_cake", "christmas"] {
            assert_eq!(seasonal_verdict(&[season.into()]).0, "FAIL");
        }
        let closed = seasonal_verdict(&["halloween".into()]);
        assert_eq!(closed.0, "SKIPPED");
        assert!(
            closed
                .1
                .starts_with("content:server-active-seasonal-window-closed")
        );
        assert_eq!(seasonal_verdict(&[]).0, "SKIPPED");
    }

    #[test]
    fn windows_are_start_inclusive_end_exclusive() {
        let in_window = |(month, day)| {
            let events = oteryn_gameplay_foundation::scheduled_events::content_schedule()
                .seasonal_at(month, day)
                .into_iter()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            seasonal_verdict(&events).0 == "FAIL"
        };
        assert!(in_window((2, 21)) && in_window((2, 26)) && !in_window((2, 27)));
        assert!(in_window((12, 12)) && in_window((12, 30)) && !in_window((12, 31)));
        assert!(!in_window((11, 10)));
    }
}

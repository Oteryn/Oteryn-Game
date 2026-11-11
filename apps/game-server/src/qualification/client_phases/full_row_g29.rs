//! G29: quest tracker and filters, achievements, titles, defence stats, blessings.
//! Drives the Cyclopedia achievements, defence-stat (protection) and blessings requests through
//! the shared client panel against the real server and requires each reply to apply. The quest
//! tracker and titles need a quest the character has started and an earned title, which no
//! player route reaches at level 1 in this run, so the row ends FAIL `needed:started-quest...`.
use super::*;
use oteryn_dev_client::DevClientSession;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(120), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G29.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn logout(client: &mut RowClient) -> TestResult {
    if client.row_logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

use oteryn_dev_client::client_runtime::raw_feature;
use oteryn_dev_client::client_ui::cyclopedia_character as cyc;

async fn panel_call(
    client: &mut DevClientSession,
    panel: &mut cyc::CyclopediaCharacterPanel,
    intent: cyc::Intent,
) -> TestResult<bool> {
    let command = cyc::request(panel, intent).ok_or_else(|| {
        format!(
            "step=cyclopedia-request observed=local-refusal:{:?}",
            panel.feedback
        )
    })?;
    let reply = raw_feature::send(client.session_mut(), &command).await?;
    Ok(cyc::apply_result(panel, &reply))
}

fn require_cyclopedia(client: &DevClientSession) -> TestResult {
    if !client.selected_capabilities().contains(&199) {
        return Err("step=admit observed=capability-199-not-selected".into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = row_connect_world(clients, 0xd9).await?;
    require_cyclopedia(&client)?;
    let mut panel = cyc::CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    if !panel_call(&mut client, &mut panel, cyc::Intent::Achievements).await? {
        return Err("step=achievements observed=reply-not-applied".into());
    }
    if !panel_call(&mut client, &mut panel, cyc::Intent::Protection).await? {
        return Err("step=defence-stats observed=reply-not-applied".into());
    }
    let rows = cyc::combat_rows(&panel).len();
    let command =
        cyc::request_blessings(&mut panel).ok_or("step=blessings observed=local-refusal")?;
    let reply = raw_feature::send(client.session_mut(), &command).await?;
    if !cyc::apply_result(&mut panel, &reply) || panel.blessings.snapshot.is_none() {
        return Err("step=blessings observed=no-store-summary".into());
    }
    let owned = panel
        .blessings
        .snapshot
        .as_ref()
        .map_or(0, |store| store.blessings.len());
    logout(&mut client).await?;
    Err(format!("achievements-defence-rows:{rows}-blessings:{owned}-proven;step=quest-tracker observed=no-started-quest-for-the-character;needed:started-quest-and-title-route").into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_tokens_cover_empty_and_whitespace() {
        assert_eq!(token(""), "");
        assert!(!token("a b\tc\nd").chars().any(char::is_whitespace));
        assert_eq!(token("source:path.rs:12"), "source:path.rs:12");
    }
}

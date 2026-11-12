//! G30: Cyclopedia character misc skills and resource bars (cap 199).
//! Selects the Misc tab and requests the resources through the shared panel against the real
//! server, requires non-empty skill cards and the stamina (2520) and offline-training (720) bars
//! within range, then logs out, admits again and requires the cards and bars to stay consistent.
use super::*;
use oteryn_dev_client::DevClientSession;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(detail)) => ("PASS", token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G30.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn token(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("-")
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
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

async fn misc(
    client: &mut DevClientSession,
) -> TestResult<(Vec<(&'static str, String)>, [(u16, u16); 2], (u32, u32))> {
    let mut panel = cyc::CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    let tab = oteryn_dev_client::client_runtime::cyclopedia_character::CharacterTab::Misc;
    if !panel_call(client, &mut panel, cyc::Intent::SelectTab(tab)).await? {
        return Err("step=misc-tab observed=reply-not-applied".into());
    }
    if !panel_call(client, &mut panel, cyc::Intent::MiscResources).await? {
        return Err("step=misc-resources observed=reply-not-applied".into());
    }
    let cards = cyc::misc_cards(&panel);
    let bars = cyc::misc_bars(&panel).ok_or("step=misc-bars observed=none")?;
    if cards.is_empty() {
        return Err("step=misc-cards observed=empty".into());
    }
    // The bars clamp, so the range is judged on the raw server values.
    let raw = panel
        .misc_general
        .map(|stats| {
            (
                u32::from(stats.stamina_minutes),
                u32::from(stats.offline_training_minutes),
            )
        })
        .ok_or("step=misc-raw observed=no-general-stats")?;
    if raw.0 > 2520 || raw.1 > 720 {
        return Err(
            format!("step=misc-raw observed={raw:?};needed=stamina<=2520-offline<=720").into(),
        );
    }
    if bars[0].1 != 2520 || bars[1].1 != 720 || bars[0].0 > 2520 || bars[1].0 > 720 {
        return Err(
            format!("step=misc-bars observed={bars:?};needed=stamina<=2520-offline<=720").into(),
        );
    }
    Ok((cards, bars, raw))
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<String> {
    let mut client = connect_world(clients, 0xda).await?;
    require_cyclopedia(&client)?;
    let (cards, _, raw) = misc(&mut client).await?;
    logout(&mut client).await?;
    let mut fresh = connect_world(clients, 0xda).await?;
    let (again, bars_again, raw_again) = misc(&mut fresh).await?;
    logout(&mut fresh).await?;
    // Skill cards are durable character state; stamina may tick while the character is offline.
    if again != cards || raw_again.1 != raw.1 || raw_again.0 < raw.0 {
        return Err(format!(
            "step=fresh-admission observed=cards:{cards:?}->{again:?};raw:{raw:?}->{raw_again:?}"
        )
        .into());
    }
    Ok(format!(
        "misc-cards:{}-bars:{bars_again:?}-durable",
        again.len()
    ))
}

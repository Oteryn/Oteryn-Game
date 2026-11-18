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

fn display_matches(
    cards: &[(&str, String)],
    bars: [(u16, u16); 2],
    magic_level: u16,
    skills: [u16; 7],
    resources: (u16, u16),
) -> bool {
    let minutes = |value: u16| format!("{}:{:02}", value / 60, value % 60);
    let labels = ["Fist", "Club", "Sword", "Axe", "Distance", "Shielding", "Fishing"];
    let expected = [("Magic level", magic_level.to_string())]
        .into_iter().chain(labels.into_iter().zip(skills.map(|level| level.to_string())))
        .chain([("Stamina", minutes(resources.0)), ("Offline training", minutes(resources.1))]);
    bars == [(resources.0, 2520), (resources.1, 720)] && expected.eq(cards.iter().cloned())
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
    let stats = panel.misc_general.ok_or("step=misc-raw observed=no-general-stats")?;
    if !display_matches(&cards, bars, stats.magic_level, stats.skills, (stats.stamina_minutes, stats.offline_training_minutes)) {
        return Err(format!("step=misc-display observed=cards:{cards:?},bars:{bars:?};needed=raw-stats:{stats:?}").into());
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

#[cfg(test)]
mod display_tests {
    use super::*;

    #[test]
    fn misc_display_rejects_fabricated_card_and_bar_values() {
        let skills = [10, 11, 12, 13, 14, 15, 16];
        let cards = [
            ("Magic level", "7".into()), ("Fist", "10".into()), ("Club", "11".into()),
            ("Sword", "12".into()), ("Axe", "13".into()), ("Distance", "14".into()),
            ("Shielding", "15".into()), ("Fishing", "16".into()), ("Stamina", "1:15".into()),
            ("Offline training", "2:01".into()),
        ];
        assert!(!display_matches(&cards, [(0, 2520), (121, 720)], 7, skills, (75, 121)));
        assert!(!display_matches(&cards, [(75, 2520), (121, 720)], 9, skills, (75, 121)));
        assert!(display_matches(&cards, [(75, 2520), (121, 720)], 7, skills, (75, 121)));
    }
}

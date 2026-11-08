use super::CyclopediaCharacterPanel;
#[must_use]
pub fn panel_text(p: &CyclopediaCharacterPanel) -> String {
    let state = p
        .result
        .as_deref()
        .unwrap_or("No server character view yet");
    let heading = format!(
        "Cyclopedia character — {:?}, offset {}, limit {}",
        p.tab, p.offset, p.limit
    );
    match p.feedback {
        Some(f) => format!("{heading}\r\n{state}\r\n{f}"),
        None => format!("{heading}\r\n{state}"),
    }
}

/// Filter only the confirmed history page; the wire pagination remains unchanged.
pub fn history_rows(p: &CyclopediaCharacterPanel) -> Vec<&super::HistoryRow> {
    if p.history_tab != Some(p.tab) {
        return Vec::new();
    }
    p.history
        .iter()
        .filter(|row| {
            row.status.is_none() || p.kill_filter.is_none() || row.status == p.kill_filter
        })
        .collect()
}

pub fn overview_cards(p: &CyclopediaCharacterPanel) -> Vec<(&'static str, String)> {
    let Some(stats) = p.general.filter(|_| {
        p.tab == oteryn_client_runtime::cyclopedia_character::CharacterTab::GeneralStats
    }) else {
        return Vec::new();
    };
    let mut cards = vec![
        ("Level", stats.level.to_string()),
        ("Experience", stats.experience.to_string()),
        ("Magic level", stats.magic_level.to_string()),
        (
            "Health (last saved)",
            stats
                .last_vitals
                .map_or_else(|| "Unavailable".into(), |(v, _)| v.to_string()),
        ),
        (
            "Mana (last saved)",
            stats
                .last_vitals
                .map_or_else(|| "Unavailable".into(), |(_, v)| v.to_string()),
        ),
    ];
    for (label, level) in [
        "Fist",
        "Club",
        "Sword",
        "Axe",
        "Distance",
        "Shielding",
        "Fishing",
    ]
    .into_iter()
    .zip(stats.skills)
    {
        cards.push((label, level.to_string()));
    }
    cards.push((
        "Stamina",
        format!(
            "{}:{:02}",
            stats.stamina_minutes / 60,
            stats.stamina_minutes % 60
        ),
    ));
    cards.push((
        "Offline training",
        format!(
            "{}:{:02}",
            stats.offline_training_minutes / 60,
            stats.offline_training_minutes % 60
        ),
    ));
    cards
}
/// Native owners cap stamina at 2520 minutes and the offline training pool at 43200 seconds.
pub fn overview_bars(p: &CyclopediaCharacterPanel) -> Option<[(u16, u16); 2]> {
    let stats = p.general?;
    Some([
        (stats.stamina_minutes.min(2520), 2520),
        (stats.offline_training_minutes.min(720), 720),
    ])
}

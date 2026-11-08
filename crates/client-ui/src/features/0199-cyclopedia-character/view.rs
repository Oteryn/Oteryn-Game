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
    general_cards(stats)
}
fn general_cards(
    stats: oteryn_client_runtime::cyclopedia_character::GeneralStats,
) -> Vec<(&'static str, String)> {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombatRow {
    pub key: oteryn_client_runtime::cyclopedia_character::combat::CombatStat,
    pub label: String,
    pub value: String,
    pub source: &'static str,
}
fn stat_label(name: &str) -> String {
    let mut label = String::new();
    for (i, c) in name.chars().enumerate() {
        if i > 0 && c.is_uppercase() {
            label.push(' ');
        }
        label.push(c);
    }
    label
}
fn stat_value(
    key: oteryn_client_runtime::cyclopedia_character::combat::CombatStat,
    value: Option<i64>,
) -> String {
    use oteryn_client_runtime::cyclopedia_character::combat::CombatStatUnit;
    let Some(v) = value else {
        return "Unavailable".into();
    };
    match key.unit() {
        CombatStatUnit::PartsPerMillion => format!(
            "{}{}.{:04}%",
            if v < 0 { "-" } else { "" },
            v.unsigned_abs() / 10_000,
            v.unsigned_abs() % 10_000
        ),
        CombatStatUnit::Seconds => format!("{v} seconds"),
        _ => v.to_string(),
    }
}
pub fn combat_rows(p: &CyclopediaCharacterPanel) -> Vec<CombatRow> {
    use oteryn_client_runtime::cyclopedia_character::combat::CombatStat;
    let Some(stats) = p.combat.as_ref().filter(|s| s.tab() == p.tab) else {
        return Vec::new();
    };
    CombatStat::for_tab(p.tab)
        .iter()
        .map(|&key| {
            let name = format!("{key:?}");
            let source = if name.contains("Equipment") {
                "Equipment"
            } else if name.contains("Imbuement") {
                "Imbuement"
            } else if name.contains("Wheel") {
                "Wheel"
            } else if name.contains("Concoction") {
                "Consumable"
            } else if name.contains("FightMode") {
                "Combat stance"
            } else if name == "FlatDamageHealing" {
                "Level"
            } else if name.starts_with("PerfectShot") || name == "Cleave" || name == "AttackElement"
            {
                "Equipment"
            } else if name.starts_with("Converted") {
                "Conversion"
            } else if name.contains("Skill") {
                "Skill"
            } else if name.contains("Base") {
                "Base"
            } else {
                "Combined"
            };
            CombatRow {
                key,
                label: stat_label(&name),
                value: stat_value(key, stats.get(key)),
                source,
            }
        })
        .collect()
}
pub fn combat_detail(
    p: &CyclopediaCharacterPanel,
    key: oteryn_client_runtime::cyclopedia_character::combat::CombatStat,
) -> String {
    let Some(stats) = p.combat.as_ref().filter(|s| s.tab() == p.tab) else {
        return "No confirmed statistics".into();
    };
    let name = format!("{key:?}");
    let contribution_family = [
        "Equipment",
        "Imbuement",
        "Wheel",
        "Concoction",
        "Base",
        "Amplification",
        "Skill",
    ]
    .iter()
    .find_map(|suffix| name.strip_suffix(suffix))
    .unwrap_or(&name);
    let family = if name.starts_with("Attack") {
        "Attack"
    } else if name.starts_with("PerfectShot") {
        "PerfectShot"
    } else if name.starts_with("Accuracy") {
        "Accuracy"
    } else if name.starts_with("Converted") {
        "Converted"
    } else {
        contribution_family
    };
    let mut out = format!(
        "Observed at: {} (server Unix ms)\r\nCharacter revision: {} | Equipment revision: {}\r\n",
        stats.observed_at_unix_ms, stats.character_revision, stats.equipment_revision
    );
    if name.starts_with("Absorb") {
        out.push_str(
            "Positive resistance reduces incoming damage; negative resistance increases it.\r\n",
        );
    }
    for row in combat_rows(p)
        .into_iter()
        .filter(|r| format!("{:?}", r.key).starts_with(family) || (family=="Attack" && r.key==oteryn_client_runtime::cyclopedia_character::combat::CombatStat::FlatDamageHealing))
    {
        out.push_str(&format!(
            "{}: {} — {}\r\n",
            row.label, row.value, row.source
        ));
    }
    out
}

pub fn store_cards(p: &CyclopediaCharacterPanel) -> Vec<(&'static str, String)> {
    let Some(store) = p.store.as_ref().filter(|_| {
        p.tab == oteryn_client_runtime::cyclopedia_character::CharacterTab::StoreSummary
    }) else {
        return Vec::new();
    };
    vec![
        (
            "Permanent prey slots",
            store.permanent_prey_slots.to_string(),
        ),
        ("Prey wildcards", store.prey_wildcards.to_string()),
        ("Hirelings", store.hirelings.to_string()),
        ("Blessings", store.blessings.len().to_string()),
    ]
}
pub fn store_blessings(p: &CyclopediaCharacterPanel) -> String {
    p.store
        .as_ref()
        .map_or_else(String::new, |s| s.blessings.join("\r\n"))
}

pub fn misc_cards(p: &CyclopediaCharacterPanel) -> Vec<(&'static str, String)> {
    let Some(stats) = p
        .misc_general
        .filter(|_| p.tab == oteryn_client_runtime::cyclopedia_character::CharacterTab::Misc)
    else {
        return Vec::new();
    };
    general_cards(stats)
        .into_iter()
        .enumerate()
        .filter_map(|(i, card)| (i == 2 || i >= 5).then_some(card))
        .collect()
}
pub fn misc_bars(p: &CyclopediaCharacterPanel) -> Option<[(u16, u16); 2]> {
    let stats = p
        .misc_general
        .filter(|_| p.tab == oteryn_client_runtime::cyclopedia_character::CharacterTab::Misc)?;
    Some([
        (stats.stamina_minutes.min(2520), 2520),
        (stats.offline_training_minutes.min(720), 720),
    ])
}

/// Full catalogue only after the server has confirmed the account's earned badges.
pub fn badge_rows(p: &CyclopediaCharacterPanel) -> Vec<super::BadgeRow> {
    let Some(earned) = &p.badges else {
        return Vec::new();
    };
    use super::BadgeFilter;
    oteryn_client_runtime::cyclopedia_character::Badge::ALL
        .into_iter()
        .map(|badge| super::BadgeRow {
            id: badge.id(),
            name: badge.name(),
            earned: earned.contains(&badge.id()),
            hidden: p.hidden_badges.contains(&badge.id()),
        })
        .filter(|row| match p.badge_filter {
            BadgeFilter::All => true,
            BadgeFilter::Earned => row.earned,
            BadgeFilter::Locked => !row.earned,
            BadgeFilter::Hidden => row.hidden,
        })
        .collect()
}
pub fn badge_text(p: &CyclopediaCharacterPanel) -> String {
    let mut text = String::from("Badges — local display preferences\r\n");
    for row in badge_rows(p) {
        text.push_str(&format!(
            "#{} {} — {}{}\r\n",
            row.id,
            row.name,
            if row.earned { "earned" } else { "locked" },
            if row.hidden { "; hidden locally" } else { "" }
        ));
    }
    text
}

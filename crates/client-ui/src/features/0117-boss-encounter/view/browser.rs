use super::BossEncounterPanel;
use std::{collections::BTreeSet, fmt::Write};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BossCategory {
    #[default]
    All,
    Bane,
    Archfoe,
    Nemesis,
    Progress,
    Cooldown,
}
impl BossCategory {
    pub fn server_filter(self) -> u8 {
        match self {
            Self::Bane => 1,
            Self::Archfoe => 2,
            Self::Nemesis => 3,
            _ => 0,
        }
    }
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Bane,
            Self::Bane => Self::Archfoe,
            Self::Archfoe => Self::Nemesis,
            Self::Nemesis => Self::Progress,
            Self::Progress => Self::Cooldown,
            Self::Cooldown => Self::All,
        }
    }
}

/// Browse only identities on the confirmed server page, never inferred catalogue entries.
pub fn boss_records(panel: &BossEncounterPanel) -> Vec<&str> {
    if panel.catalogue.is_some() {
        return super::catalogue_view::records(panel);
    }
    let Some(snapshot) = &panel.snapshot else {
        return vec![];
    };
    let mut keys = BTreeSet::new();
    if panel.browser_category != BossCategory::Cooldown {
        keys.extend(snapshot.progress.iter().map(|(key, _)| key.as_str()));
    }
    if panel.browser_category != BossCategory::Progress {
        keys.extend(snapshot.cooldowns.iter().map(|(key, _)| key.as_str()));
    }
    let search = if panel.browser_search.is_empty() {
        &panel.boss_search
    } else {
        &panel.browser_search
    }
    .trim()
    .to_lowercase();
    keys.into_iter()
        .filter(|key| key.to_lowercase().contains(&search))
        .collect()
}

pub fn select_boss(panel: &mut BossEncounterPanel, forward: bool) -> bool {
    if !panel.selected || panel.pending {
        return false;
    }
    let rows = boss_records(panel);
    if rows.is_empty() {
        return false;
    }
    let index = rows.iter().position(|key| *key == panel.fields[0]);
    let next = match (index, forward) {
        (Some(i), true) => (i + 1) % rows.len(),
        (Some(i), false) => (i + rows.len() - 1) % rows.len(),
        (None, true) => 0,
        (None, false) => rows.len() - 1,
    };
    let key = rows[next].to_owned();
    panel.fields[0] = key;
    true
}

pub fn browser_text(panel: &BossEncounterPanel) -> String {
    let mut text = if panel.catalogue.is_some() {
        super::catalogue_view::text(panel)
    } else {
        legacy_browser_text(panel)
    };
    if let Some(oteryn_client_runtime::boss_difficulty::Reply::Window {
        personal_highest,
        unlocked_max,
        maximum_difficulty,
        ..
    }) = panel.difficulty_windows.get(&panel.fields[0])
        && *unlocked_max > 0
    {
        let _ = lwrite!(
            text,
            "\nPersonal highest: {personal_highest}\nUnlocked difficulty: {unlocked_max} / {maximum_difficulty}\n",
            personal_highest = personal_highest,
            unlocked_max = unlocked_max,
            maximum_difficulty = maximum_difficulty
        );
        if let Some(pity) = panel.pity_windows.get(&panel.fields[0]) {
            let _ = writeln!(text, "{}", crate::boss_difficulty_pity::tracker_text(pity));
        }
    }
    text
}

fn legacy_browser_text(panel: &BossEncounterPanel) -> String {
    let mut text = lfmt!("\nBosstiary browser — {:?}\n", panel.browser_category);
    let rows = boss_records(panel);
    for key in &rows {
        let _ = write!(
            text,
            "{} {key}",
            if *key == panel.fields[0] { ">" } else { " " }
        );
        if let Some(snapshot) = &panel.snapshot {
            if let Some((_, kills)) = snapshot.progress.iter().find(|(id, _)| id == key) {
                let _ = lwrite!(text, ": {kills} kills", kills = kills);
            }
            if let Some((_, deadline)) = snapshot.cooldowns.iter().find(|(id, _)| id == key) {
                let _ = lwrite!(
                    text,
                    " | {} ms remaining",
                    deadline.saturating_sub(snapshot.now)
                );
            }
        }
        text.push('\n');
    }
    if rows.is_empty() {
        text.push_str(tr!(
            "No boss records match this search on the current page\n"
        ));
    }
    if let Some(snapshot) = &panel.snapshot
        && rows.contains(&panel.fields[0].as_str())
    {
        let key = &panel.fields[0];
        let _ = lwriteln!(text, "\nBoss details: {key}", key = key);
        if let Some((_, kills)) = snapshot.progress.iter().find(|(id, _)| id == key) {
            let _ = lwriteln!(text, "Progress: {kills} kills", kills = kills);
        }
        if let Some((_, deadline)) = snapshot.cooldowns.iter().find(|(id, _)| id == key) {
            let _ = lwriteln!(
                text,
                "Cooldown: {} ms at snapshot time",
                deadline.saturating_sub(snapshot.now)
            );
        }
    }
    text.push_str(tr!("Previous/Next boss selects a record for details, entry and slot actions. Next page loads more server records.\n"));
    text
}

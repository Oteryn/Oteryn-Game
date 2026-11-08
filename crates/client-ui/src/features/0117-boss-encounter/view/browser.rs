use super::BossEncounterPanel;
use std::{collections::BTreeSet, fmt::Write};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BossCategory {
    #[default]
    All,
    Progress,
    Cooldown,
}
impl BossCategory {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Progress,
            Self::Progress => Self::Cooldown,
            Self::Cooldown => Self::All,
        }
    }
}

/// Browse only identities on the confirmed server page, never inferred catalogue entries.
pub fn boss_records(panel: &BossEncounterPanel) -> Vec<&str> {
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
    let mut text = format!("\nBosstiary browser — {:?}\n", panel.browser_category);
    let rows = boss_records(panel);
    for key in &rows {
        let _ = write!(
            text,
            "{} {key}",
            if *key == panel.fields[0] { ">" } else { " " }
        );
        if let Some(snapshot) = &panel.snapshot {
            if let Some((_, kills)) = snapshot.progress.iter().find(|(id, _)| id == key) {
                let _ = write!(text, ": {kills} kills");
            }
            if let Some((_, deadline)) = snapshot.cooldowns.iter().find(|(id, _)| id == key) {
                let _ = write!(
                    text,
                    " | {} ms remaining",
                    deadline.saturating_sub(snapshot.now)
                );
            }
        }
        text.push('\n');
    }
    if rows.is_empty() {
        text.push_str("No boss records match this search on the current page\n");
    }
    if let Some(snapshot) = &panel.snapshot
        && rows.contains(&panel.fields[0].as_str())
    {
        let key = &panel.fields[0];
        let _ = writeln!(text, "\nBoss details: {key}");
        if let Some((_, kills)) = snapshot.progress.iter().find(|(id, _)| id == key) {
            let _ = writeln!(text, "Progress: {kills} kills");
        }
        if let Some((_, deadline)) = snapshot.cooldowns.iter().find(|(id, _)| id == key) {
            let _ = writeln!(
                text,
                "Cooldown: {} ms at snapshot time",
                deadline.saturating_sub(snapshot.now)
            );
        }
    }
    text.push_str("Previous/Next boss selects a record for details, entry and slot actions. Next page loads more server records.\n");
    text
}

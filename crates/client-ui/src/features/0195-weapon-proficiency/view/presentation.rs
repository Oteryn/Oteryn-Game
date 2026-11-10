use super::WeaponProficiencyPanel;
use oteryn_client_runtime::weapon_proficiency::ProficiencyCatalogueEntry;

#[must_use]
pub fn mastery_target(entry: &ProficiencyCatalogueEntry) -> u32 {
    // The server's track shape grants mastery two levels after the last perk.
    entry.thresholds[(entry.levels.len() + 1).min(8)]
}
#[must_use]
pub fn progress_level(entry: &ProficiencyCatalogueEntry) -> usize {
    entry
        .thresholds
        .iter()
        .take(entry.levels.len() + 2)
        .take_while(|threshold| **threshold <= entry.experience)
        .count()
}
#[must_use]
pub fn progress_percent(entry: &ProficiencyCatalogueEntry) -> u32 {
    let target = mastery_target(entry);
    if target == 0 {
        return 0;
    }
    (u64::from(entry.experience.min(target)) * 100 / u64::from(target)) as u32
}
impl WeaponProficiencyPanel {
    /// Indices into the complete server catalogue, so filtering never changes identity.
    #[must_use]
    pub fn visible_weapons(&self) -> Vec<usize> {
        let Some(page) = &self.catalogue else {
            return Vec::new();
        };
        let search = self.search.to_lowercase();
        let mut rows: Vec<_> = page
            .entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| {
                entry.name.to_lowercase().contains(&search)
                    && match self.progress_filter {
                        1 => entry.experience > 0 && entry.experience < mastery_target(entry),
                        2 => entry.experience >= mastery_target(entry),
                        3 => entry.experience == 0,
                        _ => true,
                    }
            })
            .map(|(index, _)| index)
            .collect();
        rows.sort_by(|a, b| {
            let a = &page.entries[*a];
            let b = &page.entries[*b];
            let progress = if self.sort_progress {
                let ac = mastery_target(a).max(1);
                let bc = mastery_target(b).max(1);
                (u64::from(b.experience.min(bc)) * u64::from(ac))
                    .cmp(&(u64::from(a.experience.min(ac)) * u64::from(bc)))
            } else {
                std::cmp::Ordering::Equal
            };
            progress
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
                .then_with(|| a.weapon.cmp(&b.weapon))
        });
        rows
    }
    #[must_use]
    pub fn perk_tree_rows(&self) -> Vec<(String, Option<(usize, usize)>)> {
        let Some(entry) = self.weapon_entry().or_else(|| self.progress_entry()) else {
            return Vec::new();
        };
        let mut rows = Vec::new();
        for (level, data) in entry.levels.iter().enumerate() {
            let unlocked = entry.experience >= entry.thresholds[level];
            rows.push((
                lfmt!(
                    "Level {} — {} ({} XP)",
                    level + 1,
                    if unlocked {
                        tr!("Unlocked")
                    } else {
                        tr!("Locked")
                    },
                    entry.thresholds[level]
                ),
                None,
            ));
            for (perk, description) in data.perks.iter().enumerate() {
                rows.push((
                    format!(
                        "  └ {} {description}",
                        if data.selected == Some(perk as u8) {
                            "☑"
                        } else {
                            "☐"
                        }
                    ),
                    unlocked.then_some((level, perk)),
                ));
            }
        }
        rows
    }
    pub fn observe_progress(&mut self, definition: u32, experience: u32) {
        let Some(entry) = self.catalogue.as_mut().and_then(|page| {
            page.entries
                .iter_mut()
                .find(|entry| entry.definition == definition)
        }) else {
            return;
        };
        let previous = progress_level(entry);
        if experience > entry.experience {
            self.progress_weapon = definition;
        }
        entry.experience = experience;
        if progress_level(entry) > previous && !self.advancements.contains(&definition) {
            self.advancements.push(definition);
            self.progress_weapon = definition;
        }
    }
    #[must_use]
    pub fn progress_entry(&self) -> Option<&ProficiencyCatalogueEntry> {
        let entries = &self.catalogue.as_ref()?.entries;
        entries
            .iter()
            .find(|entry| entry.definition == self.progress_weapon)
            .or_else(|| entries.iter().find(|entry| entry.definition == self.weapon))
            .or_else(|| {
                entries
                    .iter()
                    .find(|entry| entry.experience > 0 && entry.definition != 0)
            })
            .or_else(|| entries.iter().find(|entry| entry.definition != 0))
    }
}

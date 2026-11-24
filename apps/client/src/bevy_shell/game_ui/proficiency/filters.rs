//! Catalogue controls keep original row indices while resolving public item requirements.
use super::*;
use oteryn_client_ui::inspection::{InspectedItem, Request};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Change {
    Class,
    Level,
    Vocation,
    Hands,
}
#[derive(Clone, Default)]
pub(super) struct Filters {
    class: usize,
    level: bool,
    vocation: bool,
    hands: usize,
    attempted: std::collections::BTreeMap<u32, std::time::Duration>,
    pub rows: Vec<usize>,
    details: std::collections::BTreeMap<u32, InspectedItem>,
    activity: Option<u64>,
    character_refresh: Option<std::time::Duration>,
}
const CLASSES: [&str; 7] = ["All", "Sword", "Axe", "Club", "Distance", "Wand", "Fist"];
const HANDS: [&str; 3] = ["All", "One", "Two"];
impl Filters {
    pub fn change(&mut self, change: Change) {
        match change {
            Change::Class => self.class = (self.class + 1) % CLASSES.len(),
            Change::Level => self.level = !self.level,
            Change::Vocation => self.vocation = !self.vocation,
            Change::Hands => self.hands = (self.hands + 1) % HANDS.len(),
        }
        self.attempted.clear();
    }
    pub fn controls(&self) -> [(Change, String); 4] {
        [
            (Change::Class, format!("Weapons: {}", CLASSES[self.class])),
            (Change::Hands, format!("Hands: {}", HANDS[self.hands])),
            (
                Change::Level,
                if self.level {
                    "All character levels"
                } else {
                    "Usable at my level"
                }
                .into(),
            ),
            (
                Change::Vocation,
                if self.vocation {
                    "All vocations"
                } else {
                    "Usable by my vocation"
                }
                .into(),
            ),
        ]
    }
    pub fn update(
        &mut self,
        link: &crate::play::PlayLink,
        panel: &WeaponProficiencyPanel,
        now: std::time::Duration,
    ) -> bool {
        let Some(catalogue) = &panel.catalogue else {
            return false;
        };
        let activity = Some(link.activity_generation());
        if self.activity != activity {
            self.activity = activity;
            self.details.clear();
            self.attempted.clear();
        }
        let rows = if self.class == 0 && self.hands == 0 && !self.level && !self.vocation {
            (0..catalogue.entries.len()).collect()
        } else {
            let (level, vocation) = link.weapon_filter_character();
            if ((self.level && level.is_none()) || (self.vocation && vocation.is_none()))
                && self.character_refresh.is_none_or(|last| {
                    now.saturating_sub(last) >= std::time::Duration::from_secs(10)
                })
            {
                link.refresh_skills_character();
                self.character_refresh = Some(now);
            }
            let mut next = None;
            let available = link.with_inspected_items(|details| {
                for item in &details.inspected_items {
                    if catalogue
                        .entries
                        .iter()
                        .any(|entry| entry.definition == item.definition_ref)
                        && self.details.get(&item.definition_ref) != Some(item)
                    {
                        self.attempted.remove(&item.definition_ref);
                        self.details.insert(item.definition_ref, item.clone());
                    }
                }
                !details.open && !details.catalogue_open && !details.pending
            });
            let rows = catalogue
                .entries
                .iter()
                .enumerate()
                .filter_map(|(index, entry)| {
                    if let Some(item) = self.details.get(&entry.definition) {
                        return self.matches(item, level, vocation).then_some(index);
                    }
                    let attempted = self.attempted.get(&entry.definition).copied();
                    if entry.definition != 0
                        && attempted.is_none_or(|last| {
                            now.saturating_sub(last) >= std::time::Duration::from_secs(10)
                        })
                        && next.is_none_or(|(prior, _)| attempted < prior)
                    {
                        next = Some((attempted, entry.definition));
                    }
                    None
                })
                .collect();
            if available
                && let Some((_, id)) = next
                && link.inspect(Request::Catalogue(id))
            {
                self.attempted.insert(id, now);
            }
            rows
        };
        if self.rows == rows {
            return false;
        }
        self.rows = rows;
        true
    }
    fn matches(&self, item: &InspectedItem, level: Option<u32>, vocation: Option<&str>) -> bool {
        let attr = |key: &str| {
            item.attributes
                .iter()
                .find(|(k, _)| k == key)
                .map(|(_, v)| v.as_str())
        };
        let weapon = attr("Weapon Type");
        if self.class != 0
            && !weapon
                .and_then(|w| w.split(',').next())
                .is_some_and(|w| w.trim().eq_ignore_ascii_case(CLASSES[self.class]))
        {
            return false;
        }
        if self.hands != 0
            && !weapon.is_some_and(|w| {
                w.split(',').any(|part| part.trim() == "two-handed") == (self.hands == 2)
            })
        {
            return false;
        }
        if self.level
            && !level.is_some_and(|level| {
                attr("Required Level").is_none_or(|v| v.parse::<u32>().is_ok_and(|n| n <= level))
            })
        {
            return false;
        }
        !self.vocation
            || vocation.is_some_and(|voc| {
                let base = voc.split_whitespace().last().unwrap_or(voc);
                let canonical = |name: &str| name.to_ascii_lowercase().replace([' ', '_', '-'], "");
                if let Some(restrictions) = attr("Professions") {
                    return restrictions.split(',').any(|allowed| {
                        let key = canonical(allowed.trim());
                        key == canonical(voc) || key == canonical(base)
                    });
                }
                true
            })
    }
}

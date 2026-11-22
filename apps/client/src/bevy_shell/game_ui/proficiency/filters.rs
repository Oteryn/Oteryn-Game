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
    attempted: std::collections::BTreeSet<u32>,
    pub rows: Vec<usize>,
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
    pub fn update(&mut self, link: &crate::play::PlayLink, panel: &WeaponProficiencyPanel) -> bool {
        let Some(catalogue) = &panel.catalogue else {
            return false;
        };
        let rows = if self.class == 0 && self.hands == 0 && !self.level && !self.vocation {
            (0..catalogue.entries.len()).collect()
        } else {
            let details = link.inspection_panel();
            let (level, vocation) = link.weapon_filter_character();
            let mut next = None;
            let rows = catalogue
                .entries
                .iter()
                .enumerate()
                .filter_map(|(index, e)| {
                    if let Some(item) = details.catalogue.get(&e.definition) {
                        return self.matches(item, level, vocation).then_some(index);
                    }
                    if e.definition != 0
                        && next.is_none()
                        && !self.attempted.contains(&e.definition)
                    {
                        next = Some(e.definition);
                    }
                    None
                })
                .collect();
            if !details.open
                && !details.catalogue_open
                && !details.pending
                && let Some(id) = next
                && link.inspect(Request::Catalogue(id))
            {
                self.attempted.insert(id);
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
                attr("Professions").is_none_or(|value| {
                    value.split(',').any(|v| {
                        voc.eq_ignore_ascii_case(v.trim())
                            || voc
                                .split_whitespace()
                                .last()
                                .is_some_and(|base| base.eq_ignore_ascii_case(v.trim()))
                    })
                })
            })
    }
}

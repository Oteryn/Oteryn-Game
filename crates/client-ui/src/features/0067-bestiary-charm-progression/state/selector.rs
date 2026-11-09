use super::Panel;
use oteryn_client_runtime::charm_progression::{catalogue::Card, charm::CharmKind};
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selector {
    pub open: bool,
    pub selected_race: Option<u32>,
    pub races: Vec<Card>,
    pub race_pending: Option<Vec<u16>>,
    pub race_failed: bool,
    pub attempted: bool,
    pub pending: Option<u8>,
    pub next: Option<u8>,
    pub cards: Vec<Card>,
}
impl Panel {
    pub fn charm_choices(&self) -> Vec<(u32, String)> {
        self.reply
            .iter()
            .flat_map(|r| &r.charms.charms)
            .map(|c| {
                let id = c.charm.get();
                (
                    id,
                    self.selector
                        .cards
                        .iter()
                        .find(|d| u32::from(d.race) == id)
                        .map_or_else(
                            || format!("Charm {id} (description unavailable)"),
                            |d| d.name.clone(),
                        ),
                )
            })
            .collect()
    }
    pub fn assignment_choices(&self) -> Vec<(u32, String)> {
        let Some(reply) = &self.reply else {
            return Vec::new();
        };
        let Some(charm) = reply
            .charms
            .charms
            .iter()
            .find(|c| c.charm.to_string() == self.fields[0])
        else {
            return Vec::new();
        };
        let stage = match charm.kind {
            CharmKind::Major => 3,
            CharmKind::Minor => 1,
        };
        reply
            .bestiary
            .iter()
            .filter(|r| r.unlocked_stage() >= stage)
            .map(|r| {
                let name = self
                    .selector
                    .races
                    .iter()
                    .chain(&self.catalogue.page.cards)
                    .find(|c| u32::from(c.race) == r.race.get())
                    .map_or_else(|| format!("Creature {}", r.race), |c| c.name.clone());
                (
                    r.race.get(),
                    format!(
                        "{name} — discovery stage {}, {} kills",
                        r.unlocked_stage(),
                        r.kill_count
                    ),
                )
            })
            .collect()
    }
    pub fn select_confirmed(&mut self, charm: u32, race: Option<u32>) -> bool {
        if !self.selected
            || self.pending
            || !self
                .reply
                .as_ref()
                .is_some_and(|r| r.charms.charms.iter().any(|c| c.charm.get() == charm))
        {
            return false;
        }
        let old = self.fields.clone();
        let old_race = self.selector.selected_race;
        if self.fields[0] != charm.to_string() {
            self.fields = [charm.to_string(), String::new()];
            self.selector.selected_race = None;
        }
        if let Some(race) = race {
            if !self.assignment_choices().iter().any(|(id, _)| *id == race) {
                self.fields = old;
                self.selector.selected_race = old_race;
                return false;
            }
            self.fields[1] = race.to_string();
            self.selector.selected_race = Some(race);
        }
        true
    }
    pub fn charm_balance_text(&self) -> String {
        self.reply.as_ref().map_or_else(
            || "Query to load charm balances".into(),
            |r| {
                format!(
                    "{} Charm Points | {} Minor Charm Echoes\r\nAssignment slots: {}",
                    r.charms.charm_points_available,
                    r.charms.minor_charm_echoes_available,
                    r.charms
                        .assignment_slot_limit
                        .map_or_else(|| "unlimited".into(), |n| n.to_string())
                )
            },
        )
    }
    pub fn selected_charm_text(&self) -> String {
        let Some(reply) = &self.reply else {
            return "Query to load charms and balances".into();
        };
        let Some(charm) = reply
            .charms
            .charms
            .iter()
            .find(|c| c.charm.to_string() == self.fields[0])
        else {
            return "Select a charm".into();
        };
        let description = self
            .selector
            .cards
            .iter()
            .find(|c| u32::from(c.race) == charm.charm.get())
            .map_or("Description unavailable", |c| c.detail.as_str());
        format!(
            "{}\r\nStage: {}/3 | Next stage cost: {} {}\r\nAssigned creature: {} | {}\r\n{}",
            self.charm_choices()
                .iter()
                .find(|(id, _)| *id == charm.charm.get())
                .map_or("Charm", |(_, n)| n.as_str()),
            charm.unlocked_stage,
            charm.next_stage_cost,
            match charm.kind {
                CharmKind::Major => "Charm Points",
                CharmKind::Minor => "Minor Charm Echoes",
            },
            charm
                .assigned_race
                .map_or_else(|| "none".into(), |r| r.to_string()),
            if charm.effect_active {
                "Effect active"
            } else {
                "Effect unavailable"
            },
            description
        )
    }
}

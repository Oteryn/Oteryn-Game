#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Query,
    Unlock,
    Assign,
    Remove,
    ResetAll,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub selector: super::Selector,
    pub catalogue: super::catalogue::Catalogue,
    pub race_filter: RaceFilter,
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub pending_kind: u32,
    pub fields: [String; 2],
    pub feedback: String,
    pub summary: String,
    pub reply: Option<oteryn_client_runtime::charm_progression::CharmProgressionReply>,
}
impl Panel {
    pub fn begin(&mut self, kind: u32) -> bool {
        if !self.selected {
            self.feedback = "Unavailable in this session".into();
            return false;
        }
        if self.pending {
            self.feedback = "Waiting for the server".into();
            return false;
        }
        self.pending = true;
        self.pending_kind = kind;
        self.feedback.clear();
        true
    }
    pub fn failed(&mut self, message: &str) {
        self.pending = false;
        self.feedback = message.into();
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RaceFilter {
    #[default]
    All,
    InProgress,
    Complete,
}
impl Panel {
    pub fn race_cards(
        &self,
    ) -> Vec<&oteryn_client_runtime::charm_progression::bestiary::BestiaryRaceProgress> {
        self.reply
            .iter()
            .flat_map(|r| &r.bestiary)
            .filter(|race| match self.race_filter {
                RaceFilter::All => true,
                RaceFilter::InProgress => !race.is_complete(),
                RaceFilter::Complete => race.is_complete(),
            })
            .collect()
    }
    pub fn select_race(&mut self, previous: bool) {
        if self.pending {
            return;
        }
        let cards = self.race_cards();
        if cards.is_empty() {
            self.fields[1].clear();
            return;
        }
        let current = self.fields[1].parse::<u32>().ok();
        let selected = cards.iter().position(|r| Some(r.race.get()) == current);
        let index = selected.map_or(0, |i| {
            if previous {
                (i + cards.len() - 1) % cards.len()
            } else {
                (i + 1) % cards.len()
            }
        });
        self.fields[1] = cards[index].race.to_string();
    }
}

impl Panel {
    pub fn select_charm(&mut self, previous: bool) {
        if self.pending {
            return;
        }
        let Some(reply) = &self.reply else {
            return;
        };
        let cards = &reply.charms.charms;
        if cards.is_empty() {
            self.fields[0].clear();
            self.fields[1].clear();
            return;
        }
        let current = self.fields[0].parse::<u32>().ok();
        let selected = cards.iter().position(|c| Some(c.charm.get()) == current);
        let index = selected.map_or(0, |i| {
            if previous {
                (i + cards.len() - 1) % cards.len()
            } else {
                (i + 1) % cards.len()
            }
        });
        self.fields[0] = cards[index].charm.to_string();
        self.fields[1].clear();
    }
    pub fn select_assignment_race(&mut self, previous: bool) {
        use oteryn_client_runtime::charm_progression::charm::CharmKind;
        if self.pending {
            return;
        }
        let Some(reply) = &self.reply else {
            return;
        };
        let Some(charm) = reply
            .charms
            .charms
            .iter()
            .find(|c| self.fields[0] == c.charm.to_string())
        else {
            return;
        };
        let stage = match charm.kind {
            CharmKind::Major => crate::bestiary_charms::MAJOR_CHARM_RACE_STAGE,
            CharmKind::Minor => crate::bestiary_charms::MINOR_CHARM_RACE_STAGE,
        };
        let cards: Vec<_> = reply
            .bestiary
            .iter()
            .filter(|r| r.unlocked_stage() >= stage)
            .collect();
        if cards.is_empty() {
            self.fields[1].clear();
            return;
        }
        let current = self.fields[1].parse::<u32>().ok();
        let selected = cards.iter().position(|r| Some(r.race.get()) == current);
        let index = selected.map_or(0, |i| {
            if previous {
                (i + cards.len() - 1) % cards.len()
            } else {
                (i + 1) % cards.len()
            }
        });
        self.fields[1] = cards[index].race.to_string();
    }
}

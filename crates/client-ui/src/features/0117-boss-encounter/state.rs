use oteryn_client_runtime::boss_encounter::Snapshot;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BossEncounterPanel {
    pub catalogue: Option<oteryn_client_runtime::boss_encounter::catalogue::Page>,
    pub catalogue_query: Option<oteryn_client_runtime::boss_encounter::catalogue::Query>,
    pub pending_catalogue: Option<oteryn_client_runtime::boss_encounter::catalogue::Query>,
    pub gallery_open: bool,
    pub selected: bool,
    pub open: bool,
    pub cooldowns_open: bool,
    pub cooldown_received_at: Option<std::time::Instant>,
    pub pending: bool,
    pub pending_kind: u32,
    pub slots: Option<oteryn_client_runtime::boss_encounter::bosstiary_slots::BossSlotReply>,
    pub boss_search: String,
    pub fields: [String; 3],
    pub browser_search: String,
    pub browser_category: super::BossCategory,
    pub summary: String,
    pub hazard: Option<(u8, u8)>,
    pub rewards: Option<oteryn_client_runtime::boss_encounter::RewardChest>,
    pub snapshot: Option<Snapshot>,
    pub feedback: Option<&'static str>,
}

impl BossEncounterPanel {
    pub fn select_boss(&mut self, previous: bool) {
        if self.pending {
            return;
        }
        let cards = super::boss_records(self);
        if cards.is_empty() {
            self.fields[0].clear();
            return;
        }
        let index = cards.iter().position(|key| *key == self.fields[0]);
        let index = index.map_or(0, |i| {
            if previous {
                (i + cards.len() - 1) % cards.len()
            } else {
                (i + 1) % cards.len()
            }
        });
        self.fields[0] = cards[index].to_owned();
    }
}

impl BossEncounterPanel {
    pub fn select_slot(&mut self) {
        if self.pending {
            return;
        }
        let Some(slots) = &self.slots else {
            return;
        };
        let current = self.fields[1].parse::<u8>().unwrap_or(0);
        let order = if current == 1 { [2, 1] } else { [1, 2] };
        self.fields[1] = order
            .into_iter()
            .find(|slot| slots.unlocked & (1 << (slot - 1)) != 0)
            .map_or_else(String::new, |slot| slot.to_string());
    }
}

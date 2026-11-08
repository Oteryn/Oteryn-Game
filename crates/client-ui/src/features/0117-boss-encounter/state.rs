use oteryn_client_runtime::boss_encounter::Snapshot;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BossEncounterPanel {
    pub selected: bool,
    pub open: bool,
    pub cooldowns_open: bool,
    pub cooldown_received_at: Option<std::time::Instant>,
    pub pending: bool,
    pub pending_kind: u32,
    pub boss_search: String,
    pub fields: [String; 3],
    pub summary: String,
    pub rewards: Option<oteryn_client_runtime::boss_encounter::RewardChest>,
    pub snapshot: Option<Snapshot>,
    pub feedback: Option<&'static str>,
}

impl BossEncounterPanel {
    pub fn visible_bosses(&self) -> impl Iterator<Item = &(String, u64)> {
        let search = self.boss_search.to_lowercase();
        self.snapshot
            .iter()
            .flat_map(|s| &s.progress)
            .filter(move |(key, _)| key.to_lowercase().contains(&search))
    }
    pub fn select_boss(&mut self, previous: bool) {
        if self.pending {
            return;
        }
        let cards: Vec<_> = self.visible_bosses().collect();
        if cards.is_empty() {
            self.fields[0].clear();
            return;
        }
        let index = cards.iter().position(|(key, _)| key == &self.fields[0]);
        let index = index.map_or(0, |i| {
            if previous {
                (i + cards.len() - 1) % cards.len()
            } else {
                (i + 1) % cards.len()
            }
        });
        self.fields[0] = cards[index].0.clone();
    }
}

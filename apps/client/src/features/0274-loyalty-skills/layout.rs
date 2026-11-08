//! Configurable skills projection. Values always come from admitted server replies.
use oteryn_client_ui::{loyalty_skills::LoyaltySkillsPanel, skill_progress::SkillProgressPanel};
use serde::{Deserialize, Serialize};

pub(crate) const FAMILIES: [&str; 8] = [
    "Magic",
    "Fist",
    "Club",
    "Sword",
    "Axe",
    "Distance",
    "Shielding",
    "Fishing",
];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SkillsNumbers {
    #[default]
    All,
    Trained,
    Loyalty,
    Current,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SkillsLayout {
    pub order: Vec<u8>,
    pub numbers: SkillsNumbers,
    pub progress: bool,
}

impl Default for SkillsLayout {
    fn default() -> Self {
        Self {
            order: (0..8).collect(),
            numbers: SkillsNumbers::All,
            progress: true,
        }
    }
}

impl SkillsLayout {
    pub fn valid(&self) -> bool {
        self.order.len() <= 8
            && self
                .order
                .iter()
                .enumerate()
                .all(|(i, family)| *family < 8 && !self.order[..i].contains(family))
    }

    pub fn project(
        &self,
        progress: &SkillProgressPanel,
        loyalty: &LoyaltySkillsPanel,
    ) -> SkillsWidget {
        let complete = progress.complete.as_ref();
        let classic = loyalty.snapshot.as_ref();
        let mut header = complete.map_or_else(
            || if progress.selected || loyalty.selected { "Trained skills".into() } else { "Skills unavailable in this session".into() },
            |full| format!("Level {} — Experience {} — Level progress {}.{:02}%\r\nLoyalty skill bonus: {}%", full.level, full.experience, full.level_percent / 100, full.level_percent % 100, full.loyalty_bonus_percent),
        );
        if let Some(snapshot) = classic {
            header.push_str(&format!(
                "\r\nLoyalty: {} points — {}% {}",
                snapshot.points,
                snapshot.bonus_percent,
                snapshot.title.as_deref().unwrap_or("")
            ));
        }
        if let Some(error) = progress.error.or(loyalty.error) {
            header.push_str(&format!("\r\n{error}"));
        } else if progress.pending || loyalty.pending {
            header.push_str("\r\nUpdating…");
        }
        let rows = self
            .order
            .iter()
            .filter_map(|&family| {
                let index = usize::from(family);
                let trained = progress.snapshot.as_ref().and_then(|s| s.skills.get(index));
                let old = classic.and_then(|s| s.skills.get(index));
                Some(SkillsRow {
                    family,
                    trained: trained.map(|s| s.level).or_else(|| old.map(|s| s.base))?,
                    loyalty: complete
                        .and_then(|s| s.loyalty_levels.get(index))
                        .copied()
                        .or_else(|| old.map(|s| s.with_loyalty)),
                    current: complete
                        .and_then(|s| s.current_levels.as_ref())
                        .and_then(|levels| levels.get(index))
                        .copied(),
                    progress: trained.filter(|s| s.required > 0).map(|s| s.percent),
                    counts: trained.map(|s| (s.progress, s.required)),
                })
            })
            .collect();
        SkillsWidget {
            header,
            level_progress: complete.map(|s| s.level_percent),
            rows,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillsRow {
    pub family: u8,
    pub trained: u16,
    pub loyalty: Option<u16>,
    pub current: Option<u32>,
    pub progress: Option<u16>,
    pub counts: Option<(u64, u64)>,
}

impl SkillsRow {
    pub fn text(&self, layout: &SkillsLayout) -> String {
        let loyalty = self
            .loyalty
            .map_or_else(|| "unavailable".into(), |n| n.to_string());
        let current = self
            .current
            .map_or_else(|| "unavailable".into(), |n| n.to_string());
        let values = match layout.numbers {
            SkillsNumbers::All => format!(
                "trained {} / loyalty {loyalty} / current {current}",
                self.trained
            ),
            SkillsNumbers::Trained => format!("trained {}", self.trained),
            SkillsNumbers::Loyalty => format!("loyalty {loyalty}"),
            SkillsNumbers::Current => format!("current {current}"),
        };
        let label = FAMILIES
            .get(usize::from(self.family))
            .copied()
            .unwrap_or("Skill");
        let mut text = format!("{label}: {values}");
        if layout.progress {
            if let Some(percent) = self.progress {
                text.push_str(&format!(" — {}.{:02}%", percent / 100, percent % 100));
            }
            if let Some((count, required)) = self.counts {
                if required == 0 {
                    text.push_str(" — No next level");
                } else {
                    text.push_str(&format!(" ({count}/{required})"));
                }
            }
        }
        text
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillsWidget {
    pub header: String,
    pub level_progress: Option<u16>,
    pub rows: Vec<SkillsRow>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SkillsLayoutState {
    pub preferences: SkillsLayout,
    pub dirty: bool,
    pub edit: u64,
    pub active: u8,
}

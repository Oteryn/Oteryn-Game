//! Local analyser report settings.
use super::Tool;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Section {
    DpsGauge,
    DpsGraph,
    OutgoingTypes,
    HpsGauge,
    HpsGraph,
    InputGraph,
    InputTypes,
    InputSources,
    LootGauge,
    LootGraph,
    SupplyGauge,
    SupplyGraph,
    XpGauge,
    XpGraph,
    RawXp,
}
impl Section {
    pub const CONTROLS: &[(Tool, Self, &str)] = &[
        (Tool::Impact, Self::DpsGauge, "DPS gauge"),
        (Tool::Impact, Self::DpsGraph, "DPS graph"),
        (Tool::Impact, Self::OutgoingTypes, "Outgoing damage types"),
        (Tool::Impact, Self::HpsGauge, "HPS gauge"),
        (Tool::Impact, Self::HpsGraph, "HPS graph"),
        (Tool::Input, Self::InputGraph, "Incoming damage graph"),
        (Tool::Input, Self::InputTypes, "Incoming damage types"),
        (Tool::Input, Self::InputSources, "Incoming damage sources"),
        (Tool::Loot, Self::LootGauge, "Loot rate gauge"),
        (Tool::Loot, Self::LootGraph, "Loot rate graph"),
        (Tool::Supply, Self::SupplyGauge, "Supply rate gauge"),
        (Tool::Supply, Self::SupplyGraph, "Supply rate graph"),
        (Tool::Xp, Self::XpGauge, "XP rate gauge"),
        (Tool::Xp, Self::XpGraph, "XP rate graph"),
        (Tool::Xp, Self::RawXp, "Raw XP"),
    ];
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rate {
    Dps,
    Hps,
    Loot,
    Supply,
    Xp,
}
impl Rate {
    pub const CONTROLS: &[(Tool, Self, &str)] = &[
        (Tool::Impact, Self::Dps, "Set DPS target"),
        (Tool::Impact, Self::Hps, "Set HPS target"),
        (Tool::Loot, Self::Loot, "Set loot/h target"),
        (Tool::Supply, Self::Supply, "Set supply/h target"),
        (Tool::Xp, Self::Xp, "Set XP/h target"),
    ];
}
#[derive(Debug, Clone, Default)]
pub struct Preferences {
    pub boss_search: String,
    pub boss_name_order: bool,
    pub hidden: BTreeSet<Section>,
    pub targets: BTreeMap<Rate, u64>,
    pub impact_session: bool,
    pub input_session: bool,
}
impl Preferences {
    pub fn visible(&self, section: Section) -> bool {
        !self.hidden.contains(&section)
    }
    pub fn gauge(&self, rate: Rate, label: &str, current: u128) -> String {
        let target = self.targets.get(&rate).copied().unwrap_or(1);
        let filled = (current.saturating_mul(12) / u128::from(target)).min(12) as usize;
        format!(
            "{label} target: {target}  [{}{}]",
            "█".repeat(filled),
            "░".repeat(12 - filled)
        )
    }
}

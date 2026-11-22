//! Independent measurement intervals; catalogue, prices and preferences stay on the panel.
use super::{ItemStats, Panel, Series, measurements};
#[derive(Debug, Clone, Default)]
pub struct Report {
    pub recent: [measurements::Recent; 3],
    pub kill_chart: Series,
    pub xp_chart: Series,
    pub raw_xp_chart: Series,
    pub gold_chart: Series,
    pub damage_chart: Series,
    pub healing_chart: Series,
    pub received_chart: Series,
    pub input_types: std::collections::BTreeMap<u32, u64>,
    pub input_sources: std::collections::BTreeMap<(u8, u32), u64>,
    pub impact_types: std::collections::BTreeMap<u32, u64>,
    pub loot: std::collections::BTreeMap<u32, ItemStats>,
    pub loot_drops: std::collections::VecDeque<super::super::tracking::Drop>,
    pub omitted_drops: u64,
    pub supply: std::collections::BTreeMap<u32, ItemStats>,
    pub omitted_supplies: u64,
    pub omitted_items: u64,
    pub kills: u64,
    pub kills_by_race: std::collections::BTreeMap<u32, u64>,
    pub raw_xp: u64,
    pub gained_xp: u64,
    pub gold: u64,
    pub loot_items: u64,
    pub supplies: u64,
    pub damage: u64,
    pub healing: u64,
    pub received: u64,
    pub dropped: u64,
}
impl Report {
    pub(crate) fn swap(&mut self, panel: &mut Panel) {
        std::mem::swap(&mut self.recent, &mut panel.recent);
        std::mem::swap(&mut self.kill_chart, &mut panel.kill_chart);
        std::mem::swap(&mut self.xp_chart, &mut panel.xp_chart);
        std::mem::swap(&mut self.raw_xp_chart, &mut panel.raw_xp_chart);
        std::mem::swap(&mut self.gold_chart, &mut panel.gold_chart);
        std::mem::swap(&mut self.damage_chart, &mut panel.damage_chart);
        std::mem::swap(&mut self.healing_chart, &mut panel.healing_chart);
        std::mem::swap(&mut self.received_chart, &mut panel.received_chart);
        std::mem::swap(&mut self.input_types, &mut panel.input_types);
        std::mem::swap(&mut self.input_sources, &mut panel.input_sources);
        std::mem::swap(&mut self.impact_types, &mut panel.impact_types);
        std::mem::swap(&mut self.loot, &mut panel.loot);
        std::mem::swap(&mut self.loot_drops, &mut panel.loot_drops);
        std::mem::swap(&mut self.omitted_drops, &mut panel.omitted_drops);
        std::mem::swap(&mut self.supply, &mut panel.supply);
        std::mem::swap(&mut self.omitted_supplies, &mut panel.omitted_supplies);
        std::mem::swap(&mut self.omitted_items, &mut panel.omitted_items);
        std::mem::swap(&mut self.kills, &mut panel.kills);
        std::mem::swap(&mut self.kills_by_race, &mut panel.kills_by_race);
        std::mem::swap(&mut self.raw_xp, &mut panel.raw_xp);
        std::mem::swap(&mut self.gained_xp, &mut panel.gained_xp);
        std::mem::swap(&mut self.gold, &mut panel.gold);
        std::mem::swap(&mut self.loot_items, &mut panel.loot_items);
        std::mem::swap(&mut self.supplies, &mut panel.supplies);
        std::mem::swap(&mut self.damage, &mut panel.damage);
        std::mem::swap(&mut self.healing, &mut panel.healing);
        std::mem::swap(&mut self.received, &mut panel.received);
        std::mem::swap(&mut self.dropped, &mut panel.dropped);
    }
}

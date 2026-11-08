use super::Panel;
pub use oteryn_client_runtime::analyser::{AnalyserFact, AnalyserImpact};
/// Count kills only from Kill, never from loot fragments (analyser-emission-v1.md:7-13).
pub fn apply(panel: &mut Panel, facts: &[AnalyserFact]) {
    if !panel.selected {
        return;
    }
    for fact in facts {
        match fact {
            AnalyserFact::Kill { .. } => panel.kills = panel.kills.saturating_add(1),
            AnalyserFact::Experience { raw, gained } => {
                panel.raw_xp = panel.raw_xp.saturating_add(*raw);
                panel.gained_xp = panel.gained_xp.saturating_add(*gained);
            }
            AnalyserFact::KillLoot {
                corpse_items, gold, ..
            } => {
                panel.gold = panel.gold.saturating_add(*gold);
                for item in corpse_items {
                    panel.loot_items = panel.loot_items.saturating_add(u64::from(item.count.get()));
                }
            }
            AnalyserFact::SupplyUsed(item) => {
                panel.supplies = panel.supplies.saturating_add(u64::from(item.count.get()))
            }
            AnalyserFact::Impact(AnalyserImpact::Damage { value, .. }) => {
                panel.damage = panel.damage.saturating_add(u64::from(value.get()))
            }
            AnalyserFact::Impact(AnalyserImpact::Healing(value)) => {
                panel.healing = panel.healing.saturating_add(u64::from(value.get()))
            }
            AnalyserFact::DamageInput { value, .. } => {
                panel.received = panel.received.saturating_add(u64::from(value.get()))
            }
            AnalyserFact::Dropped(count) => {
                panel.dropped = panel.dropped.saturating_add(u64::from(count.get()))
            }
        }
    }
}
pub fn reset(panel: &mut Panel) {
    *panel = Panel {
        selected: panel.selected,
        open: panel.open,
        tool: panel.tool,
        ..Panel::default()
    };
}

use super::Panel;
pub use oteryn_client_runtime::analyser::{AnalyserFact, AnalyserImpact};
/// Count kills only from Kill, never from loot fragments (analyser-emission-v1.md:7-13).
pub fn apply(panel: &mut Panel, facts: &[AnalyserFact]) {
    apply_at(panel, facts, panel.clock.seconds());
}
/// Shared reducer with an explicit monotonic time for deterministic replay.
pub fn apply_at(panel: &mut Panel, facts: &[AnalyserFact], seconds: u64) {
    if !panel.selected {
        return;
    }
    for fact in facts {
        match fact {
            AnalyserFact::Kill { .. } => panel.kills = panel.kills.saturating_add(1),
            AnalyserFact::Experience { raw, gained } => {
                panel.xp_chart.add(seconds, *gained);
                panel.raw_xp_chart.add(seconds, *raw);
                panel.raw_xp = panel.raw_xp.saturating_add(*raw);
                panel.gained_xp = panel.gained_xp.saturating_add(*gained);
            }
            AnalyserFact::KillLoot {
                corpse_items, gold, ..
            } => {
                panel.gold = panel.gold.saturating_add(*gold);
                panel.gold_chart.add(seconds, *gold);
                for item in corpse_items {
                    if !super::state::items::record_item(
                        &mut panel.loot,
                        item.item_type.get(),
                        u64::from(item.count.get()),
                        seconds,
                    ) {
                        panel.omitted_items = panel
                            .omitted_items
                            .saturating_add(u64::from(item.count.get()));
                    }
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
    let remaining = panel
        .goal_xp
        .map(|goal| goal.saturating_sub(panel.gained_xp));
    *panel = Panel {
        tool: panel.tool,
        loot_prices: std::mem::take(&mut panel.loot_prices),
        item_info: std::mem::take(&mut panel.item_info),
        price_input: std::mem::take(&mut panel.price_input),
        goal_xp: remaining,
        goal_input: remaining.map_or_else(String::new, |value| value.to_string()),
        selected: panel.selected,
        open: panel.open,
        ..Panel::default()
    };
}

/// Fetch bounded identity pages only for items the confirmed session has observed.
pub fn catalogue_request(
    panel: &mut Panel,
) -> Option<oteryn_client_runtime::raw_feature::FeatureCommand> {
    use oteryn_client_runtime::analyser::{
        COMMAND_TYPE_ANALYSER_ITEM_QUERY, MAX_ITEMS, encode_request,
    };
    if !panel.selected || !panel.catalogue_pending.is_empty() {
        return None;
    }
    let remaining = super::state::items::MAX_ITEM_DETAILS.saturating_sub(panel.item_info.len());
    if remaining == 0 {
        panel.feedback = Some("Item identity cache is full; unresolved values remain excluded");
        return None;
    }
    let ids: Vec<_> = panel
        .loot
        .keys()
        .filter(|id| !panel.item_info.contains_key(id))
        .take(MAX_ITEMS.min(remaining))
        .copied()
        .collect();
    if ids.is_empty() {
        panel.feedback = Some("Item identities are current");
        return None;
    }
    let payload = encode_request(&ids).ok()?;
    panel.catalogue_pending = ids;
    Some(oteryn_client_runtime::raw_feature::FeatureCommand {
        capability: 10,
        kind: COMMAND_TYPE_ANALYSER_ITEM_QUERY,
        payload,
    })
}
pub fn catalogue_result(
    panel: &mut Panel,
    result: &oteryn_client_runtime::raw_feature::GatedCommandResult,
) -> bool {
    use oteryn_client_runtime::{CommandStatus, analyser::decode_reply};
    let requested = std::mem::take(&mut panel.catalogue_pending);
    if requested.is_empty() {
        return false;
    }
    if matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) && let Ok(rows) = decode_reply(&result.payload)
        && rows.iter().map(|row| row.item_type).eq(requested)
    {
        for row in rows {
            panel.item_info.insert(row.item_type, row);
        }
        panel.feedback = Some("Item identities received; resolve again if more items remain");
        return true;
    }
    panel.feedback = Some("Item identities unavailable; unresolved values remain excluded");
    false
}

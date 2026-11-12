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
            AnalyserFact::Kill { race } => {
                panel.kill_chart.add(seconds, 1);
                panel.kills = panel.kills.saturating_add(1);
                let count = panel
                    .kills_by_race
                    .entry(race.map_or(0, |race| race.get()))
                    .or_default();
                *count = count.saturating_add(1);
            }
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
                    super::tracking::record(
                        panel,
                        item.item_type.get(),
                        u64::from(item.count.get()),
                        seconds,
                    );
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
                let count = u64::from(item.count.get());
                panel.supplies = panel.supplies.saturating_add(count);
                if !super::state::items::record_item(
                    &mut panel.supply,
                    item.item_type.get(),
                    count,
                    seconds,
                ) {
                    panel.omitted_supplies = panel.omitted_supplies.saturating_add(count);
                }
            }
            AnalyserFact::Impact(AnalyserImpact::Damage { value, element }) => {
                let value = u64::from(value.get());
                panel.damage = panel.damage.saturating_add(value);
                panel.damage_chart.add(seconds, value);
                let total = panel.impact_types.entry(element.wire()).or_default();
                *total = total.saturating_add(value);
            }
            AnalyserFact::Impact(AnalyserImpact::Healing(value)) => {
                let value = u64::from(value.get());
                panel.healing = panel.healing.saturating_add(value);
                panel.healing_chart.add(seconds, value);
            }
            AnalyserFact::DamageInput {
                value,
                element,
                source,
            } => {
                use oteryn_client_runtime::analyser::AnalyserDamageSource;
                let value = u64::from(value.get());
                panel.received = panel.received.saturating_add(value);
                panel.received_chart.add(seconds, value);
                let total = panel.input_types.entry(element.wire()).or_default();
                *total = total.saturating_add(value);
                let key = match source {
                    AnalyserDamageSource::Creature(race) => (1, race.map_or(0, |race| race.get())),
                    AnalyserDamageSource::Player => (2, 0),
                    AnalyserDamageSource::None => (3, 0),
                };
                let total = panel.input_sources.entry(key).or_default();
                *total = total.saturating_add(value);
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
        // Party counters use server connection intervals, independent of this local reset.
        party: std::mem::take(&mut panel.party),
        party_adjustments: std::mem::take(&mut panel.party_adjustments),
        party_import: panel.party_import.take(),
        party_pending: panel.party_pending,
        party_feedback: panel.party_feedback,
        party_revision: panel.party_revision,
        party_pending_revision: panel.party_pending_revision,
        kill_goals: std::mem::take(&mut panel.kill_goals),
        task_progress: std::mem::take(&mut panel.task_progress),
        task_feedback: panel.task_feedback,
        loot_tracked: std::mem::take(&mut panel.loot_tracked),
        loot_only_tracked: panel.loot_only_tracked,
        export_path: std::mem::take(&mut panel.export_path),
        loot_prices: std::mem::take(&mut panel.loot_prices),
        supply_prices: std::mem::take(&mut panel.supply_prices),
        item_info: std::mem::take(&mut panel.item_info),
        price_input: std::mem::take(&mut panel.price_input),
        goal_xp: remaining,
        goal_input: remaining.map_or_else(String::new, |value| value.to_string()),
        selected: panel.selected,
        open: panel.open,
        ..Panel::default()
    };
}

/// A replaced server snapshot cannot establish that old item references still name the same definitions.
pub fn recover(panel: &mut Panel) {
    *panel = Panel {
        tool: panel.tool,
        selected: panel.selected,
        open: panel.open,
        feedback: Some(tr!(
            "Server state replaced; a fresh measurement interval and item settings were started"
        )),
        ..Panel::default()
    };
}

/// Fetch bounded identity pages for observed items and explicit tracker selections.
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
        panel.feedback = Some(tr!(
            "Item identity cache is full; unresolved values remain excluded"
        ));
        return None;
    }
    let mut ids = Vec::new();
    for id in panel
        .loot
        .keys()
        .chain(panel.supply.keys())
        .chain(panel.loot_tracked.iter())
    {
        if !panel.item_info.contains_key(id) && !ids.contains(id) {
            ids.push(*id);
        }
        if ids.len() == MAX_ITEMS.min(remaining) {
            break;
        }
    }
    if ids.is_empty() {
        panel.feedback = Some(tr!("Item identities are current"));
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
        panel.feedback = Some(tr!(
            "Item identities received; resolve again if more items remain"
        ));
        return true;
    }
    panel.feedback = Some(tr!(
        "Item identities unavailable; unresolved values remain excluded"
    ));
    false
}

/// Build the bounded confirmed-party query.
pub fn party_request(
    panel: &mut Panel,
) -> Option<oteryn_client_runtime::raw_feature::FeatureCommand> {
    super::party::request(panel)
}

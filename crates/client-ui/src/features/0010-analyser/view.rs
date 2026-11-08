use super::Panel;
#[must_use]
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return "Analysers unavailable in this session".into();
    }
    let mut text = match panel.tool {
        super::Tool::Overview => format!("Overview\r\n{}", overview(panel)),
        super::Tool::Hunting => format!("Hunting analyser\r\nKills: {}", panel.kills),
        tool @ (super::Tool::Damage | super::Tool::Healing) => {
            outgoing_text(panel, panel.clock.seconds(), tool)
        }
        super::Tool::Xp => xp_text(panel, panel.clock.seconds()),
        super::Tool::Loot => loot_text(panel, panel.clock.seconds()),
        super::Tool::Supply => supply_text(panel, panel.clock.seconds()),
        super::Tool::Impact => impact_text(panel, panel.clock.seconds()),
        super::Tool::Input => input_text(panel, panel.clock.seconds()),
        super::Tool::Session => session_text(panel),
        super::Tool::Party => super::party::text(panel),
        super::Tool::LootTracker => super::tracking::text(panel, panel.clock.seconds()),
        super::Tool::Kills => super::kills::text(panel),
        super::Tool::Bosses => "Boss Tracker\r\nRefresh cooldowns and personal drop records".into(),
    };
    if panel.dropped != 0 {
        text.push_str(&format!(
            "\r\nSession feed incomplete: {} fact(s) dropped",
            panel.dropped
        ));
    }
    text
}
fn overview(panel: &Panel) -> String {
    format!(
        "Kills: {}\r\nExperience: {} raw / {} gained\r\nLoot: {} items / {} gold\r\nSupplies used: {}\r\nDamage dealt: {} | Healing: {} | Damage received: {}\r\nDropped facts: {} (totals may be incomplete)",
        panel.kills,
        panel.raw_xp,
        panel.gained_xp,
        panel.loot_items,
        panel.gold,
        panel.supplies,
        panel.damage,
        panel.healing,
        panel.received,
        panel.dropped
    )
}

pub(super) fn xp_text(panel: &Panel, seconds: u64) -> String {
    use super::state::{eta_seconds, per_hour};
    let remaining = panel
        .goal_xp
        .map(|goal| goal.saturating_sub(panel.gained_xp));
    let eta = remaining.and_then(|value| eta_seconds(value, panel.gained_xp, seconds));
    let goal = remaining.map_or_else(
        || "Set the XP still needed for your level goal".into(),
        |value| format!("Level goal remaining: {value} XP"),
    );
    let eta = eta.map_or_else(
        || "Collect experience and set a level goal to estimate time".into(),
        |value| format!("Estimated time to level goal: {value} seconds"),
    );
    format!(
        "XP Analyser\r\nElapsed: {seconds} seconds\r\nExperience: {} raw / {} gained\r\nXP/h: {}\r\nRaw XP/h: {}\r\n{goal}\r\n{eta}\r\nGained XP\r\n{}\r\nRaw XP\r\n{}\r\n{}",
        panel.raw_xp,
        panel.gained_xp,
        per_hour(panel.gained_xp, seconds),
        per_hour(panel.raw_xp, seconds),
        panel.xp_chart.chart(seconds),
        panel.raw_xp_chart.chart(seconds),
        panel.feedback.unwrap_or("")
    )
}

pub(super) fn loot_text(panel: &Panel, seconds: u64) -> String {
    use super::state::{chart_values, per_hour};
    use std::fmt::Write;
    let unpriced = panel
        .loot
        .iter()
        .filter(|(item, _)| {
            panel
                .item_info
                .get(item)
                .is_none_or(|info| info.coin_worth == 0 && !panel.loot_prices.contains_key(item))
        })
        .fold(0u64, |total, (_, row)| total.saturating_add(row.count));
    let mut text = format!(
        "Loot Analyser\r\nGold: {} | Looted items: {}\r\nValued loot: {} (gold + priced items)\r\nLoot value/h: {}\r\nUnpriced items: {unpriced} | Unlisted items: {}\r\n{}\r\n",
        panel.gold,
        panel.loot_items,
        panel.loot_value(),
        per_hour(panel.loot_value(), seconds),
        panel.omitted_items,
        chart_values(&panel.loot_values(seconds))
    );
    for (item, row) in &panel.loot {
        let price = panel.item_info.get(item).map_or_else(
            || "identity unresolved; resolve item info".into(),
            |info| {
                if info.coin_worth != 0 {
                    "currency (already included in gold)".into()
                } else {
                    panel.loot_prices.get(item).map_or_else(
                        || "unpriced".into(),
                        |price| {
                            format!("unit {price} | value {}", row.count.saturating_mul(*price))
                        },
                    )
                }
            },
        );
        let _ = writeln!(
            text,
            "Item {item}: {} | {}/h | {price} | {}\r",
            row.count,
            per_hour(row.count, seconds),
            panel
                .item_info
                .get(item)
                .map_or("", |info| info.name.as_str())
        );
    }
    if let Some(feedback) = panel.feedback {
        text.push_str(feedback);
    }
    text
}

pub(super) fn supply_text(panel: &Panel, seconds: u64) -> String {
    use super::state::{chart_values, per_hour};
    use std::fmt::Write;
    let unpriced = panel
        .supply
        .iter()
        .filter(|(item, _)| panel.supply_unit_cost(**item).is_none())
        .fold(0u64, |total, (_, row)| total.saturating_add(row.count));
    let mut text = format!(
        "Supply Analyser\r\nSupplies used: {}\r\nSupply cost: {} (priced entries)\r\nSupply cost/h: {}\r\nUnpriced supply units: {unpriced} | Unlisted units: {}\r\n{}\r\n",
        panel.supplies,
        panel.supply_cost(),
        per_hour(panel.supply_cost(), seconds),
        panel.omitted_supplies,
        chart_values(&panel.supply_values(seconds))
    );
    for (item, row) in &panel.supply {
        let price = panel.supply_unit_cost(*item).map_or_else(
            || "unpriced or unresolved identity".into(),
            |price| format!("unit {price} | cost {}", row.count.saturating_mul(price)),
        );
        let _ = writeln!(
            text,
            "Item {item}: {} | {}/h | {price} | {}\r",
            row.count,
            per_hour(row.count, seconds),
            panel
                .item_info
                .get(item)
                .map_or("", |info| info.name.as_str())
        );
    }
    if let Some(feedback) = panel.feedback {
        text.push_str(feedback);
    }
    text
}

/// Integer hundredths avoid rounding away large confirmed totals.
fn rate(value: u64, seconds: u64) -> String {
    let hundredths = u128::from(value) * 100 / u128::from(seconds.max(1));
    if seconds == 0 {
        return "0.00".into();
    }
    format!("{}.{:02}", hundredths / 100, hundredths % 100)
}
fn element_name(code: u32) -> &'static str {
    match code {
        1 => "Physical",
        2 => "Fire",
        3 => "Earth",
        4 => "Energy",
        5 => "Ice",
        6 => "Holy",
        7 => "Death",
        8 => "Life drain",
        9 => "Mana drain",
        10 => "Drowning",
        _ => "Untyped",
    }
}
fn gauge(value: u64, total: u128) -> String {
    let share = u128::from(value) * 10000 / total.max(1);
    let bars = "█".repeat((share / 500) as usize);
    format!("{bars} {}.{:02}%", share / 100, share % 100)
}
pub(super) fn outgoing_text(panel: &Panel, seconds: u64, tool: super::Tool) -> String {
    use std::fmt::Write;
    let (mut text, value, label, chart) = if tool == super::Tool::Healing {
        (
            format!("Healing analyser\r\nHealing: {}", panel.healing),
            panel.healing,
            "HPS",
            &panel.healing_chart,
        )
    } else {
        (
            format!(
                "Damage analyser\r\nDamage dealt: {} | Damage received: {}",
                panel.damage, panel.received
            ),
            panel.damage,
            "DPS",
            &panel.damage_chart,
        )
    };
    let _ = write!(
        text,
        "\r\nElapsed: {seconds} seconds\r\n{label}: {}\r\n{} per minute\r\n{}",
        rate(value, seconds),
        tool.title(),
        chart.chart(seconds)
    );
    if value == 0 {
        text.push_str("\r\nNo confirmed measurements yet");
    }
    if tool == super::Tool::Damage {
        for (element, amount) in &panel.impact_types {
            let _ = write!(
                text,
                "\r\n{}: {amount} {}",
                element_name(*element),
                gauge(*amount, u128::from(value))
            );
        }
    }
    text
}
pub(super) fn impact_text(panel: &Panel, seconds: u64) -> String {
    use std::fmt::Write;
    let total = u128::from(panel.damage) + u128::from(panel.healing);
    let mut text = format!(
        "Impact Analyser\r\nElapsed: {seconds} seconds\r\nDPS: {} | HPS: {}\r\nDamage dealt: {} {}\r\nHealing given: {} {}\r\nDamage per minute\r\n{}\r\nHealing per minute\r\n{}",
        rate(panel.damage, seconds),
        rate(panel.healing, seconds),
        panel.damage,
        gauge(panel.damage, total),
        panel.healing,
        gauge(panel.healing, total),
        panel.damage_chart.chart(seconds),
        panel.healing_chart.chart(seconds)
    );
    if total == 0 {
        text.push_str("\r\nNo impact yet");
    }
    for (element, value) in &panel.impact_types {
        let _ = write!(
            text,
            "\r\n{}: {value} {}",
            element_name(*element),
            gauge(*value, total)
        );
    }
    text
}

pub(super) fn input_text(panel: &Panel, seconds: u64) -> String {
    use std::fmt::Write;
    let total = u128::from(panel.received);
    let mut text = format!(
        "Input Analyser\r\nElapsed: {seconds} seconds\r\nDamage received: {}\r\nIncoming DPS: {}\r\n{}",
        panel.received,
        rate(panel.received, seconds),
        panel.received_chart.chart(seconds)
    );
    if total == 0 {
        text.push_str("\r\nNo damage received yet");
    }
    text.push_str("\r\nDamage types");
    for (element, value) in &panel.input_types {
        let _ = write!(
            text,
            "\r\n{}: {value} {}",
            element_name(*element),
            gauge(*value, total)
        );
    }
    text.push_str("\r\nSources");
    for ((kind, race), value) in &panel.input_sources {
        let name = match (kind, race) {
            (1, 0) => "Creature (race unavailable)".into(),
            (1, race) => format!("Creature race #{race}"),
            (2, _) => "Player".into(),
            _ => "Unattributed".into(),
        };
        let _ = write!(text, "\r\n{name}: {value} {}", gauge(*value, total));
    }
    text
}

/// A report independent of which analyser tab is currently selected.
pub fn session_text(panel: &Panel) -> String {
    session_text_at(panel, panel.clock.seconds())
}
pub(super) fn session_text_at(panel: &Panel, seconds: u64) -> String {
    use std::fmt::Write;
    let balance = i128::from(panel.loot_value()) - i128::from(panel.supply_cost());
    let balance_rate = if seconds == 0 {
        0
    } else {
        balance * 3600 / i128::from(seconds)
    };
    let mut text = format!(
        "Hunting Session\r\nElapsed: {seconds} seconds\r\nXP/h: {}\r\nRaw XP/h: {}\r\nPriced balance: {balance} gold\r\nPriced balance/h: {balance_rate} gold\r\nOnly resolved, priced items and confirmed gold enter the balance.\r\n{}\r\nKills by creature race",
        super::state::per_hour(panel.gained_xp, seconds),
        super::state::per_hour(panel.raw_xp, seconds),
        overview(panel)
    );
    for (race, count) in &panel.kills_by_race {
        if *race == 0 {
            let _ = write!(text, "\r\nRace unavailable: {count}");
        } else {
            let _ = write!(text, "\r\nRace #{race}: {count}");
        }
    }
    let _ = write!(
        text,
        "\r\n\r\n{}\r\n\r\n{}\r\nSession gained XP per minute\r\n{}\r\nSession raw XP per minute\r\n{}",
        loot_text(panel, seconds),
        supply_text(panel, seconds),
        panel.xp_chart.chart(seconds),
        panel.raw_xp_chart.chart(seconds)
    );
    text
}

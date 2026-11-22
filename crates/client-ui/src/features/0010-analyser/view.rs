#[path = "view/experience.rs"]
mod experience;
use super::{
    Panel,
    display::{Rate, Section},
};
pub(super) use experience::xp_text;
#[must_use]
pub fn panel_text(panel: &Panel) -> String {
    if !panel.selected {
        return tr!("Analysers unavailable in this session").into();
    }
    let mut projected;
    let panel = if let Some(report) = panel.reports.get(panel.tool.key()) {
        projected = panel.clone();
        report.clone().swap(&mut projected);
        &projected
    } else {
        panel
    };
    let mut text = match panel.tool {
        super::Tool::Overview => lfmt!("Overview\r\n{}", overview(panel)),
        super::Tool::Hunting => format!(
            "Hunting analyser\r\nKills: {}\r\nKills/h: {}\r\n{}",
            panel.kills,
            super::state::per_hour(
                panel.kills,
                report_elapsed(panel, panel.clock.seconds(), super::Tool::Hunting)
            ),
            report_chart(
                panel,
                &panel.kill_chart,
                panel.clock.seconds(),
                super::Tool::Hunting
            )
        ),
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
        super::Tool::Bosses => {
            tr!("Boss Tracker\r\nRefresh cooldowns and personal drop records").into()
        }
    };
    if panel.dropped != 0 {
        text.push_str(&lfmt!(
            "\r\nSession feed incomplete: {} fact(s) dropped",
            panel.dropped
        ));
    }
    text
}
fn report_elapsed(panel: &Panel, seconds: u64, tool: super::Tool) -> u64 {
    let index = super::Tool::ALL
        .iter()
        .position(|candidate| *candidate == tool)
        .unwrap_or(0);
    seconds.saturating_sub(panel.reset_at[index])
}
pub(super) fn report_chart(
    panel: &Panel,
    series: &super::state::Series,
    seconds: u64,
    tool: super::Tool,
) -> String {
    interval_chart(
        &series.values(seconds),
        seconds,
        report_elapsed(panel, seconds, tool),
    )
}
fn interval_chart(values: &[u64], seconds: u64, elapsed: u64) -> String {
    let start_minute = seconds.saturating_sub(elapsed) / 60;
    let count = usize::try_from(seconds / 60 - start_minute + 1)
        .unwrap_or(usize::MAX)
        .min(values.len());
    super::state::chart_values(&values[values.len() - count..])
}
fn overview(panel: &Panel) -> String {
    lfmt!(
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

pub(super) fn loot_text(panel: &Panel, seconds: u64) -> String {
    loot_text_at(
        panel,
        seconds,
        report_elapsed(panel, seconds, super::Tool::Loot),
        false,
    )
}
fn loot_text_at(panel: &Panel, seconds: u64, elapsed: u64, consolidated: bool) -> String {
    use super::state::per_hour;
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
    let mut text = lfmt!(
        "Loot Analyser\r\nGold: {} | Looted items: {}\r\nValued loot: {} (gold + priced items)\r\nLoot value/h: {}\r\nUnpriced items: {unpriced} | Unlisted items: {}\r\n{}\r\n",
        panel.gold,
        panel.loot_items,
        panel.loot_value(),
        per_hour(panel.loot_value(), elapsed),
        panel.omitted_items,
        if consolidated || panel.display.visible(Section::LootGraph) {
            interval_chart(&panel.loot_values(seconds), seconds, elapsed)
        } else {
            String::new()
        },
        unpriced = unpriced
    );
    if !consolidated && panel.display.visible(Section::LootGauge) {
        let _ = writeln!(
            text,
            "{}",
            panel.display.gauge(
                Rate::Loot,
                "Loot/h",
                per_hour(panel.loot_value(), elapsed).into()
            )
        );
    }
    for (item, row) in &panel.loot {
        let price = panel.item_info.get(item).map_or_else(
            || tr!("identity unresolved; resolve item info").into(),
            |info| {
                if info.coin_worth != 0 {
                    tr!("currency (already included in gold)").into()
                } else {
                    panel.loot_prices.get(item).map_or_else(
                        || "unpriced".into(),
                        |price| {
                            lfmt!(
                                "unit {price} | value {}",
                                row.count.saturating_mul(*price),
                                price = price
                            )
                        },
                    )
                }
            },
        );
        let _ = lwriteln!(
            text,
            "Item {item}: {} | {}/h | {price} | {}\r",
            row.count,
            per_hour(row.count, elapsed),
            panel
                .item_info
                .get(item)
                .map_or("", |info| info.name.as_str()),
            item = item,
            price = price
        );
    }
    if let Some(feedback) = panel.feedback {
        text.push_str(feedback);
    }
    text
}

pub(super) fn supply_text(panel: &Panel, seconds: u64) -> String {
    supply_text_at(
        panel,
        seconds,
        report_elapsed(panel, seconds, super::Tool::Supply),
        false,
    )
}
fn supply_text_at(panel: &Panel, seconds: u64, elapsed: u64, consolidated: bool) -> String {
    use super::state::per_hour;
    use std::fmt::Write;
    let unpriced = panel
        .supply
        .iter()
        .filter(|(item, _)| panel.supply_unit_cost(**item).is_none())
        .fold(0u64, |total, (_, row)| total.saturating_add(row.count));
    let mut text = lfmt!(
        "Supply Analyser\r\nSupplies used: {}\r\nSupply cost: {} (priced entries)\r\nSupply cost/h: {}\r\nUnpriced supply units: {unpriced} | Unlisted units: {}\r\n{}\r\n",
        panel.supplies,
        panel.supply_cost(),
        per_hour(panel.supply_cost(), elapsed),
        panel.omitted_supplies,
        if consolidated || panel.display.visible(Section::SupplyGraph) {
            interval_chart(&panel.supply_values(seconds), seconds, elapsed)
        } else {
            String::new()
        },
        unpriced = unpriced
    );
    if !consolidated && panel.display.visible(Section::SupplyGauge) {
        let _ = writeln!(
            text,
            "{}",
            panel.display.gauge(
                Rate::Supply,
                "Supply/h",
                per_hour(panel.supply_cost(), elapsed).into()
            )
        );
    }
    for (item, row) in &panel.supply {
        let price = panel.supply_unit_cost(*item).map_or_else(
            || tr!("unpriced or unresolved identity").into(),
            |price| {
                lfmt!(
                    "unit {price} | cost {}",
                    row.count.saturating_mul(price),
                    price = price
                )
            },
        );
        let _ = lwriteln!(
            text,
            "Item {item}: {} | {}/h | {price} | {}\r",
            row.count,
            per_hour(row.count, elapsed),
            panel
                .item_info
                .get(item)
                .map_or("", |info| info.name.as_str()),
            item = item,
            price = price
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
        1 => tr!("Physical"),
        2 => tr!("Fire"),
        3 => tr!("Earth"),
        4 => tr!("Energy"),
        5 => tr!("Ice"),
        6 => tr!("Holy"),
        7 => tr!("Death"),
        8 => tr!("Life drain"),
        9 => tr!("Mana drain"),
        10 => tr!("Drowning"),
        _ => tr!("Untyped"),
    }
}
fn gauge(value: u64, total: u128) -> String {
    let share = u128::from(value) * 10000 / total.max(1);
    let bars = "█".repeat((share / 500) as usize);
    format!("{bars} {}.{:02}%", share / 100, share % 100)
}
pub(super) fn outgoing_text(panel: &Panel, seconds: u64, tool: super::Tool) -> String {
    let elapsed = report_elapsed(panel, seconds, tool);
    use std::fmt::Write;
    let (mut text, value, label, chart) = if tool == super::Tool::Healing {
        (
            lfmt!("Healing analyser\r\nHealing: {}", panel.healing),
            panel.healing,
            "HPS",
            &panel.healing_chart,
        )
    } else {
        (
            lfmt!(
                "Damage analyser\r\nDamage dealt: {} | Damage received: {}",
                panel.damage,
                panel.received
            ),
            panel.damage,
            "DPS",
            &panel.damage_chart,
        )
    };
    let _ = write!(
        text,
        "\r\nElapsed: {elapsed} seconds\r\n{label}: {}\r\n{} per minute\r\n{}",
        panel.recent[usize::from(tool == super::Tool::Healing)].rate(seconds, 10),
        tool.title(),
        interval_chart(&chart.values(seconds), seconds, elapsed)
    );
    text.push_str(&format!(
        "\r\nSession average {label}: {}",
        rate(value, elapsed)
    ));
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
    let elapsed = report_elapsed(panel, seconds, super::Tool::Impact);
    use std::fmt::Write;
    let total = u128::from(panel.damage) + u128::from(panel.healing);
    let rates = if panel.display.impact_session {
        [rate(panel.damage, elapsed), rate(panel.healing, elapsed)]
    } else {
        [
            panel.recent[0].rate(seconds, 10),
            panel.recent[1].rate(seconds, 10),
        ]
    };
    let mut text = lfmt!(
        "Impact Analyser\r\nElapsed: {seconds} seconds\r\nDPS: {} | HPS: {}\r\nDamage dealt: {} {}\r\nHealing given: {} {}",
        rates[0],
        rates[1],
        panel.damage,
        gauge(panel.damage, total),
        panel.healing,
        gauge(panel.healing, total),
        seconds = elapsed
    );
    for (index, label, setting, gauge_section, graph_section, series, graph_label) in [
        (
            0,
            "DPS",
            Rate::Dps,
            Section::DpsGauge,
            Section::DpsGraph,
            &panel.damage_chart,
            tr!("Damage per minute"),
        ),
        (
            1,
            "HPS",
            Rate::Hps,
            Section::HpsGauge,
            Section::HpsGraph,
            &panel.healing_chart,
            tr!("Healing per minute"),
        ),
    ] {
        let _ = write!(
            text,
            "\r\nAll-time high {label}: {}",
            panel.impact_high[index]
        );
        if panel.display.visible(gauge_section) {
            let current = rates[index]
                .split('.')
                .next()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0);
            let _ = write!(text, "\r\n{}", panel.display.gauge(setting, label, current));
        }
        if panel.display.visible(graph_section) {
            let _ = write!(
                text,
                "\r\n{graph_label}\r\n{}",
                interval_chart(&series.values(seconds), seconds, elapsed)
            );
        }
    }
    let _ = write!(
        text,
        "\r\nRate mode: {}",
        if panel.display.impact_session {
            "session"
        } else {
            "current"
        }
    );
    let _ = write!(
        text,
        "\r\nSession average DPS: {} | HPS: {}",
        rate(panel.damage, elapsed),
        rate(panel.healing, elapsed)
    );
    if total == 0 {
        text.push_str(tr!("\r\nNo impact yet"));
    }
    if !panel.display.visible(Section::OutgoingTypes) {
        return text;
    }
    text.push_str("\r\nDamage types");
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
    let elapsed = report_elapsed(panel, seconds, super::Tool::Input);
    use std::fmt::Write;
    let total = u128::from(panel.received);
    let mut text = lfmt!(
        "Input Analyser\r\nElapsed: {seconds} seconds\r\nDamage received: {}\r\nIncoming DPS: {}\r\n{}",
        panel.received,
        if panel.display.input_session {
            rate(panel.received, elapsed)
        } else {
            panel.recent[2].rate(seconds, 3)
        },
        if panel.display.visible(Section::InputGraph) {
            interval_chart(&panel.received_chart.values(seconds), seconds, elapsed)
        } else {
            String::new()
        },
        seconds = elapsed
    );
    let _ = write!(
        text,
        "\r\nRate mode: {}",
        if panel.display.input_session {
            "session"
        } else {
            "current"
        }
    );
    text.push_str(&format!(
        "\r\nSession average Incoming DPS: {}",
        rate(panel.received, elapsed)
    ));
    if total == 0 {
        text.push_str(tr!("\r\nNo damage received yet"));
    }
    if panel.display.visible(Section::InputTypes) {
        text.push_str(tr!("\r\nDamage types"));
        for (element, value) in &panel.input_types {
            let _ = write!(
                text,
                "\r\n{}: {value} {}",
                element_name(*element),
                gauge(*value, total)
            );
        }
    }
    if !panel.display.visible(Section::InputSources) {
        return text;
    }
    text.push_str(tr!("\r\nSources"));
    for ((kind, race), value) in &panel.input_sources {
        let name = match (kind, race) {
            (1, 0) => tr!("Creature (race unavailable)").into(),
            (1, race) => lfmt!("Creature race #{race}", race = race),
            (2, _) => tr!("Player").into(),
            _ => tr!("Unattributed").into(),
        };
        let _ = write!(text, "\r\n{name}: {value} {}", gauge(*value, total));
    }
    text
}

/// A report independent of which analyser tab is currently selected.
/// Hours:minutes:seconds form of the confirmed session clock.
pub(super) fn clock_text(seconds: u64) -> String {
    format!(
        "{}:{:02}:{:02}",
        seconds / 3600,
        seconds / 60 % 60,
        seconds % 60
    )
}
pub fn session_text(panel: &Panel) -> String {
    session_text_at(panel, panel.clock.seconds())
}
pub fn session_text_at(panel: &Panel, seconds: u64) -> String {
    use std::fmt::Write;
    let chart_seconds = seconds;
    let seconds = report_elapsed(panel, seconds, super::Tool::Session);
    let balance = i128::from(panel.loot_value()) - i128::from(panel.supply_cost());
    let balance_rate = if seconds == 0 {
        0
    } else {
        balance * 3600 / i128::from(seconds)
    };
    let mut text = lfmt!("Session clock: {}\r\n", clock_text(seconds));
    text.push_str(&lfmt!(
        "Hunting Session\r\nElapsed: {seconds} seconds\r\nXP/h: {}\r\nRaw XP/h: {}\r\nPriced balance: {balance} gold\r\nPriced balance/h: {balance_rate} gold\r\nOnly resolved, priced items and confirmed gold enter the balance.\r\n{}\r\nKills by creature race",
        super::state::per_hour(panel.gained_xp, seconds),
        super::state::per_hour(panel.raw_xp, seconds),
        overview(panel),
        balance = balance,
        balance_rate = balance_rate,
        seconds = seconds
    ));
    for (race, count) in &panel.kills_by_race {
        if *race == 0 {
            let _ = lwrite!(text, "\r\nRace unavailable: {count}", count = count);
        } else {
            let _ = lwrite!(
                text,
                "\r\nRace #{race}: {count}",
                count = count,
                race = race
            );
        }
    }
    let _ = write!(
        text,
        "\r\n\r\n{}\r\n\r\n{}\r\nSession gained XP per minute\r\n{}\r\nSession raw XP per minute\r\n{}",
        loot_text_at(panel, chart_seconds, seconds, true),
        supply_text_at(panel, chart_seconds, seconds, true),
        interval_chart(
            &panel.xp_chart.values(chart_seconds),
            chart_seconds,
            seconds
        ),
        interval_chart(
            &panel.raw_xp_chart.values(chart_seconds),
            chart_seconds,
            seconds
        )
    );
    text
}

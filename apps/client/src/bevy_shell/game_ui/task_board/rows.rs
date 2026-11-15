use super::*;

pub(super) fn entry(rows: &mut Vec<Entry>, text: impl Into<String>, action: Option<Kind>) {
    rows.push(Entry {
        text: text.into(),
        action,
    });
}

pub(super) fn completed(panel: &TaskBoardPanel) -> bool {
    panel.weekly.as_ref().is_some_and(|r| {
        let b = &r.bounty;
        b.selected != 0
            && b.active
                .as_ref()
                .or_else(|| b.offers.get(usize::from(b.selected.saturating_sub(1))))
                .is_some_and(|a| b.current >= a.required)
    })
}

/// The server generates bounty offers only on OpenBounty (task_board_full.rs),
/// so a board with no offers and no active task needs it.
pub(super) fn unopened(b: &model::task_board::full::Bounty) -> bool {
    b.selected == 0 && b.offers.is_empty() && b.active.is_none()
}

pub(super) fn entries(panel: &TaskBoardPanel, tab: Tab, confirm: bool) -> Vec<Entry> {
    let mut rows = Vec::new();
    let Some(r) = &panel.weekly else {
        label(&mut rows, "Refresh to load the Task Board");
        return rows;
    };
    match tab {
        Tab::Bounty => {
            let b = &r.bounty;
            for node in model::tree_nodes(panel) {
                let action = if let Some(path) = node.path.filter(|_| {
                    node.label.contains("— level") || node.label.starts_with("Next level")
                }) {
                    r.talisman
                        .get(usize::from(path))
                        .filter(|v| v.level < 355)
                        .map(|_| Kind::Talisman(model::talisman::Intent::Upgrade(path)))
                } else if node.label.starts_with("Purchase: 5000 gold") {
                    Some(Kind::Talisman(model::talisman::Intent::Buy))
                } else {
                    None
                };
                entry(&mut rows, node.label, action);
            }
            for (index, slot) in b.lists.iter().enumerate() {
                let Ok(index) = u8::try_from(index) else {
                    continue;
                };
                if !slot.unlocked {
                    button(&mut rows, "Unlock list", Kind::Full(FullIntent::UnlockList));
                    continue;
                }
                for (title, verb, preferred, current) in [
                    ("Preferred", "Prefer", true, &slot.preferred),
                    ("Unwanted", "Avoid", false, &slot.unwanted),
                ] {
                    let action = |race| {
                        Kind::Full(if preferred {
                            FullIntent::Preferred(index, race)
                        } else {
                            FullIntent::Unwanted(index, race)
                        })
                    };
                    let clear = if preferred {
                        "Clear preferred"
                    } else {
                        "Clear unwanted"
                    };
                    button(
                        &mut rows,
                        format!(
                            "List {} — {title}: {} — {clear}",
                            index + 1,
                            model::creature_name(current)
                        ),
                        action(String::new()),
                    );
                    for race in model::creature_choices(panel)
                        .into_iter()
                        .filter(|r| !r.is_empty())
                    {
                        button(
                            &mut rows,
                            format!("{verb} {}", model::creature_name(&race)),
                            action(race),
                        );
                    }
                }
            }
            if panel.weekly_results_selected {
                button(
                    &mut rows,
                    if b.elite {
                        "Elite mode: enabled — disable"
                    } else {
                        "Elite mode: disabled — enable"
                    },
                    Kind::Full(FullIntent::Elite(!b.elite)),
                );
            }
            label(
                &mut rows,
                format!("Reroll Tokens: {} / {}", b.rerolls, model::REROLL_TOKEN_CAP),
            );
            label(&mut rows, "Daily tokens: claim at the Daily Reward Wall");
            if unopened(b) {
                button(&mut rows, "Open bounty", Kind::OpenBounty);
            }
            for (index, offer) in b.offers.iter().enumerate() {
                let active = usize::from(b.selected) == index + 1;
                let progress = if active { b.current } else { 0 };
                let state = if active && completed(panel) {
                    "Completed"
                } else if active {
                    "Active"
                } else {
                    "Available"
                };
                entry(
                    &mut rows,
                    reward_text(offer, progress, state, ""),
                    if b.selected == 0 {
                        u8::try_from(index).ok().map(Kind::Select)
                    } else {
                        None
                    },
                );
            }
            if let Some(active) = &b.active
                && b.offers.get(usize::from(b.selected.saturating_sub(1))) != Some(active)
            {
                label(
                    &mut rows,
                    reward_text(
                        active,
                        b.current,
                        if completed(panel) {
                            "Completed"
                        } else {
                            "Active"
                        },
                        "Active: ",
                    ),
                );
            }
            entry(
                &mut rows,
                "Claim reward",
                completed(panel).then_some(Kind::Claim),
            );
            label(
                &mut rows,
                format!("Task Difficulty: {}", model::difficulty_name(b.difficulty)),
            );
            for (i, (name, _)) in model::DIFFICULTIES.iter().enumerate() {
                button(&mut rows, *name, Kind::Difficulty(i as u8));
            }
            if confirm {
                label(&mut rows, "Reroll these tasks for 1 token?");
                button(&mut rows, "Confirm reroll (1 token)", Kind::ConfirmReroll);
                button(&mut rows, "Cancel", Kind::Cancel);
            } else {
                entry(
                    &mut rows,
                    "Reroll (1 token)",
                    (b.rerolls > 0).then_some(Kind::Reroll),
                );
            }
        }
        Tab::Weekly => {
            let w = &r.weekly;
            label(
                &mut rows,
                format!(
                    "Week {} | Difficulty {} | Multiplier {}",
                    r.period, w.difficulty, w.multiplier
                ),
            );
            label(
                &mut rows,
                format!(
                    "Kill XP: {} | Delivery XP: {} | Pending points: {} | Soulseals: {}",
                    w.kill_experience, w.delivery_experience, w.pending_points, w.pending_soulseals
                ),
            );
            for task in &w.kills {
                label(
                    &mut rows,
                    format!(
                        "{}: {}/{}",
                        if task.race.is_empty() {
                            "Any creature".into()
                        } else {
                            model::creature_name(&task.race)
                        },
                        task.current,
                        task.required
                    ),
                );
            }
            for (i, task) in w.deliveries.iter().enumerate() {
                entry(
                    &mut rows,
                    format!(
                        "Item {}: {}/{}{}",
                        task.item,
                        task.current,
                        task.required,
                        if task.delivered { " (delivered)" } else { "" }
                    ),
                    (!task.delivered && task.current >= task.required)
                        .then_some(Kind::Deliver(i as u8)),
                );
            }
            if !w.started {
                for (i, (name, _)) in model::DIFFICULTIES.iter().enumerate() {
                    button(
                        &mut rows,
                        format!("Start weekly: {name}"),
                        Kind::Start(i as u8),
                    );
                }
            }
            if let Some(results) = &r.weekly_results {
                label(
                    &mut rows,
                    format!(
                        "Week {} rewards already paid: {} points, {} soulseals",
                        results.period, results.points, results.soulseals
                    ),
                );
            }
        }
        Tab::Shop => {
            label(
                &mut rows,
                format!(
                    "Points: {} | Bounty points: {} | Soulseals: {}",
                    r.points, r.bounty_points, r.soulseals
                ),
            );
            for offer in &r.shop {
                entry(
                    &mut rows,
                    model::shop_offer_text(offer, r.points),
                    (!offer.bought).then_some(Kind::Buy(offer.index)),
                );
            }
        }
    }
    rows
}

pub(super) fn label(rows: &mut Vec<Entry>, text: impl Into<String>) {
    entry(rows, text, None);
}

pub(super) fn button(rows: &mut Vec<Entry>, text: impl Into<String>, action: Kind) {
    entry(rows, text, Some(action));
}

pub(super) fn reward_text(
    offer: &model::task_board::full::BountyOffer,
    progress: u32,
    state: &str,
    prefix: &str,
) -> String {
    let model::task_board::full::BountyOffer {
        race,
        required,
        experience,
        points,
        grade,
        ..
    } = offer;
    let name = model::creature_name(race);
    let rarity = model::rarity_name(*grade);
    format!(
        "{prefix}{name}: {progress}/{required} | {rarity} | {state} | {experience} XP | {points} bounty points"
    )
}

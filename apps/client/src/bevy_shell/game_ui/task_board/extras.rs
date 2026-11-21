//! Preference and Soulseal controls reuse the retained Task Board and fight owners.
use super::*;
#[derive(Clone, PartialEq, Eq)]
pub(super) enum Action {
    Preferred(u8, String),
    Unwanted(u8, String),
    Unlock,
    Fight(u16),
    Page(u16),
}
pub(super) fn entries(panel: &TaskBoardPanel, tab: Tab) -> Vec<Entry> {
    let mut rows = Vec::new();
    if tab == Tab::Soulseals {
        entry(&mut rows, &panel.fights.feedback, None);
        let Some(reply) = &panel.fights.reply else {
            entry(&mut rows, "Refresh to load Soulseal fights", None);
            return rows;
        };
        entry(
            &mut rows,
            format!(
                "Soulseals: {}",
                reply
                    .balance
                    .map_or_else(|| "unavailable".into(), |v| v.to_string())
            ),
            None,
        );
        for fight in &reply.entries {
            entry(
                &mut rows,
                format!(
                    "{} · {} stars · {} Soulseals",
                    fight.name, fight.stars, fight.cost
                ),
                (panel.fights.pending.is_none()
                    && reply.balance.is_some_and(|b| b >= u64::from(fight.cost)))
                .then_some(Kind::Extra(Action::Fight(fight.race))),
            );
        }
        if let Some(after) = reply.next {
            entry(
                &mut rows,
                "Next fights",
                Some(Kind::Extra(Action::Page(after))),
            );
        }
        return rows;
    }
    let Some(reply) = &panel.weekly else {
        entry(&mut rows, "Refresh to load preferences", None);
        return rows;
    };
    entry(
        &mut rows,
        format!("Bounty points: {}", reply.bounty_points),
        None,
    );
    let choices = model::creature_choices(panel);
    for (i, list) in reply.bounty.lists.iter().enumerate() {
        let Ok(slot) = u8::try_from(i) else {
            continue;
        };
        if !list.unlocked {
            let cost = i as u64 * 300;
            entry(
                &mut rows,
                format!("Unlock list {} · {cost} bounty points", i + 1),
                (reply.bounty_points >= cost).then_some(Kind::Extra(Action::Unlock)),
            );
            break;
        }
        entry(
            &mut rows,
            format!(
                "List {}: preferred {} · unwanted {}",
                i + 1,
                model::creature_name(&list.preferred),
                model::creature_name(&list.unwanted)
            ),
            None,
        );
        for race in &choices {
            let label = if race.is_empty() {
                "Clear (10 bounty points)".into()
            } else {
                model::creature_name(race)
            };
            if race != &list.preferred {
                entry(
                    &mut rows,
                    format!("Preferred: {label}"),
                    Some(Kind::Extra(Action::Preferred(slot, race.clone()))),
                );
            }
            if race != &list.unwanted {
                entry(
                    &mut rows,
                    format!("Unwanted: {label}"),
                    Some(Kind::Extra(Action::Unwanted(slot, race.clone()))),
                );
            }
        }
    }
    rows
}
pub(super) fn summary(panel: &TaskBoardPanel, action: &Action) -> String {
    match action {
        Action::Preferred(slot, race) | Action::Unwanted(slot, race) => format!(
            "Set {} for list {} to {}? {}",
            if matches!(action, Action::Preferred(..)) {
                "preferred"
            } else {
                "unwanted"
            },
            slot + 1,
            if race.is_empty() {
                "none".into()
            } else {
                model::creature_name(race)
            },
            if race.is_empty() {
                "Clearing costs 10 bounty points."
            } else {
                "No assignment cost."
            }
        ),
        Action::Unlock => format!(
            "Unlock the next list for {} bounty points?",
            panel
                .weekly
                .as_ref()
                .and_then(|r| r.bounty.lists.iter().position(|l| !l.unlocked))
                .unwrap_or(0)
                * 300
        ),
        Action::Fight(race) => panel
            .fights
            .reply
            .as_ref()
            .and_then(|r| r.entries.iter().find(|f| f.race == *race))
            .map_or_else(
                || "Refresh the fight catalogue".into(),
                |f| {
                    format!(
                        "Enter a solo fight against {} for {} Soulseals?",
                        f.name, f.cost
                    )
                },
            ),
        Action::Page(_) => "Load the next fight page?".into(),
    }
}
pub(super) fn send(link: &crate::play::PlayLink, expected: &TaskBoardPanel, action: &Action) {
    match action {
        Action::Preferred(slot, race) => {
            link.task_board_preferences_bound(expected, FullIntent::Preferred(*slot, race.clone()));
        }
        Action::Unwanted(slot, race) => {
            link.task_board_preferences_bound(expected, FullIntent::Unwanted(*slot, race.clone()));
        }
        Action::Unlock => {
            link.task_board_preferences_bound(expected, FullIntent::UnlockList);
        }
        Action::Fight(race) => {
            link.soulseal_fight_bound(expected, *race);
        }
        Action::Page(after) => link.soulseal_catalogue_request(*after),
    }
}

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
    filtered(panel, tab, "", None)
}
pub(super) fn category_label(category: Option<u8>) -> &'static str {
    category
        .and_then(|c| {
            [
                "Harmless",
                "Trivial",
                "Easy",
                "Medium",
                "Hard",
                "Challenging",
            ]
            .get(usize::from(c))
            .copied()
        })
        .unwrap_or("All categories")
}
fn matches_fight(
    fight: &model::task_board::soulpit::Fight,
    search: &str,
    category: Option<u8>,
) -> bool {
    fight.name.to_lowercase().contains(search) && category.is_none_or(|c| c == fight.stars)
}

pub(super) fn restart_filter(
    link: &crate::play::PlayLink,
    panel: &TaskBoardPanel,
    state: &mut State,
) {
    if state.tab == Tab::Soulseals && state.fight_cursor != 0 {
        if panel.fights.pending.is_some() {
            state.restart_fights = true;
            return;
        }
        state.fight_cursor = 0;
        link.soulseal_catalogue_request(0);
    }
}

pub(super) fn next_matching_page(
    link: &crate::play::PlayLink,
    panel: &TaskBoardPanel,
    state: &mut State,
) {
    if panel.fights.pending.is_some() {
        return;
    }
    if state.restart_fights {
        state.restart_fights = false;
        state.fight_cursor = 0;
        link.soulseal_catalogue_request(0);
        return;
    }
    if state.search.trim().is_empty() && state.category.is_none() {
        return;
    }
    let Some(reply) = &panel.fights.reply else {
        return;
    };
    let Some(after) = reply.next.filter(|next| *next > state.fight_cursor) else {
        return;
    };
    let search = state.search.trim().to_lowercase();
    if !reply
        .entries
        .iter()
        .any(|f| matches_fight(f, &search, state.category))
    {
        state.fight_cursor = after;
        link.soulseal_catalogue_request(after);
    }
}

pub(super) fn filtered(
    panel: &TaskBoardPanel,
    tab: Tab,
    search: &str,
    category: Option<u8>,
) -> Vec<Entry> {
    let search = search.trim().to_lowercase();
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
        for fight in reply
            .entries
            .iter()
            .filter(|f| matches_fight(f, &search, category))
        {
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
        for race in choices.iter().filter(|key| {
            key.is_empty() || model::creature_name(key).to_lowercase().contains(&search)
        }) {
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
            link.task_board_action_bound(expected, FullIntent::Preferred(*slot, race.clone()));
        }
        Action::Unwanted(slot, race) => {
            link.task_board_action_bound(expected, FullIntent::Unwanted(*slot, race.clone()));
        }
        Action::Unlock => {
            link.task_board_action_bound(expected, FullIntent::UnlockList);
        }
        Action::Fight(race) => {
            link.soulseal_fight_bound(expected, *race);
        }
        Action::Page(after) => link.soulseal_catalogue_request(*after),
    }
}

pub(super) fn search_controls(
    commands: &mut Commands,
    skins: &Skins,
    frame: Entity,
    state: &State,
    draft: &str,
) {
    if matches!(state.tab, Tab::Preferences | Tab::Soulseals) {
        let mut edit = bevy::text::EditableText::new(draft);
        edit.max_characters = Some(128);
        let field = commands
            .spawn((
                SearchField(state.tab),
                bevy::ui_widgets::TextInput,
                edit,
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                bevy::input_focus::tab_navigation::TabIndex(0),
                Node {
                    height: px(26.),
                    width: percent(100.),
                    ..default()
                },
            ))
            .id();
        commands.entity(frame).add_child(field);
        widget(
            commands,
            skins,
            frame,
            state,
            "Search by name",
            Some(Kind::Search),
        );
    }
    if state.tab == Tab::Soulseals {
        let label = extras::category_label(state.category);
        widget(commands, skins, frame, state, label, Some(Kind::Category));
    }
}

pub(super) fn shop_confirmation(
    commands: &mut Commands,
    skins: &Skins,
    frame: Entity,
    state: &State,
    panel: &TaskBoardPanel,
) {
    if let Some(offer) = state.shop_selection.and_then(|id| {
        panel
            .weekly
            .as_ref()?
            .shop
            .iter()
            .find(|offer| offer.index == id)
    }) {
        widget(
            commands,
            skins,
            frame,
            state,
            &format!(
                "Confirm purchase of {} for {} points?",
                model::shop_offer_text(offer, panel.weekly.as_ref().map_or(0, |r| r.points)),
                offer.price
            ),
            Some(Kind::ConfirmShop(offer.index)),
        );
        widget(commands, skins, frame, state, "Cancel", Some(Kind::Cancel));
    }
}

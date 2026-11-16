//! Custom battle-list editor: name, name filter, maximum HP, kinds, order and the player
//! profession and Guild filters of the active list, plus adding and removing lists. The
//! generic form window presents it; the battle panel's "Edit lists" button opens it.
use super::preferences::{
    BattleListPreferences, BattleSort, GuildFilter, next_edit, validate_lists,
};
use crate::play::{
    PlayLink,
    feature_0115_mail::view::form,
    mailbox::{Mailbox, lock},
};

const SORTS: [(BattleSort, &str); 4] = [
    (BattleSort::Name, "name"),
    (BattleSort::HealthAscending, "low"),
    (BattleSort::HealthDescending, "high"),
    (BattleSort::Identity, "identity"),
];
/// Profession filter words; `none` is the vocation-less family 0, the rest follow it.
const PROFESSIONS: [&str; 6] = ["none", "sorcerer", "druid", "paladin", "knight", "monk"];
const GUILDS: [(GuildFilter, &str); 3] = [
    (GuildFilter::Any, "any"),
    (GuildFilter::SameGuild, "mine"),
    (GuildFilter::OutsideGuild, "outside"),
];

impl PlayLink {
    /// Opens the battle-list editor on the active list.
    pub fn battle_list_editor_open(&self) {
        lock(&self.mailbox).battle_lists.editing = true;
    }
    /// The battle lists the session holds now.
    #[cfg(test)]
    pub fn battle_list_preferences(&self) -> Vec<BattleListPreferences> {
        lock(&self.mailbox).battle_lists.preferences.clone()
    }
}

fn yes_no(value: bool) -> String {
    String::from(if value { "yes" } else { "no" })
}

fn fields(list: &BattleListPreferences) -> Vec<String> {
    vec![
        list.name.clone(),
        list.search.clone(),
        list.max_health.to_string(),
        yes_no(list.players),
        yes_no(list.creatures),
        SORTS
            .iter()
            .find(|(sort, _)| *sort == list.sort)
            .map_or("name", |(_, word)| *word)
            .into(),
        list.vocation
            .and_then(|family| PROFESSIONS.get(usize::from(family)))
            .map_or("any", |word| *word)
            .into(),
        GUILDS
            .iter()
            .find(|(guild, _)| *guild == list.guild)
            .map_or("any", |(_, word)| *word)
            .into(),
    ]
}

fn flag(value: &str) -> Option<bool> {
    match value.to_ascii_lowercase().as_str() {
        "yes" => Some(true),
        "no" => Some(false),
        _ => None,
    }
}

/// The list the fields describe, or `None` when a field holds no accepted value.
fn parse(values: &[String]) -> Option<BattleListPreferences> {
    let word = |index: usize| form::field(values, index).to_ascii_lowercase();
    let (sort, profession, guild) = (word(5), word(6), word(7));
    Some(BattleListPreferences {
        name: form::field(values, 0).into(),
        search: form::field(values, 1).into(),
        max_health: form::field(values, 2).parse().ok()?,
        players: flag(form::field(values, 3))?,
        creatures: flag(form::field(values, 4))?,
        sort: SORTS
            .iter()
            .find(|(_, name)| *name == sort)
            .map(|(sort, _)| *sort)?,
        vocation: if profession == "any" {
            None
        } else {
            Some(u8::try_from(PROFESSIONS.iter().position(|name| *name == profession)?).ok()?)
        },
        guild: GUILDS
            .iter()
            .find(|(_, name)| *name == guild)
            .map(|(guild, _)| *guild)?,
    })
}

fn submit(mailbox: &mut Mailbox, action: usize, values: &[String]) {
    let state = &mut mailbox.battle_lists;
    let mut candidate = state.preferences.clone();
    let mut selected = state.active.min(candidate.len().saturating_sub(1));
    match action {
        0 => {
            let Some(list) = parse(values) else {
                mailbox.battle.feedback =
                    Some("Enter HP from 0 to 100, yes or no, and one of the listed words");
                return;
            };
            let Some(slot) = candidate.get_mut(selected) else {
                return;
            };
            *slot = list;
        }
        1 if candidate.len() < 4 => {
            let Some(name) = (1..=8).map(|i| format!("List {i}")).find(|name| {
                !candidate
                    .iter()
                    .any(|list| list.name.eq_ignore_ascii_case(name))
            }) else {
                return;
            };
            candidate.push(BattleListPreferences {
                name,
                ..Default::default()
            });
            selected = candidate.len() - 1;
        }
        2 if candidate.len() > 1 => {
            candidate.remove(selected);
            selected = selected.min(candidate.len() - 1);
        }
        1 => {
            mailbox.battle.feedback = Some("At most four battle lists");
            return;
        }
        2 => {
            mailbox.battle.feedback = Some("Keep at least one battle list");
            return;
        }
        _ => return,
    }
    if validate_lists(&candidate) {
        state.preferences = candidate;
        state.active = selected;
        state.dirty = true;
        state.edit = next_edit();
        mailbox.battle.feedback = Some("List updated");
    } else {
        mailbox.battle.feedback = Some(
            "Use unique list names (1–32 bytes), a filter up to 64 bytes and HP from 0 to 100",
        );
    }
}

pub(in crate::play) fn form(_link: &PlayLink) -> form::Form {
    form::Form {
        title: "Battle lists",
        labels: &[
            "List name",
            "Name filter",
            "Maximum HP %",
            "Players (yes/no)",
            "Creatures (yes/no)",
            "Order (name, low, high, identity)",
            "Profession (any, none, sorcerer, druid, paladin, knight, monk)",
            "Guild (any, mine, outside)",
        ],
        buttons: &["Save list", "Add list", "Remove list"],
        snapshot: |m| {
            let lists = &m.battle_lists;
            let active = lists.active.min(lists.preferences.len().saturating_sub(1));
            let names = lists
                .preferences
                .iter()
                .enumerate()
                .map(|(index, list)| {
                    if index == active {
                        format!("• {}", list.name)
                    } else {
                        list.name.clone()
                    }
                })
                .collect::<Vec<_>>()
                .join("   ");
            (
                lists.editing,
                false,
                lists
                    .preferences
                    .get(active)
                    .map(fields)
                    .unwrap_or_default(),
                format!(
                    "{names}\nSpecific player filters exclude unknown details.\n{}",
                    m.battle.feedback.unwrap_or("")
                ),
            )
        },
        set_open: |m, open| m.battle_lists.editing = open,
        submit: |m, _commands, action, values| submit(m, action, &values),
    }
}

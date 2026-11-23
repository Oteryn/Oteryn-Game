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

const SORTS: [(BattleSort, &str); 9] = [
    (BattleSort::Appearance, "appearance"),
    (BattleSort::AppearanceDescending, "newest"),
    (BattleSort::DistanceAscending, "nearest"),
    (BattleSort::DistanceDescending, "farthest"),
    (BattleSort::NameDescending, "name-desc"),
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
        list.hidden_vocations
            .iter()
            .filter_map(|v| PROFESSIONS.get(usize::from(*v)).copied())
            .collect::<Vec<_>>()
            .join(", "),
        yes_no(list.hide_party),
        yes_no(list.hide_non_skull),
        yes_no(list.hide_npcs),
        yes_no(list.hide_summons),
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
        filters_collapsed: false,
        hide_summons: values
            .get(12)
            .filter(|v| !v.is_empty())
            .map_or(Some(false), |v| flag(v.trim()))?,
        hide_npcs: values
            .get(11)
            .filter(|v| !v.is_empty())
            .map_or(Some(false), |v| flag(v.trim()))?,
        hide_party: values
            .get(9)
            .filter(|v| !v.is_empty())
            .map_or(Some(false), |v| flag(v.trim()))?,
        hide_non_skull: values
            .get(10)
            .filter(|v| !v.is_empty())
            .map_or(Some(false), |v| flag(v.trim()))?,
        hidden_vocations: word(8)
            .split(',')
            .map(str::trim)
            .filter(|word| !word.is_empty())
            .map(|word| {
                PROFESSIONS
                    .iter()
                    .position(|name| *name == word)
                    .and_then(|i| u8::try_from(i).ok())
            })
            .collect::<Option<_>>()?,
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
            let Some(mut list) = parse(values) else {
                mailbox.battle.feedback =
                    Some("Enter HP from 0 to 100, yes or no, and one of the listed words");
                return;
            };
            let Some(slot) = candidate.get_mut(selected) else {
                return;
            };
            list.filters_collapsed = slot.filters_collapsed;
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
        3 if selected > 0 => {
            let default_name = (1..=8).map(|i| format!("List {i}")).find(|name| {
                !candidate
                    .iter()
                    .enumerate()
                    .any(|(index, list)| index != selected && list.name.eq_ignore_ascii_case(name))
            });
            let Some(name) = default_name else { return };
            candidate[selected] = BattleListPreferences {
                name,
                sort: BattleSort::Name,
                ..Default::default()
            };
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

pub(in crate::play) fn form(link: &PlayLink) -> form::Form {
    form::Form {
        title: "Battle lists",
        labels: &[
            "List name",
            "Name filter",
            "Maximum HP %",
            "Players (yes/no)",
            "Creatures (yes/no)",
            "Order (appearance, newest, nearest, farthest, name, name-desc, low, high, identity)",
            "Profession (any, none, sorcerer, druid, paladin, knight, monk)",
            "Guild (any, mine, outside)",
            "Hide professions (comma-separated names)",
            "Hide party members (yes/no)",
            "Hide non-skull players (yes/no)",
            "Hide NPCs (yes/no)",
            "Hide known summons (yes/no)",
        ],
        buttons: if link.active_battle_list() > 0 {
            &[
                "Save list",
                "Add list",
                "Remove list",
                "Clear configurations",
            ]
        } else {
            &["Save list", "Add list", "Remove list"]
        },
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

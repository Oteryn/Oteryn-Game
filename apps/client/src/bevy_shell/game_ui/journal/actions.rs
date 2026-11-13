use super::*;
use crate::play::feature_0287_actor_spell_catalog::view::{
    SPELLBOOK_GROUPS, SPELLBOOK_VOCATIONS, SpellbookFilter,
};
use oteryn_client_ui::quest_log::{self, QuestLogQuery, QuestSort};
#[derive(Default)]
pub(super) struct Values {
    pub search: String,
    pub minimum: String,
    pub maximum: String,
}
pub(super) fn values(fields: &Query<(&Field, &EditableText)>) -> Values {
    let mut values = Values::default();
    for (field, text) in fields {
        let target = match field {
            Field::Search => &mut values.search,
            Field::Minimum => &mut values.minimum,
            Field::Maximum => &mut values.maximum,
        };
        *target = text.value().to_string();
    }
    values
}
pub(super) fn perform(
    owner: &mut HostOwner,
    state: &mut State,
    action: Action,
    values: &Values,
) -> bool {
    if owner.generation() != action.key.generation
        || state.key != Some(action.key)
        || owner.modal_active()
    {
        return false;
    }
    let Some(play) = &owner.play else {
        return false;
    };
    let link = &play.link;
    let quest = link.quest_panel();
    let book = link.spellbook_state();
    let catalogue = link.spell_catalog_panel();
    let visible = match action.key.tab {
        Tab::Quests => quest.open || quest.tracker_open,
        Tab::Spells => book.open || catalogue.open || catalogue.archive_open,
    };
    if !visible {
        return false;
    }
    state.feedback.clear();
    match action.kind {
        Kind::Quests => {
            open_quests(owner);
            return true;
        }
        Kind::Spells => {
            open_spells(owner);
            return true;
        }
        Kind::Close => {
            link.quest_set_open(false);
            link.spellbook_set_open(false);
            return true;
        }
        Kind::Quest(id) => {
            if let Some(query) = quest_log::quest_row_query(&quest, &[id], 0) {
                link.quest_request(&query);
                return true;
            }
            return false;
        }
        Kind::Mission(quest, mission) => return link.quest_select_mission(quest, mission),
        Kind::Spell(id) => {
            if book.selected {
                if !book.page.as_ref().is_some_and(|page| {
                    page.entries
                        .iter()
                        .any(|entry| entry.spell.cast_index == id)
                }) {
                    return false;
                }
            } else if !link.spell_catalog_select(id) {
                return false;
            }
            state.selected_spell = Some(id);
            return true;
        }
        Kind::Track => {
            if let Some(query) = quest_log::tracking_query(&quest) {
                link.quest_request(&query);
                return true;
            }
            return false;
        }
        Kind::Assign(slot) => {
            let id = if book.selected {
                state.selected_spell.filter(|id| {
                    book.page.as_ref().is_some_and(|page| {
                        page.entries
                            .iter()
                            .any(|entry| entry.spell.cast_index == *id)
                    })
                })
            } else {
                catalogue.selected_spell().map(|spell| spell.cast_index)
            };
            return id.is_some_and(|id| link.assign_spell_slot(slot, Some(id)));
        }
        Kind::Previous | Kind::Next => {
            let next = matches!(action.kind, Kind::Next);
            if action.key.tab == Tab::Quests {
                let pages = quest_log::quest_rows(&quest).len().div_ceil(32).max(1);
                state.quest_page = if next {
                    state.quest_page.saturating_add(1).min(pages - 1)
                } else {
                    state.quest_page.saturating_sub(1)
                };
                return true;
            }
            if !book.selected {
                link.spell_catalog_navigate(next);
                return true;
            }
            let current = book.page.as_ref().map_or(0, |p| p.page);
            let maximum = book.page.as_ref().map_or(0, |p| {
                u16::try_from(usize::from(p.total).div_ceil(32).saturating_sub(1)).unwrap_or(0)
            });
            let page = if book.confirmed_filter.as_ref() != Some(&book.filter) {
                0
            } else if next {
                current.saturating_add(1).min(maximum)
            } else {
                current.saturating_sub(1)
            };
            return link.spellbook_request(book.filter, page);
        }
        _ => {}
    }
    if action.key.tab == Tab::Quests {
        match action.kind {
            Kind::Refresh => link.quest_request(&QuestLogQuery::ListOnly),
            Kind::Search | Kind::Sort => {
                if values.search.len() > 512 || values.search.contains('\0') {
                    state.feedback = "Search is too long.".into();
                    return false;
                }
                let sort = if matches!(action.kind, Kind::Sort) {
                    QuestSort::from_selection(((quest.list_sort.selection() + 1) % 4) as isize)
                } else {
                    quest.list_sort
                };
                link.quest_list_options(&values.search, sort);
                state.quest_page = 0;
            }
            _ => return false,
        }
        return true;
    }
    if !book.selected {
        return if matches!(action.kind, Kind::Search | Kind::Refresh) {
            link.spell_catalog_search(&values.search, &values.maximum, "", false, "")
        } else {
            false
        };
    }
    let Some(minimum) = number(&values.minimum) else {
        state.feedback = "Enter a valid minimum level.".into();
        return false;
    };
    let Some(maximum) = number(&values.maximum) else {
        state.feedback = "Enter a valid maximum level.".into();
        return false;
    };
    let mut filter = SpellbookFilter {
        search: values.search.clone(),
        min_level: minimum,
        max_level: maximum,
        ..book.filter
    };
    match action.kind {
        Kind::Vocation => {
            filter.vocation = ((usize::from(filter.vocation) + 1) % SPELLBOOK_VOCATIONS.len()) as u8
        }
        Kind::Premium => filter.premium = (filter.premium + 1) % 3,
        Kind::Group => {
            filter.group = ((usize::from(filter.group) + 1) % SPELLBOOK_GROUPS.len()) as u8
        }
        Kind::Search | Kind::Refresh => {}
        _ => return false,
    }
    if !filter.valid() {
        state.feedback = "Check the search and level range.".into();
        return false;
    }
    link.spellbook_request(filter, 0)
}
fn number(value: &str) -> Option<u32> {
    if value.trim().is_empty() {
        Some(0)
    } else {
        value.trim().parse().ok()
    }
}

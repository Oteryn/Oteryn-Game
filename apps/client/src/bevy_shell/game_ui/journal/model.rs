use super::*;
use crate::play::feature_0287_actor_spell_catalog::view::SpellbookState;
use oteryn_client_ui::quest_log::{self, QuestLogView, QuestPanel};
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Row {
    pub id: RowId,
    pub label: String,
    pub selected: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RowId {
    Quest(u32),
    Mission(u32, u32),
    Spell(u32),
}
pub(super) struct Projection {
    pub rows: Vec<Row>,
    pub missions: Vec<Row>,
    pub detail: String,
    pub status: String,
    pub selected_spell: Option<u32>,
}
pub(super) fn project(
    tab: Tab,
    quest: &QuestPanel,
    book: &SpellbookState,
    catalogue: &oteryn_client_ui::spell_catalog::Panel,
    selected: Option<u32>,
    quest_page: usize,
) -> Projection {
    if tab == Tab::Quests {
        return quests(quest, quest_page);
    }
    if !book.selected {
        return catalogue_rows(catalogue);
    }
    let current = book.page.as_ref().and_then(|page| {
        selected
            .and_then(|id| {
                page.entries
                    .iter()
                    .position(|entry| entry.spell.cast_index == id)
            })
            .or_else(|| (!page.entries.is_empty()).then_some(0))
    });
    let selected_spell = book
        .page
        .as_ref()
        .and_then(|page| current.and_then(|row| page.entries.get(row)))
        .map(|entry| entry.spell.cast_index);
    let rows = book
        .page
        .as_ref()
        .map(|page| {
            page.entries
                .iter()
                .map(|entry| Row {
                    id: RowId::Spell(entry.spell.cast_index),
                    label: format!("{}\n{}", entry.spell.name, entry.spell.words),
                    selected: Some(entry.spell.cast_index) == selected_spell,
                })
                .collect()
        })
        .unwrap_or_default();
    let status = if book.pending.is_some() {
        "Loading spells; the last confirmed page is retained.".into()
    } else if !book.feedback.is_empty() {
        book.feedback.clone()
    } else {
        book.page.as_ref().map_or_else(
            || "No spellbook page received.".into(),
            |page| {
                format!(
                    "{} matching spells · Page {} / {}",
                    page.total,
                    page.page + 1,
                    usize::from(page.total).div_ceil(32).max(1)
                )
            },
        )
    };
    Projection {
        rows,
        missions: Vec::new(),
        detail: book.text(current.unwrap_or(0)),
        status,
        selected_spell,
    }
}
fn quests(panel: &QuestPanel, page: usize) -> Projection {
    let visible = quest_log::quest_rows(panel);
    let pages = visible.len().div_ceil(32).max(1);
    let page = page.min(pages - 1);
    let line = match &panel.state.log.view {
        Some(QuestLogView::Line(line)) => Some(line),
        _ => None,
    };
    let rows = visible
        .iter()
        .skip(page * 32)
        .take(32)
        .map(|entry| Row {
            id: RowId::Quest(entry.quest_index),
            label: format!(
                "Quest {}{}",
                entry.quest_index,
                if entry.completed { " (completed)" } else { "" }
            ),
            selected: line.is_some_and(|line| line.quest_index == entry.quest_index),
        })
        .collect();
    let missions = line
        .map(|line| {
            line.missions
                .iter()
                .map(|mission| Row {
                    id: RowId::Mission(line.quest_index, mission.mission_index),
                    label: format!(
                        "{}{}",
                        mission.name,
                        if mission.done { " (completed)" } else { "" }
                    ),
                    selected: panel.mission == Some(mission.mission_index),
                })
                .collect()
        })
        .unwrap_or_default();
    let detail = if !panel.state.selected {
        "Quest log unavailable in this session.".into()
    } else if let Some(line) = line {
        line.missions
            .iter()
            .find(|mission| Some(mission.mission_index) == panel.mission)
            .map_or_else(
                || {
                    format!(
                        "Quest {}{}\nSelect a mission to read its journal entry.",
                        line.quest_index,
                        if line.completed { " (completed)" } else { "" }
                    )
                },
                |mission| format!("{}\n\n{}", mission.name, mission.text),
            )
    } else {
        quest_log::tracker_text(panel)
    };
    let status = if panel.pending_query.is_some() {
        "Waiting for server confirmation…".into()
    } else if panel.state.refusal.is_some() {
        quest_log::panel_text(panel)
            .lines()
            .nth(1)
            .unwrap_or("Quest request refused; refresh the list.")
            .to_owned()
    } else {
        format!(
            "{} quests · {} completed · {} tracked / 10 · Page {} / {}",
            visible.len(),
            panel.state.list.iter().filter(|q| q.completed).count(),
            panel.state.log.tracked.len(),
            page + 1,
            pages
        )
    };
    Projection {
        rows,
        missions,
        detail,
        status,
        selected_spell: None,
    }
}
fn catalogue_rows(panel: &oteryn_client_ui::spell_catalog::Panel) -> Projection {
    let selected = panel.selected_spell();
    let selected_spell = selected.map(|entry| entry.cast_index);
    let count = panel.visible_spells().count();
    let rows = panel
        .visible_spells()
        .skip(panel.page * 16)
        .take(16)
        .map(|entry| Row {
            id: RowId::Spell(entry.cast_index),
            label: format!("{}\n{}", entry.name, entry.words),
            selected: Some(entry.cast_index) == selected_spell,
        })
        .collect();
    let detail = selected.map_or_else(
        || oteryn_client_ui::spell_catalog::panel_text(panel),
        |entry| {
            format!(
                "{}\n{}\n\nLevel: {}\nMana: {}{}\nSoul: {}\nLearned: {}\nAvailable now: {}",
                entry.name,
                entry.words,
                entry.level,
                entry.mana,
                if entry.mana_percent { "%" } else { "" },
                entry.soul,
                entry.learned,
                entry.available
            )
        },
    );
    let status = if !panel.selected {
        "Spell catalogue unavailable in this session.".into()
    } else if panel.filter_error {
        "Check search and maximum level.".into()
    } else {
        format!(
            "{} matching spells · Page {} / {}",
            count,
            panel.page + 1,
            count.div_ceil(16).max(1)
        )
    };
    Projection {
        rows,
        missions: Vec::new(),
        detail,
        status,
        selected_spell,
    }
}

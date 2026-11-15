//! A reviewed local build is applied one fenced perk command at a time.
use super::*;
use oteryn_client_runtime::weapon_proficiency::ProficiencyCatalogueEntry;
use std::collections::BTreeMap;
#[derive(Default)]
pub(super) struct Draft {
    choices: BTreeMap<usize, usize>,
    base: Option<ProficiencyCatalogueEntry>,
    waiting: Option<(usize, usize)>,
    applying: bool,
    close: bool,
    ask_close: bool,
    feedback: String,
}
impl Draft {
    pub(super) fn controls(&self) -> (bool, bool, String) {
        (
            !self.choices.is_empty(),
            self.ask_close,
            self.feedback.clone(),
        )
    }
}
pub(super) fn projection(panel: &WeaponProficiencyPanel, draft: &Draft) -> WeaponProficiencyPanel {
    let mut result = panel.clone();
    if let Some(entry) = result
        .catalogue
        .as_mut()
        .and_then(|p| p.entries.iter_mut().find(|e| e.definition == panel.weapon))
    {
        for (&level, &perk) in &draft.choices {
            if let Some(row) = entry.levels.get_mut(level) {
                row.selected = u8::try_from(perk).ok();
            }
        }
    }
    result
}
pub(super) fn perform(
    link: &crate::play::PlayLink,
    state: &mut State,
    panel: &WeaponProficiencyPanel,
    kind: Kind,
) {
    if link.weapon_proficiency_panel() != *panel || panel.pending {
        return;
    }
    state.dirty = true;
    let d = &mut state.draft;
    if d.applying {
        return;
    }
    match kind {
        Kind::Perk(level, perk) => {
            let Some(entry) = panel.weapon_entry() else {
                return;
            };
            if !panel.selected
                || panel.mutation_uncertain
                || entry
                    .thresholds
                    .get(level)
                    .is_none_or(|xp| entry.experience < *xp)
                || entry
                    .levels
                    .get(level)
                    .is_none_or(|row| perk >= row.perks.len())
            {
                return;
            }
            d.base.get_or_insert_with(|| entry.clone());
            if entry.levels[level].selected.map(usize::from) == Some(perk) {
                d.choices.remove(&level);
            } else {
                d.choices.insert(level, perk);
            }
            d.feedback = "Preview only; Apply sends the reviewed build.".into();
        }
        Kind::Apply | Kind::SaveClose => {
            if d.base.as_ref() != panel.weapon_entry() {
                d.feedback = "Server state changed; reset and review the draft again.".into();
                return;
            }
            d.applying = true;
            d.close = matches!(kind, Kind::SaveClose);
            d.ask_close = false;
            send_next(link, d, panel);
        }
        Kind::Reset | Kind::CancelClose => {
            if matches!(kind, Kind::Reset) {
                *d = Draft::default();
            } else {
                d.ask_close = false;
            }
        }
        Kind::Close if !d.choices.is_empty() => d.ask_close = true,
        Kind::DiscardClose | Kind::Close => {
            *d = Draft::default();
            link.weapon_proficiency_action_bound(panel, ModelAction::Close);
        }
        Kind::Refresh | Kind::Weapon(_) if !d.choices.is_empty() => {
            d.feedback = "Apply or Reset the draft before changing weapons or refreshing.".into()
        }
        Kind::Refresh => {
            link.weapon_proficiency_action_bound(panel, ModelAction::Refresh);
        }
        Kind::Weapon(row) => {
            link.weapon_proficiency_action_bound(panel, ModelAction::Weapon(row));
        }
    }
}
fn send_next(link: &crate::play::PlayLink, d: &mut Draft, panel: &WeaponProficiencyPanel) {
    if let Some((&level, &perk)) = d.choices.first_key_value() {
        if link.weapon_proficiency_action_bound(panel, ModelAction::Perk(level, perk)) {
            d.waiting = Some((level, perk));
        } else {
            d.applying = false;
            d.feedback =
                "The server observation no longer permits this draft. Reset and review it again."
                    .into();
        }
    } else {
        d.applying = false;
        d.base = None;
        d.feedback = "Draft confirmed by the server.".into();
        if d.close {
            link.weapon_proficiency_action_bound(panel, ModelAction::Close);
        }
        d.close = false;
    }
}
fn same_catalogue(base: &ProficiencyCatalogueEntry, current: &ProficiencyCatalogueEntry) -> bool {
    let mut expected = base.clone();
    expected.experience = current.experience;
    for (level, observed) in expected.levels.iter_mut().zip(&current.levels) {
        level.selected = observed.selected;
    }
    expected == *current
}
pub(super) fn advance(
    owner: &HostOwner,
    state: &mut State,
    panel: Option<&WeaponProficiencyPanel>,
) {
    if state.key.generation != owner.generation() || panel.is_none() {
        state.draft = Draft::default();
        return;
    }
    let (Some(play), Some(panel)) = (&owner.play, panel) else {
        return;
    };
    let d = &mut state.draft;
    if play.view.gameplay_input_blocked(&play.link) {
        state.dirty |= !d.choices.is_empty() || d.applying || d.ask_close;
        *d = Draft::default();
        return;
    }
    if !d.applying {
        return;
    }
    if panel.pending {
        return;
    }
    if d.base
        .as_ref()
        .zip(panel.weapon_entry())
        .is_none_or(|(base, current)| !same_catalogue(base, current))
    {
        d.applying = false;
        d.close = false;
        d.waiting = None;
        d.feedback = "The weapon catalogue changed; remaining choices were not sent.".into();
        state.dirty = true;
        return;
    }
    if let Some((level, perk)) = d.waiting.take() {
        state.dirty = true;
        if panel
            .weapon_entry()
            .and_then(|e| e.levels.get(level))
            .and_then(|l| l.selected)
            .map(usize::from)
            != Some(perk)
        {
            d.applying = false;
            d.close = false;
            d.feedback =
                "The server did not confirm the draft; remaining choices were not sent.".into();
            return;
        }
        d.choices.remove(&level);
    }
    send_next(&play.link, d, panel);
}
pub(super) fn render(
    commands: &mut Commands,
    skin: &Skins,
    widgets: &widgets::Widgets,
    key: Key,
    (dirty, ask_close, feedback): (bool, bool, String),
) {
    use oteryn_client_ui_bevy as source;
    let parent = widgets.content.unwrap_or(widgets.window);
    let label = commands
        .spawn(source::button_caption(feedback, skin.font.clone()))
        .id();
    commands.entity(parent).add_child(label);
    let actions = if ask_close {
        vec![
            (Kind::SaveClose, "Save draft and close"),
            (Kind::DiscardClose, "Discard draft and close"),
            (Kind::CancelClose, "Keep editing"),
        ]
    } else if dirty {
        vec![(Kind::Apply, "Apply draft"), (Kind::Reset, "Reset draft")]
    } else {
        Vec::new()
    };
    for (kind, label) in actions {
        let e = commands
            .spawn((
                source::button(skin.buttons.clone(), 220.),
                Control { key, kind },
            ))
            .observe(super::activate)
            .id();
        let text = commands
            .spawn(source::button_caption(label, skin.font.clone()))
            .id();
        commands.entity(e).add_child(text);
        commands.entity(parent).add_child(e);
    }
}

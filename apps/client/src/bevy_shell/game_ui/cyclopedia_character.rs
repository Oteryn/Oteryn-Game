//! Character Cyclopedia presentation; all commands remain in the retained owner.
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    modal::ModalGuard,
};
use crate::play::feature_0199_cyclopedia_character::view::bound::CharacterAction as Action;
use bevy::{
    prelude::*,
    text::EditableText,
    ui_widgets::{Activate, ScrollArea, TextInput},
};
use oteryn_client_ui::cyclopedia_character::{
    self as ui, CyclopediaCharacterPanel as Panel, Intent,
};
use oteryn_client_ui_bevy as source;
#[cfg(test)]
mod tests;
mod widgets;
#[derive(Resource, Default)]
struct State {
    generation: u64,
    serial: u64,
    snapshot: Option<Panel>,
    root: Option<Entity>,
}
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    serial: u64,
    action: Action,
}
#[derive(Component)]
struct Field(usize);
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(in crate::bevy_shell) fn toggle(owner: &HostOwner) {
    if owner.phase != Phase::Playing || owner.modal_active() {
        return;
    }
    if let Some(play) = &owner.play {
        if play.view.gameplay_input_blocked(&play.link) {
            return;
        }
        let panel = play.link.cyclopedia_character_panel();
        if play.link.cyclopedia_character_bound(&panel, Action::Toggle) && !panel.open {
            let panel = play.link.cyclopedia_character_panel();
            play.link
                .cyclopedia_character_bound(&panel, Action::Refresh);
        }
    }
}
fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    if owner.modal_active() {
        return;
    }
    // Ctrl+Shift+F9 is unused by the other shell windows.
    if keys.just_pressed(KeyCode::F9)
        && (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
        && (keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight))
        && !keys.pressed(KeyCode::AltLeft)
        && !keys.pressed(KeyCode::AltRight)
        && !keys.pressed(KeyCode::SuperLeft)
        && !keys.pressed(KeyCode::SuperRight)
        && guard.allows_focus(state.root)
    {
        toggle(&owner);
    }
    if keys.just_pressed(KeyCode::Escape)
        && let Some(root) = state.root
        && guard.allows(root)
        && let Some(panel) = &state.snapshot
        && let Some(play) = &owner.play
        && state.generation == owner.generation()
    {
        play.link.cyclopedia_character_bound(panel, Action::Close);
    }
}
fn project(
    guard: ModalGuard,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
) {
    let panel = owner
        .play
        .as_ref()
        .map(|p| p.link.cyclopedia_character_panel())
        .filter(|p| p.open);
    if let Some(panel) = &panel
        && panel.misc_resources_due
        && !panel.pending
        && !owner.modal_active()
        && guard.allows_focus(state.root)
        && let Some(play) = &owner.play
        && !play.view.gameplay_input_blocked(&play.link)
    {
        play.link
            .cyclopedia_character_bound(panel, Action::Request(Intent::MiscResources));
    }
    if state.generation == owner.generation() && state.snapshot == panel {
        return;
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.generation = owner.generation();
    state.serial = state.serial.saturating_add(1);
    state.snapshot = panel;
    if let Some(panel) = &state.snapshot {
        state.root = Some(widgets::build(
            &mut commands,
            &skins,
            state.generation,
            state.serial,
            panel,
            &mut images,
        ));
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    state: Res<State>,
) {
    if !guard.allows(event.entity) || owner.modal_active() {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if control.generation != owner.generation() || control.serial != state.serial {
        return;
    }
    let (Some(panel), Some(play)) = (&state.snapshot, &owner.play) else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let mut action = control.action.clone();
    let mut values = [
        panel.offset.to_string(),
        panel.limit.to_string(),
        panel
            .target
            .map(|t| t.iter().map(|b| format!("{b:02x}")).collect())
            .unwrap_or_default(),
    ];
    for (field, text) in &fields {
        if text.pending_paste.is_some()
            || text
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
        {
            return;
        }
        if field.0 < 3 {
            values[field.0] = text.value().to_string();
        }
    }
    for (field, text) in &fields {
        if field.0 == 3 {
            match &mut action {
                Action::GalleryFilter(_, search) | Action::ItemFilter(_, search) => {
                    *search = text.value().to_string()
                }
                _ => continue,
            }
        }
    }
    if let Action::Native(id, _) = action {
        action = Action::Native(id, values);
    }
    play.link.cyclopedia_character_bound(panel, action);
}
fn body(p: &Panel) -> String {
    let mut text = ui::panel_text(p);
    if p.pending || p.blessings.pending {
        text.push_str("\nWaiting for the server");
    }
    if !p.selected {
        text.push_str("\nFeature unavailable in this session");
    }
    for (label, value) in ui::overview_cards(p)
        .into_iter()
        .chain(ui::store_cards(p))
        .chain(ui::misc_cards(p))
    {
        text.push_str(&format!("\n{label}: {value}"));
    }
    for row in ui::combat_rows(p) {
        text.push_str(&format!("\n{}: {} ({})", row.label, row.value, row.source));
    }
    text.push_str(&ui::store_blessings(p));
    for row in ui::profile_rows(p) {
        text.push_str(&format!("\n{}: {}", row.label, row.detail));
    }
    for row in ui::history_rows(p) {
        text.push_str(&format!("\n{}: {}", row.at, row.description));
    }
    if p.tab == oteryn_client_runtime::cyclopedia_character::CharacterTab::Achievements {
        text.push_str(&oteryn_client_ui::account_achievements::panel_text(
            &p.achievements,
        ));
    }
    if p.badges_open || p.command_kind == 387 {
        text.push_str(&ui::badge_text(p));
    }
    if p.augments.open {
        for row in ui::augment_rows(p) {
            text.push_str(&format!("\n{}: {}", row.label, row.detail));
        }
    }
    if p.gallery.open {
        text.push_str("\nAppearance gallery");
        if let Some(page) = &p.gallery.page {
            text.push_str(&format!(
                "\nOffset: {} · Total: {}",
                page.offset, page.total
            ));
        }
        for entry in p.gallery.rows() {
            text.push_str(&format!(
                "\n{:?}: {} (look {})",
                entry.kind, entry.key, entry.look_type
            ));
        }
    }
    if p.items.open {
        text.push_str("\nItem summary — Inventory / Depot / Stash / Inbox");
        if let Some(page) = &p.items.page {
            text.push_str(&format!(
                "\nOffset: {} · Total: {}",
                page.offset, page.total
            ));
        }
        for row in p.items.rows() {
            text.push_str(&format!("\n{}: {:?}", row.definition, row.quantities));
        }
    }
    if p.protection.open {
        text.push_str("\nDeath loss preview\n");
        if let Some(forecast) = &p.protection.forecast {
            text.push_str(&ui::protection_heading(forecast, p.vocation_id));
            for (label, value) in ui::protection_cards(forecast) {
                text.push_str(&format!("\n{label}: {value}"));
            }
            text.push_str(&ui::promotion_text(forecast, p.vocation_id));
        }
        text.push_str(ui::PROTECTION_NOTE);
    }
    text
}
#[path = "../../features/0199-cyclopedia-character/view/gallery_pixels.rs"]
mod gallery_pixels;

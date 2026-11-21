//! Supply stash over the retained panel and its existing command owner.
#[cfg(test)]
mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::stash::{Intent, StashPanel, visible_rows, visible_withdraw};
use oteryn_client_ui_bevy as source;
#[derive(Component)]
struct Quantity;
#[derive(Component)]
struct Row;
#[derive(Component, Clone, Copy)]
enum Field {
    Search,
    Category,
    Trader,
}
type StashFields = Or<(With<Field>, With<Quantity>)>;
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Close,
    Filter,
    Sort,
    Previous,
    Next,
    Refresh,
    Row(usize),
    Confirm,
    Cancel,
}
#[derive(Component)]
struct Control {
    generation: u64,
    revision: u64,
    kind: Kind,
}
struct Amount {
    row: usize,
    value: u32,
    max: u32,
}
#[derive(Resource, Default)]
struct State {
    generation: u64,
    revision: u64,
    panel: Option<StashPanel>,
    root: Option<Entity>,
    amount: Option<Amount>,
    dirty: bool,
    filter: Option<Entity>,
    confirm: Option<Entity>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(
            PostUpdate,
            (finish_filter, finish_confirm).after(bevy::text::EditableTextSystems),
        );
}
fn valid(owner: &HostOwner, state: &State) -> bool {
    !owner.modal_active()
        && owner.generation() == state.generation
        && owner.play.as_ref().is_some_and(|play| {
            !play.view.gameplay_input_blocked(&play.link)
                && state.panel.as_ref() == Some(&play.link.stash_panel())
        })
}
fn perform(owner: &HostOwner, state: &mut State, kind: Kind) {
    if !valid(owner, state) {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    match kind {
        Kind::Close => {
            play.link.stash_close();
            state.amount = None;
        }
        Kind::Row(row) => {
            if state.amount.is_some() {
                return;
            }
            let Some(panel) = &state.panel else {
                return;
            };
            if panel.pending || !panel.selected {
                return;
            }
            if let Some(index) = visible_rows(panel).get(row)
                && let Some(item) = panel.rows.get(*index).filter(|item| item.quantity > 0)
            {
                state.amount = Some(Amount {
                    row,
                    value: 1,
                    max: item.quantity,
                });
                state.dirty = true;
            }
        }
        Kind::Cancel => {
            state.amount = None;
            state.confirm = None;
            state.dirty = true;
        }
        Kind::Confirm => {
            if let Some(amount) = state.amount.take()
                && let Some(panel) = &state.panel
                && let Some(intent) = visible_withdraw(panel, amount.row, amount.value)
            {
                play.link.stash_request_bound(panel, intent);
                state.dirty = true;
            }
        }
        Kind::Sort if state.amount.is_none() => play.link.stash_sort(),
        Kind::Previous if state.amount.is_none() => play.link.stash_request(Intent::Previous),
        Kind::Next if state.amount.is_none() => play.link.stash_request(Intent::Next),
        Kind::Refresh if state.amount.is_none() => play.link.stash_request(Intent::Refresh),
        _ => {
            return;
        }
    }
    state.filter = None;
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity)
        || control.generation != state.generation
        || control.revision != state.revision
    {
        return;
    }
    if control.kind == Kind::Filter && state.amount.is_none() && valid(&owner, &state) {
        state.filter = Some(event.entity);
    } else if control.kind == Kind::Confirm {
        state.confirm = Some(event.entity);
    } else {
        perform(&owner, &mut state, control.kind);
    }
}
fn finish_filter(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<(&Field, &EditableText)>,
    guard: ModalGuard,
) {
    let Some(entity) = state.filter else {
        return;
    };
    if !guard.allows(entity) || !valid(&owner, &state) {
        state.filter = None;
        return;
    }
    if fields.iter().any(|(_, text)| {
        text.pending_paste.is_some()
            || text
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
    }) {
        return;
    }
    let (mut search, mut category, mut trader) = (String::new(), String::new(), String::new());
    for (field, text) in &fields {
        let value = text.value().to_string();
        match field {
            Field::Search => search = value,
            Field::Category => category = value,
            Field::Trader => trader = value,
        }
    }
    state.filter = None;
    if let Some(play) = &owner.play {
        play.link.stash_filter(search, category, trader);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    mut fields: Query<&mut EditableText, With<Quantity>>,
) {
    let Some(root) = state.root else {
        events.clear();
        return;
    };
    if !guard.allows(root) {
        events.clear();
        return;
    }
    // Read the typed value once, then accumulate every key event in this frame.
    if let Ok(text) = fields.single()
        && let Ok(value) = text.value().to_string().trim().parse::<u32>()
        && let Some(amount) = &mut state.amount
    {
        amount.value = value.clamp(1, amount.max);
    }
    for event in events
        .read()
        .filter(|event| event.state == bevy::input::ButtonState::Pressed)
    {
        if !valid(&owner, &state) {
            continue;
        }
        if state.amount.is_some() {
            match event.key_code {
                KeyCode::Enter if !event.repeat => {
                    state.confirm = Some(root);
                    break;
                }
                KeyCode::Escape if !event.repeat => {
                    perform(&owner, &mut state, Kind::Cancel);
                    break;
                }
                key => {
                    if matches!(
                        key,
                        KeyCode::ArrowUp | KeyCode::ArrowDown | KeyCode::PageUp | KeyCode::PageDown
                    ) {
                        amount_key(&mut state, key);
                        if let Some(amount) = &state.amount
                            && let Ok(mut text) = fields.single_mut()
                        {
                            *text = EditableText::new(amount.value.to_string());
                            text.max_characters = Some(10);
                        }
                    }
                }
            }
        } else if event.key_code == KeyCode::Escape && !event.repeat {
            perform(&owner, &mut state, Kind::Close);
        }
    }
}
fn amount_key(state: &mut State, key: KeyCode) {
    let Some(amount) = &mut state.amount else {
        return;
    };
    amount.value = match key {
        KeyCode::ArrowUp => amount.value.saturating_add(10).min(amount.max),
        KeyCode::ArrowDown => amount.value.saturating_sub(10).max(1),
        KeyCode::PageUp => amount.max,
        KeyCode::PageDown => 1,
        _ => return,
    };
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    fields: Query<(), StashFields>,
) {
    let panel = owner
        .play
        .as_ref()
        .map(|play| play.link.stash_panel())
        .filter(|panel| panel.open);
    let changed = state.generation != owner.generation() || panel != state.panel;
    if !changed && !state.dirty {
        return;
    }
    if let Some(root) = state.root.take() {
        if focus.get().is_some_and(|entity| fields.contains(entity)) {
            focus.clear();
        }
        commands.entity(root).despawn();
    }
    if changed {
        state.amount = None;
        state.filter = None;
        state.confirm = None;
    }
    state.generation = owner.generation();
    state.revision = state.revision.wrapping_add(1);
    state.panel = panel;
    state.dirty = false;
    if let Some(panel) = &state.panel {
        state.root = Some(widgets::build(
            &mut commands,
            &skins,
            &state,
            panel,
            &mut focus,
        ));
    }
}

fn finish_confirm(
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    fields: Query<&EditableText, With<Quantity>>,
    guard: ModalGuard,
) {
    let Some(entity) = state.confirm else {
        return;
    };
    if !guard.allows(entity) || !valid(&owner, &state) {
        state.confirm = None;
        return;
    }
    let Ok(text) = fields.single() else {
        state.confirm = None;
        return;
    };
    if text.pending_paste.is_some()
        || text
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
    {
        return;
    }
    let Ok(value) = text.value().to_string().trim().parse::<u32>() else {
        state.confirm = None;
        return;
    };
    if let Some(amount) = &mut state.amount {
        amount.value = value.clamp(1, amount.max);
    }
    state.confirm = None;
    perform(&owner, &mut state, Kind::Confirm);
}

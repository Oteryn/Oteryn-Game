//! Supply stash over the retained panel and its existing command owner.
#[cfg(test)]
pub(super) mod tests;
mod widgets;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::stash::{
    Batch, BatchKind, Intent, StashPanel,
    container_batch::Request,
    stash::{StashAction, StashRowAction, StashRowIntent},
    visible_rows, visible_withdraw,
};
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
    ManageContainers,
    Sort,
    Reverse,
    Previous,
    Next,
    Refresh,
    Row(usize),
    Stow(usize),
    StowQuantity(usize),
    StowMatching(usize),
    StowContents(usize),
    Preview(usize),
    ConfirmPreview,
    RowAction(usize, StashRowAction),
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
    stow: bool,
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
    carried_containers: Vec<bool>,
    row_actions: [bool; 3],
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
    let Some(panel) = &state.panel else {
        return;
    };
    if matches!(panel.container_batch, Batch::Quote { .. })
        && !matches!(kind, Kind::ConfirmPreview | Kind::Cancel | Kind::Close)
    {
        return;
    }
    let stackable = |appearance| play.view.scene().stackable_flag(appearance);
    match kind {
        Kind::ManageContainers if state.amount.is_none() => {
            if !play.link.quick_loot_panel().selected {
                return;
            }
            play.link.stash_close();
            play.link.discovery_open(98);
        }
        Kind::Close => {
            play.link.stash_close();
            state.amount = None;
        }
        Kind::Row(row) | Kind::StowQuantity(row) => {
            if state.amount.is_some() {
                return;
            }
            if panel.pending || !panel.selected {
                return;
            }
            let stow = matches!(kind, Kind::StowQuantity(_));
            let max = if stow {
                panel
                    .carried
                    .get(row)
                    .filter(|_| panel.actions_selected)
                    .map(|item| item.count.get())
            } else {
                visible_rows(panel)
                    .get(row)
                    .map(|index| panel.rows[*index].quantity)
            };
            if let Some(max) = max.filter(|max| *max > 0) {
                state.amount = Some(Amount {
                    row,
                    value: 1,
                    max,
                    stow,
                });
                state.dirty = true;
            }
        }
        Kind::Cancel => {
            play.link.stash_cancel_preview();
            state.amount = None;
            state.confirm = None;
            state.dirty = true;
        }
        Kind::Confirm => {
            if let Some(amount) = state.amount.take() {
                if amount.stow {
                    if let Some(item) = panel.carried.get(amount.row) {
                        play.link.stash_stow_action(
                            panel,
                            StashAction::StowQuantity {
                                handle: item.handle,
                                amount: amount.value,
                            },
                            stackable,
                        );
                    }
                } else if let Some(intent) = visible_withdraw(panel, amount.row, amount.value) {
                    play.link.stash_request_bound(panel, intent);
                }
                state.dirty = true;
            }
        }
        Kind::Stow(row) if state.amount.is_none() => {
            play.link.stash_stow_selected(panel, row, stackable);
        }
        Kind::StowMatching(row) | Kind::StowContents(row) if state.amount.is_none() => {
            if let Some(item) = panel.carried.get(row) {
                let action = if matches!(kind, Kind::StowMatching(_)) {
                    StashAction::StowMatching(item.handle)
                } else {
                    StashAction::StowContents(item.handle)
                };
                play.link.stash_stow_action(panel, action, stackable);
            }
        }
        Kind::Preview(row) if state.amount.is_none() => {
            if let Some(item) = panel.carried.get(row) {
                play.link.stash_batch_action(
                    panel,
                    Request::Preview {
                        source: item.handle,
                        recursive: true,
                    },
                );
            }
        }
        Kind::ConfirmPreview => {
            if let Batch::Quote { id, .. } = panel.container_batch {
                play.link
                    .stash_batch_action(panel, Request::Confirm { preview: id });
            }
        }
        Kind::RowAction(row, action) if state.amount.is_none() => {
            if let Some(index) = visible_rows(panel).get(row) {
                play.link.stash_row_action(
                    panel,
                    StashRowIntent {
                        row: panel.rows[*index].handle,
                        action,
                    },
                );
            }
        }
        Kind::Reverse if state.amount.is_none() => play.link.stash_reverse(),
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
                        KeyCode::ArrowLeft
                            | KeyCode::ArrowRight
                            | KeyCode::ArrowUp
                            | KeyCode::ArrowDown
                            | KeyCode::PageUp
                            | KeyCode::PageDown
                    ) {
                        amount_key(&mut state, key);
                        if let Some(amount) = &state.amount
                            && let Ok(mut text) = fields.single_mut()
                        {
                            text.editor_mut().set_text(&amount.value.to_string());
                        }
                    }
                }
            }
        } else if state
            .panel
            .as_ref()
            .is_some_and(|panel| matches!(panel.container_batch, Batch::Quote { .. }))
        {
            match event.key_code {
                KeyCode::Enter if !event.repeat => {
                    perform(&owner, &mut state, Kind::ConfirmPreview)
                }
                KeyCode::Escape if !event.repeat => perform(&owner, &mut state, Kind::Cancel),
                _ => continue,
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
        KeyCode::ArrowRight => amount.value.saturating_add(1).min(amount.max),
        KeyCode::ArrowLeft => amount.value.saturating_sub(1).max(1),
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
    let (carried_containers, row_actions) = if let (Some(panel), Some(play)) = (&panel, &owner.play)
    {
        let views = play.link.container_panel().views;
        (
            panel
                .carried
                .iter()
                .map(|item| {
                    play.view.scene().container_flag(item.appearance_id) == Some(true)
                        || views.as_ref().is_some_and(|views| {
                            views.views.iter().any(|view| {
                                view.browse.is_none() && view.container_handle == item.handle
                            })
                        })
                })
                .collect::<Vec<_>>(),
            [
                play.link.inspection_panel().selected,
                play.link.market_panel().selected,
                play.link.quick_loot_panel().selected,
            ],
        )
    } else {
        (Vec::new(), [false; 3])
    };
    let changed = state.generation != owner.generation()
        || panel != state.panel
        || carried_containers != state.carried_containers
        || row_actions != state.row_actions;
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
    state.carried_containers = carried_containers;
    state.row_actions = row_actions;
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

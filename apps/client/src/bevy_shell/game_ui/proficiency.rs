//! Weapon list and level-gated perk grid over the retained proficiency owner.
mod draft;
mod filters;
#[cfg(test)]
mod tests;
mod track;
mod widgets;
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0195_weapon_proficiency::view::ProficiencyAction as ModelAction;
#[derive(Clone, Copy)]
enum Kind {
    Close,
    Refresh,
    Filter(filters::Change),
    Weapon(usize),
    Perk(usize, usize),
    Apply,
    Reset,
    ResetCommitted,
    SaveClose,
    DiscardClose,
    CancelClose,
    Slot(u8),
    Offer(u8),
    Modify(usize),
    Shape(oteryn_client_ui::weapon_proficiency::Intent),
}
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::weapon_proficiency::WeaponProficiencyPanel;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
struct Key {
    generation: u64,
    serial: u64,
}
#[derive(Component, Clone, Copy)]
struct Control {
    key: Key,
    kind: Kind,
}
#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct Progress;
#[derive(Component)]
struct Search;
#[derive(Resource, Default)]
struct State {
    key: Key,
    snapshot: Option<WeaponProficiencyPanel>,
    root: Option<widgets::Widgets>,
    draft: draft::Draft,
    dirty: bool,
    search: String,
    filters: filters::Filters,
}
pub(super) fn install(app: &mut App) {
    track::install(app);
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    time: Res<Time<Real>>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    fields: Query<&bevy::text::EditableText, With<Search>>,
) {
    let panel = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing)
        .and_then(|play| {
            if play.link.weapon_proficiency_is_open() && !state.draft.applying {
                play.link.weapon_proficiency_tick(std::time::Instant::now());
            }
            let panel = play.link.weapon_proficiency_panel();
            panel.open.then_some(panel)
        });
    if state.key.generation != owner.generation() || panel.is_none() {
        state.search.clear();
        state.filters = default();
    } else if let Ok(field) = fields.single()
        && field.value() != state.search.as_str()
    {
        state.search = field.value().to_string();
        state.dirty = true;
    }
    if let (Some(panel), Some(play)) = (&panel, &owner.play)
        && state.filters.update(&play.link, panel, time.elapsed())
    {
        state.dirty = true;
    }
    draft::advance(&owner, &mut state, panel.as_ref());
    if !state.dirty && state.key.generation == owner.generation() && state.snapshot == panel {
        return;
    }
    if (state.key.generation != owner.generation() || panel.is_none())
        && let Some(widgets) = state.root.take()
    {
        commands.entity(widgets.overlay).despawn();
    }
    state.dirty = false;
    state.key = Key {
        generation: owner.generation(),
        serial: state.key.serial.saturating_add(1),
    };
    let key = state.key;
    if let Some(panel) = &panel {
        let mut projected = draft::projection(panel, &state.draft);
        projected.search.clone_from(&state.search);
        let state = &mut *state;
        let filters = &state.filters;
        let draft_controls = state.draft.controls();
        let widgets = state
            .root
            .get_or_insert_with(|| widgets::build(&mut commands, &skins));
        widgets::render(&mut commands, &skins, widgets, key, &projected, filters);
        draft::render(&mut commands, &skins, widgets, key, draft_controls);
    }
    state.snapshot = panel;
}
fn perform(owner: &HostOwner, state: &mut State, control: Control) {
    if owner.phase != Phase::Playing || owner.modal_active() || state.key != control.key {
        return;
    }
    if let (Some(play), Some(expected)) = (&owner.play, &state.snapshot)
        && owner.generation() == control.key.generation
        && (matches!(control.kind, Kind::Close) || !play.view.gameplay_input_blocked(&play.link))
    {
        let expected = expected.clone();
        draft::perform(&play.link, state, &expected, control.kind);
    }
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        perform(&owner, &mut state, *control);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    overlays: Query<Entity, With<Overlay>>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    for event in events.read() {
        if event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
            && event.key_code == KeyCode::Escape
            && overlays.iter().any(|entity| guard.allows(entity))
        {
            let control = Control {
                key: state.key,
                kind: Kind::Close,
            };
            perform(&owner, &mut state, control);
        }
    }
}

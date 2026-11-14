//! Coin views, review and retry project the retained MARKET_V1 coin owner.
mod widgets;
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    GameModal,
    modal::ModalGuard,
};
use bevy::{input_focus::InputFocus, prelude::*, text::EditableText, ui_widgets::Activate};
use oteryn_client_ui::market::CoinPanel;
use oteryn_client_ui_bevy as source;
#[derive(Component)]
struct Field(usize);
#[derive(Component)]
struct Control {
    generation: u64,
    binding: usize,
    revision: u64,
    action: Option<usize>,
}
#[derive(Resource, Default)]
struct State {
    generation: u64,
    binding: usize,
    revision: u64,
    seen: Option<CoinPanel>,
    root: Option<Entity>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>().add_systems(Update, project);
}
fn available(owner: &HostOwner) -> bool {
    owner.phase == Phase::Playing
        && !owner.modal_active()
        && owner
            .play
            .as_ref()
            .is_some_and(|play| !play.view.gameplay_input_blocked(&play.link))
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    fields: Query<(&Field, &EditableText)>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    guard: ModalGuard,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity)
        || !available(&owner)
        || control.generation != owner.generation()
        || control.generation != state.generation
        || control.binding != state.binding
        || control.revision != state.revision
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let Some(panel) = &state.seen else {
        return;
    };
    if control.binding != play.link.market_session_binding()
        || play.link.market_panel().coins != *panel
    {
        return;
    }
    let Some(action) = control.action else {
        play.link.coin_market_close_bound(control.binding, panel);
        return;
    };
    let mut values = panel.fields.clone();
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
    play.link
        .coin_market_bound(control.binding, panel, action, values);
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    fields: Query<(), With<Field>>,
) {
    let panel = owner
        .play
        .as_ref()
        .filter(|_| available(&owner))
        .map(|play| {
            (
                play.link.market_session_binding(),
                play.link.market_panel().coins,
            )
        })
        .filter(|(_, panel)| panel.open);
    let binding = panel.as_ref().map_or(0, |(binding, _)| *binding);
    let panel = panel.map(|(_, panel)| panel);
    if state.binding == binding && state.generation == owner.generation() && state.seen == panel {
        return;
    }
    if focus.get().is_some_and(|entity| fields.contains(entity)) {
        focus.clear();
    }
    if let Some(root) = state.root.take() {
        commands.entity(root).despawn();
    }
    state.binding = binding;
    state.generation = owner.generation();
    state.revision = state.revision.wrapping_add(1);
    state.seen = panel;
    if let Some(panel) = &state.seen {
        state.root = Some(widgets::build(&mut commands, &skins, &state, panel));
    }
}

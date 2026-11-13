//! Source-shaped quest journal and spell archive over the retained feature owners.
mod actions;
mod model;
mod widgets;
use super::modal::ModalGuard;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
};
use bevy::{
    input_focus::{FocusCause, InputFocus},
    prelude::*,
    text::EditableText,
    ui::Checked,
    ui_widgets::Activate,
};
use oteryn_client_ui_bevy as source;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Tab {
    #[default]
    Quests,
    Spells,
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    generation: u64,
    tab: Tab,
}
#[derive(Clone, Copy, Component)]
struct Action {
    key: Key,
    kind: Kind,
}
#[derive(Clone, Copy)]
enum Kind {
    Quests,
    Spells,
    Close,
    Refresh,
    Search,
    Sort,
    Track,
    Quest(u32),
    Mission(u32, u32),
    Spell(u32),
    Previous,
    Next,
    Vocation,
    Premium,
    Group,
    Assign(usize),
}
#[derive(Clone, Copy, Component, PartialEq, Eq)]
enum Field {
    Search,
    Minimum,
    Maximum,
}
#[derive(Component)]
struct Overlay;
#[derive(Component)]
struct Detail;
#[derive(Component)]
struct Status;
#[derive(Component)]
struct FilterCaption(Kind);
#[derive(Resource, Default)]
struct State {
    key: Option<Key>,
    tab: Tab,
    widgets: Option<widgets::Widgets>,
    selected_spell: Option<u32>,
    rows: Vec<model::Row>,
    missions: Vec<model::Row>,
    feedback: String,
    quest_page: usize,
}
pub(super) fn open_quests(owner: &mut HostOwner) {
    if owner.modal_active() {
        return;
    }
    if let Some(play) = &owner.play {
        play.link.spellbook_set_open(false);
        play.link.quest_set_open(true);
        play.link
            .quest_request(&oteryn_client_ui::quest_log::QuestLogQuery::List);
    }
}
pub(super) fn open_spells(owner: &mut HostOwner) {
    if owner.modal_active() {
        return;
    }
    if let Some(play) = &owner.play {
        play.link.quest_set_open(false);
        play.link.spell_catalog_open();
    }
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Startup, setup.after(super::setup))
        .add_systems(Update, (keyboard, project, refresh_labels).chain());
}
fn setup(mut commands: Commands) {
    commands.spawn((
        Overlay,
        GameModal,
        GlobalZIndex(155),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100.),
            height: percent(100.),
            display: Display::None,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
    ));
}
// Bevy injects independent projection, hierarchy and focus owners.
#[allow(clippy::too_many_arguments)]
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<State>,
    mut overlays: Query<(Entity, &mut Node), With<Overlay>>,
    mut text: Query<(&mut Text, Has<Detail>, Has<Status>)>,
    mut focus: ResMut<InputFocus>,
    controls: Query<(), Or<(With<Field>, With<Action>)>>,
) {
    let Some(play) = &owner.play else {
        hide(
            &mut commands,
            &mut state,
            &mut overlays,
            &mut focus,
            &controls,
        );
        return;
    };
    if !play.link.quest_window_open() && !play.link.spellbook_window_open() {
        hide(
            &mut commands,
            &mut state,
            &mut overlays,
            &mut focus,
            &controls,
        );
        return;
    }
    let quests = play.link.quest_panel();
    let book = play.link.spellbook_state();
    let catalogue = play.link.spell_catalog_panel();
    let q = quests.open || quests.tracker_open;
    let s = book.open || catalogue.open || catalogue.archive_open;
    if !q && !s {
        hide(
            &mut commands,
            &mut state,
            &mut overlays,
            &mut focus,
            &controls,
        );
        return;
    }
    let tab = if q && !s {
        Tab::Quests
    } else if s && !q {
        Tab::Spells
    } else {
        state.tab
    };
    let key = Key {
        generation: owner.generation(),
        tab,
    };
    let Ok((overlay, mut node)) = overlays.single_mut() else {
        return;
    };
    node.display = Display::Flex;
    if state.key != Some(key) {
        if let Some(old) = state.widgets.take() {
            commands.entity(old.root).despawn();
        }
        state.key = Some(key);
        state.tab = tab;
        state.selected_spell = None;
        state.rows.clear();
        state.missions.clear();
        state.quest_page = 0;
        state.feedback.clear();
        let built = widgets::build(
            &mut commands,
            &skin,
            overlay,
            key,
            &quests,
            &book,
            &catalogue,
        );
        if !owner.modal_active() {
            focus.set(built.search, FocusCause::Navigated);
        }
        state.widgets = Some(built);
    }
    let Some(widgets) = state.widgets else {
        return;
    };
    let projection = model::project(
        tab,
        &quests,
        &book,
        &catalogue,
        state.selected_spell,
        state.quest_page,
    );
    state.selected_spell = projection.selected_spell;
    if state.rows != projection.rows {
        commands.entity(widgets.rows).despawn_children();
        widgets::rows(&mut commands, &skin, widgets.rows, key, &projection.rows);
        state.rows = projection.rows;
    }
    if state.missions != projection.missions {
        commands.entity(widgets.missions).despawn_children();
        widgets::rows(
            &mut commands,
            &skin,
            widgets.missions,
            key,
            &projection.missions,
        );
        state.missions = projection.missions;
    }
    for (mut text, detail, status) in &mut text {
        let value = if detail {
            Some(projection.detail.as_str())
        } else if status {
            Some(if state.feedback.is_empty() {
                projection.status.as_str()
            } else {
                state.feedback.as_str()
            })
        } else {
            None
        };
        if let Some(value) = value
            && text.0 != value
        {
            text.0 = value.into();
        }
    }
}
fn hide(
    commands: &mut Commands,
    state: &mut State,
    overlays: &mut Query<(Entity, &mut Node), With<Overlay>>,
    focus: &mut InputFocus,
    controls: &Query<(), Or<(With<Field>, With<Action>)>>,
) {
    for (_, mut node) in overlays {
        node.display = Display::None;
    }
    if let Some(widgets) = state.widgets.take() {
        if focus.get().is_some_and(|id| controls.contains(id)) {
            focus.clear();
        }
        commands.entity(widgets.root).despawn();
    }
    state.key = None;
    state.rows.clear();
    state.missions.clear();
}
fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    actions: Query<&Action>,
    fields: Query<(&Field, &EditableText)>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<State>,
) {
    if guard.allows(event.entity)
        && let Ok(action) = actions.get(event.entity)
    {
        actions::perform(&mut owner, &mut state, *action, &actions::values(&fields));
    }
}
fn keyboard(
    focus: Res<InputFocus>,
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    overlays: Query<Entity, With<Overlay>>,
    fields: Query<(&Field, &EditableText)>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<State>,
) {
    if owner.modal_active()
        || (!guard.allows_focus(None)
            && !overlays.single().is_ok_and(|overlay| guard.allows(overlay)))
    {
        events.clear();
        return;
    }
    for event in events.read() {
        if event.state != bevy::input::ButtonState::Pressed || event.repeat {
            continue;
        }
        let Some(key) = state.key else {
            continue;
        };
        let kind = match event.key_code {
            KeyCode::Escape => Kind::Close,
            KeyCode::Enter | KeyCode::NumpadEnter
                if focus.get().is_some_and(|id| fields.get(id).is_ok()) =>
            {
                Kind::Search
            }
            _ => continue,
        };
        actions::perform(
            &mut owner,
            &mut state,
            Action { key, kind },
            &actions::values(&fields),
        );
    }
}
fn refresh_labels(owner: NonSend<HostOwner>, mut labels: Query<(&FilterCaption, &mut Text)>) {
    if labels.is_empty() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let quest = play.link.quest_panel();
    let book = play.link.spellbook_state();
    for (label, mut text) in &mut labels {
        let value: String = match label.0 {
            Kind::Sort => {
                oteryn_client_ui::quest_log::QUEST_SORT_LABELS[quest.list_sort.selection()].into()
            }
            Kind::Vocation => {
                crate::play::feature_0287_actor_spell_catalog::view::SPELLBOOK_VOCATIONS
                    .get(usize::from(book.filter.vocation))
                    .copied()
                    .unwrap_or("Any vocation")
                    .into()
            }
            Kind::Premium => ["All accounts", "Free spells", "Premium spells"]
                .get(usize::from(book.filter.premium))
                .copied()
                .unwrap_or("All accounts")
                .into(),
            Kind::Group => crate::play::feature_0287_actor_spell_catalog::view::SPELLBOOK_GROUPS
                .get(usize::from(book.filter.group))
                .copied()
                .unwrap_or("Other recovery category")
                .into(),
            _ => continue,
        };
        if text.0 != value {
            text.0 = value;
        }
    }
}
#[cfg(test)]
mod tests;

#[cfg(test)]
mod input_tests;

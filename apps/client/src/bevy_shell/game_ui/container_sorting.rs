//! Container sorting is a projection over the existing session owner and dispatcher.
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameModal,
    modal::ModalGuard,
};
use crate::play::feature_0146_container_sort::view::{
    ContainerSortingControl as Kind, ContainerSortingSnapshot as Snapshot,
};
use bevy::{input::ButtonState, input_focus::InputFocus, prelude::*, ui_widgets::Activate};
use oteryn_client_ui_bevy as source;
#[derive(Component)]
struct Overlay;
#[derive(Component, Clone)]
struct Control {
    generation: u64,
    snapshot: Snapshot,
    kind: Action,
}
#[derive(Clone, PartialEq, Eq)]
enum Action {
    Owner(Kind),
    Confirm,
    Cancel,
}
#[derive(Resource, Default)]
struct State {
    shown: Option<(u64, Snapshot, bool)>,
    overlay: Option<Entity>,
    confirmation: Option<(u64, Snapshot)>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
pub(super) fn open(owner: &mut HostOwner) {
    if !owner.modal_active()
        && let Some(play) = &owner.play
        && !play.link.server_modal_active()
    {
        play.link.container_sort_open();
    }
}
fn current(owner: &HostOwner, control: &Control) -> bool {
    control.generation == owner.generation()
        && !owner.modal_active()
        && owner.play.as_ref().is_some_and(|play| {
            !play.link.server_modal_active()
                && play.link.container_sorting_snapshot() == control.snapshot
        })
}
fn perform(owner: &HostOwner, state: &mut State, control: &Control) {
    if !current(owner, control) {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    match &control.kind {
        Action::Confirm => {
            if control.snapshot.panel.selected && !control.snapshot.panel.pending {
                state.confirmation = Some((control.generation, control.snapshot.clone()));
            }
        }
        Action::Cancel => state.confirmation = None,
        Action::Owner(kind) => {
            if matches!(kind, Kind::Sort { .. })
                && state.confirmation.as_ref()
                    != Some(&(control.generation, control.snapshot.clone()))
            {
                return;
            }
            let _ = play
                .link
                .container_sorting_dispatch_bound(&control.snapshot, kind.clone());
            state.confirmation = None;
        }
    }
}
fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    controls: Query<&Control>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        perform(&owner, &mut state, control);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    for event in events.read() {
        if event.state != ButtonState::Pressed || event.repeat {
            continue;
        }
        let chord = keys.as_ref().is_some_and(|keys| {
            (keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight))
                && (keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
                && !keys.pressed(KeyCode::ShiftLeft)
                && !keys.pressed(KeyCode::ShiftRight)
                && !keys.pressed(KeyCode::SuperLeft)
                && !keys.pressed(KeyCode::SuperRight)
        });
        if event.key_code == KeyCode::F9 && chord && guard.allows_focus(state.overlay) {
            if let Some(play) = &owner.play
                && !owner.modal_active()
            {
                let snapshot = play.link.container_sorting_snapshot();
                if snapshot.panel.open {
                    close(&owner, &mut state);
                } else if !play.link.server_modal_active() {
                    play.link.container_sort_open();
                }
            }
        } else if event.key_code == KeyCode::Escape
            && state.overlay.is_some_and(|overlay| guard.allows(overlay))
        {
            close(&owner, &mut state);
        }
    }
}
fn close(owner: &HostOwner, state: &mut State) {
    let Some((generation, snapshot, _)) = &state.shown else {
        return;
    };
    let control = Control {
        generation: *generation,
        snapshot: snapshot.clone(),
        kind: Action::Owner(Kind::Close),
    };
    perform(owner, state, &control);
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
) {
    let snapshot = owner
        .play
        .as_ref()
        .map(|play| play.link.container_sorting_snapshot())
        .filter(|snapshot| snapshot.panel.open);
    let generation = owner.generation();
    if state
        .confirmation
        .as_ref()
        .is_some_and(|(old, expected)| *old != generation || Some(expected) != snapshot.as_ref())
    {
        state.confirmation = None;
    }
    let projected = snapshot.map(|snapshot| (generation, snapshot, state.confirmation.is_some()));
    if state.shown == projected {
        return;
    }
    if let Some(overlay) = state.overlay.take() {
        let mut focused = focus.get();
        while let Some(entity) = focused {
            if entity == overlay {
                focus.clear();
                break;
            }
            focused = parents.get(entity).ok().map(ChildOf::parent);
        }
        commands.entity(overlay).despawn();
    }
    state.shown = projected.clone();
    let Some((generation, snapshot, confirming)) = projected else {
        return;
    };
    let overlay = commands
        .spawn((
            Overlay,
            GameModal,
            bevy::input_focus::tab_navigation::TabGroup {
                order: 2,
                modal: true,
            },
            GlobalZIndex(165),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ))
        .id();
    state.overlay = Some(overlay);
    let panel = commands
        .spawn((
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(560.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect::all(px(20.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            bevy::ui_widgets::ScrollArea,
        ))
        .id();
    commands.entity(overlay).add_child(panel);
    source::dialog_caption(
        &mut commands,
        panel,
        "Container sorting",
        skins.font.clone(),
    );
    let model = Control {
        generation,
        snapshot: snapshot.clone(),
        kind: Action::Cancel,
    };
    text(
        &mut commands,
        &skins,
        panel,
        &oteryn_client_ui::container_sort::panel_text(&snapshot.panel),
    );
    for (index, label) in [
        "Open container view (0-15)",
        "Sort key: 1 name, 2 weight, 3 expiry, 4 stack",
        "Descending: 0 no, 1 yes",
        "Recursive: 0 no, 1 yes",
    ]
    .iter()
    .enumerate()
    {
        text(
            &mut commands,
            &skins,
            panel,
            &format!("{label}: {}", snapshot.panel.fields[index]),
        );
        let values: Vec<String> = if index == 0 {
            snapshot
                .views
                .as_ref()
                .map(|views| {
                    views
                        .views
                        .iter()
                        .filter(|view| view.browse.is_none())
                        .map(|view| view.view_id.to_string())
                        .collect()
                })
                .unwrap_or_default()
        } else if index == 1 {
            (1..=4).map(|value| value.to_string()).collect()
        } else {
            vec!["0".into(), "1".into()]
        };
        let row = commands
            .spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(4.),
                ..default()
            })
            .id();
        commands.entity(panel).add_child(row);
        for value in values {
            button(
                &mut commands,
                &skins,
                row,
                &model,
                Action::Owner(Kind::Edit(index, value.clone())),
                &value,
            );
        }
    }
    if confirming {
        text(
            &mut commands,
            &skins,
            panel,
            "Confirm sorting this container with the options above?",
        );
        button(
            &mut commands,
            &skins,
            panel,
            &model,
            Action::Owner(Kind::Sort { confirmed: true }),
            "Confirm sort",
        );
        button(
            &mut commands,
            &skins,
            panel,
            &model,
            Action::Cancel,
            "Cancel",
        );
    } else {
        button(
            &mut commands,
            &skins,
            panel,
            &model,
            Action::Confirm,
            "Sort",
        );
    }
    button(
        &mut commands,
        &skins,
        panel,
        &model,
        Action::Owner(Kind::Close),
        "Close (Ctrl+Alt+F9)",
    );
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: &str) {
    let text = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .insert(TextLayout::default())
        .id();
    commands.entity(parent).add_child(text);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    model: &Control,
    kind: Action,
    caption: &str,
) {
    let close = kind == Action::Owner(Kind::Close);
    let mut button = commands.spawn((
        source::button(skins.buttons.clone(), 220.),
        Control {
            kind,
            ..model.clone()
        },
    ));
    if !close && (!model.snapshot.panel.selected || model.snapshot.panel.pending) {
        button.insert(bevy::ui::InteractionDisabled);
    }
    let button = button.observe(activate).id();
    let caption = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(button).add_child(caption);
    commands.entity(parent).add_child(button);
}

#[cfg(test)]
mod tests;

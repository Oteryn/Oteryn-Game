//! Public item encyclopedia; inventory/player inspection remains a separate window.
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    GameModal,
    modal::ModalGuard,
};
use bevy::{
    input_focus::InputFocus,
    prelude::*,
    text::EditableText,
    ui::InteractionDisabled,
    ui_widgets::{Activate, ScrollArea, TextInput},
};
use oteryn_client_ui::inspection::Panel;
use oteryn_client_ui_bevy as source;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Action {
    Browse,
    Next,
    Category(bool),
    Select(u32),
    Details,
    Prices,
    Close,
}
#[derive(Component, Clone, Copy)]
struct Control {
    generation: u64,
    serial: u64,
    action: Action,
}
#[derive(Component)]
struct Search;
#[derive(Resource, Default)]
struct State {
    overlay: Option<Entity>,
    snapshot: Option<(u64, Panel, String)>,
    serial: u64,
    pending: Option<(Entity, Control)>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(
            Update,
            (
                keyboard,
                finish.after(bevy::text::EditableTextSystems),
                project,
            )
                .chain(),
        );
}
fn keyboard(
    keys: Res<ButtonInput<KeyCode>>,
    owner: NonSend<HostOwner>,
    state: Res<State>,
    guard: ModalGuard,
) {
    if owner.phase != Phase::Playing || owner.modal_active() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if let Some(overlay) = state.overlay
        && guard.allows(overlay)
        && keys.just_pressed(KeyCode::Escape)
    {
        play.link.item_catalogue_set_open(false);
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
    search: Query<&EditableText, With<Search>>,
) {
    let model = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing)
        .map(|play| {
            (
                play.link.inspection_panel(),
                play.link.item_catalogue_text(),
            )
        })
        .filter(|(p, _)| p.catalogue_open);
    let snapshot = model.map(|(panel, text)| (owner.generation(), panel, text));
    if state.snapshot == snapshot {
        return;
    }
    if let Some(overlay) = state.overlay.take() {
        let mut current = focus.get();
        while let Some(entity) = current {
            if entity == overlay {
                focus.clear();
                break;
            }
            current = parents.get(entity).ok().map(ChildOf::parent);
        }
        commands.entity(overlay).despawn();
    }
    let search_value = snapshot
        .as_ref()
        .map(|(generation, panel, _)| {
            if state
                .snapshot
                .as_ref()
                .is_some_and(|(previous, _, _)| previous == generation)
            {
                search
                    .single()
                    .map(|text| text.value().to_string())
                    .unwrap_or_else(|_| panel.search.clone())
            } else {
                panel.search.clone()
            }
        })
        .unwrap_or_default();
    state.snapshot = snapshot;
    let Some((generation, panel, text)) = &state.snapshot else {
        return;
    };
    let Some(serial) = state.serial.checked_add(1) else {
        return;
    };
    let control = Control {
        generation: *generation,
        serial,
        action: Action::Close,
    };
    let overlay = build(&mut commands, &skin, panel, text, &search_value, control);
    state.serial = serial;
    state.overlay = Some(overlay);
}
fn build(
    commands: &mut Commands,
    skin: &Skins,
    panel: &Panel,
    details: &str,
    search_value: &str,
    control: Control,
) -> Entity {
    let overlay = commands
        .spawn((
            GameModal,
            GlobalZIndex(170),
            bevy::input_focus::tab_navigation::TabGroup {
                order: 2,
                modal: true,
            },
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
    let window = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(820.),
                height: px(620.),
                max_width: percent(95.),
                max_height: percent(94.),
                padding: UiRect::all(px(22.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(window);
    source::dialog_caption(commands, window, "Cyclopedia Items", skin.font.clone());
    label(
        commands,
        skin,
        window,
        "Search item name; choose a category, then Browse. Prices include market offers and completed trades.",
    );
    let search = commands
        .spawn((
            Search,
            TextInput,
            EditableText::new(search_value),
            source::font(skin.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            Node {
                width: percent(100.),
                min_height: px(24.),
                ..default()
            },
        ))
        .id();
    if panel.pending || !panel.selected {
        commands.entity(search).insert(InteractionDisabled);
    }
    commands.entity(window).add_child(search);
    label(
        commands,
        skin,
        window,
        &format!(
            "Category: {}",
            if panel.category.is_empty() {
                "All"
            } else {
                &panel.category
            }
        ),
    );
    let buttons = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(window).add_child(buttons);
    let selected = panel
        .catalogue_definition
        .parse::<u32>()
        .ok()
        .is_some_and(|id| panel.catalogue.contains_key(&id));
    for (action, caption, enabled) in [
        (Action::Browse, "Browse", true),
        (Action::Next, "Next page", panel.catalogue_next.is_some()),
        (
            Action::Category(true),
            "Previous category",
            !panel.categories().is_empty(),
        ),
        (
            Action::Category(false),
            "Next category",
            !panel.categories().is_empty(),
        ),
        (Action::Details, "Item details", selected),
        (Action::Prices, "Market prices", selected),
        (Action::Close, "Close", true),
    ] {
        button(
            commands,
            skin,
            buttons,
            Control { action, ..control },
            caption,
            action != Action::Close && (!enabled || panel.pending || !panel.selected),
        );
    }
    let scroll = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                flex_grow: 1.,
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(4.),
                ..default()
            },
        ))
        .id();
    commands.entity(window).add_child(scroll);
    for item in panel.catalogue.values() {
        button(
            commands,
            skin,
            scroll,
            Control {
                action: Action::Select(item.definition_ref),
                ..control
            },
            &item.name,
            panel.pending || !panel.selected,
        );
    }
    label(commands, skin, scroll, &details.replace("\r\n", "\n"));
    if panel.pending {
        label(commands, skin, scroll, "Waiting for the server");
    }
    overlay
}
fn label(commands: &mut Commands, skin: &Skins, parent: Entity, text: &str) {
    let child = commands
        .spawn((
            source::text(text, source::font(skin.font.clone(), false)),
            Node {
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(child);
}
fn button(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    control: Control,
    caption: &str,
    disabled: bool,
) {
    let button = commands
        .spawn((source::button(skin.buttons.clone(), 130.), control))
        .observe(activate)
        .id();
    if disabled {
        commands.entity(button).insert(InteractionDisabled);
    }
    let caption = commands
        .spawn(source::button_caption(caption, skin.font.clone()))
        .id();
    commands.entity(button).add_child(caption);
    commands.entity(parent).add_child(button);
}
fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    controls: Query<&Control>,
    mut state: ResMut<State>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        state.pending = Some((event.entity, *control));
    }
}
fn finish(
    guard: ModalGuard,
    search: Query<&EditableText, With<Search>>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Some((entity, control)) = state.pending.take() else {
        return;
    };
    if !guard.allows(entity) || owner.modal_active() || owner.phase != Phase::Playing {
        return;
    }
    if control.generation != owner.generation() || control.serial != state.serial {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.inspection_panel();
    if !panel.catalogue_open
        || state
            .snapshot
            .as_ref()
            .is_none_or(|(_, previous, _)| previous != &panel)
    {
        return;
    }
    if control.action == Action::Close {
        play.link.item_catalogue_set_open(false);
        return;
    }
    if panel.pending || !panel.selected || play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    match control.action {
        Action::Select(id) => {
            play.link.item_catalogue_select(id);
        }
        Action::Details => play.link.item_catalogue_request(6),
        Action::Prices => {
            play.link.item_catalogue_request(9);
            if let Ok(definition) = panel.catalogue_definition.parse() {
                play.link
                    .market_query(oteryn_client_ui::market::details::Query::Statistics {
                        definition,
                    });
            }
        }
        Action::Browse | Action::Next | Action::Category(_) => {
            let mut changed = false;
            if let Ok(search) = search.single() {
                if search.pending_paste.is_some()
                    || search
                        .pending_edits
                        .iter()
                        .any(bevy::text::TextEdit::is_destructive)
                {
                    state.pending = Some((entity, control));
                    return;
                }
                changed = search.value().to_string() != panel.search;
                play.link
                    .item_catalogue_filter(&search.value().to_string(), &panel.category);
            }
            let action = match control.action {
                Action::Next if !changed => 1,
                Action::Category(true) => 2,
                Action::Category(false) => 3,
                _ => 0,
            };
            play.link.item_catalogue_request(action);
        }
        Action::Close => {}
    }
}
#[cfg(test)]
mod tests;

//! Shared desktop character picker over server-owned rows and the existing selector callback.
mod bridge;
mod training;
use super::{
    game_ui::{GameModal, modal::ModalGuard},
    owner::HostOwner,
};
use crate::character_selection::SelectionAction;
use bevy::{
    input_focus::{FocusCause, InputFocus, tab_navigation::TabGroup},
    prelude::*,
    ui::{Checked, InteractionDisabled},
    ui_widgets::Activate,
};
pub(super) use bridge::SelectionBridge;
use oteryn_client_ui_bevy as source;

/// Tibia Global character list: column widths, Character fills the rest.
const OUTFIT_WIDTH: f32 = 66.;
const COLUMNS: [(&str, f32); 4] = [
    ("Status", 55.),
    ("Level", 50.),
    ("Vocation", 115.),
    ("World", 200.),
];
#[derive(Component)]
pub(super) struct Picker;
/// Stable semantic identifiers for native UI audits; no character identity is exported.
#[derive(Component)]
pub(super) struct PickerControl {
    pub request_id: u64,
    pub id: String,
}
#[derive(Component, Clone)]
enum Choice {
    Row(usize),
    Channel,
    Play,
    Train,
    /// Adds or removes the selected row in the offline-training batch.
    Mark,
    /// Trains every marked character, one after another.
    TrainMarked,
    Refresh,
    Create,
    Account,
    Cancel,
}
#[derive(Component)]
struct Target {
    request: u64,
    choice: Choice,
}
#[derive(Resource, Default)]
pub(super) struct PickerState {
    request: Option<u64>,
    selected: Option<usize>,
    channel: Option<usize>,
    /// Show outfits: 66px rows with the outfit column, otherwise 29px rows.
    outfits: bool,
    revision: u64,
    rendered: Option<(u64, u64)>,
    last_click: Option<(u64, usize, std::time::Instant)>,
    /// Rows marked for offline training with the channel chosen when marked, in marking order.
    marked: Vec<(usize, Option<usize>)>,
}
#[derive(Resource)]
pub(super) struct Skin {
    buttons: source::ButtonImages,
    frame: Handle<Image>,
    inset: Handle<Image>,
    checked: Handle<Image>,
    unchecked: Handle<Image>,
    font: Handle<Font>,
}
/// The Show outfits checkbox of one request.
#[derive(Component)]
struct OutfitToggle(u64);

pub(super) fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(PickerState::default());
    commands.insert_resource(Skin {
        buttons: source::ButtonImages {
            up: assets.load("images/skin/classic/button-grey-up.png"),
            down: assets.load("images/skin/classic/button-grey-down.png"),
        },
        frame: assets.load("images/skin/classic/dialog-frame-borderimage.png"),
        inset: assets.load("images/1pixel-down-frame.png"),
        checked: assets.load("images/skin/classic/checkbox-checked.png"),
        unchecked: assets.load("images/skin/classic/checkbox-unchecked.png"),
        font: assets.load("fonts/verdanab.ttf"),
    });
    commands.spawn((
        Picker,
        GameModal,
        GlobalZIndex(200),
        Node {
            position_type: PositionType::Absolute,
            width: percent(100.),
            height: percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0., 0., 0., 0.7)),
    ));
}
pub(super) fn poll(mut owner: NonSendMut<HostOwner>) {
    owner.selection.poll();
}

pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skin>,
    mut state: ResMut<PickerState>,
    mut roots: Query<(Entity, &mut Node), With<Picker>>,
    mut focus: ResMut<InputFocus>,
    controls: Query<&PickerControl>,
) {
    let Some(request) = owner.selection.pending() else {
        if focus
            .get()
            .is_some_and(|entity| controls.get(entity).is_ok())
        {
            focus.clear();
        }
        for (root, mut node) in &mut roots {
            node.display = Display::None;
            if state.request.is_some() {
                commands.entity(root).despawn_children();
            }
        }
        *state = PickerState::default();
        return;
    };
    if state.request != Some(request.id) {
        let outfits = state.outfits;
        *state = PickerState {
            request: Some(request.id),
            selected: request
                .rows
                .iter()
                .position(|row| row.available && row.character.character_id == request.preselect)
                .or_else(|| request.rows.iter().position(|row| row.available)),
            outfits,
            ..PickerState::default()
        };
    }
    let key = (request.id, state.revision);
    if state.rendered == Some(key) {
        return;
    }
    for (root, mut node) in &mut roots {
        node.display = Display::Flex;
        commands.entity(root).despawn_children();
        let panel = commands
            .spawn((
                super::window_drag::DraggableDialog,
                Name::new("selection.window"),
                TabGroup {
                    order: 5,
                    modal: true,
                },
                source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
                // Tibia Global character list: 745 wide around a 340px scrolling list.
                crate::bevy_shell::ui::window_layout::window(
                    "selection",
                    Node {
                        max_width: percent(95.),
                        max_height: percent(95.),
                        padding: UiRect {
                            left: px(16.),
                            right: px(16.),
                            top: px(29.),
                            bottom: px(16.),
                        },
                        flex_direction: FlexDirection::Column,
                        row_gap: px(5.),
                        overflow: Overflow::clip(),
                        ..default()
                    },
                ),
            ))
            .id();
        commands.entity(root).add_child(panel);
        source::dialog_caption(&mut commands, panel, "Select Character", skin.font.clone());
        let mut header = Vec::new();
        if state.outfits {
            header.push(("Outfit".to_owned(), Some(OUTFIT_WIDTH)));
        }
        header.push(("Character".to_owned(), None));
        header.extend(COLUMNS.map(|(title, width)| (title.to_owned(), Some(width))));
        let heading = line(&mut commands, &skin, &header, 13.);
        commands
            .entity(heading)
            .insert(Name::new("selection.columns"));
        commands.entity(panel).add_child(heading);
        let list = commands
            .spawn((
                bevy::ui_widgets::ScrollArea,
                Name::new("selection.list"),
                source::frame(skin.inset.clone(), [1.; 4]),
                Node {
                    width: percent(100.),
                    height: px(340.),
                    min_height: px(0.),
                    flex_grow: 1.,
                    flex_shrink: 1.,
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
            ))
            .id();
        commands.entity(panel).add_child(list);
        let height = if state.outfits { 66. } else { 29. };
        for (index, row) in request.rows.iter().enumerate() {
            // OwnerCharacter carries no level, vocation or outfit: those cells stay empty.
            let mut cells = Vec::new();
            if state.outfits {
                cells.push((String::new(), Some(OUTFIT_WIDTH)));
            }
            cells.push((
                format!("{}{}", row.character.name, training::suffix(&state, index)),
                None,
            ));
            cells.push((
                if row.available { "" } else { "Unavailable" }.to_owned(),
                Some(COLUMNS[0].1),
            ));
            cells.push((String::new(), Some(COLUMNS[1].1)));
            cells.push((String::new(), Some(COLUMNS[2].1)));
            cells.push((row.world_name.clone(), Some(COLUMNS[3].1)));
            let entry = line(&mut commands, &skin, &cells, height);
            let id = format!("selection.row.{index}");
            commands
                .entity(entry)
                .insert((
                    bevy::ui_widgets::Button,
                    bevy::input_focus::tab_navigation::TabIndex(0),
                    Target {
                        request: request.id,
                        choice: Choice::Row(index),
                    },
                    PickerControl {
                        request_id: request.id,
                        id,
                    },
                    BackgroundColor(if state.selected == Some(index) {
                        Color::srgb_u8(88, 88, 88)
                    } else {
                        Color::NONE
                    }),
                ))
                .observe(activate)
                .observe(row_click);
            if !row.available {
                commands.entity(entry).insert(InteractionDisabled);
            }
            if state.selected == Some(index) {
                commands.entity(entry).insert(Checked);
            }
            commands.entity(list).add_child(entry);
        }
        if request.rows.is_empty() {
            let text = commands
                .spawn(source::text(
                    "No characters available.",
                    source::font(skin.font.clone(), false),
                ))
                .id();
            commands.entity(list).add_child(text);
        }
        let row = state.selected.and_then(|index| request.rows.get(index));
        let options = commands
            .spawn(Node {
                column_gap: px(5.),
                align_items: AlignItems::Center,
                flex_shrink: 0.,
                ..default()
            })
            .id();
        let toggle = commands
            .spawn((
                source::checkbox(source::checkbox_row(false)),
                OutfitToggle(request.id),
                PickerControl {
                    request_id: request.id,
                    id: "selection.show_outfits".into(),
                },
            ))
            .observe(toggle_outfits)
            .id();
        if state.outfits {
            commands.entity(toggle).insert(Checked);
        }
        let indicator = commands
            .spawn(source::checkbox_indicator(if state.outfits {
                skin.checked.clone()
            } else {
                skin.unchecked.clone()
            }))
            .id();
        let caption = commands
            .spawn(source::text(
                "Show Outfits",
                source::font(skin.font.clone(), false),
            ))
            .id();
        commands.entity(toggle).add_children(&[indicator, caption]);
        let channel = row
            .and_then(|row| state.channel.and_then(|index| row.channels.get(index)))
            .map_or("Default channel", |channel| channel.1.as_str());
        let channel = add_button(
            &mut commands,
            &skin,
            request.id,
            Choice::Channel,
            channel,
            200.,
            row.is_none_or(|row| row.channels.is_empty()),
        );
        let [mark, train_marked] =
            training::buttons(&mut commands, &skin, request.id, &state, &request.rows);
        commands
            .entity(options)
            .add_children(&[toggle, channel, mark, train_marked]);
        commands.entity(panel).add_child(options);
        let footer = commands
            .spawn(Node {
                column_gap: px(5.),
                flex_shrink: 0.,
                ..default()
            })
            .id();
        for (choice, label, width, disabled) in [
            (Choice::Create, "Create Character", 108., false),
            (Choice::Account, "Manage Account", 108., false),
            (Choice::Refresh, "Refresh", 60., false),
            (
                Choice::Train,
                "Offline Training",
                110.,
                row.is_none_or(|row| !row.available),
            ),
            (
                Choice::Play,
                "Ok",
                43.,
                row.is_none_or(|row| !row.available),
            ),
            (Choice::Cancel, "Cancel", 43., false),
        ] {
            let initial_focus = matches!(choice, Choice::Play) && !disabled
                || matches!(choice, Choice::Cancel) && row.is_none_or(|row| !row.available);
            let button = add_button(
                &mut commands,
                &skin,
                request.id,
                choice.clone(),
                label,
                width,
                disabled,
            );
            if matches!(choice, Choice::Play) {
                commands.entity(button).insert(Node {
                    width: px(width),
                    height: px(20.),
                    margin: UiRect::left(Val::Auto),
                    border: px(1.).all(),
                    flex_shrink: 0.,
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                });
            }
            if initial_focus {
                focus.set(button, FocusCause::Navigated);
            }
            commands.entity(footer).add_child(button);
        }
        commands.entity(panel).add_child(footer);
    }
    state.rendered = Some(key);
}
/// One list line: fixed-width cells, a `None` width fills the remaining space.
fn line(
    commands: &mut Commands,
    skin: &Skin,
    cells: &[(String, Option<f32>)],
    height: f32,
) -> Entity {
    let entry = commands
        .spawn((
            crate::bevy_shell::ui::window_layout::Scaled {
                height: Some(height),
                ..default()
            },
            Node {
                width: percent(100.),
                height: px(height),
                flex_shrink: 0.,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .id();
    for (value, width) in cells {
        let cell = commands
            .spawn(source::text(value, source::font(skin.font.clone(), false)))
            .insert((
                TextLayout::no_wrap(),
                bevy::picking::Pickable::IGNORE,
                crate::bevy_shell::ui::window_layout::Scaled {
                    width: *width,
                    ..default()
                },
                Node {
                    width: width.map_or(Val::Auto, px),
                    flex_grow: if width.is_none() { 1. } else { 0. },
                    flex_shrink: 0.,
                    min_width: px(0.),
                    padding: UiRect::horizontal(px(4.)),
                    overflow: Overflow::clip(),
                    ..default()
                },
            ))
            .id();
        commands.entity(entry).add_child(cell);
    }
    entry
}
fn toggle_outfits(
    event: On<bevy::ui_widgets::ValueChange<bool>>,
    toggles: Query<&OutfitToggle>,
    guard: ModalGuard,
    mut state: ResMut<PickerState>,
) {
    if guard.allows(event.source)
        && let Ok(toggle) = toggles.get(event.source)
        && state.request == Some(toggle.0)
    {
        state.outfits = event.value;
        state.revision = state.revision.wrapping_add(1);
    }
}
fn add_button(
    commands: &mut Commands,
    skin: &Skin,
    request: u64,
    choice: Choice,
    label: &str,
    width: f32,
    disabled: bool,
) -> Entity {
    let id = match &choice {
        Choice::Row(index) => format!("selection.row.{index}"),
        Choice::Channel => "selection.channel".into(),
        Choice::Play => "selection.play".into(),
        Choice::Train => "selection.training".into(),
        Choice::Mark => "selection.training.mark".into(),
        Choice::TrainMarked => "selection.training.batch".into(),
        Choice::Refresh => "selection.refresh".into(),
        Choice::Account => "selection.account".into(),
        Choice::Create => "selection.create".into(),
        Choice::Cancel => "selection.cancel".into(),
    };
    let button = commands
        .spawn((
            source::button(skin.buttons.clone(), width),
            Target { request, choice },
            PickerControl {
                request_id: request,
                id,
            },
        ))
        .observe(activate)
        .observe(row_click)
        .id();
    if disabled {
        commands.entity(button).insert(InteractionDisabled);
    }
    let text = commands
        .spawn(source::button_caption(label, skin.font.clone()))
        .id();
    commands.entity(button).add_child(text);
    button
}
mod interaction;
pub(super) use interaction::keyboard;
use interaction::{activate, row_click};

#[cfg(test)]
mod tests;

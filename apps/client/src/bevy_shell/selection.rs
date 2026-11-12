//! Shared desktop character picker over server-owned rows and the existing selector callback.
mod bridge;
use super::owner::HostOwner;
use crate::character_selection::SelectionAction;
use bevy::{
    input_focus::{FocusCause, InputFocus, tab_navigation::TabGroup},
    prelude::*,
    ui::{Checked, InteractionDisabled},
    ui_widgets::Activate,
};
pub(super) use bridge::SelectionBridge;
use oteryn_client_ui_bevy as source;

const PAGE_ROWS: usize = 8;
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
    Previous,
    Next,
    Play,
    Refresh,
    Create,
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
    page: usize,
    revision: u64,
    rendered: Option<(u64, u64)>,
}
#[derive(Resource)]
pub(super) struct Skin {
    buttons: source::ButtonImages,
    frame: Handle<Image>,
    font: Handle<Font>,
}

pub(super) fn setup(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(PickerState::default());
    commands.insert_resource(Skin {
        buttons: source::ButtonImages {
            up: assets.load("images/skin/classic/button-grey-up.png"),
            down: assets.load("images/skin/classic/button-grey-down.png"),
        },
        frame: assets.load("images/skin/classic/dialog-frame-borderimage.png"),
        font: assets.load("fonts/verdanab.ttf"),
    });
    commands.spawn((
        Picker,
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
        state.request = Some(request.id);
        state.selected = request
            .rows
            .iter()
            .position(|row| row.available && row.character.character_id == request.preselect)
            .or_else(|| request.rows.iter().position(|row| row.available));
        state.channel = None;
        state.page = state.selected.unwrap_or(0) / PAGE_ROWS;
        state.revision = 0;
        state.rendered = None;
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
                TabGroup {
                    order: 5,
                    modal: true,
                },
                source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
                Node {
                    width: px(620.),
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
            ))
            .id();
        commands.entity(root).add_child(panel);
        source::dialog_caption(&mut commands, panel, "Select Character", skin.font.clone());
        let title = commands
            .spawn(source::text(
                "Character / World",
                source::font(skin.font.clone(), false),
            ))
            .id();
        commands.entity(panel).add_child(title);
        for (index, row) in request
            .rows
            .iter()
            .enumerate()
            .skip(state.page * PAGE_ROWS)
            .take(PAGE_ROWS)
        {
            let label = format!(
                "{} — {}{}",
                row.character.name,
                row.world_name,
                if row.available { "" } else { " (unavailable)" }
            );
            let button = add_button(
                &mut commands,
                &skin,
                request.id,
                Choice::Row(index),
                &label,
                560.,
                !row.available,
            );
            if state.selected == Some(index) {
                commands.entity(button).insert(Checked);
            }
            commands.entity(panel).add_child(button);
        }
        if request.rows.is_empty() {
            let text = commands
                .spawn(source::text(
                    "No characters available.",
                    source::font(skin.font.clone(), false),
                ))
                .id();
            commands.entity(panel).add_child(text);
        }
        let navigation = commands
            .spawn(Node {
                column_gap: px(5.),
                ..default()
            })
            .id();
        let pages = request.rows.len().max(1).div_ceil(PAGE_ROWS);
        let previous = add_button(
            &mut commands,
            &skin,
            request.id,
            Choice::Previous,
            "Previous",
            70.,
            state.page == 0,
        );
        let next = add_button(
            &mut commands,
            &skin,
            request.id,
            Choice::Next,
            "Next",
            70.,
            state.page + 1 >= pages,
        );
        let page = commands
            .spawn(source::text(
                format!("Page {} / {}", state.page + 1, pages),
                source::font(skin.font.clone(), false),
            ))
            .id();
        commands
            .entity(navigation)
            .add_children(&[previous, page, next]);
        commands.entity(panel).add_child(navigation);
        let row = state.selected.and_then(|index| request.rows.get(index));
        let channel = row
            .and_then(|row| state.channel.and_then(|index| row.channels.get(index)))
            .map_or("Default channel", |channel| channel.1.as_str());
        let channel = add_button(
            &mut commands,
            &skin,
            request.id,
            Choice::Channel,
            channel,
            300.,
            row.is_none_or(|row| row.channels.is_empty()),
        );
        commands.entity(panel).add_child(channel);
        let unavailable = commands
            .spawn(source::text(
                "Training and account services are unavailable in this frontend.",
                source::font(skin.font.clone(), false),
            ))
            .insert(TextColor(source::DISABLED_TEXT_COLOR))
            .id();
        commands.entity(panel).add_child(unavailable);
        let footer = commands
            .spawn(Node {
                column_gap: px(5.),
                justify_content: JustifyContent::End,
                ..default()
            })
            .id();
        for (choice, label, width, disabled) in [
            (Choice::Create, "Create", 60., false),
            (Choice::Refresh, "Refresh", 60., false),
            (
                Choice::Play,
                "Play",
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
                choice,
                label,
                width,
                disabled,
            );
            if initial_focus {
                focus.set(button, FocusCause::Navigated);
            }
            commands.entity(footer).add_child(button);
        }
        commands.entity(panel).add_child(footer);
    }
    state.rendered = Some(key);
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
        Choice::Previous => "selection.previous".into(),
        Choice::Next => "selection.next".into(),
        Choice::Play => "selection.play".into(),
        Choice::Refresh => "selection.refresh".into(),
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
fn activate(
    event: On<Activate>,
    choices: Query<(&Target, Has<InteractionDisabled>)>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<PickerState>,
) {
    let Ok((target, disabled)) = choices.get(event.entity) else {
        return;
    };
    if disabled || state.request != Some(target.request) {
        return;
    }
    let Some(request) = owner
        .selection
        .pending()
        .filter(|request| request.id == target.request)
    else {
        return;
    };
    let action = match target.choice {
        Choice::Row(index) => {
            if request.rows.get(index).is_some_and(|row| row.available) {
                state.selected = Some(index);
                state.channel = None;
            }
            None
        }
        Choice::Channel => {
            if let Some(row) = state.selected.and_then(|index| request.rows.get(index)) {
                state.channel = match state.channel {
                    None if !row.channels.is_empty() => Some(0),
                    Some(index) if index + 1 < row.channels.len() => Some(index + 1),
                    _ => None,
                };
            }
            None
        }
        Choice::Previous => {
            state.page = state.page.saturating_sub(1);
            None
        }
        Choice::Next => {
            state.page = (state.page + 1).min(request.rows.len().max(1).div_ceil(PAGE_ROWS) - 1);
            None
        }
        Choice::Play => state
            .selected
            .and_then(|index| request.rows.get(index))
            .filter(|row| row.available)
            .map(|row| SelectionAction::Play {
                character_id: row.character.character_id.clone(),
                channel_id: state
                    .channel
                    .and_then(|index| row.channels.get(index))
                    .map(|channel| channel.0.clone()),
            }),
        Choice::Create => Some(SelectionAction::Create),
        Choice::Refresh => Some(SelectionAction::Refresh),
        Choice::Cancel => Some(SelectionAction::Cancel),
    };
    if let Some(action) = action {
        owner.selection.respond(target.request, action);
    }
    state.revision = state.revision.wrapping_add(1);
}

/// Escape replies through the same request identity as the Cancel button.
pub(super) fn keyboard(
    mut keys: MessageReader<bevy::input::keyboard::KeyboardInput>,
    mut owner: NonSendMut<HostOwner>,
) {
    if keys.read().any(|key| {
        key.key_code == KeyCode::Escape
            && key.state == bevy::input::ButtonState::Pressed
            && !key.repeat
    }) && let Some(id) = owner.selection.pending().map(|request| request.id)
    {
        owner.selection.respond(id, SelectionAction::Cancel);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::AssetPlugin;
    use oteryn_foundation::CancellationToken;
    use oteryn_platform_client::native_login::{CharacterAvailability, OwnerCharacter};
    use std::{
        thread,
        time::{Duration, Instant},
    };
    #[test]
    fn picker_buttons_return_server_channel_through_existing_callback()
    -> Result<(), Box<dyn std::error::Error>> {
        let (bridge, callback) = SelectionBridge::new();
        let mut owner = HostOwner::new(None)?;
        owner.selection = bridge;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<Font>()
            .init_resource::<InputFocus>()
            .insert_non_send(owner)
            .add_systems(Startup, setup)
            .add_systems(Update, (poll, project).chain());
        app.update();
        let worker = thread::spawn(move || {
            callback(
                &[crate::character_selection::CharacterRow {
                    character: OwnerCharacter {
                        character_id: "server-character".into(),
                        world_id: "server-world".into(),
                        name: "Server character".into(),
                        availability: CharacterAvailability::Available,
                    },
                    world_name: "Server world".into(),
                    channels: vec![("server-channel".into(), "Server channel".into())],
                    available: true,
                }],
                "server-character",
                CancellationToken::new(),
            )
        });
        let deadline = Instant::now() + Duration::from_secs(3);
        while app
            .world()
            .non_send::<HostOwner>()
            .selection
            .pending()
            .is_none()
            && Instant::now() < deadline
        {
            app.update();
            thread::yield_now();
        }
        app.update();
        let find = |app: &mut App, channel: bool| -> Result<Entity, &'static str> {
            app.world_mut()
                .query::<(Entity, &Target)>()
                .iter(app.world())
                .find(|(_, target)| {
                    if channel {
                        matches!(target.choice, Choice::Channel)
                    } else {
                        matches!(target.choice, Choice::Play)
                    }
                })
                .map(|(entity, _)| entity)
                .ok_or("picker button missing")
        };
        let initial_play = find(&mut app, false)?;
        assert_eq!(
            app.world().resource::<InputFocus>().get(),
            Some(initial_play)
        );
        let channel = find(&mut app, true)?;
        app.world_mut().trigger(Activate { entity: channel });
        app.update();
        let play = find(&mut app, false)?;
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(play));
        app.world_mut().trigger(Activate { entity: play });
        app.update();
        assert_eq!(
            worker.join().map_err(|_| "selector worker")?,
            SelectionAction::Play {
                character_id: "server-character".into(),
                channel_id: Some("server-channel".into())
            }
        );
        assert!(
            app.world()
                .non_send::<HostOwner>()
                .selection
                .pending()
                .is_none()
        );
        assert_eq!(app.world().resource::<InputFocus>().get(), None);
        Ok(())
    }
}

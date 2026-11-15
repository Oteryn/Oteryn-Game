//! House catalogue, auctions, access lists and reviewed ownership transfers.
use super::*;
use crate::play::PlayLink;
use bevy::{
    input::keyboard::KeyboardInput,
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui::house::Panel;
use oteryn_client_ui_bevy as source;

#[derive(Component)]
pub(super) struct Field(pub usize);
#[derive(Component)]
struct Control {
    generation: u64,
    root: Entity,
    selection: Option<usize>,
}
#[derive(Resource, Default)]
struct HouseWindow {
    generation: Option<u64>,
    open: bool,
    root: Option<Entity>,
    model: Option<Panel>,
    fields: [String; 7],
    request: Option<usize>,
    ownership: Option<oteryn_client_ui::house::sale::OwnershipPanel>,
    review: Option<oteryn_client_ui::house::sale::Confirmation>,
    feedback: String,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<HouseWindow>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
fn keyboard(
    mut events: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    focus: Res<InputFocus>,
    editors: Query<(), With<EditableText>>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<HouseWindow>,
) {
    let mut close = false;
    let open = events.read().any(|event| {
        close |= event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat;
        event.key_code == KeyCode::KeyH
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    });
    if close && state.root.is_some_and(|root| guard.allows(root)) {
        state.open = false;
        state.request = None;
        state.review = None;
    }
    let requested = owner.take_house_request();
    if (requested
        || (open
            && keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight])
            && keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight])))
        && !state.open
        && !owner.modal_active()
        && guard.allows_focus(focus.get())
        && focus.get().is_none_or(|entity| !editors.contains(entity))
        && owner
            .play
            .as_ref()
            .is_some_and(|play| !play.view.gameplay_input_blocked(&play.link))
    {
        state.generation = Some(owner.generation());
        state.open = true;
    }
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<HouseWindow>,
    mut focus: ResMut<InputFocus>,
    fields: Query<(), With<Field>>,
) {
    if state.generation != Some(owner.generation()) || owner.play.is_none() {
        state.open = false;
        state.review = None;
    }
    let ownership = owner
        .play
        .as_ref()
        .map(|play| play.link.house_ownership_panel().model);
    let projected = owner
        .play
        .as_ref()
        .filter(|_| state.open)
        .map(|play| play.link.house_panel());
    if let Some(panel) = &projected
        && state.model.as_ref() == Some(&panel.model)
        && state.fields == panel.fields
        && state.ownership == ownership
    {
        return;
    }
    if let Some(root) = state.root.take() {
        if focus.get().is_some_and(|entity| fields.contains(entity)) {
            focus.clear();
        }
        commands.entity(root).despawn();
    }
    state.model = None;
    state.request = None;
    let Some(panel) = projected else {
        return;
    };
    let mut message = oteryn_client_ui::house::panel_text(&panel.model);
    if let Some(model) = &ownership {
        message.push_str("\nOwnership / incoming transfers:\n");
        if let Some(data) = model.data() {
            for (index, row) in data.rows.iter().enumerate() {
                message.push_str(&format!("{}. {}\n", index + 1, row.name));
            }
        }
        message.push_str(&oteryn_client_ui::house::ownership_text(model));
    }
    if let Some(review) = &state.review {
        message.push_str(review.text());
    }
    message.push_str(&state.feedback);
    state.ownership = ownership;
    state.root = Some(build(
        &mut commands,
        &skin,
        owner.generation(),
        &panel.fields,
        &panel.model,
        &message,
    ));
    state.fields = panel.fields;
    state.model = Some(panel.model);
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<HouseWindow>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity)
        || control.generation != owner.generation()
        || state.root != Some(control.root)
        || owner.modal_active()
    {
        return;
    }
    if let Some(selection) = control.selection {
        state.request = Some(selection);
    } else {
        state.open = false;
        state.review = None;
    }
}
fn finish(
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<HouseWindow>,
    fields: Query<(&Field, &EditableText)>,
) {
    let Some(selection) = state.request else {
        return;
    };
    if fields.iter().any(|(_, text)| {
        text.pending_paste.is_some()
            || text
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
    }) {
        return;
    }
    state.request = None;
    if !state.open
        || state.generation != Some(owner.generation())
        || owner.modal_active()
        || state.root.is_none_or(|root| !guard.allows(root))
    {
        return;
    }
    let (Some(play), Some(model)) = (&owner.play, &state.model) else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let mut values = state.fields.clone();
    for (field, text) in &fields {
        values[field.0] = text.value().to_string();
    }
    if selection >= 20 {
        if state.ownership.as_ref() != Some(&play.link.house_ownership_panel().model) {
            state.review = None;
            return;
        }
        match selection {
            20 => {
                state.review = None;
                if play.link.house_ownership_panel().model.available() {
                    state.feedback.clear();
                    play.link.house_ownership_open();
                } else {
                    state.feedback = "House ownership is unavailable in this session".into();
                }
            }
            21..=26 => {
                state.review = None;
                let result = values[2]
                    .trim()
                    .parse::<usize>()
                    .ok()
                    .and_then(|row| row.checked_sub(1))
                    .ok_or("Select an ownership row".to_owned())
                    .and_then(|row| play.link.house_ownership_select(row).map_err(str::to_owned))
                    .and_then(|()| {
                        play.link.house_ownership_review(
                            (selection - 20) as u8,
                            [values[4].clone(), values[5].clone(), values[3].clone()],
                        )
                    });
                match result {
                    Ok(review) => {
                        state.review = Some(review);
                        state.feedback.clear();
                    }
                    Err(error) => state.feedback = error,
                }
            }
            27 | 28 => {
                if let Some(review) = state.review.take() {
                    play.link.house_ownership_confirm(review, selection == 27);
                }
                state.feedback.clear();
            }
            29 => {
                state.review = None;
                play.link.house_ownership_page(1);
            }
            30 => {
                state.review = None;
                play.link.house_ownership_page(-1);
            }
            _ => {}
        }
        state.model = None;
    } else {
        submit(&play.link, model, &state.fields, values, selection);
        state.review = None;
    }
}
pub(super) fn submit(
    link: &PlayLink,
    expected: &Panel,
    expected_fields: &[String; 7],
    fields: [String; 7],
    selection: usize,
) {
    let current = link.house_panel();
    if current.model != *expected
        || current.fields != *expected_fields
        || current.model.pending.is_some()
    {
        return;
    }
    for (index, value) in fields.into_iter().enumerate() {
        link.house_edit(index, value);
    }
    link.house_request(selection);
}

pub(super) fn build(
    commands: &mut Commands,
    skin: &Skins,
    generation: u64,
    fields: &[String; 7],
    model: &Panel,
    message: &str,
) -> Entity {
    let root = commands
        .spawn((
            GameModal,
            GlobalZIndex(165),
            TabGroup {
                order: 3,
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
            BackgroundColor(Color::srgba(0., 0., 0., 0.55)),
        ))
        .id();
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(540.),
                max_width: percent(96.),
                max_height: percent(94.),
                padding: UiRect {
                    top: px(24.),
                    bottom: px(10.),
                    left: px(12.),
                    right: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    let title = model.page.rows.get(model.index).map_or_else(
        || "Houses — Browser, Auctions and Transfers".to_owned(),
        |row| {
            let name = if row.name.is_empty() {
                &row.house
            } else {
                &row.name
            };
            format!("Edit House List — {name}")
        },
    );
    source::dialog_caption(commands, panel, &title, skin.font.clone());
    let content = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                min_height: px(0.),
                flex_shrink: 1.,
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(content);
    label(commands, skin, content, message);
    for (index, prompt) in [
        (0, "World UUID"),
        (1, "Town key"),
        (2, "House row (1 onwards)"),
        (3, "Maximum auction bid / transfer price (gold)"),
        (4, "Enter one member UUID per line."),
        (
            5,
            "Door: x y floor / transfer date: YYYY-MM-DD HH:MM:SS UTC",
        ),
        (6, "Type CONFIRM for changes"),
    ] {
        label(commands, skin, content, prompt);
        if index == 4 {
            label(
                commands,
                skin,
                content,
                "Names cannot be resolved by this server for access lists; an empty list removes all members. For transfers, enter the recipient character name here.",
            );
        }
        let mut editable = EditableText::new(&fields[index]);
        editable.allow_newlines = index == 4;
        editable.max_characters = Some(8192);
        editable.visible_lines = Some(if index == 4 { 4. } else { 1. });
        let field = commands
            .spawn((
                Field(index),
                TextInput,
                editable,
                TabIndex(index as i32),
                source::font(skin.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::default(),
                TextCursorStyle::default(),
                Node {
                    width: percent(100.),
                    min_height: px(26.),
                    padding: px(4.).all(),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
            ))
            .id();
        commands.entity(content).add_child(field);
    }
    let footer = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(5.),
            row_gap: px(5.),
            ..default()
        })
        .id();
    commands.entity(panel).add_child(footer);
    for (selection, caption) in [
        (Some(0), "Query"),
        (Some(1), "Next"),
        (Some(2), "Bid"),
        (Some(20), "Owned / incoming"),
        (Some(30), "Previous owned"),
        (Some(29), "Next owned"),
        (Some(21), "Review move-out"),
        (Some(22), "Review cancel move-out"),
        (Some(23), "Review transfer"),
        (Some(24), "Review cancel transfer"),
        (Some(25), "Review accept transfer"),
        (Some(26), "Review reject transfer"),
        (Some(27), "Confirm reviewed action"),
        (Some(28), "Cancel review"),
        (Some(11), "Load guests"),
        (Some(12), "Load subowners"),
        (Some(13), "Load door"),
        (Some(3), "Save guests"),
        (Some(4), "Save subowners"),
        (Some(5), "Save door"),
        (None, "Close"),
    ] {
        let button = commands
            .spawn((
                source::button(skin.buttons.clone(), 120.),
                Control {
                    generation,
                    root,
                    selection,
                },
            ))
            .observe(activate)
            .id();
        let text = commands
            .spawn(source::button_caption(caption, skin.font.clone()))
            .id();
        commands.entity(button).add_child(text);
        commands.entity(footer).add_child(button);
    }
    root
}
fn label(commands: &mut Commands, skin: &Skins, parent: Entity, text: &str) {
    let entity = commands
        .spawn(source::text(text, source::font(skin.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(entity);
}

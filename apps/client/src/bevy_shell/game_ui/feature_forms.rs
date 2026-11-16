//! The generic window for the session forms without a dedicated presenter: mail, guild,
//! houses services, boss tools and the other windows the native shell drew as text forms.
//! The session owns every value; this window shows the most recently opened form, keeps
//! typed text until the session's own values change, and submits through the form.
use super::{GameModal, modal::ModalGuard};
use crate::bevy_shell::{owner::HostOwner, ui::Skins};
use crate::play::{FeatureForm, FormView};
use bevy::{
    ecs::system::SystemParam,
    input::keyboard::KeyboardInput,
    input_focus::{
        InputFocus,
        tab_navigation::{TabGroup, TabIndex},
    },
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui::InteractionDisabled,
    ui_widgets::{Activate, ScrollArea, TextInput},
};
use oteryn_client_ui_bevy as source;

#[derive(Component)]
struct Field(usize);
#[derive(Component)]
struct Summary;
#[derive(Component)]
struct Control {
    generation: u64,
    root: Entity,
    /// The form's button index; `None` is the window's own Close.
    action: Option<usize>,
}

/// What the built window was made from; a change rebuilds it.
#[derive(PartialEq, Eq)]
struct Layout {
    form: usize,
    title: &'static str,
    labels: &'static [&'static str],
    buttons: &'static [&'static str],
    fields: Vec<String>,
}
struct Shown {
    layout: Layout,
    root: Entity,
    text: String,
    pending: bool,
}
#[derive(Resource, Default)]
struct FormWindow {
    generation: Option<u64>,
    /// Open forms by registry index, the most recently opened last.
    order: Vec<usize>,
    shown: Option<Shown>,
    /// A pressed form button, submitted once the text editors settled this frame.
    request: Option<(usize, usize)>,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<FormWindow>()
        .add_message::<KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}

fn keyboard(
    mut events: MessageReader<KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<FormWindow>,
) {
    let escape = events.read().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    });
    if !escape {
        return;
    }
    let Some(shown) = &state.shown else {
        return;
    };
    if !guard.allows(shown.root) {
        return;
    }
    let form = shown.layout.form;
    state.request = None;
    if let (Some(play), Some(entry)) = (&owner.play, FeatureForm::all().get(form)) {
        play.link.feature_form_close(*entry);
    }
}

/// The shown window's summary text and buttons, updated in place while its layout holds.
#[derive(SystemParam)]
struct ShownWidgets<'w, 's> {
    summaries: Query<'w, 's, &'static mut Text, With<Summary>>,
    controls: Query<'w, 's, (Entity, &'static Control)>,
}

fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skin: Res<Skins>,
    mut state: ResMut<FormWindow>,
    mut focus: ResMut<InputFocus>,
    fields: Query<(), With<Field>>,
    mut shown_widgets: ShownWidgets,
) {
    let generation = owner.generation();
    if state.generation != Some(generation) || owner.play.is_none() {
        state.generation = Some(generation);
        state.order.clear();
        state.request = None;
    }
    let mut top: Option<(usize, FormView)> = None;
    if let Some(play) = &owner.play {
        let mut open = Vec::new();
        for (index, form) in FeatureForm::all().iter().enumerate() {
            play.link.feature_form_tick(*form);
            let view = play.link.feature_form_view(*form);
            if view.open {
                open.push((index, view));
            }
        }
        state
            .order
            .retain(|index| open.iter().any(|(at, _)| at == index));
        for (index, _) in &open {
            if !state.order.contains(index) {
                state.order.push(*index);
            }
        }
        if let Some(last) = state.order.last() {
            top = open.into_iter().find(|(index, _)| index == last);
        }
    }
    let Some((form, view)) = top else {
        despawn(&mut commands, &mut state, &mut focus, &fields);
        return;
    };
    let layout = Layout {
        form,
        title: view.title,
        labels: view.labels,
        buttons: view.buttons,
        fields: view.fields,
    };
    if let Some(shown) = &mut state.shown
        && shown.layout == layout
    {
        if shown.text != view.text {
            for mut text in &mut shown_widgets.summaries {
                text.0.clone_from(&view.text);
            }
            shown.text = view.text;
        }
        if shown.pending != view.pending {
            for (entity, control) in &shown_widgets.controls {
                if control.root == shown.root && control.action.is_some() {
                    if view.pending {
                        commands.entity(entity).insert(InteractionDisabled);
                    } else {
                        commands.entity(entity).remove::<InteractionDisabled>();
                    }
                }
            }
            shown.pending = view.pending;
        }
        return;
    }
    despawn(&mut commands, &mut state, &mut focus, &fields);
    let root = build(
        &mut commands,
        &skin,
        generation,
        &layout,
        &view.text,
        view.pending,
    );
    state.shown = Some(Shown {
        layout,
        root,
        text: view.text,
        pending: view.pending,
    });
}

fn despawn(
    commands: &mut Commands,
    state: &mut FormWindow,
    focus: &mut InputFocus,
    fields: &Query<(), With<Field>>,
) {
    if let Some(shown) = state.shown.take() {
        if focus.get().is_some_and(|entity| fields.contains(entity)) {
            focus.clear();
        }
        commands.entity(shown.root).despawn();
    }
}

fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<FormWindow>,
) {
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    let Some(shown) = &state.shown else {
        return;
    };
    if !guard.allows(event.entity)
        || control.generation != owner.generation()
        || shown.root != control.root
        || owner.modal_active()
    {
        return;
    }
    let form = shown.layout.form;
    match control.action {
        Some(action) => state.request = Some((form, action)),
        None => {
            state.request = None;
            if let (Some(play), Some(entry)) = (&owner.play, FeatureForm::all().get(form)) {
                play.link.feature_form_close(*entry);
            }
        }
    }
}

fn finish(
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<FormWindow>,
    fields: Query<(&Field, &EditableText)>,
) {
    let Some((form, action)) = state.request else {
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
    let Some(shown) = &state.shown else {
        return;
    };
    if shown.layout.form != form
        || state.generation != Some(owner.generation())
        || owner.modal_active()
        || !guard.allows(shown.root)
    {
        return;
    }
    let (Some(play), Some(entry)) = (&owner.play, FeatureForm::all().get(form)) else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let mut values = shown.layout.fields.clone();
    if values.len() < shown.layout.labels.len() {
        values.resize(shown.layout.labels.len(), String::new());
    }
    for (field, text) in &fields {
        if let Some(value) = values.get_mut(field.0) {
            *value = text.value().to_string();
        }
    }
    play.link.feature_form_submit(*entry, action, values);
}

fn build(
    commands: &mut Commands,
    skin: &Skins,
    generation: u64,
    layout: &Layout,
    text: &str,
    pending: bool,
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
                width: px(560.),
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
    source::dialog_caption(commands, panel, layout.title, skin.font.clone());
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
    let summary = commands
        .spawn((
            Summary,
            source::text(text, source::font(skin.font.clone(), false)),
        ))
        .id();
    commands.entity(content).add_child(summary);
    for (index, prompt) in layout.labels.iter().enumerate() {
        let label = commands
            .spawn(source::text(
                *prompt,
                source::font(skin.font.clone(), false),
            ))
            .id();
        commands.entity(content).add_child(label);
        let mut editable = EditableText::new(layout.fields.get(index).map_or("", String::as_str));
        editable.max_characters = Some(8192);
        editable.visible_lines = Some(1.);
        let field = commands
            .spawn((
                Field(index),
                TextInput,
                editable,
                TabIndex(i32::try_from(index).unwrap_or(i32::MAX)),
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
    let actions = layout
        .buttons
        .iter()
        .enumerate()
        .map(|(index, caption)| (Some(index), *caption))
        .chain(std::iter::once((None, "Close")));
    for (action, caption) in actions {
        let button = commands
            .spawn((
                source::button(skin.buttons.clone(), 120.),
                Control {
                    generation,
                    root,
                    action,
                },
            ))
            .observe(activate)
            .id();
        if pending && action.is_some() {
            commands.entity(button).insert(InteractionDisabled);
        }
        let text = commands
            .spawn(source::button_caption(caption, skin.font.clone()))
            .id();
        commands.entity(button).add_child(text);
        commands.entity(footer).add_child(button);
    }
    root
}

#[cfg(test)]
#[path = "feature_forms/tests.rs"]
mod tests;

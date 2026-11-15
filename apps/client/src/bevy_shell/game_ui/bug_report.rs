//! Bug reports over the retained report owner and its reviewed-draft flow.
use super::{GameModal, modal::ModalGuard};
use crate::bevy_shell::{owner::HostOwner, ui::Skins, window_drag::DraggableDialog};
use bevy::{
    input_focus::{
        FocusCause, InputFocus,
        tab_navigation::{TabGroup, TabIndex},
    },
    prelude::*,
    text::{EditableText, TextCursorStyle},
    ui::{Checked, InteractionDisabled},
    ui_widgets::{Activate, ScrollArea, TextInput},
};
use oteryn_client_ui::player_report::Panel;
use oteryn_client_ui_bevy as source;

#[derive(Component)]
struct Field(usize);
#[derive(Component, Clone, Copy)]
struct Action {
    generation: u64,
    kind: Kind,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Category(usize),
    Review,
    Send,
    Close,
}
struct Widgets {
    root: Entity,
    fields: [Entity; 3],
    categories: [Entity; 4],
    review: Entity,
    send: Entity,
    preview: Entity,
    feedback: Entity,
}
#[derive(Resource, Default)]
struct State {
    generation: u64,
    widgets: Option<Widgets>,
    pending: Option<Entity>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(PostUpdate, finish.after(bevy::text::EditableTextSystems));
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
) {
    let panel = owner
        .play
        .as_ref()
        .map(|play| play.link.player_report_panel());
    let open = panel.as_ref().is_some_and(|panel| panel.bug.open);
    if (!open || state.generation != owner.generation())
        && let Some(widgets) = state.widgets.take()
    {
        let mut current = focus.get();
        while let Some(entity) = current {
            if entity == widgets.root {
                focus.clear();
                break;
            }
            current = parents.get(entity).ok().map(ChildOf::parent);
        }
        commands.entity(widgets.root).despawn();
        state.pending = None;
    }
    if open
        && state.widgets.is_none()
        && let Some(panel) = panel
    {
        state.generation = owner.generation();
        let widgets = build(&mut commands, &skins, state.generation, &panel);
        focus.set(widgets.fields[1], FocusCause::Navigated);
        state.widgets = Some(widgets);
    }
}
fn activate(
    event: On<Activate>,
    guard: ModalGuard,
    mut state: ResMut<State>,
    owner: NonSend<HostOwner>,
    actions: Query<&Action>,
) {
    if guard.allows(event.entity)
        && let Ok(action) = actions.get(event.entity)
        && action.generation == owner.generation()
    {
        if action.kind == Kind::Close {
            if let Some(play) = &owner.play {
                play.link.bug_report_close();
            }
            state.pending = None;
        } else if state.pending.is_none() {
            state.pending = Some(event.entity);
        }
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    actions: Query<(Entity, &Action)>,
) {
    if events.read().any(|event| {
        event.key_code == KeyCode::Escape
            && event.state == bevy::input::ButtonState::Pressed
            && !event.repeat
    }) && actions.iter().any(|(entity, action)| {
        action.kind == Kind::Close
            && action.generation == owner.generation()
            && guard.allows(entity)
    }) && let Some(play) = &owner.play
    {
        play.link.bug_report_close();
    }
}
fn finish(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    guard: ModalGuard,
    fields: Query<(&Field, &EditableText)>,
    actions: Query<&Action>,
    mut texts: Query<&mut Text>,
) {
    let Some(widgets) = &state.widgets else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if state.generation != owner.generation() || !play.link.player_report_panel().bug.open {
        state.pending = None;
        return;
    }
    let action = state.pending.and_then(|entity| {
        actions
            .get(entity)
            .ok()
            .filter(|action| action.generation == owner.generation() && guard.allows(entity))
            .map(|action| action.kind)
    });
    let committed = widgets.fields.iter().all(|entity| {
        fields.get(*entity).is_ok_and(|(_, editor)| {
            editor.pending_paste.is_none()
                && !editor
                    .pending_edits
                    .iter()
                    .any(bevy::text::TextEdit::is_destructive)
        })
    });
    let mut valid = committed;
    if committed {
        for (index, entity) in widgets.fields.iter().enumerate() {
            if let Ok((field, editor)) = fields.get(*entity) {
                valid &= field.0 == index
                    && play.link.bug_report_edit(index, editor.value().to_string());
            }
        }
    }
    let panel = play.link.player_report_panel();
    let ready = valid && panel.selected && !panel.pending && !panel.bug.fields[1].trim().is_empty();
    if committed && !owner.modal_active() {
        let requested = match action {
            Some(Kind::Category(category)) if panel.selected && !panel.pending => Some(category),
            Some(Kind::Review) if ready => Some(4),
            Some(Kind::Send) if ready && panel.bug.reviewed.is_some() => Some(5),
            _ => None,
        };
        if let Some(requested) = requested {
            play.link.bug_report_action(requested);
        }
    }
    let panel = play.link.player_report_panel();
    for (category, entity) in widgets.categories.iter().enumerate() {
        commands.entity(*entity).remove::<Checked>();
        if usize::from(panel.bug.category) == category {
            commands.entity(*entity).insert(Checked);
        }
        disabled(&mut commands, *entity, panel.pending || !panel.selected);
    }
    for entity in widgets.fields {
        disabled(&mut commands, entity, panel.pending);
    }
    disabled(&mut commands, widgets.review, !ready);
    disabled(
        &mut commands,
        widgets.send,
        !ready || panel.bug.reviewed.is_none(),
    );
    if let Ok(mut text) = texts.get_mut(widgets.preview) {
        text.0 = panel.bug.preview;
    }
    if let Ok(mut text) = texts.get_mut(widgets.feedback) {
        text.0 = if !panel.selected {
            tr!("Reports are unavailable in this session").into()
        } else if panel.pending {
            tr!("Waiting for the server").into()
        } else if panel.feedback == tr!("Report accepted") && !panel.summary.is_empty() {
            "Bug report sent.".into()
        } else {
            panel.feedback
        };
    }
    if committed {
        state.pending = None;
    }
}
fn disabled(commands: &mut Commands, entity: Entity, value: bool) {
    if value {
        commands.entity(entity).insert(InteractionDisabled);
    } else {
        commands.entity(entity).remove::<InteractionDisabled>();
    }
}
fn build(commands: &mut Commands, skins: &Skins, generation: u64, model: &Panel) -> Widgets {
    let root = commands
        .spawn((
            GameModal,
            GlobalZIndex(160),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.45)),
        ))
        .id();
    let panel = commands
        .spawn((
            DraggableDialog,
            TabGroup {
                order: 4,
                modal: true,
            },
            ScrollArea,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(500.),
                max_width: percent(96.),
                max_height: percent(94.),
                padding: UiRect {
                    top: px(25.),
                    bottom: px(10.),
                    left: px(12.),
                    right: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(6.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    source::dialog_caption(commands, panel, tr!("Bug report"), skins.font.clone());
    label(
        commands,
        skins,
        panel,
        "Please use this dialog to only report bugs. Do not report rule violations here!",
    );
    let categories = [
        "Map location",
        "Text or spelling",
        "Client behavior",
        "Other issue",
    ]
    .map(oteryn_client_ui::localization::tr);
    let categories = std::array::from_fn(|category| {
        button(
            commands,
            skins,
            panel,
            generation,
            Kind::Category(category),
            categories[category],
        )
    });
    let fields = std::array::from_fn(|index| {
        label(
            commands,
            skins,
            panel,
            [
                tr!("Map location: x,y,floor (map reports only)"),
                tr!("Steps, observed behavior and expected behavior"),
                tr!("Optional build details: enter YES to include build, OS and architecture"),
            ][index],
        );
        let mut editable = EditableText::new(&model.bug.fields[index]);
        editable.allow_newlines = index == 1;
        editable.max_characters = Some(1024);
        editable.visible_lines = Some(if index == 1 { 4. } else { 1. });
        let field = commands
            .spawn((
                Field(index),
                TextInput,
                editable,
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::default(),
                TextCursorStyle::default(),
                TabIndex(0),
                source::frame(skins.inset.clone(), [1.; 4]),
                Node {
                    width: percent(100.),
                    min_height: px(if index == 1 { 80. } else { 26. }),
                    flex_shrink: 0.,
                    padding: px(4.).all(),
                    ..default()
                },
            ))
            .id();
        commands.entity(panel).add_child(field);
        field
    });
    let preview = label(commands, skins, panel, &model.bug.preview);
    let feedback = label(commands, skins, panel, &model.feedback);
    let review = button(
        commands,
        skins,
        panel,
        generation,
        Kind::Review,
        tr!("Review report"),
    );
    let send = button(commands, skins, panel, generation, Kind::Send, tr!("Send"));
    button(
        commands,
        skins,
        panel,
        generation,
        Kind::Close,
        tr!("Cancel"),
    );
    Widgets {
        root,
        fields,
        categories,
        review,
        send,
        preview,
        feedback,
    }
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, text: &str) -> Entity {
    let entity = commands
        .spawn(source::text(text, source::font(skins.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    generation: u64,
    kind: Kind,
    text: &str,
) -> Entity {
    let entity = commands
        .spawn((
            source::button(skins.buttons.clone(), 220.),
            Action { generation, kind },
        ))
        .observe(activate)
        .id();
    let caption = commands
        .spawn(source::button_caption(text, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    commands.entity(parent).add_child(entity);
    entity
}

#[cfg(test)]
mod tests;

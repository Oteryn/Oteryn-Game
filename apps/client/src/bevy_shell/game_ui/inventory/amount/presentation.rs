use super::super::super::{GameModal, ItemIcon};
use super::*;
use crate::bevy_shell::game_ui::modal::ModalGuard;
use bevy::{
    input_focus::{
        FocusCause, InputFocus,
        tab_navigation::{TabGroup, TabIndex},
    },
    text::TextCursorStyle,
    ui_widgets::{SelectAllOnFocus, TextInput},
};
use oteryn_client_ui_bevy as source;
type AmountLabelRole = Or<(With<Feedback>, With<Range>)>;

#[allow(clippy::too_many_arguments)] // Separate editing and label queries preserve the live editor through model refreshes.
pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut ui: ResMut<InventoryUi>,
    mut previous: Local<Option<(Prompt, Entity)>>,
    mut fields: Query<(Entity, &Field, &mut EditableText)>,
    mut labels: Query<(&mut Text, Has<Feedback>, Has<Range>), AmountLabelRole>,
    mut focus: ResMut<InputFocus>,
) {
    if previous.as_ref().map(|(prompt, _)| *prompt) != ui.prompt {
        if let Some((_, root)) = previous.take() {
            if focus.get().is_some_and(|entity| fields.contains(entity)) {
                focus.clear();
            }
            commands.entity(root).despawn();
        }
        if let Some(prompt) = ui.prompt {
            let root = build(&mut commands, &skins, prompt, &ui);
            *previous = Some((prompt, root));
        }
    }
    let Some(prompt) = ui.prompt else {
        return;
    };
    refresh(&owner, &mut ui, prompt);
    let raw = value(&ui);
    for (_, field, mut editor) in &mut fields {
        if field.0 == prompt
            && !editor
                .pending_edits
                .iter()
                .any(bevy::text::TextEdit::is_destructive)
            && editor.pending_paste.is_none()
        {
            set_editor(&mut editor, &raw);
        }
    }
    let maximum = maximum(&owner, &ui, prompt);
    for (mut text, feedback, range) in &mut labels {
        let value = if feedback {
            ui.split_feedback.clone()
        } else if range {
            maximum.map_or_else(
                || "Item unavailable".into(),
                |maximum| format!("of {maximum}"),
            )
        } else {
            continue;
        };
        if text.0 != value {
            text.0 = value;
        }
    }
}
pub(super) fn focus_new(
    fields: Query<(Entity, &Field)>,
    guard: ModalGuard,
    mut focus: ResMut<InputFocus>,
    mut previous: Local<Option<Entity>>,
) {
    let Ok((entity, _)) = fields.single() else {
        *previous = None;
        return;
    };
    if *previous != Some(entity) && guard.allows(entity) {
        focus.set(entity, FocusCause::Navigated);
        *previous = Some(entity);
    }
}
fn build(commands: &mut Commands, skins: &Skins, prompt: Prompt, ui: &InventoryUi) -> Entity {
    build_dialog(
        commands,
        skins,
        DialogSpec {
            source: DialogSource::Movement(prompt),
            title: "Move stack",
            confirm: "Move",
            value: &value(ui),
            maximum: prompt.item.count.min(100),
            item: ui.split_entry,
        },
    )
}
pub(super) fn build_dialog(commands: &mut Commands, skins: &Skins, spec: DialogSpec<'_>) -> Entity {
    let root = commands
        .spawn((
            Root,
            GameModal,
            GlobalZIndex(match spec.source {
                DialogSource::Movement(_) => 90,
                DialogSource::Caller(_) => 210,
            }),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.3)),
        ))
        .id();
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            TabGroup {
                order: 4,
                modal: true,
            },
            Node {
                width: px(245.),
                max_width: percent(95.),
                padding: UiRect::axes(px(12.), px(24.)),
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    source::dialog_caption(commands, panel, spec.title, skins.font.clone());
    let row = commands
        .spawn(Node {
            column_gap: px(8.),
            align_items: AlignItems::Center,
            ..default()
        })
        .id();
    commands.entity(panel).add_child(row);
    let slot = commands
        .spawn((
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: px(34.),
                height: px(34.),
                padding: UiRect::all(px(1.)),
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(row).add_child(slot);
    if let Some(item) = spec.item {
        let icon = commands
            .spawn((
                ItemIcon(item),
                ImageNode {
                    color: Color::NONE,
                    ..default()
                },
                Node {
                    width: px(32.),
                    height: px(32.),
                    ..default()
                },
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(slot).add_child(icon);
    }
    let frame = commands
        .spawn((
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: px(72.),
                height: px(25.),
                ..default()
            },
        ))
        .id();
    let input = commands
        .spawn((
            TextInput,
            SelectAllOnFocus,
            EditableText {
                max_characters: Some(20),
                ..EditableText::new(spec.value)
            },
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(0),
            Node {
                width: percent(100.),
                height: px(25.),
                padding: UiRect::all(px(4.)),
                ..default()
            },
        ))
        .id();
    match spec.source {
        DialogSource::Movement(prompt) => {
            commands
                .entity(input)
                .insert(Field(prompt))
                .observe(super::input::changed);
        }
        DialogSource::Caller(id) => {
            commands
                .entity(input)
                .insert((caller::Field(id), Name::new("amount.caller.input")))
                .observe(caller::changed);
        }
    }
    commands.entity(frame).add_child(input);
    let range = commands
        .spawn((source::text(
            format!("of {}", spec.maximum),
            source::font(skins.font.clone(), false),
        ),))
        .id();
    match spec.source {
        DialogSource::Movement(_) => {
            commands.entity(range).insert(Range);
        }
        DialogSource::Caller(_) => {
            commands.entity(range).insert(caller::Range);
        }
    }
    commands.entity(row).add_children(&[frame, range]);
    let feedback = commands
        .spawn((
            source::text("", source::font(skins.font.clone(), false)),
            Node {
                width: percent(100.),
                ..default()
            },
        ))
        .insert(TextLayout::default())
        .id();
    match spec.source {
        DialogSource::Movement(_) => {
            commands.entity(feedback).insert(Feedback);
        }
        DialogSource::Caller(_) => {
            commands.entity(feedback).insert(caller::Feedback);
        }
    }
    commands.entity(panel).add_child(feedback);
    let buttons = commands
        .spawn(Node {
            column_gap: px(4.),
            justify_content: JustifyContent::End,
            ..default()
        })
        .id();
    commands.entity(panel).add_child(buttons);
    for (caption, action, width) in [
        ("All", AmountAction::All, 30.),
        ("−", AmountAction::Change(-1), 22.),
        ("+", AmountAction::Change(1), 22.),
        (spec.confirm, AmountAction::Confirm, 50.),
        ("Cancel", AmountAction::Cancel, 50.),
    ] {
        match spec.source {
            DialogSource::Movement(prompt) => {
                let control = match action {
                    AmountAction::All => Control::All(prompt),
                    AmountAction::Change(delta) => Control::Count(prompt, delta),
                    AmountAction::Confirm => Control::Confirm(prompt),
                    AmountAction::Cancel => Control::Cancel {
                        generation: prompt.generation,
                        prompt: Some(prompt),
                    },
                };
                super::super::dialogs::button(commands, skins, buttons, caption, control, width);
            }
            DialogSource::Caller(id) => {
                let name = match action {
                    AmountAction::All => "amount.caller.all",
                    AmountAction::Change(delta) if delta < 0 => "amount.caller.decrease",
                    AmountAction::Change(_) => "amount.caller.increase",
                    AmountAction::Confirm => "amount.caller.confirm",
                    AmountAction::Cancel => "amount.caller.cancel",
                };
                let entity = commands
                    .spawn((
                        source::button(skins.buttons.clone(), width),
                        caller::Control { id, action },
                        Name::new(name),
                    ))
                    .observe(caller::activate)
                    .id();
                let label = commands
                    .spawn(source::button_caption(caption, skins.font.clone()))
                    .id();
                commands.entity(entity).add_child(label);
                commands.entity(buttons).add_child(entity);
            }
        }
    }
    root
}

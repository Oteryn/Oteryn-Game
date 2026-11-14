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
    let root = commands
        .spawn((
            Root,
            GameModal,
            GlobalZIndex(90),
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
    source::dialog_caption(commands, panel, "Move stack", skins.font.clone());
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
    if let Some(item) = ui.split_entry {
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
            Field(prompt),
            TextInput,
            SelectAllOnFocus,
            EditableText {
                max_characters: Some(20),
                ..EditableText::new(value(ui))
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
        .observe(super::input::changed)
        .id();
    commands.entity(frame).add_child(input);
    let range = commands
        .spawn((
            Range,
            source::text(
                format!("of {}", prompt.item.count.min(100)),
                source::font(skins.font.clone(), false),
            ),
        ))
        .id();
    commands.entity(row).add_children(&[frame, range]);
    let feedback = commands
        .spawn((
            Feedback,
            source::text("", source::font(skins.font.clone(), false)),
            Node {
                width: percent(100.),
                ..default()
            },
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(panel).add_child(feedback);
    let buttons = commands
        .spawn(Node {
            column_gap: px(4.),
            justify_content: JustifyContent::End,
            ..default()
        })
        .id();
    commands.entity(panel).add_child(buttons);
    for (caption, control, width) in [
        ("All", Control::All(prompt), 30.),
        ("−", Control::Count(prompt, -1), 22.),
        ("+", Control::Count(prompt, 1), 22.),
        ("Move", Control::Confirm(prompt), 50.),
        (
            "Cancel",
            Control::Cancel {
                generation: prompt.generation,
                prompt: Some(prompt),
            },
            50.,
        ),
    ] {
        super::super::dialogs::button(commands, skins, buttons, caption, control, width);
    }
    root
}

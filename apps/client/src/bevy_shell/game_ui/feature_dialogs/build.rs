use super::*;
use crate::bevy_shell::window_drag::DraggableDialog;
use bevy::{
    asset::RenderAssetUsages,
    input_focus::tab_navigation::{TabGroup, TabIndex},
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    text::TextCursorStyle,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui::server_modal::Panel;
use oteryn_client_ui_bevy as source;

pub(super) fn panel(
    commands: &mut Commands,
    skin: &Skins,
    overlay: Entity,
    key: Key,
    model: &Panel,
    images: &mut Assets<Image>,
) -> (Entity, Option<Entity>) {
    let panel = commands
        .spawn((
            DraggableDialog,
            TabGroup {
                order: 4,
                modal: true,
            },
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(520.),
                max_width: percent(96.),
                max_height: percent(94.),
                padding: UiRect {
                    top: px(24.),
                    bottom: px(10.),
                    left: px(12.),
                    right: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(panel);
    let Some(dialog) = &model.active else {
        return (panel, None);
    };
    source::dialog_caption(commands, panel, &dialog.title, skin.font.clone());
    let content = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                min_height: px(0.),
                flex_shrink: 1.,
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(panel).add_child(content);
    if let Some(image) = &dialog.image {
        let handle = images.add(Image::new(
            Extent3d {
                width: u32::from(image.width),
                height: u32::from(image.height),
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            image.rgba.clone(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        ));
        let icon = commands
            .spawn((
                ImageNode::new(handle),
                Node {
                    width: px(f32::from(image.width)),
                    height: px(f32::from(image.height)),
                    flex_shrink: 0.,
                    ..default()
                },
            ))
            .id();
        commands.entity(content).add_child(icon);
    }
    let message = commands
        .spawn(source::text(
            &dialog.message,
            source::font(skin.font.clone(), false),
        ))
        .insert((
            TextLayout::default(),
            Node {
                width: percent(100.),
                ..default()
            },
        ))
        .id();
    commands.entity(content).add_child(message);
    let mut first = None;
    for (id, label) in &dialog.choices {
        let choice = action(
            commands,
            skin,
            content,
            key,
            ActionKind::Choice(*id),
            label,
            460.,
        );
        let caption = commands
            .spawn(source::button_caption(label, skin.font.clone()))
            .insert(TextLayout::default())
            .id();
        commands.entity(choice).add_child(caption);
        first = first.or(Some(choice));
    }
    if let Some(input) = &model.active_input {
        let prompt = commands
            .spawn(source::text(
                server_modal_input::prompt(input),
                source::font(skin.font.clone(), false),
            ))
            .id();
        commands.entity(content).add_child(prompt);
        let frame = commands
            .spawn((
                source::frame(skin.inset.clone(), [1.; 4]),
                Node {
                    width: percent(100.),
                    min_height: px(26.),
                    ..default()
                },
            ))
            .id();
        let mut editable = EditableText::new(server_modal_input::initial_text(input));
        editable.allow_newlines =
            input.kind == oteryn_client_runtime::server_modal::input::InputKind::Edit;
        editable.max_characters = Some(
            if input.kind == oteryn_client_runtime::server_modal::input::InputKind::Amount {
                20 // Maximum decimal length of the accepted u64; the core validates numeric limits.
            } else {
                input.maximum as usize
            },
        );
        editable.visible_lines = Some(if editable.allow_newlines { 4. } else { 1. });
        let field = commands
            .spawn((
                ValueField,
                TextInput,
                editable,
                source::font(skin.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::default(),
                TextCursorStyle::default(),
                TabIndex(0),
                Node {
                    width: percent(100.),
                    min_height: px(26.),
                    padding: px(4.).all(),
                    ..default()
                },
            ))
            .id();
        commands.entity(frame).add_child(field);
        commands.entity(content).add_child(frame);
        first = Some(field);
    }
    if model.input {
        let remember = commands
            .spawn((
                source::checkbox(source::checkbox_row(false)),
                Action {
                    key,
                    kind: ActionKind::Remember,
                },
            ))
            .observe(super::remember)
            .id();
        let indicator = commands
            .spawn((
                source::checkbox_indicator(skin.unchecked.clone()),
                RememberIndicator,
            ))
            .id();
        let caption = commands
            .spawn(source::text(
                "Don't show again",
                source::font(skin.font.clone(), false),
            ))
            .id();
        commands
            .entity(remember)
            .add_children(&[indicator, caption]);
        commands.entity(content).add_child(remember);
    }
    let feedback = commands
        .spawn((
            source::text("", source::font(skin.font.clone(), false)),
            Feedback,
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(panel).add_child(feedback);
    let footer = commands
        .spawn(Node {
            width: percent(100.),
            flex_wrap: FlexWrap::Wrap,
            justify_content: JustifyContent::End,
            column_gap: px(5.),
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(panel).add_child(footer);
    for (id, label) in &dialog.buttons {
        let button = action(
            commands,
            skin,
            footer,
            key,
            ActionKind::Button(*id),
            label,
            100.,
        );
        first = first.or(Some(button));
    }
    (panel, first)
}
fn action(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    key: Key,
    kind: ActionKind,
    label: &str,
    width: f32,
) -> Entity {
    let entity = commands
        .spawn((
            source::button(skin.buttons.clone(), width),
            Action { key, kind },
        ))
        .insert(Node {
            width: px(width),
            max_width: percent(100.),
            min_height: px(20.),
            flex_shrink: 0.,
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        })
        .observe(activate)
        .id();
    if matches!(kind, ActionKind::Button(_)) {
        let caption = commands
            .spawn(source::button_caption(label, skin.font.clone()))
            .id();
        commands.entity(entity).add_child(caption);
    }
    commands.entity(parent).add_child(entity);
    entity
}

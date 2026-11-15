use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup, picking::Pickable, ui::InteractionDisabled,
    ui_widgets::ScrollArea,
};
use oteryn_client_ui_bevy as source;
pub(super) struct Widgets {
    pub overlay: Entity,
    pub panel: Entity,
    pub content: Option<Entity>,
}
pub(super) fn build(commands: &mut Commands, skins: &Skins) -> Widgets {
    let overlay = commands
        .spawn((
            Overlay,
            GameModal,
            GlobalZIndex(165),
            TabGroup {
                order: 2,
                modal: true,
            },
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
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(780.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect {
                    left: px(12.),
                    right: px(12.),
                    top: px(24.),
                    bottom: px(10.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(overlay).add_child(panel);
    source::dialog_caption(commands, panel, "Wheel of Destiny", skins.font.clone());
    Widgets {
        overlay,
        panel,
        content: None,
    }
}
pub(super) fn render(
    commands: &mut Commands,
    skins: &Skins,
    widgets: &mut Widgets,
    key: Key,
    panel: &WheelPanel,
) {
    if let Some(content) = widgets.content.take() {
        commands.entity(content).despawn();
    }
    let content = commands
        .spawn((
            ScrollArea,
            crate::bevy_shell::game_ui::install::RememberScroll("wheel", 0),
            Node {
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                overflow: Overflow::scroll(),
                min_height: px(0.),
                ..default()
            },
        ))
        .id();
    commands.entity(widgets.panel).add_child(content);
    widgets.content = Some(content);
    label(
        commands,
        skins,
        content,
        "Left click: add one point. Right click: remove one point.",
    );
    let metadata = diagram::metadata(panel);
    if let Some(reply) = metadata {
        let spent: u32 = panel.points.iter().map(|p| u32::from(*p)).sum();
        label(
            commands,
            skins,
            content,
            format!(
                "Promotion points: {}   Allocated: {}   Remaining: {}",
                reply.available,
                spent,
                reply.available.saturating_sub(spent)
            ),
        );
    }
    if !panel.selected || !panel.diagram.selected {
        label(
            commands,
            skins,
            content,
            "Wheel allocation is unavailable in this session.",
        );
    } else if metadata.is_none() {
        label(
            commands,
            skins,
            content,
            "Waiting for current Wheel data. Refresh if the preview is unavailable.",
        );
    }
    if panel.pending.is_some() || panel.diagram.pending.is_some() {
        label(commands, skins, content, "Waiting for the server…");
    }
    for message in [
        panel.feedback.unwrap_or(""),
        &panel.diagram.feedback,
        &panel.preset_feedback,
    ] {
        if !message.is_empty() {
            label(commands, skins, content, message);
        }
    }
    if let Some(reply) = diagram::current(panel).filter(|r| !r.draft_valid) {
        label(commands, skins, content, &reply.reason);
    }
    let ready = panel.ready()
        && panel.diagram.pending.is_none()
        && panel.gems.pending.is_none()
        && panel.gems.review.is_none()
        && metadata.is_some_and(|r| r.eligible && r.nodes.len() == 36);
    let body = row(commands, content);
    if let Some(reply) = metadata.filter(|r| r.nodes.len() == 36) {
        let board = commands
            .spawn((
                Node {
                    position_type: PositionType::Relative,
                    width: px(540.),
                    height: px(540.),
                    flex_shrink: 0.,
                    ..default()
                },
                BackgroundColor(Color::srgb(0.06, 0.07, 0.10)),
            ))
            .id();
        commands.entity(body).add_child(board);
        // Use the retained native graph's six-by-six positions and server-provided links.
        if let Some(edges) = diagram::graph::edges(&reply.nodes) {
            for (a, b) in edges {
                if let (Some((x, y)), Some((u, v))) =
                    (diagram::graph::center(a), diagram::graph::center(b))
                {
                    let start = Vec2::new(x as f32, y as f32) * 1.40625;
                    let end = Vec2::new(u as f32, v as f32) * 1.40625;
                    let length = start.distance(end);
                    let mid = (start + end) * 0.5;
                    let edge = commands
                        .spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                left: px(mid.x - length / 2.),
                                top: px(mid.y - 1.),
                                width: px(length),
                                height: px(2.),
                                ..default()
                            },
                            UiTransform::from_rotation(Rot2::radians(
                                (end.y - start.y).atan2(end.x - start.x),
                            )),
                            BackgroundColor(Color::srgb(0.33, 0.35, 0.4)),
                            Pickable::IGNORE,
                        ))
                        .id();
                    commands.entity(board).add_child(edge);
                }
            }
        }
        for (index, node) in reply.nodes.iter().enumerate() {
            if let Some((x, y)) = diagram::graph::center(index) {
                let color = match node.domain {
                    0 => Color::srgb(0.14, 0.40, 0.22),
                    1 => Color::srgb(0.45, 0.18, 0.16),
                    2 => Color::srgb(0.34, 0.20, 0.45),
                    _ => Color::srgb(0.17, 0.32, 0.45),
                };
                let entity = commands
                    .spawn((
                        Control {
                            key,
                            kind: Kind::Node(index),
                        },
                        Node {
                            position_type: PositionType::Absolute,
                            left: px(x as f32 * 1.40625 - 40.),
                            top: px(y as f32 * 1.40625 - 36.),
                            width: px(80.),
                            height: px(72.),
                            border: UiRect::all(px(1.)),
                            border_radius: BorderRadius::all(px(12.)),
                            padding: UiRect::all(px(3.)),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(color),
                        BorderColor::all(if panel.points[index] == node.capacity {
                            Color::srgb(0.95, 0.85, 0.4)
                        } else {
                            Color::srgb(0.5, 0.5, 0.55)
                        }),
                    ))
                    .observe(node_click)
                    .id();
                commands.entity(board).add_child(entity);
                let text = commands
                    .spawn((
                        Text::new(format!(
                            "{}\n{} / {}",
                            node.name, panel.points[index], node.capacity
                        )),
                        TextFont {
                            font: skins.font.clone().into(),
                            font_size: bevy::text::FontSize::Px(11.),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        TextLayout::justify(Justify::Center),
                        Pickable::IGNORE,
                    ))
                    .id();
                commands.entity(entity).add_child(text);
            }
        }
    }
    let presets = commands
        .spawn(Node {
            width: px(180.),
            flex_direction: FlexDirection::Column,
            row_gap: px(5.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(body).add_child(presets);
    label(commands, skins, presets, "Point presets");
    if panel.preset_book.0.is_empty() {
        label(commands, skins, presets, "No saved point presets.");
    }
    for name in panel.preset_book.0.keys() {
        button(
            commands,
            skins,
            presets,
            Control {
                key,
                kind: Kind::Preset(name.clone()),
            },
            name,
            ready,
        );
    }
    if !panel.preset_fields[0].is_empty() {
        label(
            commands,
            skins,
            presets,
            format!("Selected: {}", panel.preset_fields[0]),
        );
    }
    let buttons = row(commands, content);
    for (kind, caption, enabled) in [
        (
            Kind::Save,
            "Save",
            ready && panel.dirty() && diagram::current(panel).is_some_and(|r| r.draft_valid),
        ),
        (
            Kind::Reset,
            "Reset",
            ready && metadata.is_some_and(|r| r.removal_permitted),
        ),
        (
            Kind::Refresh,
            "Refresh",
            panel.selected && panel.pending.is_none(),
        ),
        (
            Kind::Retry,
            "Retry",
            panel.retry.is_some() && panel.pending.is_none() && panel.diagram.pending.is_none(),
        ),
        (Kind::Close, "Close", true),
    ] {
        button(
            commands,
            skins,
            buttons,
            Control { key, kind },
            caption,
            enabled,
        );
    }
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            column_gap: px(8.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn label(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn((
            source::text(value, source::font(skins.font.clone(), false)),
            Node {
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    control: Control,
    caption: &str,
    enabled: bool,
) {
    let mut entity = commands.spawn((source::button(skins.buttons.clone(), 100.), control));
    if !enabled {
        entity.insert(InteractionDisabled);
    }
    let entity = entity.observe(activate).id();
    let text = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(text);
    commands.entity(parent).add_child(entity);
}

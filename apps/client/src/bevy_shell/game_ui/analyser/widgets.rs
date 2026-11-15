use super::*;
use bevy::{input_focus::tab_navigation::TabGroup, ui_widgets::ScrollArea};
use oteryn_client_ui_bevy as source;

pub(super) struct Widgets {
    pub key: (u64, Tool),
    pub overlay: Entity,
    pub report: Entity,
    pub text: String,
    pub tracker_filter: Option<Entity>,
}
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    generation: u64,
    tool: Tool,
) -> Widgets {
    let overlay = commands
        .spawn((
            Overlay(tool),
            GlobalZIndex(160),
            TabGroup {
                order: 2,
                modal: false,
            },
            Node {
                position_type: PositionType::Absolute,
                left: px(180.),
                top: px(80.),
                width: px(700.),
                max_width: percent(95.),
                max_height: percent(90.),
                ..default()
            },
        ))
        .id();
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(700.),
                max_width: percent(95.),
                max_height: percent(90.),
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
    source::dialog_caption(commands, panel, tool.title(), skins.font.clone());
    let tracker_filter =
        (tool == Tool::LootTracker).then(|| tracker::build(commands, skins, panel, generation));
    let scroll = commands
        .spawn((
            ScrollArea,
            Node {
                width: percent(100.),
                max_height: px(500.),
                min_height: px(0.),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    let report = commands
        .spawn((
            Report,
            source::text("", source::font(skins.font.clone(), false)),
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(scroll).add_child(report);
    commands.entity(panel).add_child(scroll);
    let goal = matches!(tool, Tool::Xp | Tool::Hunting | Tool::Session);
    let prices = matches!(
        tool,
        Tool::Loot | Tool::Supply | Tool::Hunting | Tool::Session | Tool::Party
    );
    if goal || prices {
        let inputs = commands
            .spawn(Node {
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(6.),
                ..default()
            })
            .id();
        for (index, label) in [
            (0, "XP still needed"),
            (1, "Item number"),
            (2, "Unit price"),
        ] {
            if (index == 0 && !goal) || (index != 0 && !prices) {
                continue;
            }
            let group = commands
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .id();
            let label = commands
                .spawn(source::text(label, source::font(skins.font.clone(), false)))
                .id();
            let field = commands
                .spawn((
                    ConfigField(index, tool),
                    bevy::ui_widgets::TextInput,
                    bevy::text::EditableText {
                        max_characters: Some(20),
                        ..default()
                    },
                    bevy::text::TextCursorStyle::default(),
                    source::font(skins.font.clone(), false),
                    TextColor(source::TEXT_COLOR),
                    bevy::input_focus::tab_navigation::TabIndex(0),
                    Node {
                        width: px(160.),
                        height: px(24.),
                        ..default()
                    },
                    BackgroundColor(Color::srgb(0.05, 0.05, 0.05)),
                ))
                .id();
            commands.entity(group).add_children(&[label, field]);
            commands.entity(inputs).add_child(group);
        }
        commands.entity(panel).add_child(inputs);
        let actions = commands
            .spawn(Node {
                column_gap: px(6.),
                ..default()
            })
            .id();
        for (kind, label) in [
            (ConfigKind::Goal, "Set XP goal"),
            (ConfigKind::Loot, "Set loot value"),
            (ConfigKind::Supply, "Set supply cost"),
        ] {
            if (matches!(kind, ConfigKind::Goal) && !goal)
                || (!matches!(kind, ConfigKind::Goal) && !prices)
            {
                continue;
            }
            let button = commands
                .spawn((
                    source::button(skins.buttons.clone(), 130.),
                    ConfigControl {
                        generation,
                        tool,
                        kind,
                    },
                ))
                .id();
            let caption = commands
                .spawn(source::button_caption(label, skins.font.clone()))
                .id();
            commands.entity(button).add_child(caption);
            commands.entity(actions).add_child(button);
        }
        commands.entity(panel).add_child(actions);
    }
    let buttons = commands
        .spawn(Node {
            column_gap: px(8.),
            flex_wrap: FlexWrap::Wrap,
            flex_shrink: 0.,
            ..default()
        })
        .id();
    use oteryn_client_ui::boss_encounter::Intent;
    let queries = match tool {
        Tool::Party => vec![("Refresh party", None)],
        Tool::Bosses => vec![
            ("Refresh cooldowns", Some(Intent::Refresh)),
            ("Next cooldowns", Some(Intent::Next)),
            ("Refresh rewards", Some(Intent::Rewards)),
            ("Next rewards", Some(Intent::RewardsNext)),
        ],
        _ => Vec::new(),
    };
    for (label, boss) in queries {
        let button = commands
            .spawn((
                source::button(skins.buttons.clone(), 130.),
                Refresh {
                    generation,
                    tool,
                    boss,
                },
            ))
            .id();
        let caption = commands
            .spawn(source::button_caption(label, skins.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(buttons).add_child(button);
    }
    for (reset, label) in [(true, "Reset session"), (false, "Close")] {
        let button = commands
            .spawn((
                source::button(skins.buttons.clone(), 110.),
                Control {
                    generation,
                    tool,
                    reset,
                },
            ))
            .id();
        let caption = commands
            .spawn(source::button_caption(label, skins.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(buttons).add_child(button);
    }
    commands.entity(panel).add_child(buttons);
    Widgets {
        key: (generation, tool),
        overlay,
        report,
        text: String::new(),
        tracker_filter,
    }
}

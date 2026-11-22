use super::*;
use bevy::{input_focus::tab_navigation::TabGroup, ui_widgets::ScrollArea};
use oteryn_client_ui_bevy as source;

pub(super) struct Widgets {
    pub key: (u64, Tool),
    pub overlay: Entity,
    pub report: Entity,
    pub text: String,
    pub boss_rows: Option<Entity>,
    pub boss_keys: Vec<String>,
    pub needs_cooldowns: bool,
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
    let boss_rows = (tool == Tool::Bosses).then(|| {
        let rows = commands
            .spawn(Node {
                flex_direction: FlexDirection::Column,
                ..default()
            })
            .id();
        commands.entity(scroll).add_child(rows);
        rows
    });
    let goal = matches!(tool, Tool::Xp | Tool::Hunting | Tool::Session);
    let prices = matches!(
        tool,
        Tool::Loot | Tool::Supply | Tool::Hunting | Tool::Session | Tool::Party
    );
    let rates = Rate::CONTROLS.iter().filter(|(owner, _, _)| *owner == tool);
    let has_rate = rates.clone().next().is_some();
    if goal || prices || has_rate || matches!(tool, Tool::Input | Tool::Bosses) {
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
            (3, "Paste party hunt log"),
            (4, "Member name"),
            (5, "Extra supply cost"),
            (6, "Rate target"),
            (7, "New report file path"),
            (8, "Boss name"),
        ] {
            if (index == 0 && !goal)
                || ((1..=2).contains(&index) && !prices)
                || ((3..=5).contains(&index) && tool != Tool::Party)
                || (index == 6 && !has_rate)
                || (index == 7 && tool != Tool::Hunting)
                || (index == 8 && tool != Tool::Bosses)
            {
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
                        allow_newlines: index == 3,
                        max_characters: Some(match index {
                            3 => 8192,
                            4 | 8 => 128,
                            7 => 4096,
                            _ => 20,
                        }),
                        ..default()
                    },
                    bevy::text::TextCursorStyle::default(),
                    source::font(skins.font.clone(), false),
                    TextColor(source::TEXT_COLOR),
                    bevy::input_focus::tab_navigation::TabIndex(0),
                    Node {
                        width: px(160.),
                        height: px(if index == 3 { 64. } else { 24. }),
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
                flex_wrap: FlexWrap::Wrap,
                column_gap: px(6.),
                ..default()
            })
            .id();
        let mut controls = vec![
            (ConfigKind::Goal, "Set XP goal"),
            (ConfigKind::Loot, "Set loot value"),
            (ConfigKind::Supply, "Set supply cost"),
            (ConfigKind::Split, "Calculate split"),
            (ConfigKind::Cost, "Apply extra cost"),
            (ConfigKind::Include(false), "Exclude member"),
            (ConfigKind::Include(true), "Include member"),
        ];
        controls.retain(|(kind, _)| match kind {
            ConfigKind::Goal => goal,
            ConfigKind::Loot | ConfigKind::Supply => prices,
            _ => tool == Tool::Party,
        });
        if matches!(tool, Tool::Hunting | Tool::Input | Tool::Party) {
            controls.push((ConfigKind::Copy, "Copy report"));
        }
        if tool == Tool::Hunting {
            controls.extend([
                (ConfigKind::Save(false), "Save hunt report"),
                (ConfigKind::Save(true), "Export hunt JSON"),
            ]);
        }
        if tool == Tool::Party {
            controls.extend([
                (ConfigKind::Handoff, "Send to Loot Splitter"),
                (ConfigKind::CopySplit, "Copy settlement instructions"),
            ]);
        }
        if tool == Tool::Bosses {
            controls.extend([
                (ConfigKind::BossSearch, "Search bosses"),
                (ConfigKind::BossOrder(true), "Sort by name"),
                (ConfigKind::BossOrder(false), "Sort by cooldown"),
            ]);
        }
        controls.extend(rates.map(|(_, rate, label)| (ConfigKind::Target(*rate), *label)));
        controls.extend(
            Section::CONTROLS
                .iter()
                .filter(|(owner, _, _)| *owner == tool)
                .map(|(_, section, label)| (ConfigKind::Toggle(*section), *label)),
        );
        if matches!(tool, Tool::Impact | Tool::Input) {
            controls.push((ConfigKind::Mode, "Switch current/session rate"));
        }
        if tool == Tool::Impact {
            controls.push((ConfigKind::ResetHigh, "Reset all-time high"));
        }
        for (kind, label) in controls {
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
    let reset_label = if tool == Tool::Hunting {
        "Start new session"
    } else {
        "Reset session"
    };
    for (reset, label) in [(true, reset_label), (false, "Close")] {
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
        boss_keys: Vec::new(),
        boss_rows,
        needs_cooldowns: tool == Tool::Bosses,
        tracker_filter,
    }
}

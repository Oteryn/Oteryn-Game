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
            Overlay,
            GameModal,
            GlobalZIndex(160),
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
    let buttons = commands
        .spawn(Node {
            column_gap: px(8.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
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

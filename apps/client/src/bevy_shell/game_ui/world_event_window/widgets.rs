use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup, ui::InteractionDisabled, ui_widgets::ScrollArea,
};
use oteryn_client_ui_bevy as source;
pub(super) struct Widgets {
    pub root: Entity,
}
pub(super) fn build(commands: &mut Commands, skins: &Skins, key: Key, panel: &Panel) -> Widgets {
    let root = commands
        .spawn((
            Name::new("world_event.window"),
            crate::bevy_shell::window_drag::DraggableDialog,
            TabGroup::new(2),
            GlobalZIndex(40),
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                position_type: PositionType::Absolute,
                left: percent(20.),
                top: percent(20.),
                width: px(540.),
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
    source::dialog_caption(commands, root, "World events — Ctrl+N", skins.font.clone());
    let body = commands
        .spawn((
            ScrollArea,
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                height: px(130.),
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                padding: UiRect::all(px(5.)),
                ..default()
            },
        ))
        .id();
    let text = commands
        .spawn((
            Name::new("world_event.status"),
            source::text(
                oteryn_client_ui::world_event::panel_text(panel),
                source::font(skins.font.clone(), false),
            ),
            TextLayout::default(),
        ))
        .id();
    commands.entity(body).add_child(text);
    commands.entity(root).add_child(body);
    let row = commands
        .spawn(Node {
            column_gap: px(8.),
            ..default()
        })
        .id();
    commands.entity(root).add_child(row);
    for (kind, caption) in [(Kind::Refresh, "Refresh"), (Kind::Close, "Close")] {
        let button = commands
            .spawn((
                source::button(skins.buttons.clone(), 120.),
                Control { key, kind },
            ))
            .observe(activate)
            .id();
        if kind == Kind::Refresh && (!panel.selected || panel.pending) {
            commands.entity(button).insert(InteractionDisabled);
        }
        let caption = commands
            .spawn(source::button_caption(caption, skins.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(row).add_child(button);
    }
    Widgets { root }
}

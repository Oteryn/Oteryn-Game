use super::*;
use bevy::{
    input_focus::tab_navigation::TabGroup, ui::InteractionDisabled, ui_widgets::ScrollArea,
};
use oteryn_client_ui_bevy as source;
pub(super) struct Widgets {
    pub overlay: Entity,
    pub panel: Entity,
    content: Option<Entity>,
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
                width: px(640.),
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
    source::dialog_caption(
        commands,
        panel,
        "Animus Mastery — Ctrl+A",
        skins.font.clone(),
    );
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
    panel: &Panel,
) {
    if let Some(content) = widgets.content.take() {
        commands.entity(content).despawn();
    }
    let content = commands
        .spawn(Node {
            width: percent(100.),
            min_height: px(0.),
            flex_direction: FlexDirection::Column,
            row_gap: px(8.),
            ..default()
        })
        .id();
    widgets.content = Some(content);
    commands.entity(widgets.panel).add_child(content);
    let body = commands
        .spawn((
            source::frame(skins.inset.clone(), [1.; 4]),
            ScrollArea,
            Node {
                height: px(420.),
                max_height: percent(75.),
                padding: UiRect::all(px(6.)),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    commands.entity(content).add_child(body);
    let text = commands
        .spawn(source::text(
            oteryn_client_ui::animus_mastery::panel_text(panel).replace('\r', ""),
            source::font(skins.font.clone(), false),
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(body).add_child(text);
    let footer = commands
        .spawn(Node {
            column_gap: px(8.),
            flex_wrap: FlexWrap::Wrap,
            ..default()
        })
        .id();
    commands.entity(content).add_child(footer);
    let ready = panel.selected && panel.pending.is_none();
    for (kind, label, enabled) in [
        (Kind::Request(Intent::Refresh), "Refresh", ready),
        (
            Kind::Request(Intent::Previous),
            "Previous",
            ready && panel.has_previous(),
        ),
        (
            Kind::Request(Intent::Next),
            "Next",
            ready && panel.has_next(),
        ),
        (Kind::Close, "Close", true),
    ] {
        let mut button = commands.spawn((
            source::button(skins.buttons.clone(), 100.),
            Control { key, kind },
        ));
        if !enabled {
            button.insert(InteractionDisabled);
        }
        let button = button.observe(activate).id();
        let caption = commands
            .spawn(source::button_caption(label, skins.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(footer).add_child(button);
    }
}

use super::*;
use bevy::{input_focus::tab_navigation::TabGroup, ui::InteractionDisabled};
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
                width: px(700.),
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
    source::dialog_caption(commands, panel, "Raid schedule", skins.font.clone());
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
    let content = column(commands, widgets.panel);
    widgets.content = Some(content);
    text(
        commands,
        skins,
        content,
        oteryn_client_ui::raid_schedule::panel_text(panel),
    );
    let footer = row_node(commands, content);
    button(
        commands,
        skins,
        footer,
        key,
        Kind::Refresh,
        "Refresh",
        90.,
        panel,
    );
    button(
        commands,
        skins,
        footer,
        key,
        Kind::Close,
        "Close",
        70.,
        panel,
    );
}

fn column(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            width: percent(100.),
            min_height: px(0.),
            flex_direction: FlexDirection::Column,
            row_gap: px(4.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn row_node(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            column_gap: px(4.),
            flex_wrap: FlexWrap::Wrap,
            row_gap: px(3.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn text(commands: &mut Commands, skins: &Skins, parent: Entity, value: impl Into<String>) {
    let entity = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                max_width: percent(100.),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
}
// The button receives the canonical snapshot key and the source control's size/caption.
#[allow(clippy::too_many_arguments)]
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    kind: Kind,
    label: &str,
    width: f32,
    panel: &Panel,
) {
    let enabled = kind == Kind::Close || panel.selected && !panel.pending;
    let mut entity = commands.spawn((
        source::button(skins.buttons.clone(), width),
        Control { key, kind },
    ));
    if !enabled {
        entity.insert(InteractionDisabled);
    }
    let entity = entity.observe(activate).id();
    let text = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(text);
    commands.entity(parent).add_child(entity);
}

use super::*;
use oteryn_client_ui_bevy as source;
pub(super) struct Widgets {
    pub root: Entity,
    pub panel: Entity,
    pub content: Option<Entity>,
}
pub(super) fn build(commands: &mut Commands, skin: &Skins, target: Target) -> Widgets {
    let root = commands
        .spawn((
            Pickable::IGNORE,
            GlobalZIndex(160),
            bevy::input_focus::tab_navigation::TabGroup {
                order: 2,
                modal: false,
            },
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .id();
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            source::frame(skin.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(600.),
                max_width: percent(95.),
                max_height: percent(95.),
                padding: UiRect {
                    left: px(16.),
                    right: px(16.),
                    top: px(22.),
                    bottom: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(8.),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    source::dialog_caption(commands, panel, target.title(), skin.font.clone());
    Widgets {
        root,
        panel,
        content: None,
    }
}
pub(super) fn render(
    commands: &mut Commands,
    skin: &Skins,
    widgets: &mut Widgets,
    snapshot: &Snapshot,
    generation: u64,
    revision: u64,
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
    let scroll = commands
        .spawn((
            bevy::ui_widgets::ScrollArea,
            source::frame(skin.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                max_height: px(500.),
                min_height: px(0.),
                overflow: Overflow::scroll_y(),
                padding: UiRect::all(px(5.)),
                ..default()
            },
        ))
        .id();
    let text = commands
        .spawn(source::text(
            snapshot.text(),
            source::font(skin.font.clone(), false),
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(content).add_child(scroll);
    commands.entity(scroll).add_child(text);
    let footer = commands
        .spawn(Node {
            justify_content: JustifyContent::End,
            column_gap: px(8.),
            ..default()
        })
        .id();
    commands.entity(content).add_child(footer);
    for (action, caption) in [(Action::Refresh, "Refresh"), (Action::Close, "Close")] {
        let button = commands
            .spawn((
                Control {
                    target: snapshot.target,
                    generation,
                    revision,
                    action,
                },
                source::button(skin.buttons.clone(), 80.),
            ))
            .observe(activate)
            .id();
        if action == Action::Refresh && !snapshot.ready() {
            commands
                .entity(button)
                .insert(bevy::ui::InteractionDisabled);
        }
        let label = commands
            .spawn(source::button_caption(caption, skin.font.clone()))
            .id();
        commands.entity(button).add_child(label);
        commands.entity(footer).add_child(button);
    }
}

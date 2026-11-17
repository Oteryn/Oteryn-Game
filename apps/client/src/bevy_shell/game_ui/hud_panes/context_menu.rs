//! Cursor-anchored HUD menus use the scaled dimensions of their UI target.
use crate::bevy_shell::gameplay_dialogs::popup;
use bevy::prelude::*;

#[derive(Component)]
pub(super) struct CursorAnchor(Vec2);

// Reserve the menu's intrinsic size first. The preceding track can then grow
// up to the cursor coordinate using only the space left on this axis.
fn tracks(cursor: f32) -> Vec<RepeatedGridTrack> {
    vec![
        GridTrack::minmax(
            MinTrackSizingFunction::Px(0.),
            MaxTrackSizingFunction::Px(cursor.max(0.)),
        ),
        GridTrack::max_content(),
    ]
}

pub(in crate::bevy_shell::game_ui) fn spawn(
    commands: &mut Commands,
    look: &popup::Look,
    pointer: Vec2,
    scale: Option<&UiScale>,
    items: &[popup::Item],
) -> (popup::Popup, Entity) {
    let anchor = pointer / scale.map_or(1., |scale| scale.0);
    let menu = popup::spawn(commands, look, Vec2::ZERO, items);
    commands
        .entity(menu.root)
        .remove::<popup::PopupMenu>()
        .entry::<Node>()
        .and_modify(|mut node| {
            node.position_type = PositionType::Relative;
            node.grid_column = GridPlacement::start(2);
            node.grid_row = GridPlacement::start(2);
        });
    let layer = commands
        .spawn((
            super::super::GameModal,
            GlobalZIndex(200),
            CursorAnchor(pointer),
            Node {
                position_type: PositionType::Absolute,
                display: Display::Grid,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Start,
                align_content: AlignContent::Start,
                grid_template_columns: tracks(anchor.x),
                grid_template_rows: tracks(anchor.y),
                ..default()
            },
        ))
        .add_children(&[menu.scrim, menu.root])
        .id();
    (menu, layer)
}

pub(super) fn layout(scale: Option<Res<UiScale>>, mut menus: Query<(&CursorAnchor, &mut Node)>) {
    let Some(scale) = scale.filter(|scale| scale.is_changed()) else {
        return;
    };
    for (cursor, mut node) in &mut menus {
        node.grid_template_columns = tracks(cursor.0.x / scale.0);
        node.grid_template_rows = tracks(cursor.0.y / scale.0);
    }
}

#[cfg(test)]
pub(in crate::bevy_shell::game_ui) fn assert_anchor(world: &World, frame: Entity, anchor: Vec2) {
    let layer = world
        .get::<ChildOf>(frame)
        .map(ChildOf::parent)
        .and_then(|parent| world.get::<Node>(parent));
    assert!(
        layer.is_some_and(|node| node.grid_template_columns == tracks(anchor.x)
            && node.grid_template_rows == tracks(anchor.y))
    );
}

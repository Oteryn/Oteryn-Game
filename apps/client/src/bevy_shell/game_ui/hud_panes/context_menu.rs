//! Cursor-anchored HUD menus use the scaled dimensions of their UI target.
use crate::bevy_shell::gameplay_dialogs::popup;
use bevy::prelude::*;

#[derive(Component)]
pub(super) struct CursorAnchor(Vec2);

pub(in crate::bevy_shell::game_ui) fn spawn(
    commands: &mut Commands,
    look: &popup::Look,
    pointer: Vec2,
    scale: Option<&UiScale>,
    items: &[popup::Item],
) -> popup::Popup {
    let anchor = pointer / scale.map_or(1., |scale| scale.0);
    let menu = popup::spawn(commands, look, anchor, items);
    commands
        .entity(menu.root)
        .remove::<popup::PopupMenu>()
        .insert(CursorAnchor(pointer));
    menu
}

pub(super) fn layout(
    scale: Option<Res<UiScale>>,
    mut menus: Query<(
        &CursorAnchor,
        &ComputedNode,
        &bevy::ui::ComputedUiRenderTargetInfo,
        &mut Node,
    )>,
) {
    for (cursor, computed, target, mut node) in &mut menus {
        if target.physical_size() == UVec2::ZERO {
            continue;
        }
        let anchor = cursor.0 / scale.as_deref().map_or(1., |scale| scale.0);
        let size = computed.size() * computed.inverse_scale_factor;
        let at = popup::clamp(anchor, size, target.logical_size());
        if node.left != px(at.x) || node.top != px(at.y) {
            node.left = px(at.x);
            node.top = px(at.y);
        }
    }
}

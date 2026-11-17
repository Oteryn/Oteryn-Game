//! Pointer gestures share the camera and marker state used by the minimap projection.
use super::*;
use bevy::picking::{
    events::{PointerDrag, PointerScroll},
    pointer::PointerButton,
};

#[derive(Component, Default)]
pub(super) struct DragState {
    remainder: Vec2,
    dragged: bool,
}
pub(super) fn press(
    event: On<bevy::picking::events::PointerPress>,
    mut nodes: Query<&mut DragState>,
) {
    if let Ok(mut state) = nodes.get_mut(event.entity) {
        *state = DragState::default();
    }
}
pub(super) fn drag(
    event: On<PointerDrag>,
    windows: Query<&Window>,
    mut nodes: Query<(&MiniImage, &mut DragState, &ComputedNode)>,
    mut owner: NonSendMut<HostOwner>,
    guard: super::super::modal::ModalGuard,
) {
    if owner.modal_active() || !guard.allows(event.entity) || event.button != PointerButton::Primary
    {
        return;
    }
    let Ok((image, mut state, computed)) = nodes.get_mut(event.entity) else {
        return;
    };
    if image.0 != owner.generation() {
        return;
    }
    let Some(play) = &mut owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    state.dragged = true;
    use bevy::ecs::entity::ContainsEntity;
    let dpi = match &event.pointer.target {
        bevy::camera::NormalizedRenderTarget::Window(window) => windows
            .get(window.entity())
            .map_or(1., Window::scale_factor),
        _ => 1.,
    };
    let size = computed.size() / dpi;
    let scale = if size.x > 0. && size.y > 0. {
        size / SIZE.as_vec2()
    } else {
        Vec2::ONE
    };
    state.remainder -= event.delta / (4. * scale);
    let delta = state.remainder.trunc();
    state.remainder -= delta;
    let _ = play
        .view
        .navigate_minimap(crate::play::MinimapNavigation::Pan(
            delta.x as i32,
            delta.y as i32,
        ));
}
pub(super) fn scroll(
    mut event: On<PointerScroll>,
    nodes: Query<&MiniImage>,
    mut owner: NonSendMut<HostOwner>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    guard: super::super::modal::ModalGuard,
) {
    if owner.modal_active() || !guard.allows(event.entity) || event.y == 0. {
        return;
    }
    let Ok(image) = nodes.get(event.entity) else {
        return;
    };
    if image.0 != owner.generation() {
        return;
    }
    let Some(play) = &mut owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    use crate::play::MinimapNavigation as Nav;
    let floor = keys
        .as_ref()
        .is_some_and(|keys| keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]));
    let action = match (floor, event.y > 0.) {
        (true, true) => Nav::FloorDown,
        (true, false) => Nav::FloorUp,
        (false, true) => Nav::ZoomIn,
        (false, false) => Nav::ZoomOut,
    };
    let _ = play.view.navigate_minimap(action);
    event.propagate(false);
}
pub(super) fn was_dragged(state: &mut DragState) -> bool {
    std::mem::take(&mut state.dragged)
}

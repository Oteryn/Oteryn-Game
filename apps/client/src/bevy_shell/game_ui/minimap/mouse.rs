//! Pointer gestures share the camera and marker state used by the minimap projection.
use super::{
    super::{hud_panes::context_menu, modal::ModalGuard},
    markers::{Editor, MarkerKey},
    *,
};
use crate::bevy_shell::gameplay_dialogs::popup;
use bevy::picking::{
    events::{PointerDrag, PointerScroll},
    pointer::PointerButton,
};
use bevy::ui_widgets::Activate;

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

#[derive(Resource, Default)]
pub(super) struct MarkMenu(Option<(Entity, MarkerKey)>);

pub(super) fn install(app: &mut App) {
    app.init_resource::<MarkMenu>()
        .add_systems(Update, close_stale_menu);
}

pub(super) fn secondary_click(
    event: On<PointerClick>,
    nodes: Query<&MiniImage>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    (skins, scale): (Res<Skins>, Option<Res<UiScale>>),
    mut menu: ResMut<MarkMenu>,
    mut commands: Commands,
) {
    if event.button != PointerButton::Secondary
        || menu.0.is_some()
        || owner.modal_active()
        || !guard.allows(event.entity)
    {
        return;
    }
    let Ok(node) = nodes.get(event.entity) else {
        return;
    };
    // Bevy UI picking reports coordinates centred on the entire image, including its crop.
    let Some(hit) = event.hit.position else {
        return;
    };
    let point = hit.truncate() + Vec2::splat(0.5);
    if !(0.0..1.0).contains(&point.x) || !(0.0..1.0).contains(&point.y) {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    if node.0 != owner.generation() || play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    let (x, y, floor, zoom, _) = play.view.minimap_camera();
    let (Some(x), Some(y)) = (
        x.checked_add(((point.x * 31.) as i32 - 15) * zoom),
        y.checked_add(((point.y * 25.) as i32 - 12) * zoom),
    ) else {
        return;
    };
    if !(0..=65535).contains(&x) || !(0..=65535).contains(&y) {
        return;
    }

    let at = (x, y, floor);
    let names: &[&str] = if super::markers::has_world_mark(&play.view, at)
        || play.view.remembered_minimap_mark(at).is_some()
    {
        &["Edit Mark", "Delete Mark"]
    } else {
        &["Set Mark"]
    };
    let items: Vec<_> = names.iter().map(|name| popup::Item::entry(*name)).collect();
    let look = popup::Look {
        font: skins.font.clone(),
        frame: skins.frame.clone(),
        checked: None,
        unchecked: None,
    };
    let (opened, layer) = context_menu::spawn(
        &mut commands,
        &look,
        event.pointer.location().position,
        scale.as_deref(),
        &items,
    );
    commands.entity(opened.scrim).observe(dismiss_menu);
    for (entry, &name) in opened.entries.into_iter().zip(names) {
        let mut entity = commands.entity(entry);
        entity.insert(Name::new(name));
        if name == "Delete Mark" {
            entity.observe(delete_mark);
        } else {
            entity.observe(set_mark);
        }
    }
    let key = (
        node.0,
        (x, y, floor),
        play.view.minimap_binding_generation(),
    );
    menu.0 = Some((layer, key));
}

impl MarkMenu {
    fn close(&mut self, commands: &mut Commands) -> Option<MarkerKey> {
        let (layer, key) = self.0.take()?;
        commands.entity(layer).despawn();
        Some(key)
    }
}

fn menu_target_is_current(owner: &HostOwner, key: MarkerKey) -> bool {
    key.0 == owner.generation()
        && !owner.modal_active()
        && owner.play.as_ref().is_some_and(|play| {
            key.2 == play.view.minimap_binding_generation()
                && !play.view.gameplay_input_blocked(&play.link)
        })
}

fn close_stale_menu(
    owner: NonSend<HostOwner>,
    keys: Res<ButtonInput<KeyCode>>,
    mut menu: ResMut<MarkMenu>,
    mut commands: Commands,
) {
    if let Some((_, key)) = menu.0
        && (keys.just_pressed(KeyCode::Escape) || !menu_target_is_current(&owner, key))
    {
        menu.close(&mut commands);
    }
}

fn dismiss_menu(
    event: On<PointerClick>,
    guard: ModalGuard,
    mut menu: ResMut<MarkMenu>,
    mut commands: Commands,
) {
    if guard.allows(event.entity) {
        menu.close(&mut commands);
    }
}

fn set_mark(
    event: On<Activate>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut menu: ResMut<MarkMenu>,
    mut editor: ResMut<Editor>,
    mut commands: Commands,
) {
    if guard.allows(event.entity)
        && let Some(key) = menu.close(&mut commands)
        && menu_target_is_current(&owner, key)
    {
        editor.open(key);
    }
}

fn delete_mark(
    event: On<Activate>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
    mut menu: ResMut<MarkMenu>,
    mut commands: Commands,
) {
    if guard.allows(event.entity)
        && let Some(key) = menu.close(&mut commands)
        && menu_target_is_current(&owner, key)
    {
        super::markers::remove_mark(&mut owner, key.1);
    }
}

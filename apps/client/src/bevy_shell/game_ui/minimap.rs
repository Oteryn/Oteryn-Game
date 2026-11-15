//! Sidebar projection of the existing Scene minimap, rendered through an upstream image target.
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameRegions,
};
use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    render::render_resource::TextureFormat,
    ui::RelativeCursorPosition,
};
use oteryn_client_ui_bevy as source;
pub(in crate::bevy_shell) const SIZE: UVec2 = UVec2::new(124, 100);
#[derive(Resource)]
struct State {
    image: Handle<Image>,
    panel: Option<Entity>,
    generation: Option<u64>,
}
#[derive(Component)]
struct MiniImage(u64);
pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup.after(super::setup))
        .add_systems(Update, (project, shortcuts, camera_label).chain());
}
fn setup(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let image = images.add(Image::new_target_texture(
        SIZE.x,
        SIZE.y,
        TextureFormat::Rgba8UnormSrgb,
        None,
    ));
    commands.spawn((
        Camera2d,
        Camera {
            order: -1,
            ..default()
        },
        RenderTarget::Image(image.clone().into()),
        RenderLayers::layer(2),
    ));
    commands.insert_resource(State {
        image,
        panel: None,
        generation: None,
    });
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    regions: Res<GameRegions>,
    skins: Res<Skins>,
) {
    let generation = owner.play.as_ref().map(|_| owner.generation());
    if state.generation == generation {
        return;
    }
    if let Some(panel) = state.panel.take() {
        commands.entity(panel).despawn();
    }
    state.generation = generation;
    let Some(generation) = generation else {
        return;
    };
    let panel = commands
        .spawn((
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: percent(100.),
                min_height: px(120.),
                flex_shrink: 0.,
                padding: UiRect::all(px(3.)),
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    let title = commands
        .spawn(source::text(
            "Minimap",
            source::font(skins.font.clone(), false),
        ))
        .id();
    let image = commands
        .spawn((
            MiniImage(generation),
            ImageNode::new(state.image.clone()),
            RelativeCursorPosition::default(),
            Node {
                width: px(SIZE.x as f32),
                height: px(SIZE.y as f32),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .observe(click)
        .id();
    commands.entity(panel).add_children(&[title, image]);
    navigation_controls(&mut commands, &skins, panel, generation);
    commands.entity(regions.minimap).add_child(panel);
    state.panel = Some(panel);
}
fn click(
    event: On<PointerClick>,
    nodes: Query<(&MiniImage, &RelativeCursorPosition)>,
    mut owner: NonSendMut<HostOwner>,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
    guard: super::modal::ModalGuard,
) {
    if event.button != bevy::picking::pointer::PointerButton::Primary
        || owner.modal_active()
        || !guard.allows(event.entity)
    {
        return;
    }
    let Ok((node, cursor)) = nodes.get(event.entity) else {
        return;
    };
    if node.0 != owner.generation() {
        return;
    }
    let Some(point) = cursor.normalized else {
        return;
    };
    let Some(play) = &mut owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    focus.clear();
    play.view.minimap_click(
        f64::from(point.x) * f64::from(SIZE.x),
        f64::from(point.y) * f64::from(SIZE.y),
    );
}

#[cfg(test)]
mod tests;

#[derive(Component)]
struct NavigationControl {
    generation: u64,
    action: crate::play::MinimapNavigation,
}
#[derive(Component)]
struct CameraLabel;
fn navigation_controls(commands: &mut Commands, skins: &Skins, panel: Entity, generation: u64) {
    use crate::play::MinimapNavigation as Nav;
    for row in [
        vec![("N", Nav::Pan(0, -5))],
        vec![
            ("W", Nav::Pan(-5, 0)),
            ("Centre", Nav::Centre),
            ("E", Nav::Pan(5, 0)),
        ],
        vec![("S", Nav::Pan(0, 5))],
        vec![("Zoom out", Nav::ZoomOut), ("Zoom in", Nav::ZoomIn)],
        vec![("Floor up", Nav::FloorUp), ("Floor down", Nav::FloorDown)],
    ] {
        let width = SIZE.x as f32 / row.len() as f32;
        let bar = commands
            .spawn(Node {
                width: px(SIZE.x as f32),
                justify_content: JustifyContent::Center,
                ..default()
            })
            .id();
        commands.entity(panel).add_child(bar);
        for (label, action) in row {
            let button = commands
                .spawn((
                    source::button(skins.buttons.clone(), width),
                    NavigationControl { generation, action },
                ))
                .observe(navigate)
                .id();
            let text = commands
                .spawn(source::text(label, source::font(skins.font.clone(), false)))
                .id();
            commands.entity(button).add_child(text);
            commands.entity(bar).add_child(button);
        }
    }
    let status = commands
        .spawn((
            source::text("", source::font(skins.font.clone(), false)),
            CameraLabel,
        ))
        .id();
    commands.entity(panel).add_child(status);
}
fn apply_navigation(owner: &mut HostOwner, action: crate::play::MinimapNavigation) {
    let Some(play) = &mut owner.play else {
        return;
    };
    if play.view.gameplay_input_blocked(&play.link) {
        return;
    }
    if let Err(error) = play.view.navigate_minimap(action) {
        owner.status = format!("Minimap navigation failed: {error}");
    }
}
fn navigate(
    event: On<bevy::ui_widgets::Activate>,
    controls: Query<&NavigationControl>,
    mut owner: NonSendMut<HostOwner>,
    guard: super::modal::ModalGuard,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
) {
    if owner.modal_active() || !guard.allows(event.entity) {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    if control.generation != owner.generation() {
        return;
    }
    focus.clear();
    apply_navigation(&mut owner, control.action);
}
fn shortcuts(
    keys: Res<ButtonInput<KeyCode>>,
    mut owner: NonSendMut<HostOwner>,
    regions: Res<GameRegions>,
    guard: super::modal::ModalGuard,
    focus: Res<bevy::input_focus::InputFocus>,
    editors: Query<(), With<bevy::text::EditableText>>,
) {
    if !(keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight))
        || [
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::ShiftLeft,
            KeyCode::ShiftRight,
            KeyCode::SuperLeft,
            KeyCode::SuperRight,
        ]
        .iter()
        .any(|key| keys.pressed(*key))
        || owner.modal_active()
        || !guard.allows(regions.minimap)
        || !owner.settings.preferences.panel_visible("minimap")
        || focus.get().is_some_and(|entity| editors.contains(entity))
    {
        return;
    }
    use crate::play::MinimapNavigation as Nav;
    for (key, action) in [
        (KeyCode::ArrowLeft, Nav::Pan(-5, 0)),
        (KeyCode::ArrowRight, Nav::Pan(5, 0)),
        (KeyCode::ArrowUp, Nav::Pan(0, -5)),
        (KeyCode::ArrowDown, Nav::Pan(0, 5)),
        (KeyCode::Home, Nav::ZoomOut),
        (KeyCode::End, Nav::ZoomIn),
        (KeyCode::PageUp, Nav::FloorUp),
        (KeyCode::PageDown, Nav::FloorDown),
    ] {
        if keys.just_pressed(key) {
            apply_navigation(&mut owner, action);
        }
    }
}
fn camera_label(owner: NonSend<HostOwner>, mut labels: Query<&mut Text, With<CameraLabel>>) {
    let Some(play) = &owner.play else {
        return;
    };
    let (_, _, floor, zoom, tracking) = play.view.minimap_camera();
    let mode = if tracking { "Following" } else { "Browsing" };
    let label = format!("Floor {floor} / {zoom} tiles\n{mode}");
    for mut text in &mut labels {
        if text.0 != label {
            text.0.clone_from(&label);
        }
    }
}

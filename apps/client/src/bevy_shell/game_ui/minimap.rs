//! Sidebar projection of the existing Scene minimap, rendered through an upstream image target.
mod markers;
mod mouse;
use super::{
    super::{owner::HostOwner, ui::Skins},
    GameRegions, art,
};
use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
    render::render_resource::TextureFormat,
    ui::RelativeCursorPosition,
};
use oteryn_client_ui_bevy as source;
pub(in crate::bevy_shell) const SIZE: UVec2 = UVec2::new(124, 100);
// The sidebar atlas uses the builtin six-level RGB palette (r * 36 + g * 6 + b).
pub(super) fn palette_color(index: u8) -> Color {
    Color::srgb_u8(
        (index / 36 % 6) * 51,
        (index / 6 % 6) * 51,
        (index % 6) * 51,
    )
}
#[derive(Resource)]
struct State {
    image: Handle<Image>,
    panel: Option<Entity>,
    generation: Option<u64>,
}
#[derive(Component)]
struct MiniImage(u64);
pub(super) fn install(app: &mut App) {
    markers::install(app);
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
    // Tibia Global minimap widget: a 108x111 viewport, the 43x43 rose at the top right and
    // the layer and zoom buttons below the rose.
    let panel = commands
        .spawn((
            Name::new("Minimap"),
            Node {
                width: percent(100.),
                height: px(VIEWPORT.y),
                flex_shrink: 0.,
                column_gap: px(2.),
                ..default()
            },
        ))
        .id();
    let viewport = commands
        .spawn((
            Name::new("Minimap viewport"),
            Node {
                width: px(VIEWPORT.x),
                height: px(VIEWPORT.y),
                flex_shrink: 0.,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .id();
    // The rendered map keeps its full size, so clicks and drags map 1:1 to camera pixels; the
    // viewport crops the side columns to the Tibia width.
    let image = commands
        .spawn((
            MiniImage(generation),
            mouse::DragState::default(),
            ImageNode::new(state.image.clone()),
            RelativeCursorPosition::default(),
            Node {
                position_type: PositionType::Absolute,
                left: px(-CROP),
                width: px(SIZE.x as f32),
                height: px(SIZE.y as f32),
                ..default()
            },
        ))
        .observe(click)
        .observe(markers::click)
        .observe(mouse::press)
        .observe(mouse::drag)
        .observe(mouse::scroll)
        .id();
    commands.entity(viewport).add_child(image);
    let controls = commands
        .spawn(Node {
            width: px(ROSE),
            flex_shrink: 0.,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: px(2.),
            ..default()
        })
        .id();
    commands.entity(panel).add_children(&[viewport, controls]);
    navigation_controls(&mut commands, &skins, controls, generation);
    commands.entity(regions.minimap).add_child(panel);
    state.panel = Some(panel);
}
fn click(
    event: On<PointerClick>,
    mut nodes: Query<(&MiniImage, &RelativeCursorPosition, &mut mouse::DragState)>,
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
    let Ok((node, cursor, mut drag)) = nodes.get_mut(event.entity) else {
        return;
    };
    if mouse::was_dragged(&mut drag) || node.0 != owner.generation() {
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
/// Tibia Global minimap viewport (minimap.qml) and rose size.
const VIEWPORT: Vec2 = Vec2::new(108., 111.);
const ROSE: f32 = 43.;
/// The columns cropped from each side of the rendered map to fit the viewport width.
const CROP: f32 = (SIZE.x as f32 - VIEWPORT.x) / 2.;
fn navigation_controls(commands: &mut Commands, skins: &Skins, panel: Entity, generation: u64) {
    use crate::play::MinimapNavigation as Nav;
    // The rose: its eight points pan five tiles that way and its centre resumes following.
    let rose = art::fixed(commands, &skins.font, "minimap-rose", Vec2::splat(ROSE));
    commands.entity(rose).insert(Node {
        width: px(ROSE),
        height: px(ROSE),
        flex_shrink: 0.,
        display: Display::Grid,
        grid_template_columns: RepeatedGridTrack::fr(3, 1.),
        grid_template_rows: RepeatedGridTrack::fr(3, 1.),
        ..default()
    });
    for (name, action) in [
        ("Rose NW", Nav::Pan(-5, -5)),
        ("Rose N", Nav::Pan(0, -5)),
        ("Rose NE", Nav::Pan(5, -5)),
        ("Rose W", Nav::Pan(-5, 0)),
        ("Rose centre", Nav::Centre),
        ("Rose E", Nav::Pan(5, 0)),
        ("Rose SW", Nav::Pan(-5, 5)),
        ("Rose S", Nav::Pan(0, 5)),
        ("Rose SE", Nav::Pan(5, 5)),
    ] {
        let point = commands
            .spawn((
                Name::new(name),
                NavigationControl { generation, action },
                bevy::ui_widgets::Button,
                Node {
                    position_type: PositionType::Relative,
                    ..default()
                },
            ))
            .observe(navigate)
            .id();
        commands.entity(rose).add_child(point);
    }
    commands.entity(panel).add_child(rose);
    let buttons = commands
        .spawn(Node {
            width: px(ROSE),
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::px(2, 20.),
            column_gap: px(2.),
            row_gap: px(2.),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .id();
    commands.entity(panel).add_child(buttons);
    for (key, action) in [
        ("minimap-floor-up", Some(Nav::FloorUp)),
        ("minimap-zoom-in", Some(Nav::ZoomIn)),
        ("minimap-floor-down", Some(Nav::FloorDown)),
        ("minimap-zoom-out", Some(Nav::ZoomOut)),
        ("minimap-centre", Some(Nav::Centre)),
        ("minimap-full-map", None),
    ] {
        let button = art::fixed(commands, &skins.font, key, Vec2::splat(20.));
        commands
            .entity(button)
            .insert(bevy::ui_widgets::Button)
            .observe(navigate);
        match action {
            Some(action) => commands
                .entity(button)
                .insert(NavigationControl { generation, action }),
            None => commands.entity(button).insert(FullMap(generation)),
        };
        commands.entity(buttons).add_child(button);
    }
    // The layer indicator: the shown floor as a slider-style track of the 16 floors.
    let layer = commands
        .spawn((
            Name::new("Minimap layer"),
            Node {
                width: px(ROSE),
                height: px(4.),
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.1, 0.1)),
        ))
        .id();
    let knob = commands
        .spawn((
            LayerKnob,
            Node {
                position_type: PositionType::Absolute,
                width: px(5.),
                height: percent(100.),
                ..default()
            },
            BackgroundColor(Color::srgb(0.75, 0.75, 0.75)),
        ))
        .id();
    commands.entity(layer).add_child(knob);
    commands.entity(panel).add_child(layer);
}
/// The full map button of one session generation.
#[derive(Component)]
struct FullMap(u64);
/// The marker of the shown floor on the layer track.
#[derive(Component)]
struct LayerKnob;
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
    full_map: Query<&FullMap>,
    mut owner: NonSendMut<HostOwner>,
    guard: super::modal::ModalGuard,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
) {
    if owner.modal_active() || !guard.allows(event.entity) {
        return;
    }
    if let Ok(full_map) = full_map.get(event.entity) {
        if full_map.0 == owner.generation() {
            focus.clear();
            super::super::ui::toolbar::open_toolbar(
                &mut owner,
                super::super::ui::Control::Shortcut("map"),
            );
        }
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
/// Places the layer knob at the shown floor: the deepest (-15) left, the highest (0) right.
fn camera_label(owner: NonSend<HostOwner>, mut knobs: Query<&mut Node, With<LayerKnob>>) {
    let Some(play) = &owner.play else {
        return;
    };
    let (_, _, floor, _, _) = play.view.minimap_camera();
    let left = px(f32::from((floor + 15).clamp(0, 15)) * (ROSE - 5.) / 15.);
    for mut node in &mut knobs {
        if node.left != left {
            node.left = left;
        }
    }
}

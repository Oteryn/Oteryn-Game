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
        .add_systems(Update, project);
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
    commands.entity(regions.minimap).add_child(panel);
    state.panel = Some(panel);
}
fn click(
    event: On<PointerClick>,
    nodes: Query<(&MiniImage, &RelativeCursorPosition)>,
    mut owner: NonSendMut<HostOwner>,
    mut focus: ResMut<bevy::input_focus::InputFocus>,
) {
    if event.button != bevy::picking::pointer::PointerButton::Primary || owner.modal_active() {
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

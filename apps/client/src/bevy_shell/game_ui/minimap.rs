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
mod tests {
    use super::*;
    use bevy::{
        input_focus::{FocusCause, InputFocus},
        picking::{
            backend::HitData,
            events::Pointer,
            pointer::{Location, PointerButton, PointerId},
        },
    };
    #[test]
    fn minimap_click_respects_foreground_modal() -> Result<(), Box<dyn std::error::Error>> {
        let mut host = HostOwner::new(None)?;
        let (link, _, _) = crate::play::play_channel();
        host.play = Some(crate::bevy_shell::owner::ActivePlay {
            view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
            link,
        });
        let generation = host.generation();
        let mut app = App::new();
        app.insert_non_send(host).init_resource::<InputFocus>();
        let image = app
            .world_mut()
            .spawn((
                MiniImage(generation),
                RelativeCursorPosition {
                    normalized: Some(Vec2::splat(0.5)),
                    ..default()
                },
                Node::default(),
            ))
            .observe(click)
            .id();
        let focused = app.world_mut().spawn_empty().id();
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(focused, FocusCause::Navigated);
        app.world_mut()
            .spawn((super::super::GameModal, Node::default(), GlobalZIndex(999)));
        app.world_mut().trigger(PointerClick {
            entity: image,
            pointer: Pointer::new(
                PointerId::Mouse,
                Location {
                    target: bevy::camera::NormalizedRenderTarget::None {
                        width: 100,
                        height: 100,
                    },
                    position: Vec2::ZERO,
                },
            ),
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
            duration: std::time::Duration::ZERO,
            count: 1,
        });
        assert_eq!(app.world().resource::<InputFocus>().get(), Some(focused));
        Ok(())
    }
}

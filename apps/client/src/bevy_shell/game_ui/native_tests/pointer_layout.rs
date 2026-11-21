//! Drive the UI layout and picking backend with a headless window and camera.
use bevy::{
    asset::AssetPlugin,
    camera::{ComputedCameraValues, RenderTargetInfo},
    picking::{
        backend::PointerHits,
        events::PointerState,
        hover::HoverMap,
        pointer::{Location, PointerId, PointerInput, PointerLocation},
    },
    prelude::*,
    ui::UiGlobalTransform,
    window::{PrimaryWindow, WindowRef},
};

pub(super) fn install(app: &mut App, size: UVec2) {
    if !app.is_plugin_added::<AssetPlugin>() {
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>();
    }
    app.add_plugins((bevy::text::TextPlugin, bevy::ui::UiPlugin))
        .init_resource::<Assets<TextureAtlasLayout>>()
        .init_resource::<ButtonInput<MouseButton>>()
        .init_resource::<bevy::input::touch::Touches>()
        .init_resource::<HoverMap>()
        .init_resource::<PointerState>()
        .init_resource::<bevy::picking::pointer::PointerMap>()
        .add_message::<PointerInput>()
        .add_message::<PointerHits>();
    let world = app.world_mut();
    if let Some(mut window) = world
        .query_filtered::<&mut Window, With<PrimaryWindow>>()
        .iter_mut(world)
        .next()
    {
        window.resolution.set_physical_resolution(size.x, size.y);
    } else {
        world.spawn((
            Window {
                resolution: (size.x, size.y).into(),
                ..default()
            },
            PrimaryWindow,
        ));
    }
    if world.query::<&Camera>().iter(world).next().is_none() {
        world.spawn((Camera2d, IsDefaultUiCamera));
    }
    for mut camera in world.query::<&mut Camera>().iter_mut(world) {
        camera.computed = ComputedCameraValues {
            target_info: Some(RenderTargetInfo {
                physical_size: size,
                scale_factor: 1.,
            }),
            ..default()
        };
    }
}

pub(super) fn center(app: &App, entity: Entity) -> Result<Vec2, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .get::<UiGlobalTransform>(entity)
        .ok_or("UI transform")?
        .translation)
}

pub(super) fn hit(
    app: &mut App,
    point: Vec2,
    expected: Entity,
) -> Result<(), Box<dyn std::error::Error>> {
    app.update();
    let world = app.world_mut();
    // These headless apps omit the renderer's inherited visibility propagation.
    for mut visibility in world.query::<&mut InheritedVisibility>().iter_mut(world) {
        *visibility = InheritedVisibility::VISIBLE;
    }
    let window = world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world)?;
    let target = WindowRef::Entity(window).normalize(None).ok_or("target")?;
    world.spawn((
        PointerId::Mouse,
        PointerLocation::new(Location {
            target: bevy::camera::NormalizedRenderTarget::Window(target),
            position: point,
        }),
    ));
    world.resource_mut::<Messages<PointerHits>>().clear();
    app.update();
    let world = app.world_mut();
    let target = world
        .resource_mut::<Messages<PointerHits>>()
        .drain()
        .flat_map(|hits| hits.picks)
        .next()
        .ok_or("pointer missed")?
        .0;
    let mut current = Some(target);
    while let Some(entity) = current {
        if entity == expected {
            return Ok(());
        }
        current = world.get::<ChildOf>(entity).map(ChildOf::parent);
    }
    Err(format!(
        "hit {:?} instead of {:?} at {point:?}",
        world.get::<Name>(target),
        world.get::<Name>(expected)
    )
    .into())
}

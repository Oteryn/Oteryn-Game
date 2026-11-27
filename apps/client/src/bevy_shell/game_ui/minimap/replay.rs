//! Headless driver for the sidebar's production projection and observers.
use super::*;
use bevy::{
    asset::AssetPlugin,
    camera::NormalizedRenderTarget,
    image::ImagePlugin,
    picking::{
        backend::HitData,
        events::{Pointer, PointerDrag},
        pointer::{Location, PointerButton, PointerId},
    },
    render::{RenderPlugin, renderer::RenderDevice, view::screenshot::ScreenshotCaptured},
    text::{EditableText, TextEdit, TextPlugin},
    ui_widgets::Activate,
    window::PrimaryWindow,
};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

pub(in crate::bevy_shell) fn app(owner: HostOwner, render: bool) -> Result<App, String> {
    let mut app = App::new();
    if render {
        app.add_plugins(
            DefaultPlugins
                .build()
                .disable::<bevy::winit::WinitPlugin>()
                .disable::<bevy::log::LogPlugin>()
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: None,
                    ..default()
                })
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..default()
                }),
        );
        app.init_resource::<crate::bevy_shell::scene::SceneCache>()
            .init_resource::<super::super::viewport::WorldViewport>()
            .insert_resource(ClearColor(Color::BLACK))
            .add_systems(Update, crate::bevy_shell::scene::project);
        // The scene projector needs window dimensions. The sidebar camera still
        // renders to its image and this entity has no native surface.
        app.world_mut().spawn((Window::default(), PrimaryWindow));
    } else {
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .add_plugins(TextPlugin)
            .init_resource::<bevy::input_focus::InputFocus>()
            .init_resource::<ButtonInput<KeyCode>>();
    }
    app.add_plugins(source::GlobalUiPlugin)
        .insert_non_send(owner)
        .add_systems(
            Startup,
            (crate::bevy_shell::ui::setup, super::super::setup).chain(),
        )
        .add_systems(Update, super::super::project);
    install(&mut app);
    let start = Instant::now();
    while app.plugins_state() == bevy::app::PluginsState::Adding {
        if start.elapsed() > Duration::from_secs(30) {
            return Err("minimap replay renderer initialization timed out".into());
        }
        bevy::tasks::tick_global_task_pools_on_main_thread();
    }
    app.finish();
    app.cleanup();
    app.update();
    Ok(app)
}

pub(in crate::bevy_shell) fn activate(app: &mut App, name: &str) -> Result<(), String> {
    let world = app.world_mut();
    let entity = world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find_map(|(entity, label)| (label.as_str() == name).then_some(entity))
        .ok_or_else(|| format!("missing minimap button {name}"))?;
    world.trigger(Activate { entity });
    app.update();
    Ok(())
}

fn pointer() -> Pointer {
    Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: SIZE.x,
                height: SIZE.y,
            },
            position: Vec2::ZERO,
        },
    )
}

fn image(world: &mut World) -> Result<Entity, String> {
    world
        .query_filtered::<Entity, With<MiniImage>>()
        .single(world)
        .map_err(|error| error.to_string())
}

pub(in crate::bevy_shell) fn edit(app: &mut App, cell: [i32; 2]) -> Result<(), String> {
    let world = app.world_mut();
    let entity = image(world)?;
    let point = (Vec2::new(cell[0] as f32, cell[1] as f32) + Vec2::splat(0.5)) * 4.;
    world.entity_mut(entity).insert(RelativeCursorPosition {
        normalized: Some(point / SIZE.as_vec2() - Vec2::splat(0.5)),
        ..default()
    });
    world.trigger(PointerClick {
        entity,
        pointer: pointer(),
        button: PointerButton::Secondary,
        hit: HitData::new(entity, 0., None, None),
        duration: Duration::ZERO,
        count: 1,
    });
    app.update();
    Ok(())
}

pub(in crate::bevy_shell) fn description(app: &mut App, label: &str) -> Result<(), String> {
    let world = app.world_mut();
    let mut field = world
        .query::<(&Name, &mut EditableText)>()
        .iter_mut(world)
        .find_map(|(name, field)| (name.as_str() == "Mark description").then_some(field))
        .ok_or("missing mark description field")?;
    field.clear();
    field.queue_edit(TextEdit::Insert(label.into()));
    app.update();
    Ok(())
}

pub(in crate::bevy_shell) fn pan(app: &mut App, tiles: [i32; 2]) -> Result<(), String> {
    let world = app.world_mut();
    let entity = image(world)?;
    let size = world
        .get::<ComputedNode>(entity)
        .ok_or("missing minimap layout")?
        .size();
    let scale = if size.cmpgt(Vec2::ZERO).all() {
        size / SIZE.as_vec2()
    } else {
        Vec2::ONE
    };
    let tiles = Vec2::new(tiles[0] as f32, tiles[1] as f32);
    let delta = -(4. * tiles + 0.5 * tiles.signum() * tiles.abs().min(Vec2::ONE)) * scale;
    world.trigger(PointerDrag {
        entity,
        pointer: pointer(),
        button: PointerButton::Primary,
        distance: delta,
        delta,
    });
    app.update();
    Ok(())
}

pub(in crate::bevy_shell) fn pixels(app: &mut App) -> Result<Vec<[u8; 4]>, String> {
    let request = app.world().resource::<MinimapRenderTarget>().screenshot();
    let (send, receive) = mpsc::channel();
    app.world_mut()
        .spawn(request)
        .observe(move |event: On<ScreenshotCaptured>| {
            let _ = send.send(event.image.clone());
        });
    let start = Instant::now();
    loop {
        app.update();
        if let Ok(image) = receive.try_recv() {
            assert_eq!(image.size(), SIZE);
            let bytes = image.data.ok_or("minimap screenshot has no pixel data")?;
            let (pixels, remainder) = bytes.as_chunks::<4>();
            assert!(remainder.is_empty());
            assert_eq!(pixels.len(), (SIZE.x * SIZE.y) as usize);
            return Ok(pixels.to_vec());
        }
        if start.elapsed() > Duration::from_secs(15) {
            return Err("minimap render target readback timed out".into());
        }
        app.world()
            .resource::<RenderDevice>()
            .poll(bevy::render::render_resource::PollType::Wait {
                submission_index: None,
                timeout: Some(Duration::from_secs(5)),
            })
            .map_err(|error| error.to_string())?;
    }
}

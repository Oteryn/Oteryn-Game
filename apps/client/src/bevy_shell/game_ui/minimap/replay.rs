//! Headless driver for the sidebar's production projection and observers.
use super::*;
use bevy::{
    asset::AssetPlugin,
    camera::RenderTargetInfo,
    image::ImagePlugin,
    input::{
        ButtonState, InputPlugin,
        keyboard::{Key, KeyboardInput},
        mouse::MouseButtonInput,
    },
    picking::DefaultPickingPlugins,
    render::{RenderPlugin, renderer::RenderDevice, view::screenshot::ScreenshotCaptured},
    text::{EditableText, TextPlugin},
    ui::UiGlobalTransform,
    window::{CursorMoved, PrimaryWindow, WindowEvent},
};
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

pub(in crate::bevy_shell) fn app(owner: HostOwner, render: bool) -> Result<App, String> {
    let mut app = App::new();
    let window = WindowPlugin {
        primary_window: Some(Window {
            resolution: (1200, 900).into(),
            ..default()
        }),
        ..default()
    };
    if render {
        app.add_plugins(
            DefaultPlugins
                .build()
                .disable::<bevy::winit::WinitPlugin>()
                .disable::<bevy::log::LogPlugin>()
                .set(ImagePlugin::default_nearest())
                .set(window)
                .set(RenderPlugin {
                    synchronous_pipeline_compilation: true,
                    ..default()
                }),
        );
        app.init_resource::<crate::bevy_shell::scene::SceneCache>()
            .init_resource::<super::super::viewport::WorldViewport>()
            .insert_resource(ClearColor(Color::BLACK))
            .add_systems(Update, crate::bevy_shell::scene::project);
    } else {
        app.add_plugins((
            MinimalPlugins,
            AssetPlugin::default(),
            ImagePlugin::default_nearest(),
            window,
            InputPlugin,
            bevy::input_focus::InputDispatchPlugin,
            bevy::ui_widgets::TextInputPlugin,
            TextPlugin,
            bevy::ui::UiPlugin,
            bevy::mesh::MeshPlugin,
            bevy::camera::visibility::VisibilityPlugin,
            DefaultPickingPlugins,
            bevy::ui_widgets::ButtonPlugin,
        ))
        .init_asset::<TextureAtlasLayout>()
        .add_systems(
            Startup,
            configure_camera.after(crate::bevy_shell::ui::setup),
        );
    }
    super::super::modal::install(&mut app);
    app.insert_resource(crate::bevy_shell::ui::BuiltInFont);
    app.add_plugins(source::GlobalUiPlugin)
        .insert_non_send(owner)
        .add_systems(
            Startup,
            (crate::bevy_shell::ui::setup, super::super::setup).chain(),
        )
        .add_systems(
            Update,
            (
                super::super::project,
                crate::bevy_shell::ui::project_toolbar,
            ),
        );
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

fn configure_camera(
    mut camera: Single<&mut Camera, With<IsDefaultUiCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    // In the headless app there is no render plugin to compute the UI camera target.
    camera.computed.target_info = Some(RenderTargetInfo {
        physical_size: window.physical_size(),
        scale_factor: window.scale_factor(),
    });
}

pub(super) fn named(world: &mut World, name: &str) -> Result<Entity, String> {
    world
        .query::<(Entity, &Name)>()
        .iter(world)
        .find_map(|(entity, label)| (label.as_str() == name).then_some(entity))
        .ok_or_else(|| format!("missing minimap widget {name}"))
}

pub(in crate::bevy_shell) fn activate(app: &mut App, name: &str) -> Result<(), String> {
    let entity = named(app.world_mut(), name)?;
    let (center, _) = bounds(app.world(), entity)?;
    click(app, center, MouseButton::Left)
}

fn image(world: &mut World) -> Result<Entity, String> {
    world
        .query_filtered::<Entity, With<MiniImage>>()
        .single(world)
        .map_err(|error| error.to_string())
}

fn bounds(world: &World, entity: Entity) -> Result<(Vec2, Vec2), String> {
    let node = world
        .get::<ComputedNode>(entity)
        .ok_or("missing UI layout")?;
    let size = node.size();
    if !size.cmpgt(Vec2::ZERO).all() {
        return Err(format!(
            "zero UI layout for {:?}",
            world.get::<Name>(entity)
        ));
    }
    let transform = world
        .get::<UiGlobalTransform>(entity)
        .ok_or("missing UI transform")?;
    let scale = world
        .get::<bevy::ui::ComputedUiRenderTargetInfo>(entity)
        .ok_or("missing UI target")?
        .scale_factor();
    // The layout coordinates include both DPI and UiScale. Window events only include DPI.
    let dpi = scale / world.resource::<UiScale>().0;
    Ok((transform.translation / dpi, size / dpi))
}

fn window(app: &mut App) -> Result<Entity, String> {
    let world = app.world_mut();
    world
        .query_filtered::<Entity, With<PrimaryWindow>>()
        .single(world)
        .map_err(|error| error.to_string())
}

fn move_pointer(app: &mut App, window: Entity, position: Vec2) -> Result<(), String> {
    let world = app.world_mut();
    world
        .get_mut::<Window>(window)
        .ok_or("missing pointer window")?
        .set_cursor_position(Some(position));
    world.write_message(WindowEvent::CursorMoved(CursorMoved {
        window,
        position,
        delta: None,
    }));
    app.update();
    Ok(())
}

fn button(app: &mut App, window: Entity, button: MouseButton, state: ButtonState) {
    let input = MouseButtonInput {
        button,
        state,
        window,
    };
    app.world_mut().write_message(input);
    app.world_mut()
        .write_message(WindowEvent::MouseButtonInput(input));
    app.update();
}

pub(super) fn click(app: &mut App, point: Vec2, mouse: MouseButton) -> Result<(), String> {
    let window = window(app)?;
    move_pointer(app, window, point)?;
    button(app, window, mouse, ButtonState::Pressed);
    button(app, window, mouse, ButtonState::Released);
    Ok(())
}

fn cell_point(app: &mut App, cell: [i32; 2]) -> Result<Vec2, String> {
    let entity = image(app.world_mut())?;
    let (center, size) = bounds(app.world(), entity)?;
    let point = (Vec2::new(cell[0] as f32, cell[1] as f32) + Vec2::splat(0.5)) * 4.;
    Ok(center + (point / SIZE.as_vec2() - 0.5) * size)
}

pub(in crate::bevy_shell) fn open_menu(app: &mut App, cell: [i32; 2]) -> Result<(), String> {
    let point = cell_point(app, cell)?;
    click(app, point, MouseButton::Right)
}

pub(in crate::bevy_shell) fn hovered_description(
    app: &mut App,
    cell: [i32; 2],
) -> Result<Option<String>, String> {
    let point = cell_point(app, cell)?;
    let window = window(app)?;
    move_pointer(app, window, point)?;
    app.update();
    let world = app.world_mut();
    let (text, visibility) = world
        .query_filtered::<(&Text, &Visibility), With<MarkDescription>>()
        .single(world)
        .map_err(|error| error.to_string())?;
    Ok((*visibility == Visibility::Visible).then(|| text.0.clone()))
}

pub(in crate::bevy_shell) fn description(app: &mut App, label: &str) -> Result<(), String> {
    let entity = named(app.world_mut(), "Mark description")?;
    let (center, _) = bounds(app.world(), entity)?;
    click(app, center, MouseButton::Left)?;
    let window = window(app)?;
    let count = app
        .world()
        .get::<EditableText>(entity)
        .ok_or("missing mark description field")?
        .value()
        .chars()
        .count();
    let mut key = |code, logical_key, text| {
        app.world_mut().write_message(KeyboardInput {
            key_code: code,
            logical_key,
            state: ButtonState::Pressed,
            text,
            repeat: false,
            window,
        });
        app.update();
    };
    key(KeyCode::End, Key::End, None);
    for _ in 0..count {
        key(KeyCode::Backspace, Key::Backspace, None);
    }
    for character in label.chars() {
        let text = character.to_string();
        key(
            KeyCode::Unidentified(bevy::input::keyboard::NativeKeyCode::Unidentified),
            Key::Character(text.as_str().into()),
            Some(text.as_str().into()),
        );
    }
    assert_eq!(
        app.world()
            .get::<EditableText>(entity)
            .ok_or("missing mark description field")?
            .value(),
        label
    );
    Ok(())
}

pub(in crate::bevy_shell) fn pan(app: &mut App, tiles: [i32; 2]) -> Result<(), String> {
    let entity = image(app.world_mut())?;
    let (center, size) = bounds(app.world(), entity)?;
    let tiles = Vec2::new(tiles[0] as f32, tiles[1] as f32);
    let delta = -(4. * tiles + 0.5 * tiles.signum()) * size / SIZE.as_vec2();
    let window = window(app)?;
    move_pointer(app, window, center)?;
    button(app, window, MouseButton::Left, ButtonState::Pressed);
    move_pointer(app, window, center + delta)?;
    button(app, window, MouseButton::Left, ButtonState::Released);
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

use bevy::app::AppExit;
use bevy::asset::RenderAssetUsages;
use bevy::image::TextureAtlasLayout;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::{PresentMode, WindowResolution};
use bevy::winit::WinitSettings;
use oteryn_graphics_bakeoff_shared::{
    ATLAS_COLUMNS, ATLAS_ROWS, BenchConfig, BenchSamples, RenderSnapshot, animation_frame,
    build_snapshot, result_json, synthetic_atlas_rgba8,
};
use std::time::{Duration, Instant};

#[derive(Component)]
struct AnimatedSprite {
    phase: u32,
}

#[derive(Resource)]
struct BenchRuntime {
    config: BenchConfig,
    snapshot: RenderSnapshot,
    process_start: Instant,
    startup: Option<Duration>,
    last_tick: Instant,
    frame_number: u64,
    samples: BenchSamples,
    completed: bool,
}

impl BenchRuntime {
    fn new(config: BenchConfig, snapshot: RenderSnapshot) -> Self {
        let now = Instant::now();
        Self {
            config,
            snapshot,
            process_start: now,
            startup: None,
            last_tick: now,
            frame_number: 0,
            samples: BenchSamples::default(),
            completed: false,
        }
    }
}

fn main() {
    let config = match BenchConfig::from_env_args() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("bevy-bakeoff-error: {error}");
            return;
        }
    };
    let snapshot = build_snapshot(&config);
    let width = config.width;
    let height = config.height;
    let _exit = App::new()
        .insert_resource(BenchRuntime::new(config, snapshot))
        .insert_resource(WinitSettings::continuous())
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Oteryn graphics bake-off — Bevy 0.19.1".to_owned(),
                        present_mode: PresentMode::AutoNoVsync,
                        resolution: WindowResolution::new(width, height)
                            .with_scale_factor_override(1.0),
                        ..default()
                    }),
                    ..default()
                }),
        )
        .add_systems(Startup, setup)
        .add_systems(Update, benchmark_tick)
        .run();
}

fn setup(
    mut commands: Commands,
    runtime: Res<BenchRuntime>,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let atlas_width = runtime.config.sprite_px * ATLAS_COLUMNS;
    let atlas_height = runtime.config.sprite_px * ATLAS_ROWS;
    let image = Image::new(
        Extent3d {
            width: atlas_width,
            height: atlas_height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        synthetic_atlas_rgba8(runtime.config.sprite_px),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let image_handle = images.add(image);
    let layout_handle = layouts.add(TextureAtlasLayout::from_grid(
        UVec2::splat(runtime.config.sprite_px),
        ATLAS_COLUMNS,
        ATLAS_ROWS,
        None,
        None,
    ));

    commands.spawn(Camera2d);

    let static_batch: Vec<_> = runtime
        .snapshot
        .static_quads
        .iter()
        .copied()
        .map(|quad| {
            (
                Sprite {
                    image: image_handle.clone(),
                    texture_atlas: Some(TextureAtlas {
                        layout: layout_handle.clone(),
                        index: quad.frame as usize,
                    }),
                    custom_size: Some(Vec2::splat(quad.logical_size)),
                    ..default()
                },
                Transform::from_xyz(quad.x, quad.y, quad.z),
            )
        })
        .collect();
    commands.spawn_batch(static_batch);

    let animated_batch: Vec<_> = runtime
        .snapshot
        .animated_quads
        .iter()
        .copied()
        .map(|quad| {
            (
                Sprite {
                    image: image_handle.clone(),
                    texture_atlas: Some(TextureAtlas {
                        layout: layout_handle.clone(),
                        index: quad.frame as usize,
                    }),
                    custom_size: Some(Vec2::splat(quad.logical_size)),
                    ..default()
                },
                Transform::from_xyz(quad.x, quad.y, quad.z),
                AnimatedSprite { phase: quad.phase },
            )
        })
        .collect();
    commands.spawn_batch(animated_batch);
}

fn benchmark_tick(
    mut runtime: ResMut<BenchRuntime>,
    mut sprites: Query<(&AnimatedSprite, &mut Sprite)>,
    mut app_exit: MessageWriter<AppExit>,
) {
    if runtime.completed {
        return;
    }

    let frame_start = Instant::now();
    let frame_interval = frame_start.duration_since(runtime.last_tick);
    runtime.last_tick = frame_start;
    if runtime.startup.is_none() {
        runtime.startup = Some(runtime.process_start.elapsed());
    }

    let prep_start = Instant::now();
    let frame_number = runtime.frame_number;
    for (animated, mut sprite) in &mut sprites {
        if let Some(atlas) = sprite.texture_atlas.as_mut() {
            atlas.index = animation_frame(animated.phase, frame_number) as usize;
        }
    }
    let prep = prep_start.elapsed();

    if runtime.frame_number >= runtime.config.warmup_frames {
        runtime.samples.record(frame_interval, prep);
    }
    runtime.frame_number += 1;

    if runtime.samples.len() >= runtime.config.sample_frames as usize {
        let startup = runtime.startup.unwrap_or_default();
        println!(
            "{}",
            result_json(
                "bevy-0.19.1",
                &runtime.config,
                &runtime.samples,
                startup,
                "physical-adapter-recorded-by-run-wrapper",
                None,
            )
        );
        runtime.completed = true;
        app_exit.write(AppExit::Success);
    }
}

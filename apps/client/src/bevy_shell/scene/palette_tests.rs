//! Constant palette sampling through sprite projection and native capture.
use super::{tests::bounds, *};
use oteryn_renderer::TileCoord;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn constant_uv_preserves_texel_tint_clipping_and_dpi() -> TestResult {
    let atlas = AtlasImage::new(
        2,
        1,
        1,
        vec![
            0, 0, 0, 255, 128, 64, 192, 128, 255, 0, 0, 255, 0, 255, 0, 255,
        ],
    )?;
    let quad = QuadInstance {
        position: [-8., -4.],
        size: [32., 16.],
        uv: [0.75, 0.25, 0.75, 0.25],
    };
    let (sprite, transform) = project_quad(
        &quad,
        [0.5, 0.25, 1., 0.5],
        3,
        &Handle::default(),
        &atlas,
        QuadViewport {
            scale: 2.,
            size: Vec2::splat(64.),
            offset: Vec2::ZERO,
            clip_size: Vec2::new(16., 8.),
            scroll: Vec2::ZERO,
        },
    )
    .ok_or("palette quad")?;
    assert_eq!(
        sprite.rect, None,
        "constant sampling must not use an empty source rectangle"
    );
    let expected = Color::srgba_u8(128, 64, 192, 128).to_linear();
    assert_eq!(
        sprite.color.to_linear(),
        LinearRgba::new(
            expected.red * 0.5,
            expected.green * 0.25,
            expected.blue,
            expected.alpha * 0.5,
        )
    );
    assert_eq!(sprite.custom_size, Some(Vec2::new(8., 4.)));
    assert_eq!(transform.translation, Vec3::new(-12., 14., 0.003));
    Ok(())
}

/// Set `OTERYN_BEVY_SCENE_CAPTURE=1` to qualify these sprites through native GPU
/// readback. The test saves `scene.png` in a fresh temporary directory. Pass
/// `-- --nocapture` to Cargo to see the saved path.
#[test]
fn palette_gauges_health_icons_and_text_reach_bevy_and_native_capture() -> TestResult {
    use crate::play::{PlayView, play_channel};
    use oteryn_session::*;
    let mut view = PlayView::new(TileCoord::new(100, 100), -7, &[])?;
    let mut preferences = view.preferences().clone();
    preferences.creature_bars = true;
    preferences.hud_npc_icons = true;
    view.set_preferences(preferences)?;
    let (link, _commands, outbox) = play_channel();
    outbox.bevy_hud_fixture(
        &[],
        Some(ActorVitals {
            health: 50,
            max_health: 100,
            mana: 25,
            max_mana: 100,
            harmony: 3,
            serene: true,
            ..default()
        }),
    );
    let position = ActorPosition {
        x: 100,
        y: 100,
        floor: -7,
    };
    let entity = EntityRef {
        identity: [7; 16],
        generation: 1,
    };
    let actor = WorldSpatialEntity {
        motion: None,
        entity,
        kind: EntityKind::Npc,
        position,
        display_name: None,
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 50,
        },
    };
    view = view.with_entities(vec![actor])?;
    view.set_presentations(vec![VisibleAppearance {
        entity: actor,
        look: None,
        light: None,
        forge_stacks: None,
    }])?;
    outbox.push(vec![SessionEvent::Presentation(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: vec![PresentationEvent {
            decision_ordinal: 1,
            emission_ordinal: 1,
            kind: PresentationEventKind::ValueText {
                kind: PresentationValueKind::Healed,
                position: ActorPosition {
                    floor: 7,
                    ..position
                },
                target: entity,
                attacker: None,
                primary: PresentationValue {
                    value: 8,
                    color: PresentationColor::Orange,
                },
                secondary: None,
            },
        }],
    })]);
    assert!(view.tick(&link).is_ok());
    let scene = view.scene();
    let (width, height) = scene.view().viewport_px();
    let [x, y] = scene.view().tile_to_screen(TileCoord::new(100, 100));
    let side = scene.view().tile_px() as f32;
    // Filled and empty gauge regions, active/inactive Harmony, actor health,
    // NPC badge and the first lit pixel in the original 3x5 value glyph.
    let samples = vec![
        ([12., 60.], [255, 0, 0]),
        ([100., 60.], [204; 3]),
        ([12., 74.], [0, 0, 255]),
        ([100., 74.], [204; 3]),
        ([12., 89.], [255, 255, 0]),
        ([108., 89.], [204; 3]),
        ([12., 103.], [102, 255, 102]),
        // The merged creature HUD places its 50% health bar above the actor;
        // the native palette quantizes the yellow threshold to three 51-step channels.
        ([x + 4., (y - 4.).max(10.) + 1.], [153, 153, 0]),
        ([x + side - 7., y + 1.], [0, 204, 255]),
        ([x + side / 2. - 2., y + 1.], [255, 153, 0]),
    ];
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(super::super::owner::ActivePlay { view, link });
    let mut app = App::new();
    app.insert_non_send(owner)
        .init_resource::<SceneCache>()
        .init_resource::<Time>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<ColorMaterial>>()
        .insert_resource(WorldViewport {
            origin: UVec2::ZERO,
            size: UVec2::new(width, height),
        })
        .add_systems(Update, project);
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.update();
    if std::env::var_os("OTERYN_BEVY_SCENE_CAPTURE").is_some() {
        let fixture = crate::play::screenshot::TestDir::new()?;
        let path = fixture.join("scene.png");
        let sprites = app
            .world_mut()
            .query_filtered::<(&Sprite, &Transform, &RenderLayers), With<WorldQuad>>()
            .iter(app.world())
            .map(|(s, t, l)| (s.clone(), *t, l.clone()))
            .collect();
        let handle = app
            .world()
            .resource::<SceneCache>()
            .image
            .as_ref()
            .ok_or("atlas handle")?;
        let atlas = app
            .world()
            .resource::<Assets<Image>>()
            .get(handle)
            .ok_or("atlas image")?
            .clone();
        native_palette_capture(
            UVec2::new(width, height),
            atlas,
            sprites,
            samples.clone(),
            path.clone(),
        )?;
        fixture.keep();
        println!("Scene capture saved to {}", path.display());
    }
    for (point, rgb) in &samples {
        let sprite = app
            .world_mut()
            .query_filtered::<(&Sprite, &Transform), With<WorldQuad>>()
            .iter(app.world())
            .filter(|(s, t)| {
                bounds(
                    s,
                    t,
                    WorldViewport {
                        origin: UVec2::ZERO,
                        size: UVec2::new(width, height),
                    },
                    1.,
                )
                .contains(Vec2::from(*point))
            })
            .max_by(|(_, a), (_, b)| a.translation.z.total_cmp(&b.translation.z))
            .map(|(s, _)| s)
            .ok_or("sample sprite")?;
        assert_eq!(sprite.rect, None, "palette sample at {point:?}");
        assert_eq!(
            sprite.color.to_srgba().to_u8_array(),
            [rgb[0], rgb[1], rgb[2], 255]
        );
    }
    Ok(())
}

fn native_palette_capture(
    size: UVec2,
    atlas: Image,
    sprites: Vec<(Sprite, Transform, RenderLayers)>,
    samples: Vec<([f32; 2], [u8; 3])>,
    path: std::path::PathBuf,
) -> TestResult {
    use bevy::{
        camera::RenderTarget,
        render::view::screenshot::{Screenshot, ScreenshotCaptured},
    };
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    };
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(bevy::render::RenderPlugin {
                synchronous_pipeline_compilation: true,
                ..default()
            })
            .set(WindowPlugin {
                primary_window: None,
                ..default()
            })
            .disable::<bevy::winit::WinitPlugin>(),
    );
    let ready = std::time::Instant::now();
    while app.plugins_state() == bevy::app::PluginsState::Adding {
        if ready.elapsed() > std::time::Duration::from_secs(45) {
            return Err("native renderer initialization timed out".into());
        }
        bevy::tasks::tick_global_task_pools_on_main_thread();
    }
    app.finish();
    app.cleanup();
    let mut image = Image::new_target_texture(size.x, size.y, TextureFormat::Rgba8UnormSrgb, None);
    image.texture_descriptor.usage |= bevy::render::render_resource::TextureUsages::COPY_SRC;
    let target = app.world_mut().resource_mut::<Assets<Image>>().add(image);
    app.world_mut().spawn((
        Camera2d,
        RenderTarget::Image(target.clone().into()),
        bevy::render::view::Tonemapping::None,
        RenderLayers::layer(1),
    ));
    let atlas = app.world_mut().resource_mut::<Assets<Image>>().add(atlas);
    for (mut sprite, transform, layer) in sprites {
        if sprite.image != Handle::default() {
            sprite.image = atlas.clone();
        }
        app.world_mut().spawn((sprite, transform, layer));
    }
    let result = Arc::new(Mutex::new(None));
    let pending = Arc::new(AtomicBool::new(false));
    let started = std::time::Instant::now();
    while started.elapsed() < std::time::Duration::from_secs(45) {
        if !pending.swap(true, Ordering::Relaxed) {
            let pending = Arc::clone(&pending);
            let output = Arc::clone(&result);
            let samples = samples.clone();
            let path = path.clone();
            app.world_mut()
                .spawn(Screenshot::image(target.clone()))
                .observe(move |event: On<ScreenshotCaptured>| {
                    pending.store(false, Ordering::Relaxed);
                    let Ok(image) = event.image.clone().try_into_dynamic() else {
                        return;
                    };
                    let rgba = image.to_rgba8();
                    if samples.iter().all(|(point, rgb)| {
                        rgba.get_pixel_checked(point[0] as u32, point[1] as u32)
                            .is_some_and(|p| {
                                p.0[..3].iter().zip(rgb).all(|(a, b)| a.abs_diff(*b) <= 1)
                            })
                    }) && let Ok(mut result) = output.lock()
                    {
                        *result = Some(image.save(&path).map_err(|e| e.to_string()));
                    }
                });
        }
        app.update();
        // Headless updates have no window frame pacing. Complete each GPU
        // submission before queuing another frame, including on a mismatch.
        app.world()
            .resource::<bevy::render::renderer::RenderDevice>()
            .poll(bevy::render::render_resource::PollType::Wait {
                submission_index: None,
                timeout: Some(std::time::Duration::from_secs(10)),
            })
            .map_err(|error| format!("native GPU readback: {error}"))?;
        if let Some(result) = result.lock().map_err(|_| "capture lock")?.take() {
            drop(app);
            bevy::tasks::tick_global_task_pools_on_main_thread();
            return result.map_err(Into::into);
        }
    }
    drop(app);
    bevy::tasks::tick_global_task_pools_on_main_thread();
    Err("native palette pixel readback timed out".into())
}

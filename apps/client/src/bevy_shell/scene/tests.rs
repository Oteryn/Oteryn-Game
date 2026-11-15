use super::*;
use oteryn_renderer::{SpriteBatch, TileCoord, TileView};

type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(super) fn bounds(
    sprite: &Sprite,
    transform: &Transform,
    map: WorldViewport,
    scale: f32,
) -> Rect {
    let center = map.origin.as_vec2()
        + map.size.as_vec2() * 0.5
        + Vec2::new(transform.translation.x, -transform.translation.y) * scale;
    Rect::from_center_size(center, sprite.custom_size.unwrap_or(Vec2::ZERO) * scale)
}

#[test]
fn displaced_first_tile_does_not_draw_in_noninteractive_letterbox() -> TestResult {
    let view = TileView::new(TileCoord::new(0, 0), 32, 2, 2)?;
    let atlas = AtlasImage::new(32, 1, 1, vec![255; 32 * 32 * 4])?;
    let mut batch = SpriteBatch::new();
    assert!(batch.push_offset(&view, &atlas, TileCoord::new(0, 0), 0, [-8, 0], 32)?);
    let quad = batch.instances().first().ok_or("quad")?;
    assert_eq!(quad.position, [-8., 0.]);
    let map = WorldViewport {
        origin: UVec2::new(100, 50),
        size: UVec2::new(128, 128),
    };
    let size = UVec2::new(64, 64);
    let offset = map.scene_offset(size);
    let rejected = map.origin.as_vec2() + offset + Vec2::new(-4., 16.);
    assert!(map.scene_point(rejected, size).is_none());
    let projected: Option<(Sprite, Transform)> = project_quad(
        quad,
        [1.; 4],
        7,
        &Handle::default(),
        &atlas,
        QuadViewport {
            scale: 1.,
            size: map.size.as_vec2(),
            offset,
            clip_size: size.as_vec2(),
            scroll: Vec2::ZERO,
        },
    );
    let (sprite, transform) = projected.ok_or("partially visible quad")?;
    assert!(
        !bounds(&sprite, &transform, map, 1.).contains(rejected),
        "a point rejected by input must not contain displaced artwork"
    );
    let accepted = map.origin.as_vec2() + offset + Vec2::new(4., 16.);
    assert!(map.scene_point(accepted, size).is_some());
    assert!(bounds(&sprite, &transform, map, 1.).contains(accepted));
    assert_eq!(sprite.custom_size, Some(Vec2::new(24., 32.)));
    assert_eq!(sprite.rect, Some(Rect::new(8., 0., 32., 32.)));
    assert_eq!(transform.translation.z, 7. * 0.001);
    Ok(())
}

#[test]
fn real_edge_labels_reuse_dense_projected_entities() -> TestResult {
    use crate::play::{PlayView, play_channel};
    use oteryn_session::{
        ActorPosition, EntityDetail, EntityDisplayName, EntityKind, EntityRef, StepDirection,
        VisibleAppearance, WorldSpatialEntity,
    };
    let mut view = PlayView::new(TileCoord::new(100, 100), 7, &[])?;
    let origin = view.scene().view().origin();
    let right = origin.x + view.scene().view().columns() as i32 - 1;
    let mut appearances = Vec::new();
    for (identity, x) in [(1, right), (2, origin.x)] {
        appearances.push(VisibleAppearance {
            entity: WorldSpatialEntity {
                entity: EntityRef {
                    identity: [identity; 16],
                    generation: 1,
                },
                kind: EntityKind::Creature,
                position: ActorPosition {
                    x,
                    y: origin.y + 1,
                    floor: 7,
                },
                display_name: Some(
                    EntityDisplayName::new("Long boundary creature label")
                        .map_err(|e| format!("{e:?}"))?,
                ),
                detail: EntityDetail::Actor {
                    direction: StepDirection::South,
                    appearance_ref: 1,
                    health_percent: 100,
                },
            },
            look: None,
            light: None,
            forge_stacks: None,
        });
    }
    view.set_presentations(appearances)?;
    let scene = view.scene();
    let (width, height) = scene.view().viewport_px();
    let range = scene.minimap_quad_range();
    let mut expected = Vec::new();
    for (order, (quad, minimap)) in scene
        .tiles()
        .instances()
        .iter()
        .map(|q| (q, false))
        .chain(
            scene
                .sprites()
                .instances()
                .iter()
                .enumerate()
                .map(|(i, q)| (q, range.contains(&i))),
        )
        .enumerate()
    {
        let clip = if minimap {
            super::super::game_ui::minimap::SIZE.as_vec2()
        } else {
            Vec2::new(width as f32, height as f32)
        };
        if quad.position[0] < clip.x
            && quad.position[1] < clip.y
            && quad.position[0] + quad.size[0] > 0.
            && quad.position[1] + quad.size[1] > 0.
        {
            expected.push(order as f32 * 0.001);
        }
    }
    let total = scene.tiles().instances().len() + scene.sprites().instances().len();
    assert!(
        expected.len() < total,
        "real edge label must contain fully clipped glyphs"
    );
    let mut host = HostOwner::new(None)?;
    let (link, _, _) = play_channel();
    host.play = Some(super::super::owner::ActivePlay { view, link });
    let mut app = App::new();
    app.insert_non_send(host)
        .init_resource::<SceneCache>()
        .init_resource::<Time>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<ColorMaterial>>()
        .insert_resource(WorldViewport {
            origin: UVec2::ZERO,
            size: UVec2::new(width + 64, height + 64),
        })
        .add_systems(Update, project);
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.update();
    let first = app.world().resource::<SceneCache>().entities.clone();
    assert_eq!(first.len(), expected.len());
    app.update();
    assert_eq!(
        app.world().resource::<SceneCache>().entities,
        first,
        "skipped quads must not create cache holes on refresh"
    );
    for (entity, z) in first.iter().zip(expected) {
        assert_eq!(
            app.world()
                .get::<Transform>(*entity)
                .ok_or("quad transform")?
                .translation
                .z,
            z
        );
    }
    let count = app
        .world_mut()
        .query_filtered::<Entity, With<WorldQuad>>()
        .iter(app.world())
        .count();
    assert_eq!(count, first.len(), "no orphan sprites after skipping quads");
    Ok(())
}

#[test]
fn clipping_crops_both_uv_axes_without_changing_tint_order_or_dpi() -> TestResult {
    let atlas = AtlasImage::new(32, 2, 2, vec![255; 64 * 64 * 4])?;
    let tint = [0.25, 0.5, 0.75, 0.5];
    let quad = QuadInstance {
        position: [-8., -4.],
        size: [32., 16.],
        uv: [0.25, 0.5, 0.75, 1.],
    };
    let viewport = QuadViewport {
        scale: 2.,
        size: Vec2::splat(128.),
        offset: Vec2::splat(32.),
        clip_size: Vec2::new(16., 8.),
        scroll: Vec2::ZERO,
    };
    let (sprite, transform) = project_quad(&quad, tint, 13, &Handle::default(), &atlas, viewport)
        .ok_or("visible intersection")?;
    assert_eq!(sprite.rect, Some(Rect::new(24., 40., 40., 56.)));
    assert_eq!(sprite.custom_size, Some(Vec2::new(8., 4.)));
    assert_eq!(
        sprite.color,
        Color::linear_rgba(tint[0], tint[1], tint[2], tint[3])
    );
    assert_eq!(transform.translation, Vec3::new(-12., 14., 13. * 0.001));
    Ok(())
}

#[test]
fn minimap_labels_clip_to_their_own_target_and_discard_invisible_glyphs() -> TestResult {
    let atlas = AtlasImage::new(32, 1, 1, vec![255; 32 * 32 * 4])?;
    let size = super::super::game_ui::minimap::SIZE.as_vec2();
    let viewport = QuadViewport {
        scale: 1.,
        size,
        offset: Vec2::ZERO,
        clip_size: size,
        scroll: Vec2::ZERO,
    };
    let mut labels = SpriteBatch::new();
    oteryn_renderer::name_text::label_quad_scaled(
        &mut labels,
        &atlas,
        0,
        [size.x - 4., size.y - 6.],
        100,
    )?;
    oteryn_renderer::name_text::label_quad_scaled(
        &mut labels,
        &atlas,
        0,
        [size.x, size.y - 6.],
        100,
    )?;
    let mut quads = labels.instances().iter();
    let (sprite, _) = project_quad(
        quads.next().ok_or("partial glyph")?,
        [1.; 4],
        0,
        &Handle::default(),
        &atlas,
        viewport,
    )
    .ok_or("partial glyph projected")?;
    assert_eq!(sprite.custom_size, Some(Vec2::new(4., 6.)));
    assert_eq!(sprite.rect, Some(Rect::new(0., 0., 16., 16.)));
    assert!(
        project_quad(
            quads.next().ok_or("outside glyph")?,
            [1.; 4],
            1,
            &Handle::default(),
            &atlas,
            viewport
        )
        .is_none()
    );
    Ok(())
}

#[test]
fn a_camera_scroll_moves_a_projected_quad_before_clipping() -> TestResult {
    let atlas = AtlasImage::new(32, 1, 1, vec![255; 32 * 32 * 4])?;
    let quad = QuadInstance {
        position: [16., 16.],
        size: [32., 32.],
        uv: [0., 0., 1., 1.],
    };
    let at = |scroll| {
        project_quad(
            &quad,
            [1.; 4],
            0,
            &Handle::default(),
            &atlas,
            QuadViewport {
                scale: 1.,
                size: Vec2::splat(96.),
                offset: Vec2::ZERO,
                clip_size: Vec2::splat(96.),
                scroll,
            },
        )
    };
    let (_, rest) = at(Vec2::ZERO).ok_or("visible")?;
    let (_, scrolled) = at(Vec2::new(-12., 6.)).ok_or("visible")?;
    assert_eq!(
        scrolled.translation - rest.translation,
        Vec3::new(-12., -6., 0.)
    );
    // Scrolled past the left edge, the quad is cropped rather than drawn outside the view.
    let (cropped, _) = at(Vec2::new(-24., 0.)).ok_or("partly visible")?;
    assert_eq!(cropped.custom_size, Some(Vec2::new(24., 32.)));
    Ok(())
}

#[test]
fn floor_fade_reaches_bevy_sprite_alpha() -> TestResult {
    use crate::play::{PlayView, play_channel};
    use oteryn_session::ActorPosition;
    use oteryn_session::world_map::{
        MapDefinition, MapItem, MapOrigin, MapTile, MapViewHeader, WorldMapViewSnapshot,
    };
    let origin = ActorPosition {
        x: 100,
        y: 200,
        floor: -7,
    };
    let tile = |position| MapTile {
        position,
        items: vec![MapItem {
            definition: MapDefinition::Item(std::num::NonZeroU32::MIN),
            count: 1,
            sub_type: 0,
            origin: MapOrigin::DisplayOnly,
            appearance_id: 0,
        }],
        more: false,
        ground_speed: 0,
    };
    let mut map = WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: [0; 32],
            bundle_digest: [0; 32],
            reset_epoch: 1,
            origin,
            first_visible_floor: 0,
        },
        tiles: vec![
            tile(origin),
            tile(ActorPosition {
                x: 101,
                y: 201,
                floor: -6,
            }),
        ],
    };
    let mut view = PlayView::new(TileCoord::new(100, 200), -7, &[])?;
    view.set_live_map(&map)?;
    let mut host = HostOwner::new(None)?;
    let (link, _, _) = play_channel();
    host.play = Some(super::super::owner::ActivePlay { view, link });
    let mut app = App::new();
    app.insert_non_send(host)
        .init_resource::<SceneCache>()
        .init_resource::<Time>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<ColorMaterial>>()
        .insert_resource(WorldViewport {
            origin: UVec2::ZERO,
            size: UVec2::new(640, 480),
        })
        .add_systems(Update, project);
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.update();
    assert_eq!(
        app.world().resource::<SceneCache>().floor_alpha.get(&-6),
        Some(&1.0)
    );
    map.header.first_visible_floor = -7;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .view
        .set_live_map(&map)?;
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(250));
    app.update();
    assert_eq!(
        app.world().resource::<SceneCache>().floor_alpha.get(&-6),
        Some(&0.5)
    );
    let mut query = app.world_mut().query::<&Sprite>();
    assert!(
        query
            .iter(app.world())
            .any(|sprite| sprite.color.alpha() == 0.5)
    );
    app.update();
    assert_eq!(
        app.world().resource::<SceneCache>().floor_alpha.get(&-6),
        Some(&0.0)
    );
    map.header.first_visible_floor = 0;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .view
        .set_live_map(&map)?;
    app.update();
    assert_eq!(
        app.world().resource::<SceneCache>().floor_alpha.get(&-6),
        Some(&0.5)
    );
    Ok(())
}

#[test]
fn walking_projection_keeps_touch_controls_fixed_and_moves_world_quads() -> TestResult {
    use crate::play::{PlayView, play_channel};
    use oteryn_session::{
        ActorPosition, AppliedDelta, CommandStatus, StepDisposition, StepOutcome,
        WorldSpatialObservation,
    };
    let mut view = PlayView::new(TileCoord::new(100, 100), 7, &[])?;
    view.enable_touch_controls();
    view.apply(&StepOutcome {
        step_wait_ms: 0,
        command_id: 1,
        status: CommandStatus::Accepted,
        disposition: StepDisposition::Moved,
        result_server_sequence: 1,
        world_spatial_delta: Some(AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: WorldSpatialObservation {
                content_generation: [0; 32],
                actor_position: ActorPosition {
                    x: 101,
                    y: 100,
                    floor: 7,
                },
            },
        }),
        world_object_overlay_delta: None,
        gate_message_id: None,
    })?;
    assert!(view.camera_offset(std::time::Instant::now()).is_some());
    let scene = view.scene();
    let count = scene.tiles().instances().len();
    let (w, h) = scene.view().viewport_px();
    // Touch controls are appended after world sprites, close to the bottom left.
    let (i, quad) = scene
        .sprites()
        .instances()
        .iter()
        .enumerate()
        .rev()
        .find(|(_, q)| q.position[0] < 200. && q.position[1] > h as f32 / 2.)
        .ok_or("touch quad")?;
    let expected = project_quad(
        quad,
        [1.; 4],
        count + i,
        &Handle::default(),
        scene.atlas(),
        QuadViewport {
            scale: 1.,
            size: Vec2::new(w as f32, h as f32),
            offset: Vec2::ZERO,
            clip_size: Vec2::new(w as f32, h as f32),
            scroll: Vec2::ZERO,
        },
    )
    .ok_or("fixed control")?
    .1;
    let (link, _, _) = play_channel();
    let mut host = HostOwner::new(None)?;
    host.play = Some(super::super::owner::ActivePlay { view, link });
    let mut app = App::new();
    app.insert_non_send(host)
        .init_resource::<SceneCache>()
        .init_resource::<Time>()
        .init_resource::<Assets<Image>>()
        .init_resource::<Assets<Mesh>>()
        .init_resource::<Assets<ColorMaterial>>()
        .insert_resource(WorldViewport {
            origin: UVec2::ZERO,
            size: UVec2::new(w, h),
        })
        .add_systems(Update, project);
    app.world_mut().spawn((Window::default(), PrimaryWindow));
    app.update();
    let entity = app
        .world()
        .resource::<SceneCache>()
        .entities
        .iter()
        .find(|e| {
            app.world()
                .get::<Transform>(**e)
                .is_some_and(|t| t.translation.z == expected.translation.z)
        })
        .ok_or("projected control")?;
    assert_eq!(app.world().get::<Transform>(*entity), Some(&expected));
    let terrain = app
        .world()
        .resource::<SceneCache>()
        .entities
        .iter()
        .filter_map(|e| app.world().get::<Transform>(*e))
        .find(|t| t.translation.z == 0.)
        .ok_or("terrain")?;
    assert!(
        terrain.translation.x > -(w as f32) / 2. + 24.,
        "terrain scrolls during walk"
    );
    Ok(())
}

#[test]
fn resource_gauges_do_not_duplicate_sidebar_hud_in_bevy_scene() -> TestResult {
    use crate::play::{PlayView, play_channel};
    use oteryn_session::ActorVitals;

    let (link, _, outbox) = play_channel();
    link.use_shared_frontend();
    let mut view = PlayView::new(TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    let mut preferences = view.preferences().clone();
    preferences.resource_hud.combo = false;
    preferences.health_circle = false;
    view.set_preferences(preferences.clone())?;
    let (width, height) = view.scene().view().viewport_px();
    let mut host = HostOwner::new(None)?;
    host.play = Some(super::super::owner::ActivePlay { view, link });
    let mut app = App::new();
    app.insert_non_send(host)
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
    let baseline = app.world().resource::<SceneCache>().entities.len();
    for (health, harmony, serene) in [(50, 3, true), (100, 0, false)] {
        outbox.push(vec![oteryn_session::SessionEvent::ActorVitals(
            oteryn_session::AppliedDelta {
                server_sequence: u64::from(health),
                base_revision: 0,
                new_revision: 1,
                value: ActorVitals {
                    health,
                    max_health: 100,
                    mana: 20,
                    max_mana: 40,
                    harmony,
                    serene,
                    ..default()
                },
            },
        )]);
        {
            let mut host = app.world_mut().non_send_mut::<HostOwner>();
            let play = host.play.as_mut().ok_or("play")?;
            play.view.tick(&play.link).map_err(|_| "session ended")?;
            assert_eq!(play.view.actor_vitals().map(|v| v.health), Some(health));
        }
        app.update();
        assert_eq!(
            app.world().resource::<SceneCache>().entities.len(),
            baseline
        );
    }
    // Removing corner gauges must preserve the actor-following combo gauge.
    preferences.resource_hud.combo = true;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .view
        .set_preferences(preferences.clone())?;
    app.update();
    assert_eq!(
        app.world().resource::<SceneCache>().entities.len(),
        baseline + 5
    );
    // Native circle quads are still replaced by Bevy's configurable circle renderer.
    preferences.health_circle = true;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .view
        .set_preferences(preferences)?;
    app.update();
    assert_eq!(
        app.world().resource::<SceneCache>().entities.len(),
        baseline + 5
    );
    Ok(())
}

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

/// Real retained producers -> Bevy projection. Set OTERYN_BEVY_SCENE_CAPTURE to a
/// PNG path to additionally qualify these same sprites through native GPU readback.
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
    // Native gauges remain in the packed batch, while Bevy uses its sidebar HUD.
    let native_samples = [
        ([12., 60.], [255, 0, 0]),
        ([100., 60.], [204; 3]),
        ([12., 74.], [0, 0, 255]),
        ([100., 74.], [204; 3]),
        ([12., 89.], [255, 255, 0]),
        ([108., 89.], [204; 3]),
        ([12., 103.], [102, 255, 102]),
    ];
    for (point, rgb) in native_samples {
        let (index, quad) = scene
            .sprites()
            .instances()
            .iter()
            .enumerate()
            .rfind(|(_, quad)| {
                Rect::from_corners(
                    Vec2::from(quad.position),
                    Vec2::from(quad.position) + Vec2::from(quad.size),
                )
                .contains(Vec2::from(point))
            })
            .ok_or("native gauge quad")?;
        assert!(scene.native_hud_quads.contains(&index));
        let atlas = scene.atlas();
        let column = (quad.uv[0] * atlas.width() as f32) as usize;
        let row = (quad.uv[1] * atlas.height() as f32) as usize;
        let offset = (row * atlas.width() as usize + column) * 4;
        assert_eq!(
            &atlas.rgba()[offset..offset + 4],
            &[rgb[0], rgb[1], rgb[2], 255]
        );
    }
    // Actor health, NPC badge and the first lit pixel of the value glyph remain in Bevy.
    let samples = vec![
        ([x + 4., y - 3.], [153, 153, 0]),
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
    if let Some(path) = std::env::var_os("OTERYN_BEVY_SCENE_CAPTURE") {
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
            path.into(),
        )?;
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

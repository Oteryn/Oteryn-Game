use super::*;
use oteryn_renderer::{SpriteBatch, TileCoord, TileView};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn bounds(sprite: &Sprite, transform: &Transform, map: WorldViewport, scale: f32) -> Rect {
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
        .init_resource::<Assets<Image>>()
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

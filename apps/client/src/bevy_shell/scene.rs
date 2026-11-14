//! A direct projection of the existing Scene's ordered quads; no offline world.
use super::{game_ui::viewport::WorldViewport, owner::HostOwner};
use bevy::camera::visibility::RenderLayers;
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    window::PrimaryWindow,
};
use oteryn_renderer::{AtlasImage, QuadInstance, lighting::QuadLayer};

#[cfg(test)]
mod tests;

#[derive(Resource, Default)]
pub(super) struct SceneCache {
    atlas: Option<AtlasImage>,
    image: Option<Handle<Image>>,
    entities: Vec<Entity>,
}
#[derive(Component)]
pub(super) struct WorldQuad;

pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut cache: ResMut<SceneCache>,
    viewport: Res<WorldViewport>,
    mut images: ResMut<Assets<Image>>,
    mut quads: Query<(&mut Sprite, &mut Transform, &mut RenderLayers), With<WorldQuad>>,
) {
    let Some(play) = &owner.play else {
        for entity in cache.entities.drain(..) {
            commands.entity(entity).despawn();
        }
        return;
    };
    let Ok(window) = window.single() else {
        return;
    };
    let scene = play.view.scene();
    if cache.atlas.as_ref() != Some(scene.atlas()) {
        let atlas = scene.atlas();
        let image = Image::new(
            Extent3d {
                width: atlas.width(),
                height: atlas.height(),
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            atlas.rgba().to_vec(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        if let Some(handle) = &cache.image {
            if let Some(mut existing) = images.get_mut(handle) {
                *existing = image;
            }
        } else {
            cache.image = Some(images.add(image));
        }
        cache.atlas = Some(atlas.clone());
    }
    let Some(image) = cache.image.clone() else {
        return;
    };
    let scene_size = scene.view().viewport_px();
    let size = if viewport.size.x > 0 {
        viewport.size.as_vec2()
    } else {
        Vec2::new(
            window.physical_width() as f32,
            window.physical_height() as f32,
        )
    };
    let offset = ((size - Vec2::new(scene_size.0 as f32, scene_size.1 as f32)) * 0.5).floor();
    let minimap_range = scene.minimap_quad_range();
    let lighting = &scene.sprites().lighting;
    let ordered = scene
        .tiles()
        .instances()
        .iter()
        .enumerate()
        .map(|(i, q)| (q, lighting.quad_tint(q, i, QuadLayer::Terrain), false))
        .chain(
            scene
                .sprites()
                .instances()
                .iter()
                .enumerate()
                .map(|(i, q)| {
                    (
                        q,
                        lighting.quad_tint(q, i, QuadLayer::Overlay),
                        minimap_range.contains(&i),
                    )
                }),
        );
    let mut count = 0;
    for (index, (quad, tint, minimap)) in ordered.enumerate() {
        let layer = RenderLayers::layer(if minimap { 2 } else { 1 });
        let (scale, size, offset, clip_size) = if minimap {
            let size = super::game_ui::minimap::SIZE.as_vec2();
            (1., size, Vec2::ZERO, size)
        } else {
            (
                window.scale_factor(),
                size,
                offset,
                Vec2::new(scene_size.0 as f32, scene_size.1 as f32),
            )
        };
        let Some((sprite, transform)) = project_quad(
            quad,
            tint,
            index,
            &image,
            scene.atlas(),
            QuadViewport {
                scale,
                size,
                offset,
                clip_size,
            },
        ) else {
            continue;
        };
        if let Some(entity) = cache.entities.get(count) {
            if let Ok((mut old_sprite, mut old_transform, mut old_layer)) = quads.get_mut(*entity) {
                if *old_layer != layer {
                    *old_layer = layer.clone();
                }
                if *old_sprite != sprite {
                    *old_sprite = sprite;
                }
                if *old_transform != transform {
                    *old_transform = transform;
                }
            }
        } else {
            cache
                .entities
                .push(commands.spawn((sprite, transform, WorldQuad, layer)).id());
        }
        count += 1;
    }
    for entity in cache.entities.drain(count..) {
        commands.entity(entity).despawn();
    }
}
#[derive(Clone, Copy)]
struct QuadViewport {
    scale: f32,
    size: Vec2,
    offset: Vec2,
    clip_size: Vec2,
}
fn project_quad(
    quad: &QuadInstance,
    tint: [f32; 4],
    order: usize,
    image: &Handle<Image>,
    atlas: &AtlasImage,
    viewport: QuadViewport,
) -> Option<(Sprite, Transform)> {
    let QuadViewport {
        scale,
        size: viewport_size,
        offset,
        clip_size,
    } = viewport;
    let [x, y] = quad.position;
    let [w, h] = quad.size;
    let [u0, v0, u1, v1] = quad.uv;
    let start = Vec2::new(x, y).max(Vec2::ZERO);
    let end = Vec2::new(x + w, y + h).min(clip_size);
    let visible = end - start;
    if !visible.cmpgt(Vec2::ZERO).all() {
        return None;
    }
    // Crop atlas sampling with the geometry so edge artwork is not stretched.
    let uv_start = Vec2::new(u0, v0)
        + Vec2::new(u1 - u0, v1 - v0) * (start - Vec2::new(x, y)) / Vec2::new(w, h);
    let uv_end =
        Vec2::new(u0, v0) + Vec2::new(u1 - u0, v1 - v0) * (end - Vec2::new(x, y)) / Vec2::new(w, h);
    let (x, y, w, h) = (start.x, start.y, visible.x, visible.y);
    let sprite = Sprite {
        image: image.clone(),
        color: Color::linear_rgba(tint[0], tint[1], tint[2], tint[3]),
        rect: Some(Rect::new(
            uv_start.x * atlas.width() as f32,
            uv_start.y * atlas.height() as f32,
            uv_end.x * atlas.width() as f32,
            uv_end.y * atlas.height() as f32,
        )),
        custom_size: Some(Vec2::new(w / scale, h / scale)),
        ..default()
    };
    let transform = Transform::from_xyz(
        (x + w / 2. + offset.x - viewport_size.x / 2.) / scale,
        (viewport_size.y / 2. - (y + h / 2. + offset.y)) / scale,
        order as f32 * 0.001,
    );
    Some((sprite, transform))
}

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
mod palette_tests;
mod smooth;
#[cfg(test)]
mod tests;
pub(super) mod typing;
#[cfg(test)]
mod typing_tests;

#[derive(Resource, Default)]
pub(super) struct SceneCache {
    smooth: smooth::Cache,
    atlas: Option<AtlasImage>,
    image: Option<Handle<Image>>,
    entities: Vec<Entity>,
    floor_alpha: std::collections::BTreeMap<i16, f32>,
}
impl SceneCache {
    pub(crate) fn atlas_image(&self) -> Option<&Handle<Image>> {
        self.image.as_ref()
    }
}
#[derive(Component)]
pub(super) struct WorldQuad;

pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    window: Query<&Window, With<PrimaryWindow>>,
    mut cache: ResMut<SceneCache>,
    (viewport, time): (Res<WorldViewport>, Res<Time>),
    (mut images, mut mesh_assets): (ResMut<Assets<Image>>, smooth::MeshAssets),
    mut quads: Query<(&mut Sprite, &mut Transform, &mut RenderLayers), With<WorldQuad>>,
) {
    cache.smooth.hide(&mut commands);
    let Some(play) = &owner.play else {
        for entity in cache.entities.drain(..) {
            commands.entity(entity).despawn();
        }
        cache.floor_alpha.clear();
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
    let tile_count = scene.tiles().instances().len();
    let camera = play
        .view
        .camera_offset(std::time::Instant::now())
        .map_or(Vec2::ZERO, Vec2::from);
    let player_quads = scene.player_quads();
    cache.floor_alpha.retain(|floor, _| {
        scene
            .floor_quad_ranges()
            .iter()
            .any(|(_, current)| current == floor)
    });
    for (_, floor) in scene.floor_quad_ranges() {
        let visible = *floor <= scene.first_visible_floor();
        let alpha = cache
            .floor_alpha
            .entry(*floor)
            .or_insert(if visible { 1.0 } else { 0.0 });
        *alpha = crate::scene::floor_lighting::fade_alpha(*alpha, visible, time.delta_secs());
    }
    let lighting = &scene.sprites().lighting;
    let hud_quads = &scene.native_hud_quads;
    let shared_frontend = play.link.uses_shared_frontend();
    let SceneCache {
        floor_alpha,
        entities,
        smooth,
        ..
    } = &mut *cache;
    let ordered = scene
        .tiles()
        .instances()
        .iter()
        .enumerate()
        .map(|(i, q)| {
            (
                i,
                q,
                lighting.quad_tint(q, i, QuadLayer::Terrain),
                false,
                i,
                QuadLayer::Terrain,
            )
        })
        .chain(
            scene
                .sprites()
                .instances()
                .iter()
                .enumerate()
                .filter(|(i, _)| !shared_frontend || !hud_quads.contains(i))
                .map(|(i, q)| {
                    let mut tint = lighting.quad_tint(q, i, QuadLayer::Overlay);
                    if let Some((_, floor)) = scene
                        .floor_quad_ranges()
                        .iter()
                        .find(|(range, _)| range.contains(&i))
                    {
                        tint[3] *= floor_alpha.get(floor).copied().unwrap_or(1.0);
                    }
                    (
                        tile_count + i,
                        q,
                        tint,
                        minimap_range.contains(&i),
                        i,
                        QuadLayer::Overlay,
                    )
                }),
        );
    let smooth_enabled = !lighting.sources.is_empty();
    let mut batches = [smooth::Vertices::default(), smooth::Vertices::default()];
    let mut count = 0;
    for (index, quad, tint, minimap, source_index, source_layer) in ordered {
        let layer = RenderLayers::layer(if minimap { 2 } else { 1 });
        let follows_player = index >= tile_count
            && player_quads
                .iter()
                .any(|range| range.contains(&(index - tile_count)));
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
        let viewport = QuadViewport {
            scale,
            size,
            offset,
            clip_size,
            // The camera follows the player's step: the map slides, the player stays.
            scroll: if minimap || follows_player {
                Vec2::ZERO
            } else {
                camera
            },
        };
        let Some((sprite, transform)) =
            project_quad(quad, tint, index, &image, scene.atlas(), viewport)
        else {
            continue;
        };
        if smooth_enabled {
            batches[usize::from(minimap)].push(
                lighting,
                quad,
                source_index,
                source_layer,
                (&sprite, transform, viewport),
            );
            continue;
        }
        if let Some(entity) = entities.get(count) {
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
            entities.push(commands.spawn((sprite, transform, WorldQuad, layer)).id());
        }
        count += 1;
    }
    if smooth_enabled {
        smooth.draw(&mut commands, &mut mesh_assets, &image, batches);
    }
    for entity in entities.drain(count..) {
        commands.entity(entity).despawn();
    }
}
#[derive(Clone, Copy)]
struct QuadViewport {
    scale: f32,
    size: Vec2,
    offset: Vec2,
    clip_size: Vec2,
    scroll: Vec2,
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
        scroll,
    } = viewport;
    let [x, y] = (Vec2::from(quad.position) + scroll).into();
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
    let mut sprite = Sprite {
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
    // Palette primitives sample a single texel, independently of their geometry.
    // Bevy source rectangles need area; use a solid sprite to preserve constant
    // sampling without stretching neighbouring palette colours into the quad.
    if u0 == u1 && v0 == v1 {
        let x = (u0 * atlas.width() as f32)
            .floor()
            .clamp(0., atlas.width() as f32 - 1.) as usize;
        let y = (v0 * atlas.height() as f32)
            .floor()
            .clamp(0., atlas.height() as f32 - 1.) as usize;
        let at = (y * atlas.width() as usize + x) * 4;
        let rgba = atlas.rgba().get(at..at + 4)?;
        let color = Color::srgba_u8(rgba[0], rgba[1], rgba[2], rgba[3]).to_linear();
        sprite = Sprite::from_color(
            Color::linear_rgba(
                color.red * tint[0],
                color.green * tint[1],
                color.blue * tint[2],
                color.alpha * tint[3],
            ),
            Vec2::new(w / scale, h / scale),
        );
    }
    let transform = Transform::from_xyz(
        (x + w / 2. + offset.x - viewport_size.x / 2.) / scale,
        (viewport_size.y / 2. - (y + h / 2. + offset.y)) / scale,
        order as f32 * 0.001,
    );
    Some((sprite, transform))
}

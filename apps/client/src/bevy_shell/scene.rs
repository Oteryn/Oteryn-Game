//! A direct projection of the existing Scene's ordered quads; no offline world.
use super::owner::HostOwner;
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    window::PrimaryWindow,
};
use oteryn_renderer::{AtlasImage, QuadInstance, lighting::QuadLayer};

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
    mut images: ResMut<Assets<Image>>,
    mut quads: Query<(&mut Sprite, &mut Transform), With<WorldQuad>>,
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
    let lighting = &scene.sprites().lighting;
    let ordered = scene
        .tiles()
        .instances()
        .iter()
        .enumerate()
        .map(|(i, q)| (q, lighting.quad_tint(q, i, QuadLayer::Terrain)))
        .chain(
            scene
                .sprites()
                .instances()
                .iter()
                .enumerate()
                .map(|(i, q)| (q, lighting.quad_tint(q, i, QuadLayer::Overlay))),
        );
    let mut count = 0;
    for (index, (quad, tint)) in ordered.enumerate() {
        let (sprite, transform) = project_quad(quad, tint, index, &image, scene.atlas(), window);
        if let Some(entity) = cache.entities.get(index) {
            if let Ok((mut old_sprite, mut old_transform)) = quads.get_mut(*entity) {
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
                .push(commands.spawn((sprite, transform, WorldQuad)).id());
        }
        count += 1;
    }
    for entity in cache.entities.drain(count..) {
        commands.entity(entity).despawn();
    }
}
fn project_quad(
    quad: &QuadInstance,
    tint: [f32; 4],
    order: usize,
    image: &Handle<Image>,
    atlas: &AtlasImage,
    window: &Window,
) -> (Sprite, Transform) {
    let scale = window.scale_factor();
    let [x, y] = quad.position;
    let [w, h] = quad.size;
    let [u0, v0, u1, v1] = quad.uv;
    let sprite = Sprite {
        image: image.clone(),
        color: Color::linear_rgba(tint[0], tint[1], tint[2], tint[3]),
        rect: Some(Rect::new(
            u0 * atlas.width() as f32,
            v0 * atlas.height() as f32,
            u1 * atlas.width() as f32,
            v1 * atlas.height() as f32,
        )),
        custom_size: Some(Vec2::new(w / scale, h / scale)),
        ..default()
    };
    let transform = Transform::from_xyz(
        (x + w / 2.) / scale - window.width() / 2.,
        window.height() / 2. - (y + h / 2.) / scale,
        order as f32 * 0.001,
    );
    (sprite, transform)
}

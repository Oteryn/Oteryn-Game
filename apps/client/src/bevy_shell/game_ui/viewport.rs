//! One physical map rectangle shared by rendering, picking and viewport-dependent zoom.
use super::{super::owner::HostOwner, WorldRegion};
use bevy::{
    camera::{ClearColorConfig, Viewport, visibility::RenderLayers},
    prelude::*,
    window::PrimaryWindow,
};

#[derive(Component)]
pub(in crate::bevy_shell) struct WorldCamera;
#[derive(Component)]
struct WorldBackdrop;
#[derive(Resource, Default, Clone, Copy)]
pub(in crate::bevy_shell) struct WorldViewport {
    pub origin: UVec2,
    pub size: UVec2,
}
impl WorldViewport {
    pub fn scene_point(self, point: Vec2, scene_size: UVec2) -> Option<Vec2> {
        if !point.is_finite() || self.size.x == 0 || self.size.y == 0 {
            return None;
        }
        let origin = self.origin.as_vec2();
        if !point.cmpge(origin).all() || !point.cmplt(origin + self.size.as_vec2()).all() {
            return None;
        }
        let local = point - origin - self.scene_offset(scene_size);
        (local.cmpge(Vec2::ZERO).all() && local.cmplt(scene_size.as_vec2()).all()).then_some(local)
    }
    pub fn scene_offset(self, scene_size: UVec2) -> Vec2 {
        ((self.size.as_vec2() - scene_size.as_vec2()) * 0.5).floor()
    }
}
pub(in crate::bevy_shell) fn setup(
    mut commands: Commands,
    skins: Res<crate::bevy_shell::ui::Skins>,
) {
    commands.spawn((
        Camera2d,
        Camera {
            clear_color: ClearColorConfig::Custom(Color::BLACK),
            ..default()
        },
        WorldCamera,
        RenderLayers::layer(1),
    ));
    commands.spawn((
        Sprite {
            image: skins.inset.clone(),
            rect: Some(Rect::new(1., 1., 95., 95.)),
            image_mode: SpriteImageMode::Tiled {
                tile_x: true,
                tile_y: true,
                stretch_value: 1.,
            },
            ..default()
        },
        Transform::from_xyz(0., 0., -10.),
        Visibility::Hidden,
        WorldBackdrop,
        RenderLayers::layer(1),
    ));
}
pub(in crate::bevy_shell) fn project(
    owner: NonSend<HostOwner>,
    nodes: Query<(&ComputedNode, &UiGlobalTransform), With<WorldRegion>>,
    regions: Res<super::GameRegions>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut viewport: ResMut<WorldViewport>,
    mut cameras: Query<&mut Camera, With<WorldCamera>>,
    mut backdrops: Query<(&mut Sprite, &mut Visibility), With<WorldBackdrop>>,
) {
    let (Ok(window), Ok(mut camera)) = (windows.single(), cameras.single_mut()) else {
        return;
    };
    if owner.play.is_none() {
        for (_, mut visibility) in &mut backdrops {
            *visibility = Visibility::Hidden;
        }
        camera.is_active = true;
        camera.viewport = None;
        *viewport = WorldViewport::default();
        return;
    }
    let Ok((node, transform)) = nodes.get(regions.world) else {
        return;
    };
    let start = (transform.translation - node.size() * 0.5)
        .round()
        .max(Vec2::ZERO)
        .as_uvec2();
    let end = (transform.translation + node.size() * 0.5)
        .round()
        .max(Vec2::ZERO)
        .as_uvec2()
        .min(UVec2::new(
            window.physical_width(),
            window.physical_height(),
        ));
    let size = end.saturating_sub(start);
    for (mut sprite, mut visibility) in &mut backdrops {
        sprite.custom_size = Some(size.as_vec2() / window.scale_factor());
        *visibility = if size.min_element() > 0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if size.x == 0 || size.y == 0 {
        camera.is_active = false;
        camera.viewport = None;
        *viewport = WorldViewport::default();
        return;
    }
    camera.is_active = true;
    viewport.origin = start;
    viewport.size = size;
    camera.viewport = Some(Viewport {
        physical_position: start,
        physical_size: size,
        ..default()
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn map_picking_uses_visible_map_origin_and_rejects_panels()
    -> Result<(), Box<dyn std::error::Error>> {
        let map = WorldViewport {
            origin: UVec2::new(4, 28),
            size: UVec2::new(800, 560),
        };
        let scene =
            oteryn_renderer::TileView::new(oteryn_renderer::TileCoord::new(-7, -5), 48, 15, 11)?;
        let size = UVec2::new(720, 528);
        let local = map
            .scene_point(Vec2::new(44., 44.), size)
            .ok_or("visible map point rejected")?;
        assert_eq!(
            crate::input::click_tile(&scene, f64::from(local.x), f64::from(local.y)),
            Some(oteryn_renderer::TileCoord::new(-7, -5))
        );
        assert!(map.scene_point(Vec2::new(820., 80.), size).is_none());
        assert!(map.scene_point(Vec2::new(44., 620.), size).is_none());
        assert!(map.scene_point(Vec2::new(f32::NAN, 44.), size).is_none());
        // Letterbox space is not a tile, even though it belongs to the map camera.
        assert!(map.scene_point(Vec2::new(10., 30.), size).is_none());
        Ok(())
    }
}

//! One physical map rectangle shared by rendering, picking and viewport-dependent zoom.
use super::{super::owner::HostOwner, WorldRegion};
use bevy::{
    camera::{ClearColorConfig, Viewport, visibility::RenderLayers},
    prelude::*,
    window::PrimaryWindow,
};

#[derive(Default, Clone, Copy)]
pub(in crate::bevy_shell) enum ViewMode {
    #[default]
    Panel,
    Framed,
    Full,
}
impl ViewMode {
    pub fn cycle(&mut self) {
        *self = match self {
            Self::Panel => Self::Framed,
            Self::Framed => Self::Full,
            Self::Full => Self::Panel,
        };
    }
}
#[derive(Component)]
pub(in crate::bevy_shell) struct WorldCamera;
#[derive(Component)]
pub(in crate::bevy_shell) struct WorldBackdrop;
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
            image: skins.frame.clone(),
            rect: Some(Rect::new(4., 17., 100., 113.)),
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
    let (start, size) = match owner.view_mode {
        ViewMode::Panel => (start, size),
        ViewMode::Framed => {
            let fitted = (Vec2::new(15., 11.) * (size.x as f32 / 15.).min(size.y as f32 / 11.))
                .floor()
                .as_uvec2()
                .min(size);
            (start + (size - fitted) / 2, fitted)
        }
        ViewMode::Full => (
            UVec2::ZERO,
            UVec2::new(window.physical_width(), window.physical_height()),
        ),
    };
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

#[cfg(test)]
mod modes {
    use super::*;
    use crate::bevy_shell::{
        input::route_keyboard,
        owner::{ActivePlay, Phase},
    };
    #[test]
    fn control_period_cycles_camera_viewports() -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_input_actions::{
            ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent,
        };
        let mut owner = HostOwner::new(None)?;
        let (link, _, _) = crate::play::play_channel();
        owner.play = Some(ActivePlay {
            link,
            view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        });
        owner.phase = Phase::Playing;
        let mut app = App::new();
        app.insert_non_send(owner)
            .init_resource::<WorldViewport>()
            .add_systems(Update, project);
        let mut window = Window::default();
        window.resolution.set_physical_resolution(1200, 900);
        app.world_mut().spawn((window, PrimaryWindow));
        let world = app
            .world_mut()
            .spawn((
                WorldRegion,
                ComputedNode {
                    size: Vec2::new(800., 500.),
                    ..default()
                },
                UiGlobalTransform::from_translation(Vec2::new(500., 350.)),
            ))
            .id();
        app.insert_resource(super::super::GameRegions {
            world,
            vitals: world,
            minimap: world,
            equipment: world,
            containers: world,
            battle: world,
            skills: world,
            vip: world,
            analyzers: world,
            chat: world,
            bars: world,
        });
        app.world_mut().spawn((WorldCamera, Camera::default()));
        let mut areas = Vec::new();
        for _ in 0..4 {
            app.update();
            areas.push(app.world().resource::<WorldViewport>().size);
            route_keyboard(
                &mut app.world_mut().non_send_mut::<HostOwner>(),
                NormalizedInputEvent::Key {
                    code: KeyCode::new(55)?,
                    modifiers: Modifiers::one(Modifier::Control),
                    state: ButtonState::Pressed,
                    repeat: false,
                },
                false,
                None,
            );
        }
        assert_eq!(areas[0], areas[3]);
        assert_ne!(areas[0], areas[1]);
        assert!(areas[..3].contains(&UVec2::new(1200, 900)));
        Ok(())
    }
}

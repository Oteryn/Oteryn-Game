//! Optional HP/MP circles projected from confirmed session vitals.
use super::{game_ui::viewport::WorldViewport, owner::HostOwner};
use bevy::{camera::visibility::RenderLayers, prelude::*, window::PrimaryWindow};

const SEGMENTS: usize = 64;

#[derive(Component)]
struct CircleSegment {
    mana: bool,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<WorldViewport>()
        .add_systems(Update, project.after(super::scene::project));
}

fn fill(current: u32, maximum: u32) -> f32 {
    if maximum == 0 {
        0.
    } else {
        current.min(maximum) as f32 / maximum as f32
    }
}

fn health_colour(current: u32, maximum: u32) -> Color {
    // OTClient floors the percentage before comparing these strict thresholds.
    let percent = if maximum == 0 {
        0
    } else {
        u64::from(current.min(maximum)) * 100 / u64::from(maximum)
    };
    let [r, g, b] = match percent {
        93.. => [0, 188, 0],
        61..=92 => [80, 161, 80],
        31..=60 => [161, 161, 0],
        9..=30 => [191, 10, 10],
        4..=8 => [145, 15, 15],
        _ => [133, 12, 12],
    };
    Color::srgb_u8(r, g, b)
}

fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    viewport: Res<WorldViewport>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut segments: Query<(&mut CircleSegment, &mut Sprite, &mut Transform)>,
    mut entities: Local<Vec<Entity>>,
) {
    let preferences = &owner.settings.preferences;
    let active = owner
        .play
        .as_ref()
        .filter(|_| preferences.health_circle)
        .and_then(|play| play.link.actor_vitals().map(|vitals| (play, vitals)));
    let mut count = 0;
    if let Some((play, vitals)) = active {
        let scene = play.view.scene();
        let (width, height) = scene.view().viewport_px();
        let scene_size = UVec2::new(width, height);
        let size = if viewport.size.min_element() > 0 {
            viewport.size
        } else {
            scene_size
        };
        let offset = ((size.as_vec2() - scene_size.as_vec2()) * 0.5).floor();
        let center = Vec2::from(scene.health_circle_center()) + offset - size.as_vec2() * 0.5;
        let scale = windows.single().map_or(1., Window::scale_factor);
        let ui_scale = preferences.interface_scale as f32 / 100.;
        let radius = preferences.arcs.size as f32 * ui_scale;
        let distance = preferences.arcs.distance as f32 * ui_scale;
        let step = std::f32::consts::PI / SEGMENTS as f32;
        for (mana, current, maximum, side, colour) in [
            (
                false,
                vitals.health,
                vitals.max_health,
                -1.,
                health_colour(vitals.health, vitals.max_health),
            ),
            (
                true,
                vitals.mana,
                vitals.max_mana,
                1.,
                Color::srgb_u8(0, 70, 180),
            ),
        ] {
            let filled = fill(current, maximum) * SEGMENTS as f32;
            for index in 0..SEGMENTS {
                let portion = (filled - index as f32).clamp(0., 1.);
                if portion == 0. {
                    break;
                }
                // Fill from the bottom; the last segment preserves fractional vitals.
                let angle = std::f32::consts::PI - (index as f32 + portion * 0.5) * step;
                let at = center
                    + Vec2::new(
                        side * (distance + radius * angle.sin()),
                        -radius * angle.cos(),
                    );
                let sprite = Sprite::from_color(
                    colour.with_alpha(f32::from(preferences.arcs.opacity) / 100.),
                    Vec2::new(radius * step * portion, 3. * ui_scale) / scale,
                );
                let transform = Transform::from_xyz(at.x / scale, -at.y / scale, 100.)
                    .with_rotation(Quat::from_rotation_z(
                        (-angle.sin()).atan2(side * angle.cos()),
                    ));
                if let Some(entity) = entities.get(count) {
                    if let Ok((mut segment, mut old_sprite, mut old_transform)) =
                        segments.get_mut(*entity)
                    {
                        segment.mana = mana;
                        *old_sprite = sprite;
                        *old_transform = transform;
                    }
                } else {
                    entities.push(
                        commands
                            .spawn((
                                CircleSegment { mana },
                                sprite,
                                transform,
                                RenderLayers::layer(1),
                            ))
                            .id(),
                    );
                }
                count += 1;
            }
        }
    }
    for entity in entities.drain(count..) {
        commands.entity(entity).despawn();
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::bevy_shell::owner::{ActivePlay, HostOwner, Phase};
    use crate::play::{PlayView, play_channel};
    use oteryn_session::ActorVitals;

    fn segment_count(app: &mut App) -> usize {
        app.world_mut()
            .query::<&CircleSegment>()
            .iter(app.world())
            .count()
    }

    #[test]
    fn health_circle_fill() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new();
        let mut host = HostOwner::new(None)?;
        let (link, _commands, outbox) = play_channel();
        outbox.bevy_hud_fixture(
            &[],
            Some(ActorVitals {
                health: 25,
                max_health: 100,
                mana: 10,
                max_mana: 20,
                ..default()
            }),
        );
        host.phase = Phase::Playing;
        host.play = Some(ActivePlay {
            view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
            link,
        });
        app.insert_non_send(host);
        install(&mut app);
        app.update();
        assert_eq!(segment_count(&mut app), 0);
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .settings
            .preferences
            .health_circle = true;
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .settings
            .preferences
            .arcs
            .opacity = 40;
        app.update();
        let mut lengths = [0.; 2];
        for (segment, sprite) in app
            .world_mut()
            .query::<(&CircleSegment, &Sprite)>()
            .iter(app.world())
        {
            assert!((sprite.color.alpha() - 0.4).abs() < 0.001);
            lengths[usize::from(segment.mana)] += sprite.custom_size.ok_or("segment size")?.x;
        }
        assert!((lengths[0] / lengths[1] - 0.5).abs() < 0.001);
        assert!(lengths[0] > 0.);
        outbox.bevy_hud_fixture(
            &[],
            Some(ActorVitals {
                health: 100,
                max_health: 100,
                mana: 0,
                max_mana: 20,
                ..default()
            }),
        );
        app.update();
        let mut full = 0.;
        for (segment, sprite) in app
            .world_mut()
            .query::<(&CircleSegment, &Sprite)>()
            .iter(app.world())
        {
            assert!(!segment.mana);
            full += sprite.custom_size.ok_or("updated segment size")?.x;
        }
        assert!((full / lengths[0] - 4.).abs() < 0.001);
        outbox.bevy_hud_fixture(
            &[],
            Some(ActorVitals {
                health: 1,
                max_health: 100,
                mana: 1,
                max_mana: 100,
                ..default()
            }),
        );
        app.update();
        assert_eq!(segment_count(&mut app), 2);
        let partial: f32 = app
            .world_mut()
            .query::<&Sprite>()
            .iter(app.world())
            .filter_map(|sprite| sprite.custom_size.map(|size| size.x))
            .sum();
        assert!((partial / full - 0.02).abs() < 0.001);
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .settings
            .preferences
            .health_circle = false;
        app.update();
        assert_eq!(segment_count(&mut app), 0);
        assert_eq!(fill(10, 0), 0.);
        assert_eq!(fill(200, 100), 1.);
        let (link, _commands, new_outbox) = play_channel();
        {
            let mut host = app.world_mut().non_send_mut::<HostOwner>();
            host.settings.preferences.health_circle = true;
            host.play.as_mut().ok_or("active play")?.link = link;
        }
        app.update();
        assert_eq!(segment_count(&mut app), 0);
        new_outbox.bevy_hud_fixture(
            &[],
            Some(ActorVitals {
                health: 100,
                max_health: 100,
                mana: 100,
                max_mana: 100,
                ..default()
            }),
        );
        app.update();
        assert_eq!(segment_count(&mut app), 128);
        app.world_mut().non_send_mut::<HostOwner>().play = None;
        app.update();
        assert_eq!(segment_count(&mut app), 0);
        Ok(())
    }

    #[test]
    fn health_circle_colour_thresholds() {
        for (percent, rgb) in [
            (100, [0, 188, 0]),
            (93, [0, 188, 0]),
            (92, [80, 161, 80]),
            (61, [80, 161, 80]),
            (60, [161, 161, 0]),
            (31, [161, 161, 0]),
            (30, [191, 10, 10]),
            (9, [191, 10, 10]),
            (8, [145, 15, 15]),
            (4, [145, 15, 15]),
            (3, [133, 12, 12]),
            (0, [133, 12, 12]),
        ] {
            assert_eq!(
                health_colour(percent, 100),
                Color::srgb_u8(rgb[0], rgb[1], rgb[2])
            );
        }
        assert_eq!(health_colour(929, 1000), health_colour(92, 100));
    }
}

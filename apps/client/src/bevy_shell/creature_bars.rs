//! World-camera bars retain exact OTClient colours instead of quantizing the UI atlas.
use super::{game_ui::viewport::WorldViewport, owner::HostOwner};
use crate::play::feature_0023_presentation::view::health_rgb;
use bevy::{camera::visibility::RenderLayers, prelude::*, window::PrimaryWindow};

#[derive(Component)]
struct CreatureBar;

#[derive(Component)]
struct WorldBar;

pub(super) fn install(app: &mut App) {
    app.init_resource::<WorldViewport>()
        .add_systems(Update, project.after(super::scene::project));
}

fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    viewport: Res<WorldViewport>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut bars: Query<(&mut Sprite, &mut Transform), With<WorldBar>>,
    mut entities: Local<Vec<Entity>>,
) {
    let mut count = 0;
    if (owner.settings.preferences.creature_bars || owner.settings.preferences.hud_mana)
        && let Some(play) = &owner.play
    {
        let scene = play.view.scene();
        let (width, height) = scene.view().viewport_px();
        let scene_size = Vec2::new(width as f32, height as f32);
        let size = if viewport.size.min_element() > 0 {
            viewport.size.as_vec2()
        } else {
            scene_size
        };
        let offset = ((size - scene_size) * 0.5).floor();
        let scale = windows.single().map_or(1., Window::scale_factor);
        let ui_scale = f32::from(owner.settings.preferences.interface_scale) / 100.;
        for bar in play.view.creature_health_bars() {
            let [r, g, b] = if bar.covered {
                [96; 3]
            } else {
                health_rgb(bar.health)
            };
            let mana = bar
                .own
                .then(|| play.link.actor_vitals())
                .flatten()
                .filter(|v| v.max_mana > 0 && owner.settings.preferences.hud_mana);
            let gauges = [
                owner.settings.preferences.creature_bars.then_some((
                    false,
                    f32::from(bar.health) / 100.,
                    Color::srgb_u8(r, g, b),
                )),
                mana.map(|v| {
                    (
                        true,
                        v.mana.min(v.max_mana) as f32 / v.max_mana as f32,
                        Color::srgb_u8(0, 0, 255),
                    )
                }),
            ];
            for (mana, fraction, fill) in gauges.into_iter().flatten() {
                for (inset, width, height, colour) in [
                    (0., bar.width, 4. * ui_scale, Color::BLACK),
                    (
                        ui_scale,
                        (bar.width - 2. * ui_scale).max(0.) * fraction,
                        2. * ui_scale,
                        fill,
                    ),
                ] {
                    let dimensions = Vec2::new(width, height);
                    let at = Vec2::from(bar.at)
                        + Vec2::splat(inset)
                        + Vec2::Y * if mana { 6. * ui_scale } else { 0. };
                    let center = at + dimensions * 0.5 + offset - size * 0.5;
                    let sprite = Sprite::from_color(colour, dimensions / scale);
                    let transform =
                        Transform::from_xyz(center.x / scale, -center.y / scale, 101. + inset);
                    let id = if let Some(entity) = entities.get(count) {
                        if let Ok((mut old_sprite, mut old_transform)) = bars.get_mut(*entity) {
                            *old_sprite = sprite;
                            *old_transform = transform;
                        }
                        *entity
                    } else {
                        let id = commands
                            .spawn((WorldBar, sprite, transform, RenderLayers::layer(1)))
                            .id();
                        entities.push(id);
                        id
                    };
                    if mana {
                        commands.entity(id).remove::<CreatureBar>();
                    } else {
                        commands.entity(id).insert(CreatureBar);
                    }
                    count += 1;
                }
            }
        }
    }
    for entity in entities.drain(count..) {
        commands.entity(entity).despawn();
    }
}

#[cfg(test)]
mod tests;

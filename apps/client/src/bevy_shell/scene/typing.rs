//! An asset-free three-dot typing icon, projected beside the confirmed name label.
use super::*;

#[derive(Component)]
pub(in crate::bevy_shell) struct TypingDot;

pub(in crate::bevy_shell) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    window: Query<&Window, With<PrimaryWindow>>,
    viewport: Res<WorldViewport>,
    mut entities: Local<Vec<Entity>>,
    mut dots: Query<(&mut Sprite, &mut Transform), With<TypingDot>>,
) {
    let mut count = 0;
    if let Some(play) = &owner.play
        && let Ok(window) = window.single()
    {
        let scene = play.view.scene();
        let (width, height) = scene.view().viewport_px();
        let clip_size = Vec2::new(width as f32, height as f32);
        let size = if viewport.size.x > 0 {
            viewport.size.as_vec2()
        } else {
            Vec2::new(
                window.physical_width() as f32,
                window.physical_height() as f32,
            )
        };
        let projection = QuadViewport {
            scale: window.scale_factor(),
            size,
            offset: ((size - clip_size) * 0.5).floor(),
            clip_size,
        };
        for indicator in &play.link.player_typing_panel().indicators {
            let Some([x, y]) = scene.typing_position(indicator) else {
                continue;
            };
            for dx in [0., 3., 6.] {
                let quad = QuadInstance {
                    position: [x + dx, y],
                    size: [2., 2.],
                    uv: [0., 0., 1., 1.],
                };
                let order =
                    scene.tiles().instances().len() + scene.sprites().instances().len() + count;
                let Some((mut sprite, transform)) = project_quad(
                    &quad,
                    [1.; 4],
                    order,
                    &Handle::default(),
                    scene.atlas(),
                    projection,
                ) else {
                    continue;
                };
                // The default white image supplies solid dots; only geometry is clipped.
                sprite.rect = None;
                if let Some(entity) = entities.get(count) {
                    if let Ok((mut old_sprite, mut old_transform)) = dots.get_mut(*entity) {
                        *old_sprite = sprite;
                        *old_transform = transform;
                    }
                } else {
                    entities.push(
                        commands
                            .spawn((sprite, transform, TypingDot, RenderLayers::layer(1)))
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

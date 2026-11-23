//! Paint the same layered outfit cells and colours as the live scene.
use super::*;
use crate::bevy_shell::scene::SceneCache;

#[derive(Component, Default)]
pub(in crate::bevy_shell::game_ui::hud) struct Shown {
    cells: Vec<crate::map::Draw>,
    size: UVec2,
    atlas: Option<Handle<Image>>,
}

pub(in crate::bevy_shell::game_ui::hud) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    cache: Option<Res<SceneCache>>,
    mut frames: Query<(Entity, &TargetFrame, &mut Shown)>,
) {
    let Some(play) = owner.play.as_ref() else {
        return;
    };
    let scene = play.view.scene();
    let atlas = scene.atlas();
    let size = UVec2::new(atlas.width(), atlas.height());
    let handle = cache.as_deref().and_then(SceneCache::atlas_image);
    for (frame, target, mut shown) in &mut frames {
        let cells = scene.battle_portrait(target.0);
        if shown.cells == cells && shown.size == size && shown.atlas.as_ref() == handle {
            continue;
        }
        shown.cells = cells.to_vec();
        shown.size = size;
        shown.atlas = handle.cloned();
        commands.entity(frame).despawn_children();
        let (Some(handle), Some(first)) = (handle, cells.first()) else {
            continue;
        };
        let start = Vec2::new(first.offset[0] as f32, first.offset[1] as f32);
        let (min, max) = cells.iter().fold((start, start), |(min, max), draw| {
            let at = Vec2::new(draw.offset[0] as f32, draw.offset[1] as f32);
            (min.min(at), max.max(at))
        });
        let scale = (CREATURE_ICON - 2.) / ((max - min).max_element() + 32.);
        for (i, draw) in cells.iter().enumerate() {
            let Ok([u, v, r, b]) = atlas.uv_rect(draw.cell) else {
                continue;
            };
            let at = (Vec2::new(draw.offset[0] as f32, draw.offset[1] as f32) - min) * scale;
            let child = commands
                .spawn((
                    Name::new("Battle portrait cell"),
                    ImageNode {
                        image: handle.clone(),
                        rect: Some(Rect::new(
                            u * size.x as f32,
                            v * size.y as f32,
                            r * size.x as f32,
                            b * size.y as f32,
                        )),
                        ..default()
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(at.x),
                        top: px(at.y),
                        width: px(32. * scale),
                        height: px(32. * scale),
                        ..default()
                    },
                    ZIndex(i as i32),
                    Pickable::IGNORE,
                ))
                .id();
            commands.entity(frame).add_child(child);
        }
    }
}

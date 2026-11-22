use super::*;
use crate::bevy_shell::game_ui::viewport::WorldViewport;
use bevy::{camera::visibility::RenderLayers, picking::hover::Hovered, window::PrimaryWindow};

pub(in crate::bevy_shell::game_ui::hud) fn captions(
    owner: NonSend<HostOwner>,
    mut captions: Query<(&RowCaption, &mut Text)>,
) {
    let Some(play) = &owner.play else { return };
    let panel = play.link.battle_panel();
    let party = play.link.party_skull_panel();
    for (row, mut text) in &mut captions {
        let Some(actor) = panel
            .visible_actors
            .iter()
            .find(|actor| actor.entity == row.0)
        else {
            continue;
        };
        let mut caption = name(actor);
        if let Some(icon) = party.icons.iter().find(|icon| icon.entity == row.0)
            && let Some(skull) = [
                "Yellow skull",
                "Green skull",
                "White skull",
                "Red skull",
                "Black skull",
                "Orange skull",
            ]
            .get(usize::from(icon.skull).wrapping_sub(1))
        {
            caption.push_str(" · ");
            caption.push_str(skull);
        }
        if let Some(emblem) = play.view.battle_guild_mark(row.0) {
            caption.push_str(&format!(" · {emblem:?}"));
        }
        if text.0 != caption {
            text.0 = caption;
        }
    }
}

pub(in crate::bevy_shell::game_ui::hud) fn hover(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    hovered: Query<(&HudAction, &Hovered)>,
    viewport: Option<Res<WorldViewport>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut borders: Local<Vec<Entity>>,
    mut sprites: Query<(&mut Sprite, &mut Transform)>,
) {
    let target = owner.play.as_ref().and_then(|play| {
        if owner.modal_active() || play.view.gameplay_input_blocked(&play.link) {
            return None;
        }
        let target = hovered.iter().find_map(|(a, h)| match a.command {
            Command::Attack(id) if h.0 && a.generation == owner.generation() => Some(id),
            _ => None,
        })?;
        let panel = play.link.battle_panel();
        let actor = panel.visible_actors.iter().find(|a| a.entity == target)?;
        let scene = play.view.scene();
        let tile = oteryn_renderer::TileCoord::new(actor.position.x, actor.position.y);
        if !scene.view().contains(tile) {
            return None;
        }
        let (w, h) = scene.view().viewport_px();
        let clip = Vec2::new(w as f32, h as f32);
        let size = viewport
            .as_ref()
            .filter(|v| v.size.min_element() > 0)
            .map_or(clip, |v| v.size.as_vec2());
        let at = Vec2::from(scene.view().tile_to_screen(tile))
            + Vec2::from(scene.actor_offset(target))
            + play
                .view
                .camera_offset(std::time::Instant::now())
                .map_or(Vec2::ZERO, Vec2::from)
            + ((size - clip) * 0.5).floor()
            - size * 0.5;
        Some((at, scene.view().tile_px() as f32))
    });
    let Some((at, side)) = target else {
        for e in borders.drain(..) {
            commands.entity(e).try_despawn();
        }
        return;
    };
    let scale = windows.single().map_or(1., Window::scale_factor);
    for (i, (offset, size)) in [
        (Vec2::ZERO, Vec2::new(side, 1.)),
        (Vec2::Y * (side - 1.), Vec2::new(side, 1.)),
        (Vec2::ZERO, Vec2::new(1., side)),
        (Vec2::X * (side - 1.), Vec2::new(1., side)),
    ]
    .into_iter()
    .enumerate()
    {
        let center = at + offset + size * 0.5;
        let transform = Transform::from_xyz(center.x / scale, -center.y / scale, 101.);
        let sprite = Sprite::from_color(Color::WHITE, size / scale);
        if let Some(e) = borders.get(i) {
            if let Ok((mut s, mut t)) = sprites.get_mut(*e) {
                *s = sprite;
                *t = transform;
            }
        } else {
            borders.push(
                commands
                    .spawn((
                        Name::new("Battle hover border"),
                        sprite,
                        transform,
                        RenderLayers::layer(1),
                    ))
                    .id(),
            );
        }
    }
}

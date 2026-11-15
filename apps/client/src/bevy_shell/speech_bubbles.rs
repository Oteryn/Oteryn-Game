//! Local speech drawn over the map as text at the speaker's tile (OTClient's StaticText). The
//! text uses the shell's own `Text2d` on the world camera layer and the HUD font.
use super::{game_ui::viewport::WorldViewport, owner::HostOwner, ui::Skins};
use bevy::{
    camera::visibility::RenderLayers,
    prelude::*,
    sprite::Anchor,
    text::{TextBounds, TextLayoutInfo},
    window::PrimaryWindow,
};
use logic::{Speech, Texts};
use oteryn_client_ui_bevy as source;
use oteryn_renderer::TileCoord;

mod logic;
#[cfg(test)]
mod tests;

#[derive(Resource, Default)]
pub(super) struct SpeechBubbles {
    texts: Texts,
    previous: Vec<oteryn_client_runtime::chat::ChatLine>,
    seen: bool,
    entities: Vec<Entity>,
}

pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    (window, viewport): (Query<&Window, With<PrimaryWindow>>, Res<WorldViewport>),
    skins: Option<Res<Skins>>,
    time: Res<Time>,
    mut state: ResMut<SpeechBubbles>,
    mut drawn: Query<(&mut Text2d, &mut TextColor, &mut Transform, &TextLayoutInfo)>,
) {
    let state = &mut *state;
    let (Some(play), Some(window), Some(skins)) = (&owner.play, window.single().ok(), skins) else {
        state.texts.clear();
        state.previous.clear();
        state.seen = false;
        for entity in state.entities.drain(..) {
            commands.entity(entity).despawn();
        }
        return;
    };
    let now = u64::try_from(time.elapsed().as_millis()).unwrap_or(u64::MAX);
    let lines = play.link.chat_panel().lines;
    // Lines already in the log when the map opens are history, not speech.
    if state.seen {
        for line in logic::new_lines(&state.previous, &lines) {
            if let Some(speech) = Speech::from_line(line) {
                state.texts.add(&speech, now);
            }
        }
    }
    state.seen = true;
    state.previous = lines;
    state.texts.expire(now);

    let view = play.view.scene().view();
    let scene_size = view.viewport_px();
    let scene_size = Vec2::new(scene_size.0 as f32, scene_size.1 as f32);
    let size = if viewport.size.x > 0 {
        viewport.size.as_vec2()
    } else {
        Vec2::new(
            window.physical_width() as f32,
            window.physical_height() as f32,
        )
    };
    let offset = ((size - scene_size) * 0.5).floor();
    let scale = window.scale_factor();
    let visible: Vec<_> = state.texts.on_floor(play.view.floor()).collect();
    for (index, text) in visible.iter().enumerate() {
        let (x, y, _) = text.position();
        let [dx, dy] = view.tile_to_screen(TileCoord::new(x, y));
        let rgb = logic::colour(text.mode());
        let colour = Color::srgb_u8(rgb[0], rgb[1], rgb[2]);
        let Some(entity) = state.entities.get(index).copied() else {
            state.entities.push(
                commands
                    .spawn((
                        Text2d::new(text.compose()),
                        source::font(skins.font.clone(), false),
                        unmeasured(colour),
                        TextBounds::new_horizontal(logic::WRAP_WIDTH_PX),
                        Anchor::TOP_LEFT,
                        RenderLayers::layer(1),
                        Transform::from_xyz(0., 0., 900.),
                    ))
                    .id(),
            );
            continue;
        };
        let Ok((mut label, mut paint, mut transform, info)) = drawn.get_mut(entity) else {
            continue;
        };
        let composed = text.compose();
        if label.0 != composed {
            label.0 = composed;
        }
        // The size is the previous frame's layout; the text stays transparent until it exists.
        let measured = info.size;
        let shown = if measured.x > 0. {
            colour
        } else {
            colour.with_alpha(0.)
        };
        if paint.0 != shown {
            paint.0 = shown;
        }
        let (left, top) = logic::layout_rect(
            (dx / scale, dy / scale),
            (measured.x, measured.y),
            (scene_size.x / scale, scene_size.y / scale),
        );
        transform.translation.x = left + (offset.x - size.x / 2.) / scale;
        transform.translation.y = (size.y / 2. - offset.y) / scale - top;
    }
    for entity in state
        .entities
        .drain(visible.len().min(state.entities.len())..)
    {
        commands.entity(entity).despawn();
    }
}

/// A text before its first layout: transparent, but visible to the cameras. Bevy lays out
/// `Text2d` only for a camera that sees a sprite, and the world camera may see no sprite but
/// this text, so a hidden text would never be measured and never shown.
fn unmeasured(colour: Color) -> (TextColor, Visibility) {
    (TextColor(colour.with_alpha(0.)), Visibility::Inherited)
}

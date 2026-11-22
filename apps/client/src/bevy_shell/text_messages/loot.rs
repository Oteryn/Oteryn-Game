//! Per-item loot values use the same confirmed catalogue and chosen prices as the analyser.
use super::*;

#[derive(Default)]
pub(super) struct Display {
    scope: Option<(u64, u64)>,
    receipt: u64,
    last_query: Option<Duration>,
    dismissals: u64,
    expires: Duration,
    rendered: Vec<(String, u64)>,
}

fn colour(value: u64) -> Color {
    let rgb = match value {
        1_000_000.. => [255, 255, 0],
        100_000.. => [170, 85, 255],
        10_000.. => [159, 157, 253],
        1_000.. => [0, 235, 0],
        50.. => [170; 3],
        _ => [255; 3],
    };
    Color::srgb_u8(rgb[0], rgb[1], rgb[2])
}

pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    clock: Res<Time<Real>>,
    mut state: Local<Display>,
    mut areas: Query<(Entity, &ScreenArea, &mut Text, &TextFont)>,
) {
    let mut values = Vec::new();
    if let Some(play) = &owner.play {
        let (_, _, dismissals, namespace) = play.view.screen_message_controls();
        let scope = Some((owner.generation(), namespace));
        let receipt = play.link.loot_receipt();
        if state.scope != scope {
            state.dismissals = dismissals;
            state.expires = Duration::ZERO;
            state.receipt = if state
                .scope
                .is_some_and(|(generation, _)| generation == owner.generation())
            {
                receipt
            } else {
                0
            };
            state.scope = scope;
        }
        if receipt != state.receipt {
            state.receipt = receipt;
            state.last_query = None;
            let message = play.link.loot_values();
            let length = message
                .iter()
                .map(|(text, _)| text.len() + 2)
                .sum::<usize>();
            state.expires = clock
                .elapsed()
                .saturating_add(Duration::from_millis((length as u64 * 50).max(4000)));
        }
        if state.dismissals != dismissals {
            state.dismissals = dismissals;
            state.expires = Duration::ZERO;
        }
        if owner.settings.preferences.game_window.loot_messages && clock.elapsed() < state.expires {
            if state.last_query.is_none_or(|last| {
                clock.elapsed().saturating_sub(last) >= Duration::from_millis(500)
            }) {
                play.link.analyser_resolve_items();
                state.last_query = Some(clock.elapsed());
            }
            values = play.link.loot_values();
        }
    } else {
        state.scope = None;
        state.expires = Duration::ZERO;
    }
    if values == state.rendered {
        return;
    }
    for (entity, area, mut text, font) in &mut areas {
        if area.0 != Area::Loot {
            continue;
        }
        commands.entity(entity).despawn_children();
        text.0 = if values.is_empty() {
            String::new()
        } else {
            "Loot: ".into()
        };
        for (index, (name, price)) in values.iter().enumerate() {
            let prefix = if index == 0 { "" } else { ", " };
            let span = commands
                .spawn((
                    TextSpan::new(format!("{prefix}{name}")),
                    font.clone(),
                    TextColor(colour(*price)),
                ))
                .id();
            commands.entity(entity).add_child(span);
        }
    }
    state.rendered = values;
}

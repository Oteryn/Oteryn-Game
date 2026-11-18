//! OTClient screen areas over confirmed messages; chat retains its independent history.
use super::{game_ui::GameRegions, owner::HostOwner, ui::Skins};
use crate::play::{PlayView, feature_local_text_messages::view::LookMessage};
use bevy::prelude::*;
use oteryn_session::ServerMessage;
use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Area {
    Top,
    Look,
    Center,
    Bottom,
}
impl Area {
    const ALL: [Self; 4] = [Self::Top, Self::Look, Self::Center, Self::Bottom];
}
#[derive(Component)]
struct ScreenArea(Area);
struct ScreenMessage {
    text: String,
    colour: [u8; 3],
    expires: Duration,
}
#[derive(Resource, Default)]
struct ScreenMessages {
    slots: [Option<ScreenMessage>; 4],
    generation: Option<u64>,
    dismissals: u64,
    last_sequence: Option<u64>,
    namespace: u64,
    last_look: Option<u64>,
}

// Lua's #text counts UTF-8 bytes. This fork uses 50 ms/byte with a four-second minimum.
fn visible_time(text: &str) -> Duration {
    Duration::from_millis(
        u64::try_from(text.len())
            .unwrap_or(u64::MAX)
            .saturating_mul(50)
            .max(4000),
    )
}
impl ScreenMessages {
    fn show(&mut self, area: Area, text: &str, colour: [u8; 3], now: Duration) {
        self.slots[area as usize] = Some(ScreenMessage {
            text: text.into(),
            colour,
            expires: now.saturating_add(visible_time(text)),
        });
    }
    fn visible(&self, area: Area, now: Duration) -> Option<&ScreenMessage> {
        self.slots[area as usize]
            .as_ref()
            .filter(|message| now < message.expires)
    }
    fn observe(&mut self, view: &PlayView, now: Duration) {
        let namespace = view.screen_message_controls().3;
        if self.namespace != namespace {
            self.namespace = namespace;
            self.last_sequence = None;
            self.slots = default();
        }
        for message in view.server_messages() {
            if self
                .last_sequence
                .is_some_and(|last| message.server_sequence <= last)
            {
                continue;
            }
            let (area, text, colour) = match &message.value {
                ServerMessage::StatusText(text) => (Area::Bottom, text, [255; 3]),
                ServerMessage::Warning(text) => (Area::Center, text, [245, 94, 94]),
                ServerMessage::Advance(text) => (Area::Top, text, [255; 3]),
            };
            self.show(area, text, colour, now);
            self.last_sequence = Some(message.server_sequence);
        }
    }
    fn observe_look(&mut self, message: &LookMessage, now: Duration) {
        if self.last_look != Some(message.receipt) {
            self.show(Area::Look, &message.text, [0, 235, 0], now);
            self.last_look = Some(message.receipt);
        }
    }
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<ScreenMessages>()
        .add_systems(Startup, setup.after(super::game_ui::setup))
        .add_systems(Update, project);
}
fn setup(mut commands: Commands, skins: Res<Skins>, regions: Res<GameRegions>) {
    for area in Area::ALL {
        let text = commands
            .spawn((
                ScreenArea(area),
                oteryn_client_ui_bevy::text(
                    "",
                    oteryn_client_ui_bevy::font(skins.font.clone(), false),
                ),
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(10.),
                    width: percent(80.),
                    top: match area {
                        Area::Top => percent(15.),
                        Area::Look => percent(40.),
                        Area::Center => percent(60.),
                        Area::Bottom => Val::Auto,
                    },
                    bottom: if area == Area::Bottom {
                        px(8.)
                    } else {
                        Val::Auto
                    },
                    ..default()
                },
                GlobalZIndex(30),
            ))
            .insert(TextLayout::justify(Justify::Center))
            .id();
        commands.entity(regions.world).add_child(text);
    }
}
fn project(
    owner: NonSend<HostOwner>,
    time: Res<Time<Real>>,
    mut state: ResMut<ScreenMessages>,
    mut texts: Query<(&ScreenArea, &mut Text, &mut TextColor)>,
) {
    let now = time.elapsed();
    let (banners, popups) = if let Some(play) = &owner.play {
        let generation = Some(owner.generation());
        if state.generation != generation {
            *state = ScreenMessages {
                generation,
                ..default()
            };
        }
        state.observe(&play.view, now);
        if let Some(message) = play.link.look_message() {
            state.observe_look(&message, now);
        }
        let (banners, popups, dismissals, _) = play.view.screen_message_controls();
        if state.dismissals != dismissals {
            state.slots = default();
            state.dismissals = dismissals;
        }
        let feedback = play.view.take_minimap_feedback();
        if !feedback.is_empty() {
            state.show(Area::Bottom, &feedback, [255; 3], now);
        }
        (banners, popups)
    } else {
        *state = ScreenMessages::default();
        (false, false)
    };
    for (area, mut text, mut colour) in &mut texts {
        let enabled = match area.0 {
            Area::Top | Area::Bottom => banners,
            Area::Center => popups,
            Area::Look => true,
        };
        let message = state.visible(area.0, now).filter(|_| enabled);
        let value = message.map_or("", |message| message.text.as_str());
        if text.0 != value {
            text.0 = value.into();
        }
        if let Some(message) = message {
            let [r, g, b] = message.colour;
            colour.0 = Color::srgb_u8(r, g, b);
        }
    }
}

#[cfg(test)]
mod tests;

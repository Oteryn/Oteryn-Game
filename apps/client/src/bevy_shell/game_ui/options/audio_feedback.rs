//! Device/manifest failures come from the one playback owner, never an inferred UI status.
use super::*;
#[derive(Component)]
pub(super) struct Status;

pub(super) fn project(
    playback: Option<NonSend<crate::bevy_shell::audio::Playback>>,
    mut labels: Query<(&mut Text, &mut Node), With<Status>>,
) {
    // Playback is lazy. Absence of an error is not evidence of a started output stream.
    let reported = playback.as_ref().and_then(|output| output.status());
    for (mut text, mut node) in &mut labels {
        node.display = if reported.is_some() {
            Display::Flex
        } else {
            Display::None
        };
        if let Some(message) = reported {
            if text.0 != message {
                text.0 = message.to_owned();
            }
        } else if !text.0.is_empty() {
            text.0.clear();
        }
    }
}

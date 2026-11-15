//! Playback consumes the retained presentation queues and accepted settings.
mod driver;
#[cfg(test)]
mod tests;

use super::owner::HostOwner;
use crate::play::{AudioOutput, AudioSettings, SoundCue, UiSound};
use bevy::{prelude::*, window::PrimaryWindow};
use driver::{Output, Player};

impl Output for AudioOutput {
    fn update(&mut self, settings: AudioSettings, cues: Vec<SoundCue>, ui: Vec<UiSound>) {
        AudioOutput::update(self, settings, cues);
        self.play_ui(settings, ui);
    }
    fn stop(&mut self) {
        AudioOutput::stop(self);
    }
    fn status(&self) -> Option<&str> {
        AudioOutput::status(self)
    }
}

/// The Rodio stream is confined to the same main thread as its play owner.
#[derive(Default)]
pub(super) struct Playback {
    player: Player<AudioOutput>,
}
impl Playback {
    pub(super) fn status(&self) -> Option<&str> {
        self.player.status()
    }
}
pub(super) fn install(app: &mut App) {
    app.insert_non_send(Playback::default())
        .add_systems(Update, poll);
}
fn poll(
    mut owner: NonSendMut<HostOwner>,
    mut audio: NonSendMut<Playback>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let generation = owner.generation();
    let focused = windows.single().is_ok_and(|window| window.focused);
    audio.player.poll(
        owner.play.as_mut().map(|play| (generation, &mut play.view)),
        focused,
        AudioOutput::with_device,
    );
}

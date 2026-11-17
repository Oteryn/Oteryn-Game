use crate::play::{AudioSettings, PlayView, SoundCue, UiSound};

pub(super) trait Output {
    fn update(&mut self, settings: AudioSettings, cues: Vec<SoundCue>, ui: Vec<UiSound>);
    fn stop(&mut self);
    fn status(&self) -> Option<&str> {
        None
    }
}
pub(super) struct Player<O: Output> {
    output: Option<Result<O, String>>,
    generation: Option<u64>,
    device: Option<String>,
    ambient: Option<SoundCue>,
    focused: bool,
}
impl<O: Output> Default for Player<O> {
    fn default() -> Self {
        Self {
            output: None,
            generation: None,
            device: None,
            ambient: None,
            focused: false,
        }
    }
}
impl<O: Output> Player<O> {
    pub(super) fn poll(
        &mut self,
        play: Option<(u64, &mut PlayView)>,
        focused: bool,
        mut open: impl FnMut(&str) -> Result<O, String>,
    ) {
        let Some((generation, view)) = play else {
            self.reset();
            self.generation = None;
            self.ambient = None;
            return;
        };
        if self.generation != Some(generation) {
            self.reset();
            self.generation = Some(generation);
            self.ambient = None;
        }
        // Always drain the real bounded queues, even when unavailable or unfocused.
        let mut cues = view.take_sound_cues();
        let ui = view.take_ui_sounds();
        for cue in &cues {
            if cue.ambient {
                self.ambient = (cue.id != 0).then_some(*cue);
            }
        }
        if !focused {
            if self.focused {
                self.stop();
            }
            self.focused = false;
            return;
        }
        let requested = &view.preferences().audio_device;
        let changed = self.output.is_none() || self.device.as_deref() != Some(requested);
        if changed {
            self.reset();
            self.output = Some(open(requested));
            self.device = Some(requested.clone());
        }
        if (!self.focused || changed)
            && !cues.iter().any(|cue| cue.ambient)
            && let Some(ambient) = self.ambient
        {
            cues.push(ambient);
        }
        self.focused = true;
        if let Some(Ok(output)) = &mut self.output {
            output.update(view.preferences().audio, cues, ui);
        }
    }
    pub(super) fn status(&self) -> Option<&str> {
        match &self.output {
            Some(Ok(output)) => output.status(),
            Some(Err(error)) => Some(error),
            None => None,
        }
    }
    fn stop(&mut self) {
        if let Some(Ok(output)) = &mut self.output {
            output.stop();
        }
    }
    fn reset(&mut self) {
        self.stop();
        self.output = None;
        self.device = None;
        self.focused = false;
    }
}
impl<O: Output> Drop for Player<O> {
    fn drop(&mut self) {
        self.stop();
    }
}

//! Desktop screenshot caller: rendered client pixels and live session observations.
use super::*;
use oteryn_client::screenshots::{CaptureBacklog, CaptureController, Frame, Rect};
use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};

#[derive(Default)]
pub(super) struct Captures {
    controller: CaptureController,
    backlog: CaptureBacklog,
    last_query: Option<std::time::Instant>,
}
impl Application {
    pub(super) fn capture_input(&mut self, events: &[NormalizedInputEvent]) -> bool {
        for event in events {
            if let NormalizedInputEvent::Key {
                code,
                state: ButtonState::Pressed,
                repeat: false,
                modifiers,
            } = event
            {
                if code.get() == 70 && *modifiers == oteryn_input_actions::Modifiers::NONE {
                    self.captures.controller.manual();
                    return true;
                }
                if code.get() == 67
                    && modifiers.contains(Modifier::Control)
                    && modifiers.contains(Modifier::Shift)
                    && !modifiers.contains(Modifier::Alt)
                    && !modifiers.contains(Modifier::Super)
                {
                    let mut preferences = self.settings.screenshot_preferences();
                    preferences.game_only = ask(
                        "Capture only the game map? Choose No for the entire client render surface. Screenshots can contain visible player information. Print Screen saves a manual capture.",
                    );
                    preferences.on_level = ask(
                        "Automatically save a screenshot when your server-reported level increases? This queries skill progress while playing.",
                    );
                    preferences.on_death =
                        ask("Automatically save a screenshot when your character dies?");
                    preferences.on_achievement =
                        ask("Automatically save a screenshot when you earn an achievement?");
                    self.settings.set_screenshot_preferences(preferences);
                    return true;
                }
            }
        }
        false
    }
    pub(super) fn capture_tick(&mut self) {
        self.settings
            .screenshot_preferences()
            .apply_policy(&mut self.captures.controller);
        if let Some(play) = &self.play {
            let now = std::time::Instant::now();
            if self.captures.controller.policy.level
                && play.link.skill_progress_panel().complete_selected
                && self
                    .captures
                    .last_query
                    .is_none_or(|last| now.duration_since(last).as_secs() >= 2)
            {
                play.link.skill_progress_request();
                self.captures.last_query = Some(now);
            }
            self.captures
                .controller
                .observe(play.link.screenshot_facts());
        }
    }
    pub(super) fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        let scene = self
            .play
            .as_ref()
            .map(|play| play.view.scene())
            .or(self.scene.as_ref());
        let (Some(renderer), Some(scene)) = (&mut self.renderer, scene) else {
            return;
        };
        if !redraw_eligible(Some(renderer.state().phase())) {
            return;
        }
        if self.atlas.as_ref() != Some(scene.atlas()) {
            renderer.set_atlas(scene.atlas().clone());
            self.atlas = Some(scene.atlas().clone());
        }
        let mut preferences = self.settings.screenshot_preferences();
        if self.play.is_none() {
            preferences.backlog_seconds = 0;
        }
        let now = std::time::Instant::now();
        let sample = self
            .captures
            .backlog
            .sample_due(now, preferences.backlog_seconds);
        if self.captures.controller.pending() || sample {
            renderer.request_capture();
        }
        if renderer
            .render_batches(self.generation, scene.tiles(), scene.sprites())
            .is_err()
        {
            self.fail(event_loop, ShellError::RendererRender);
            return;
        }
        self.frame_timing.presented();
        if let Some(frame) = renderer.take_capture() {
            let result = (|| -> Result<(), Box<dyn std::error::Error>> {
                let frame = frame?;
                let (w, h) = scene.view().viewport_px();
                let directory = if !self.captures.controller.pending() {
                    std::path::PathBuf::new()
                } else if preferences.directory.is_empty() {
                    oteryn_client::native_login::settings_path()?
                        .parent()
                        .ok_or("No screenshot directory")?
                        .join("screenshots")
                } else {
                    std::path::PathBuf::from(&preferences.directory)
                };
                let paths = self.captures.backlog.process(
                    &mut self.captures.controller,
                    &preferences,
                    &directory,
                    Frame {
                        width: frame.width,
                        height: frame.height,
                        bgra: &frame.bgra,
                        game: Rect {
                            x: 0,
                            y: 0,
                            width: w.min(frame.width),
                            height: h.min(frame.height),
                        },
                    },
                    now,
                )?;
                for path in paths {
                    println!("Oteryn: screenshot saved to {}", path.display());
                }
                Ok(())
            })();
            if let Err(error) = result {
                self.captures.controller.cancel();
                oteryn_client::win_mutex::show_error(&format!(
                    "Screenshot was not saved: {error}. Press Print Screen to retry."
                ));
            }
        }
    }
}
// Native settings prompts run synchronously on the owning shell thread.
#[allow(unsafe_code)]
fn ask(text: &str) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IDYES, MB_DEFBUTTON2, MB_ICONQUESTION, MB_YESNO, MessageBoxW,
    };
    let wide = |value: &str| value.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide(text).as_ptr(),
            wide("Oteryn — Screenshots (Ctrl+Shift+F10)").as_ptr(),
            MB_YESNO | MB_DEFBUTTON2 | MB_ICONQUESTION,
        ) == IDYES
    }
}

//! Desktop screenshot caller: rendered client pixels and live session observations.
use super::*;
use oteryn_input_actions::{ButtonState, Modifier, NormalizedInputEvent};

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
                    self.settings.request_screenshot();
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
                        "Capture only the game map? Choose No for the entire client window. Screenshots can contain visible player information. Print Screen saves a manual capture.",
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
    pub(super) fn redraw(&mut self, event_loop: &ActiveEventLoop) {
        if !self.frame_timing.begin_redraw(self.settings.performance()) { return; }
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
        let before = renderer.state().presented_frames();
        if renderer
            .render_batches(self.generation, scene.tiles(), scene.sprites())
            .is_err()
        {
            self.fail(event_loop, ShellError::RendererRender);
            return;
        }
        if renderer.state().presented_frames() > before { self.frame_timing.presented(); }
        self.settings
            .screenshots(renderer, scene, self.play.as_ref().map(|play| &play.link));
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

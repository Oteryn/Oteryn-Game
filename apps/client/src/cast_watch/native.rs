//! Standalone native Watch shell: no PlayView, character login or private panels.
use super::{
    WatchConfig, WatchLink,
    picture::{Assets, Drawn},
};
use oteryn_client_runtime::ClientRuntime;
use oteryn_client_ui::cast_view::{ChatDraft, Panel, panel_text};
use oteryn_foundation::ProcessGeneration;
use oteryn_renderer::WindowsRenderer;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};
#[path = "native_hud.rs"]
mod hud;

pub fn run() -> Result<(), String> {
    let config = WatchConfig::from_env(|key| std::env::var(key).ok()).map_err(str::to_owned)?;
    let runtime = ClientRuntime::new().map_err(|e| e.to_string())?;
    let link = super::spawn(&runtime, config).map_err(|e| e.to_string())?;
    let mut app = Application {
        link: link.clone(),
        window: None,
        renderer: None,
        hud: None,
        assets: Assets::new(),
        drawn: None,
        panel: Panel::default(),
        draft: ChatDraft::default(),
        version: u64::MAX,
        generation: ProcessGeneration::new(1),
    };
    let events = EventLoop::new().map_err(|e| e.to_string())?;
    let result = events.run_app(&mut app).map_err(|e| e.to_string());
    link.leave();
    runtime.shutdown(Duration::from_millis(500));
    result
}
struct Application {
    link: WatchLink,
    window: Option<Arc<Window>>,
    renderer: Option<WindowsRenderer<Arc<Window>>>,
    hud: Option<hud::Hud>,
    assets: Assets,
    drawn: Option<Drawn>,
    panel: Panel,
    draft: ChatDraft,
    version: u64,
    generation: ProcessGeneration,
}
impl Application {
    fn refresh(&mut self) {
        let Some((version, panel)) = self.link.snapshot() else {
            self.link.fail();
            return;
        };
        let expiring = self
            .panel
            .picture
            .effects
            .iter()
            .any(|(at, _)| at.elapsed() >= Duration::from_millis(800));
        if version != self.version || expiring {
            self.panel = panel;
            self.panel.picture.expire(Instant::now());
            self.version = version;
            match self.assets.draw(&self.panel.picture, Instant::now()) {
                Ok(drawn) => {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.set_atlas(drawn.atlas.clone());
                    }
                    self.drawn = Some(drawn);
                }
                Err(_) => {
                    self.panel.picture.clear();
                    self.panel.picture.ended = true;
                    self.panel.feedback = tr!("Cast picture unavailable").into();
                    self.drawn = self.assets.draw(&self.panel.picture, Instant::now()).ok();
                    if let (Some(renderer), Some(drawn)) = (&mut self.renderer, &self.drawn) {
                        renderer.set_atlas(drawn.atlas.clone());
                    }
                    self.link.leave();
                }
            }
            if let Some(window) = &self.window {
                window.set_title(&lfmt!("Oteryn Watch — {}", self.panel.picture.status()));
                window.request_redraw();
            }
        }
        if let Some(hud) = &self.hud {
            hud.update(&lfmt!(
                "{}\nCast chat: {}",
                panel_text(&self.panel),
                self.draft.0
            ));
        }
    }
}
impl ApplicationHandler for Application {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        if let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) {
            let size = window.inner_size();
            if renderer
                .resume(self.generation, size.width, size.height)
                .is_err()
            {
                events.exit();
            }
            return;
        }
        let Ok(window) = events.create_window(
            Window::default_attributes()
                .with_title(tr!("Oteryn Watch — connecting"))
                .with_inner_size(LogicalSize::new(576.0, 700.0)),
        ) else {
            events.exit();
            return;
        };
        let window = Arc::new(window);
        let size = window.inner_size();
        let Ok(renderer) =
            WindowsRenderer::new(window.clone(), self.generation, size.width, size.height)
        else {
            events.exit();
            return;
        };
        self.hud = hud::Hud::new(&window);
        self.window = Some(window);
        self.renderer = Some(renderer);
        self.refresh();
    }
    fn suspended(&mut self, events: &ActiveEventLoop) {
        if let Some(renderer) = &mut self.renderer
            && renderer.suspend(self.generation).is_err()
        {
            events.exit();
        }
    }
    fn window_event(&mut self, events: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.link.leave();
                events.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer
                    && renderer
                        .resize(self.generation, size.width, size.height)
                        .is_err()
                {
                    events.exit();
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(renderer), Some(drawn)) = (&mut self.renderer, &self.drawn)
                    && renderer
                        .render_batches(self.generation, &drawn.tiles, &drawn.sprites)
                        .is_err()
                {
                    events.exit();
                }
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == ElementState::Pressed && !event.repeat =>
            {
                match event.logical_key {
                    Key::Named(NamedKey::Escape) => {
                        self.link.leave();
                        events.exit();
                    }
                    Key::Named(NamedKey::Enter) => {
                        if let Some(text) = self.draft.take() {
                            self.link.chat(text);
                        }
                    }
                    Key::Named(NamedKey::Backspace) => self.draft.backspace(),
                    _ => {
                        if let Some(text) = event.text {
                            self.draft.append(&text);
                        }
                    }
                }
                self.refresh();
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, events: &ActiveEventLoop) {
        self.refresh();
        events.set_control_flow(ControlFlow::WaitUntil(
            Instant::now() + Duration::from_millis(50),
        ));
    }
}

use oteryn_client::input::{MouseActions, arrow_step, click_tile};
use oteryn_client::play::{PlayLink, PlayView};
use oteryn_client::pre_native_status;
use oteryn_client::scene::PlaceholderScene;
use oteryn_client::{AdmittedSession, ClientBootstrap, GameplayEntryError};
use oteryn_foundation::ProcessGeneration;
use oteryn_input_platform::InputPlatformAdapter;
use oteryn_platform_client::native_login::PublicClass;
use oteryn_renderer::{SurfacePhase, WindowsRenderer};
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellError {
    EventLoopCreation,
    EventLoopRun,
    WindowCreation,
    InputInitialization,
    RendererInitialization,
    RendererResume,
    RendererSuspend,
    RendererResize,
    RendererRender,
    RendererClose,
}

impl Display for ShellError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EventLoopCreation => "client event loop creation failed",
            Self::EventLoopRun => "client event loop failed",
            Self::WindowCreation => "client window creation failed",
            Self::InputInitialization => "client input initialization failed",
            Self::RendererInitialization => "client renderer initialization failed",
            Self::RendererResume => "client renderer resume failed",
            Self::RendererSuspend => "client renderer suspend failed",
            Self::RendererResize => "client renderer resize failed",
            Self::RendererRender => "client renderer render failed",
            Self::RendererClose => "client renderer close failed",
        })
    }
}

impl std::error::Error for ShellError {}

struct Application {
    smoke: bool,
    window: Option<Arc<Window>>,
    renderer: Option<WindowsRenderer<Arc<Window>>>,
    scene: Option<PlaceholderScene>,
    client: Option<ClientBootstrap>,
    play: Option<Play>,
    generation: ProcessGeneration,
    input: InputPlatformAdapter,
    actions: MouseActions,
    fatal_error: Option<ShellError>,
}

/// The view of an admitted session and the link to its task on the client runtime.
struct Play {
    view: PlayView,
    link: PlayLink,
}

impl Application {
    fn new(
        smoke: bool,
        play: Option<(ClientBootstrap, AdmittedSession)>,
    ) -> Result<Self, ShellError> {
        let (client, play) = match play {
            Some((client, admitted)) => {
                let (view, link) = client
                    .start_play(admitted)
                    .map_err(|_error| ShellError::RendererInitialization)?;
                (Some(client), Some(Play { view, link }))
            }
            None => (None, None),
        };
        Ok(Self {
            smoke,
            window: None,
            renderer: None,
            scene: None,
            client,
            play,
            generation: ProcessGeneration::new(1),
            input: InputPlatformAdapter::new(),
            actions: MouseActions::new().map_err(|_error| ShellError::InputInitialization)?,
            fatal_error: None,
        })
    }

    fn handle_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) {
        if let Some(play) = &mut self.play {
            if let Some(direction) = arrow_step(events) {
                play.view.arrow(direction);
            }
            let mut render_failed = false;
            for click in self.actions.route(events) {
                if let Some(tile) = click_tile(play.view.scene().view(), click.x, click.y) {
                    render_failed |= play.view.click(tile).is_err();
                }
            }
            if render_failed {
                self.fail(event_loop, ShellError::RendererRender);
            }
            return;
        }
        for click in self.actions.route(events) {
            let picked = self.scene.as_mut().and_then(|scene| {
                let tile = click_tile(scene.view(), click.x, click.y)?;
                Some((tile, scene.select_tile(tile)))
            });
            // Without a session a click only selects a visible entity or object.
            if let Some((_, Err(_error))) = picked {
                self.fail(event_loop, ShellError::RendererRender);
            }
        }
    }

    /// A session end returns to the pre-admission state: the placeholder scene, with the public
    /// class in the title and on the console. The session task is gone with its link.
    fn return_to_login(&mut self, class: PublicClass) {
        self.play = None;
        let message = GameplayEntryError::Rejected(class).to_string();
        println!("Oteryn: {message}");
        if let Some(window) = &self.window {
            window.set_title(&format!("Oteryn — {message}"));
        }
        let Ok(scene) = PlaceholderScene::new() else {
            return;
        };
        if let Some(renderer) = &mut self.renderer {
            renderer.set_atlas(scene.atlas().clone());
        }
        self.scene = Some(scene);
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: ShellError) {
        retain_first_error(&mut self.fatal_error, error);
        event_loop.exit();
    }
}

fn retain_first_error(slot: &mut Option<ShellError>, error: ShellError) {
    if slot.is_none() {
        *slot = Some(error);
    }
}

const fn redraw_eligible(phase: Option<SurfacePhase>) -> bool {
    matches!(phase, Some(SurfacePhase::Configured))
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) {
            let size = window.inner_size();
            if renderer
                .resume(self.generation, size.width, size.height)
                .is_err()
            {
                self.fail(event_loop, ShellError::RendererResume);
            }
            return;
        }
        if self.window.is_some() {
            return;
        }
        let attributes = WindowAttributes::default()
            .with_title(format!("Oteryn — {}", pre_native_status()))
            .with_inner_size(LogicalSize::new(960.0, 540.0));
        let Ok(window) = event_loop.create_window(attributes) else {
            self.fail(event_loop, ShellError::WindowCreation);
            return;
        };
        let window = Arc::new(window);
        if self.smoke {
            self.window = Some(window);
            event_loop.exit();
            return;
        }
        let size = window.inner_size();
        let Ok(scene) = PlaceholderScene::new() else {
            self.fail(event_loop, ShellError::RendererInitialization);
            return;
        };
        let Ok(mut renderer) = WindowsRenderer::new(
            Arc::clone(&window),
            self.generation,
            size.width,
            size.height,
        ) else {
            self.fail(event_loop, ShellError::RendererInitialization);
            return;
        };
        let atlas = self
            .play
            .as_ref()
            .map_or_else(|| scene.atlas(), |play| play.view.scene().atlas());
        renderer.set_atlas(atlas.clone());
        window.request_redraw();
        self.window = Some(window);
        self.renderer = Some(renderer);
        self.scene = Some(scene);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(renderer) = &mut self.renderer
            && renderer.suspend(self.generation).is_err()
        {
            self.fail(event_loop, ShellError::RendererSuspend);
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let Ok(events) = self.input.process_device_event(&event) {
            self.handle_input(event_loop, &events);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // Every window event goes through the adapter and router first; select or walk is
        // triggered only by the routed gameplay action.
        if let Ok(events) = self.input.process_window_event(&event) {
            self.handle_input(event_loop, &events);
        }
        match event {
            WindowEvent::CloseRequested => {
                if let Some(renderer) = &mut self.renderer
                    && renderer.close(self.generation).is_err()
                {
                    self.fail(event_loop, ShellError::RendererClose);
                    return;
                }
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer
                    && renderer
                        .resize(self.generation, size.width, size.height)
                        .is_err()
                {
                    self.fail(event_loop, ShellError::RendererResize);
                }
            }
            WindowEvent::RedrawRequested => {
                let scene = self
                    .play
                    .as_ref()
                    .map(|play| play.view.scene())
                    .or(self.scene.as_ref());
                if let (Some(renderer), Some(scene)) = (&mut self.renderer, scene)
                    && redraw_eligible(Some(renderer.state().phase()))
                    && renderer
                        .render_batches(self.generation, scene.tiles(), scene.sprites())
                        .is_err()
                {
                    self.fail(event_loop, ShellError::RendererRender);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(play) = &mut self.play
            && let Err(class) = play.view.tick(&play.link)
        {
            self.return_to_login(class);
        }
        if let Some(window) = &self.window
            && redraw_eligible(
                self.renderer
                    .as_ref()
                    .map(|renderer| renderer.state().phase()),
            )
        {
            window.request_redraw();
        }
    }
}

pub fn run(play: Option<(ClientBootstrap, AdmittedSession)>) -> Result<(), ShellError> {
    let event_loop = EventLoop::new().map_err(|_error| ShellError::EventLoopCreation)?;
    let smoke = std::env::args().any(|argument| argument == "--smoke");
    let mut application = Application::new(smoke, play)?;
    let run_result = event_loop
        .run_app(&mut application)
        .map_err(|_error| ShellError::EventLoopRun);
    drop(application.play.take());
    if let Some(client) = application.client.take() {
        client.shutdown();
    }
    if let Some(error) = application.fatal_error {
        return Err(error);
    }
    run_result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fatal_retention_keeps_the_first_error() {
        let mut error = None;
        retain_first_error(&mut error, ShellError::RendererResize);
        retain_first_error(&mut error, ShellError::RendererRender);
        assert_eq!(error, Some(ShellError::RendererResize));
    }

    #[test]
    fn redraw_requires_a_configured_renderer() {
        assert!(redraw_eligible(Some(SurfacePhase::Configured)));
        assert!(!redraw_eligible(None));
        assert!(!redraw_eligible(Some(SurfacePhase::Unconfigured)));
        assert!(!redraw_eligible(Some(SurfacePhase::Suspended)));
        assert!(!redraw_eligible(Some(SurfacePhase::Lost)));
        assert!(!redraw_eligible(Some(SurfacePhase::Closing)));
    }
}

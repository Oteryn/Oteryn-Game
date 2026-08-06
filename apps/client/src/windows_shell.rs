use oteryn_client::pre_native_status;
use oteryn_foundation::ProcessGeneration;
use oteryn_renderer::WindowsRenderer;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellError {
    EventLoopCreation,
    EventLoopRun,
}

impl Display for ShellError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EventLoopCreation => "client event loop creation failed",
            Self::EventLoopRun => "client event loop failed",
        })
    }
}

impl std::error::Error for ShellError {}

struct Application {
    smoke: bool,
    window: Option<Arc<Window>>,
    renderer: Option<WindowsRenderer<Arc<Window>>>,
    generation: ProcessGeneration,
}

impl Application {
    fn new(smoke: bool) -> Self {
        Self {
            smoke,
            window: None,
            renderer: None,
            generation: ProcessGeneration::new(1),
        }
    }
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = WindowAttributes::default()
            .with_title(format!("Oteryn — {}", pre_native_status()))
            .with_inner_size(LogicalSize::new(960.0, 540.0));
        let Ok(window) = event_loop.create_window(attributes) else {
            event_loop.exit();
            return;
        };
        let window = Arc::new(window);
        if self.smoke {
            self.window = Some(window);
            event_loop.exit();
            return;
        }
        let size = window.inner_size();
        let Ok(renderer) = WindowsRenderer::new(
            Arc::clone(&window),
            self.generation,
            size.width,
            size.height,
        ) else {
            event_loop.exit();
            return;
        };
        window.request_redraw();
        self.window = Some(window);
        self.renderer = Some(renderer);
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(renderer) = &mut self.renderer {
            let _result = renderer.suspend(self.generation);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(renderer) = &mut self.renderer {
                    let _result = renderer.close(self.generation);
                }
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer
                    && renderer
                        .resize(self.generation, size.width, size.height)
                        .is_err()
                {
                    event_loop.exit();
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(renderer) = &mut self.renderer
                    && renderer.render(self.generation).is_err()
                {
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

pub fn run() -> Result<(), ShellError> {
    let event_loop = EventLoop::new().map_err(|_error| ShellError::EventLoopCreation)?;
    let smoke = std::env::args().any(|argument| argument == "--smoke");
    let mut application = Application::new(smoke);
    event_loop
        .run_app(&mut application)
        .map_err(|_error| ShellError::EventLoopRun)
}

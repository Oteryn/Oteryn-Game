#![recursion_limit = "256"]
#![allow(
    dead_code,
    clippy::collapsible_if,
    clippy::manual_is_multiple_of,
    reason = "the bounded evidence model retains semantic-only fields, explicit parity formulas and a readable final-frame timestamp block"
)]

mod gpu;
mod model;

use gpu::{RenderOutcome, Renderer};
use model::{BenchConfig, CorpusShape, SceneModel};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("world-vfx-prototype-error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let config = BenchConfig::from_args()?;
    let corpus = CorpusShape::load(config.census_path.as_deref())?;
    let model = SceneModel::new(corpus);
    let event_loop = EventLoop::new().map_err(|error| format!("event loop: {error}"))?;
    let mut application = Application::new(config, corpus, model);
    event_loop
        .run_app(&mut application)
        .map_err(|error| format!("event loop run: {error}"))?;
    application.fatal_error.map_or(Ok(()), Err)
}

struct Application {
    config: BenchConfig,
    corpus: CorpusShape,
    model: SceneModel,
    process_start: Instant,
    semantic_frame: u64,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    fatal_error: Option<String>,
}

impl Application {
    fn new(config: BenchConfig, corpus: CorpusShape, model: SceneModel) -> Self {
        Self {
            config,
            corpus,
            model,
            process_start: Instant::now(),
            semantic_frame: 0,
            window: None,
            renderer: None,
            fatal_error: None,
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl Into<String>) {
        if self.fatal_error.is_none() {
            self.fatal_error = Some(error.into());
        }
        event_loop.exit();
    }
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            return;
        }
        let attributes = WindowAttributes::default()
            .with_title("Oteryn World + VFX Prototype — Issue #480")
            .with_inner_size(LogicalSize::new(
                f64::from(self.config.width),
                f64::from(self.config.height),
            ));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                self.fail(event_loop, format!("window creation: {error}"));
                return;
            }
        };
        let initial_plan = self.model.plan(&self.config, 0);
        match Renderer::new(
            Arc::clone(&window),
            &self.config,
            &initial_plan,
            self.corpus,
            self.process_start.elapsed(),
        ) {
            Ok(renderer) => {
                window.request_redraw();
                self.window = Some(window);
                self.renderer = Some(renderer);
            }
            Err(error) => self.fail(event_loop, error),
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
            }
            WindowEvent::RedrawRequested => {
                let Some(renderer) = &mut self.renderer else {
                    return;
                };
                let plan_started = Instant::now();
                let plan = self.model.plan(&self.config, self.semantic_frame);
                let model_prep = plan_started.elapsed();
                match renderer.render(&self.config, &plan, model_prep) {
                    Ok(RenderOutcome::Continue) => {
                        self.semantic_frame = self.semantic_frame.saturating_add(1);
                    }
                    Ok(RenderOutcome::Complete(result)) => {
                        println!("{result}");
                        event_loop.exit();
                    }
                    Err(error) => self.fail(event_loop, error),
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

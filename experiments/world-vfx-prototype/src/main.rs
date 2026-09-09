mod config;
mod metrics;
mod model;
mod renderer;

use crate::config::{AssetProof, BenchConfig, CorpusCensus, hash_asset_zip, require_asset_match};
use crate::model::{AnimationEvaluationEvidence, SceneModel};
use crate::renderer::Renderer;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
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
    let census = CorpusCensus::load(&config.census_path)?;
    validate_corpus(&census)?;
    let asset = config
        .asset_zip
        .as_deref()
        .map(|path| hash_asset_zip(path, &census.source.zip_sha256))
        .transpose()?;
    if let Some(proof) = &asset {
        require_asset_match(proof)?;
    }

    let scene = SceneModel::new(census.clone());
    let event_loop = EventLoop::new().map_err(|error| format!("event loop: {error}"))?;
    let mut application = Application::new(config, census, scene, asset);
    event_loop
        .run_app(&mut application)
        .map_err(|error| format!("event loop run: {error}"))?;
    application.fatal_error.map_or(Ok(()), Err)
}

fn validate_corpus(census: &CorpusCensus) -> Result<(), String> {
    let expected = [
        ("object", census.census.object.appearances, 43_514_u32),
        ("outfit", census.census.outfit.appearances, 1_480_u32),
        ("effect", census.census.effect.appearances, 243_u32),
        ("missile", census.census.missile.appearances, 76_u32),
    ];
    for (family, actual, required) in expected {
        if actual != required {
            return Err(format!(
                "15.32 census mismatch for {family}: expected {required}, got {actual}"
            ));
        }
    }
    if census.source.label != "15.32" {
        return Err(format!(
            "prototype requires source label 15.32, got {}",
            census.source.label
        ));
    }
    if census.total_appearances() != 45_313 {
        return Err(format!(
            "unexpected total appearance count: {}",
            census.total_appearances()
        ));
    }
    Ok(())
}

struct Application {
    config: BenchConfig,
    census: CorpusCensus,
    scene: SceneModel,
    asset: Option<AssetProof>,
    animation_evaluation: AnimationEvaluationEvidence,
    process_start: Instant,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    semantic_frame: u64,
    result_printed: bool,
    fatal_error: Option<String>,
}

impl Application {
    fn new(
        config: BenchConfig,
        census: CorpusCensus,
        scene: SceneModel,
        asset: Option<AssetProof>,
    ) -> Self {
        let animation_evaluation = scene.animation_evaluation_evidence();
        Self {
            config,
            census,
            scene,
            asset,
            animation_evaluation,
            process_start: Instant::now(),
            window: None,
            renderer: None,
            semantic_frame: 0,
            result_printed: false,
            fatal_error: None,
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl Into<String>) {
        if self.fatal_error.is_none() {
            self.fatal_error = Some(error.into());
        }
        event_loop.exit();
    }

    fn print_result(
        &mut self,
        event_loop: &ActiveEventLoop,
        result: &crate::metrics::RunResult,
    ) -> Result<(), String> {
        if !self.result_printed {
            let encoded = serde_json::to_string(result)
                .map_err(|error| format!("serialize result: {error}"))?;
            println!("{encoded}");
            self.result_printed = true;
        }
        if !self.config.preview {
            event_loop.exit();
        }
        Ok(())
    }
}
impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            return;
        }
        let attributes = WindowAttributes::default()
            .with_title("Oteryn World + VFX prototype")
            .with_inner_size(PhysicalSize::new(self.config.width, self.config.height));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                self.fail(event_loop, format!("window creation: {error}"));
                return;
            }
        };
        match Renderer::new(
            Arc::clone(&window),
            &self.config,
            self.animation_evaluation.clone(),
            self.process_start.elapsed(),
            self.asset.as_ref(),
        ) {
            Ok(renderer) => {
                eprintln!(
                    "pipeline-prewarm-ms={:.3}",
                    renderer.pipeline_prewarm().as_secs_f64() * 1000.0
                );
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
                let elapsed_ms = self.process_start.elapsed().as_millis() as u64;
                let snapshot = self
                    .scene
                    .frame(&self.config, elapsed_ms, self.semantic_frame);
                self.semantic_frame = self.semantic_frame.saturating_add(1);
                let Some(renderer) = &mut self.renderer else {
                    return;
                };
                match renderer.render(&self.config, &self.census, &snapshot) {
                    Ok(Some(result)) => {
                        if let Err(error) = self.print_result(event_loop, &result) {
                            self.fail(event_loop, error);
                        }
                    }
                    Ok(None) => {}
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_census_shape_is_accepted() -> Result<(), String> {
        let census_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/contracts/OTERYN_ATLAS_15_32_ANIMATION_CENSUS_V1.json");
        let census = CorpusCensus::load(&census_path)?;
        validate_corpus(&census)
    }
}

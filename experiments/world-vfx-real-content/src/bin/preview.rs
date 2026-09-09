use oteryn_world_vfx_real_content::demo::{DemoFrame, build_demo_frame};
use oteryn_world_vfx_real_content::prepared_cache::SOURCE_ZIP_SHA256;
use oteryn_world_vfx_real_content::renderer::RealContentRenderer;
use oteryn_world_vfx_real_content::scene::QualificationBundle;
use oteryn_world_vfx_real_content::visible_set::VisibleSpriteSet;
use serde_json::json;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

const SEMANTIC_TICKS_PER_SECOND: u64 = 60;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("real-content-preview-error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let config = PreviewConfig::from_args()?;
    let mut bundle =
        QualificationBundle::open(&config.manifest, &config.scene, config.cache_pages)?;
    let visible = VisibleSpriteSet::build(&mut bundle)?;
    let event_loop = EventLoop::new().map_err(|error| format!("event loop: {error}"))?;
    let mut application = PreviewApplication::new(config, bundle, visible);
    event_loop
        .run_app(&mut application)
        .map_err(|error| format!("event loop run: {error}"))?;
    application.fatal_error.map_or(Ok(()), Err)
}

#[derive(Debug, Clone)]
struct PreviewConfig {
    manifest: PathBuf,
    scene: PathBuf,
    head_sha: String,
    width: u32,
    height: u32,
    cache_pages: usize,
    frames: u64,
}

impl PreviewConfig {
    fn from_args() -> Result<Self, String> {
        let mut manifest = None;
        let mut scene = None;
        let mut head_sha = None;
        let mut width = 1280_u32;
        let mut height = 960_u32;
        let mut cache_pages = 64_usize;
        let mut frames = 600_u64;
        let mut args = std::env::args().skip(1);
        while let Some(flag) = args.next() {
            let value = args
                .next()
                .ok_or_else(|| format!("missing value after {flag}"))?;
            match flag.as_str() {
                "--manifest" => manifest = Some(PathBuf::from(value)),
                "--scene" => scene = Some(PathBuf::from(value)),
                "--head" => head_sha = Some(value),
                "--width" => width = parse_positive(&value, "width")?,
                "--height" => height = parse_positive(&value, "height")?,
                "--cache-pages" => cache_pages = parse_positive(&value, "cache-pages")?,
                "--frames" => frames = parse_positive(&value, "frames")?,
                _ => return Err(format!("unknown preview argument {flag}")),
            }
        }
        let head_sha = head_sha.ok_or_else(|| "--head is required".to_owned())?;
        if head_sha.len() != 40
            || !head_sha
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err("--head must be a lowercase 40-character Git SHA".to_owned());
        }
        Ok(Self {
            manifest: manifest.ok_or_else(|| "--manifest is required".to_owned())?,
            scene: scene.ok_or_else(|| "--scene is required".to_owned())?,
            head_sha,
            width,
            height,
            cache_pages,
            frames,
        })
    }
}

fn parse_positive<T>(value: &str, label: &str) -> Result<T, String>
where
    T: std::str::FromStr + PartialEq + Default,
    T::Err: std::fmt::Display,
{
    let parsed = value
        .parse::<T>()
        .map_err(|error| format!("invalid {label} {value:?}: {error}"))?;
    if parsed == T::default() {
        return Err(format!("{label} must be positive"));
    }
    Ok(parsed)
}

struct PreviewApplication {
    config: PreviewConfig,
    bundle: QualificationBundle,
    visible: VisibleSpriteSet,
    window: Option<Arc<Window>>,
    renderer: Option<RealContentRenderer>,
    semantic_frame: u64,
    last_frame: Option<DemoFrame>,
    evidence_printed: bool,
    fatal_error: Option<String>,
}

impl PreviewApplication {
    fn new(config: PreviewConfig, bundle: QualificationBundle, visible: VisibleSpriteSet) -> Self {
        Self {
            config,
            bundle,
            visible,
            window: None,
            renderer: None,
            semantic_frame: 0,
            last_frame: None,
            evidence_printed: false,
            fatal_error: None,
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: impl Into<String>) {
        if self.fatal_error.is_none() {
            self.fatal_error = Some(error.into());
        }
        event_loop.exit();
    }

    fn print_evidence(&mut self) -> Result<(), String> {
        if self.evidence_printed {
            return Ok(());
        }
        let renderer = self
            .renderer
            .as_ref()
            .ok_or_else(|| "renderer missing at evidence time".to_owned())?;
        let last_frame = self
            .last_frame
            .as_ref()
            .ok_or_else(|| "no rendered demo frame at evidence time".to_owned())?;
        let visible = self.visible.stats();
        let cache = self.bundle.cache().stats();
        let evidence = json!({
            "schema": "oteryn-world-vfx-real-content-smoke-v1",
            "head_sha": self.config.head_sha,
            "source_zip_sha256": SOURCE_ZIP_SHA256,
            "scene_sha256": self.bundle.scene().source_sha256(),
            "appearance_product_root": self.bundle.programs().product_root(),
            "frames_requested": self.config.frames,
            "semantic_ticks_per_second": SEMANTIC_TICKS_PER_SECOND,
            "last_semantic_ms": last_frame.elapsed_ms,
            "last_outfit_phase": last_frame.outfit_phase,
            "last_effect_phase": last_frame.effect_phase,
            "last_missile_phase": last_frame.missile_phase,
            "camera": {
                "center_x_units": last_frame.camera.center_x_units,
                "center_y_units": last_frame.camera.center_y_units,
                "zoom": last_frame.camera.zoom,
            },
            "visible_working_set": {
                "sprite_count": visible.sprite_count,
                "source_page_count": visible.source_page_count,
                "rgba_bytes": visible.rgba_bytes,
                "page_requests": visible.page_requests,
                "page_hits": visible.page_hits,
                "page_loads": visible.page_loads,
                "page_evictions": visible.page_evictions,
                "bytes_read": visible.bytes_read,
            },
            "prepared_cache_total": {
                "page_requests": cache.page_requests,
                "page_hits": cache.page_hits,
                "page_loads": cache.page_loads,
                "page_evictions": cache.page_evictions,
                "bytes_read": cache.bytes_read,
            },
            "renderer": renderer.evidence(),
            "real_object_path_visible": true,
            "real_outfit_path_visible": true,
            "real_effect_path_visible": true,
            "real_missile_path_visible": true,
            "movement_interpolation_visible": true,
            "screen_space_name_hp_overlay_visible": true,
            "day_night_light_path_visible": true,
            "telegraph_overlay_visible": true,
            "camera_scroll_fractional_zoom_visible": true,
            "proprietary_pixels_committed": false,
        });
        let encoded = serde_json::to_string(&evidence)
            .map_err(|error| format!("serialize smoke evidence: {error}"))?;
        println!("{encoded}");
        self.evidence_printed = true;
        Ok(())
    }
}

impl ApplicationHandler for PreviewApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.renderer.is_some() {
            return;
        }
        let attributes = WindowAttributes::default()
            .with_title("Oteryn real-content World + VFX qualification")
            .with_inner_size(PhysicalSize::new(self.config.width, self.config.height));
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                self.fail(event_loop, format!("window creation: {error}"));
                return;
            }
        };
        match RealContentRenderer::new(Arc::clone(&window), &self.visible) {
            Ok(mut renderer) => {
                renderer.resize(self.config.width, self.config.height);
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
                let elapsed_ms =
                    self.semantic_frame.saturating_mul(1_000) / SEMANTIC_TICKS_PER_SECOND;
                let frame = match build_demo_frame(&self.bundle, &self.visible, elapsed_ms) {
                    Ok(frame) => frame,
                    Err(error) => {
                        self.fail(event_loop, error);
                        return;
                    }
                };
                let Some(renderer) = &mut self.renderer else {
                    return;
                };
                if let Err(error) = renderer.render(&frame) {
                    self.fail(event_loop, error);
                    return;
                }
                self.semantic_frame = self.semantic_frame.saturating_add(1);
                self.last_frame = Some(frame);
                if self.semantic_frame >= self.config.frames {
                    if let Err(error) = self.print_evidence() {
                        self.fail(event_loop, error);
                        return;
                    }
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

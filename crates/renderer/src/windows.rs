use crate::scene_gpu::SceneGpu;
use crate::{
    AtlasImage, RendererError, SpriteBatch, SurfaceDecision, SurfaceEvent, SurfaceSize,
    SurfaceState, TileBatch,
};
use oteryn_foundation::ProcessGeneration;

pub struct WindowsRenderer<T>
where
    T: Clone + Into<wgpu::SurfaceTarget<'static>>,
{
    target: T,
    instance: wgpu::Instance,
    surface: wgpu::Surface<'static>,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    configuration: Option<wgpu::SurfaceConfiguration>,
    state: SurfaceState,
    atlas: Option<AtlasImage>,
    scene: Option<SceneGpu>,
}

/// Tile and sprite batches for one frame; sprites are drawn over tiles.
#[derive(Clone, Copy)]
struct FrameBatches<'a> {
    tiles: &'a TileBatch,
    sprites: &'a SpriteBatch,
}

impl<T> WindowsRenderer<T>
where
    T: Clone + Into<wgpu::SurfaceTarget<'static>>,
{
    pub fn new(
        target: T,
        process_generation: ProcessGeneration,
        width: u32,
        height: u32,
    ) -> Result<Self, RendererError> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..wgpu::InstanceDescriptor::new_without_display_handle()
        });
        let surface = instance
            .create_surface(target.clone())
            .map_err(|_error| RendererError::SurfaceCreation)?;
        let (adapter, device, queue) = pollster::block_on(async {
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    ..wgpu::RequestAdapterOptions::default()
                })
                .await
                .map_err(|_error| RendererError::AdapterUnavailable)?;
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("oteryn-renderer-device"),
                    ..wgpu::DeviceDescriptor::default()
                })
                .await
                .map_err(|_error| RendererError::DeviceRequest)?;
            Ok::<_, RendererError>((adapter, device, queue))
        })?;
        let mut renderer = Self {
            target,
            instance,
            surface,
            adapter,
            device,
            queue,
            configuration: None,
            state: SurfaceState::new(process_generation),
            atlas: None,
            scene: None,
        };
        renderer.resize(process_generation, width, height)?;
        Ok(renderer)
    }

    #[must_use]
    pub const fn state(&self) -> &SurfaceState {
        &self.state
    }

    pub fn resize(
        &mut self,
        generation: ProcessGeneration,
        width: u32,
        height: u32,
    ) -> Result<(), RendererError> {
        let decision = self.state.apply(SurfaceEvent::Resize {
            generation,
            width,
            height,
        })?;
        self.execute(decision, generation)
    }

    pub fn suspend(&mut self, generation: ProcessGeneration) -> Result<(), RendererError> {
        let decision = self.state.apply(SurfaceEvent::Suspend { generation })?;
        self.execute(decision, generation)
    }

    pub fn resume(
        &mut self,
        generation: ProcessGeneration,
        width: u32,
        height: u32,
    ) -> Result<(), RendererError> {
        let resize = self.state.apply(SurfaceEvent::Resize {
            generation,
            width,
            height,
        })?;
        self.execute(resize, generation)
    }

    /// Installs the atlas the batches sample. Required before `render_batches`.
    pub fn set_atlas(&mut self, atlas: AtlasImage) {
        self.atlas = Some(atlas);
        self.scene = None;
    }

    /// Clears the surface.
    pub fn render(&mut self, generation: ProcessGeneration) -> Result<(), RendererError> {
        self.render_frame(generation, None)
    }

    /// Clears the surface, then draws the tile batch and the sprite batch over it.
    pub fn render_batches(
        &mut self,
        generation: ProcessGeneration,
        tiles: &TileBatch,
        sprites: &SpriteBatch,
    ) -> Result<(), RendererError> {
        self.render_frame(generation, Some(FrameBatches { tiles, sprites }))
    }

    fn render_frame(
        &mut self,
        generation: ProcessGeneration,
        batches: Option<FrameBatches<'_>>,
    ) -> Result<(), RendererError> {
        if batches.is_some() && self.atlas.is_none() {
            return Err(RendererError::Validation);
        }
        let frame = crate::resources::acquire_frame(&self.state, generation, || {
            self.surface.get_current_texture()
        })?;
        match frame {
            wgpu::CurrentSurfaceTexture::Success(frame) => {
                self.present(frame, generation, false, batches)
            }
            wgpu::CurrentSurfaceTexture::Suboptimal(frame) => {
                self.present(frame, generation, true, batches)
            }
            wgpu::CurrentSurfaceTexture::Timeout => {
                let decision = self.state.apply(SurfaceEvent::Timeout { generation })?;
                self.execute(decision, generation)
            }
            wgpu::CurrentSurfaceTexture::Occluded => {
                let decision = self.state.apply(SurfaceEvent::Occluded { generation })?;
                self.execute(decision, generation)
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                let decision = self.state.apply(SurfaceEvent::Outdated { generation })?;
                self.execute(decision, generation)
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                let decision = self.state.apply(SurfaceEvent::Lost { generation })?;
                self.execute(decision, generation)
            }
            wgpu::CurrentSurfaceTexture::Validation => Err(RendererError::Validation),
        }
    }

    pub fn close(&mut self, generation: ProcessGeneration) -> Result<(), RendererError> {
        let decision = self.state.apply(SurfaceEvent::Close { generation })?;
        self.execute(decision, generation)
    }

    fn present(
        &mut self,
        frame: wgpu::SurfaceTexture,
        generation: ProcessGeneration,
        suboptimal: bool,
        batches: Option<FrameBatches<'_>>,
    ) -> Result<(), RendererError> {
        self.prepare_scene(batches)?;
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("oteryn-renderer-clear-encoder"),
            });
        let attachments = [Some(wgpu::RenderPassColorAttachment {
            view: &view,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color {
                    r: 0.035,
                    g: 0.055,
                    b: 0.090,
                    a: 1.0,
                }),
                store: wgpu::StoreOp::Store,
            },
        })];
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("oteryn-renderer-clear-pass"),
                color_attachments: &attachments,
                ..wgpu::RenderPassDescriptor::default()
            });
            if let (Some(batches), Some(scene)) = (batches, &self.scene) {
                scene.draw(&mut render_pass, batches.tiles, batches.sprites);
            }
        }
        let _submission = self.queue.submit([encoder.finish()]);
        self.queue.present(frame);

        let decision = self.state.apply(SurfaceEvent::Presented {
            generation,
            suboptimal,
        })?;
        self.execute(decision, generation)
    }

    /// Builds (or rebuilds after a surface format change) the scene resources and uploads the
    /// batches. Runs before the render pass so the pass only records draws.
    fn prepare_scene(&mut self, batches: Option<FrameBatches<'_>>) -> Result<(), RendererError> {
        let Some(batches) = batches else {
            return Ok(());
        };
        let (Some(atlas), Some(configuration)) = (&self.atlas, &self.configuration) else {
            return Err(RendererError::Validation);
        };
        if self
            .scene
            .as_ref()
            .is_none_or(|scene| scene.format() != configuration.format)
        {
            self.scene = Some(SceneGpu::new(
                &self.device,
                &self.queue,
                configuration.format,
                atlas,
            ));
        }
        if let Some(scene) = &self.scene {
            scene.upload(
                &self.queue,
                self.state.size(),
                batches.tiles,
                batches.sprites,
            );
        }
        Ok(())
    }

    fn execute(
        &mut self,
        decision: SurfaceDecision,
        generation: ProcessGeneration,
    ) -> Result<(), RendererError> {
        match decision {
            SurfaceDecision::Configure(size) | SurfaceDecision::Reconfigure(size) => {
                self.configure(size, generation)
            }
            SurfaceDecision::Recreate(size) => {
                self.surface = self
                    .instance
                    .create_surface(self.target.clone())
                    .map_err(|_error| RendererError::SurfaceCreation)?;
                self.configure(size, generation)
            }
            SurfaceDecision::PresentAndReconfigure(size) => self.configure(size, generation),
            SurfaceDecision::Suspend | SurfaceDecision::Close => {
                self.configuration = None;
                Ok(())
            }
            SurfaceDecision::None
            | SurfaceDecision::Present
            | SurfaceDecision::SkipTimeout
            | SurfaceDecision::SkipOccluded => Ok(()),
        }
    }

    fn configure(
        &mut self,
        size: SurfaceSize,
        generation: ProcessGeneration,
    ) -> Result<(), RendererError> {
        if size.is_zero() {
            return Err(RendererError::InvalidTransition {
                phase: self.state.phase(),
                event: crate::SurfaceEventKind::Configured,
            });
        }
        let configuration = self
            .surface
            .get_default_config(&self.adapter, size.width(), size.height())
            .ok_or(RendererError::SurfaceUnsupported)?;
        self.surface.configure(&self.device, &configuration);
        self.configuration = Some(configuration);
        let decision = self.state.apply(SurfaceEvent::Configured { generation })?;
        if decision == SurfaceDecision::None {
            Ok(())
        } else {
            Err(RendererError::BackendFatal)
        }
    }
}

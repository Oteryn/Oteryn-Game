#![allow(
    unsafe_code,
    reason = "Target borrows the creating thread's HWND until renderer drop"
)]
use super::*;
use oteryn_foundation::ProcessGeneration;
use oteryn_renderer::{SpriteBatch, WindowsRenderer};
use std::num::NonZeroIsize;
use windows_sys::Win32::Foundation::HWND;
use winit::raw_window_handle::{
    DisplayHandle, HandleError, HasDisplayHandle, HasWindowHandle, RawDisplayHandle,
    RawWindowHandle, Win32WindowHandle, WindowHandle, WindowsDisplayHandle,
};
#[derive(Clone, Copy)]
struct Target(NonZeroIsize);
impl HasWindowHandle for Target {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        Ok(unsafe {
            WindowHandle::borrow_raw(RawWindowHandle::Win32(Win32WindowHandle::new(self.0)))
        })
    }
}
impl HasDisplayHandle for Target {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::Windows(WindowsDisplayHandle::new()))
        })
    }
}
pub(in super::super) struct Surface {
    renderer: WindowsRenderer<Target>,
    generation: ProcessGeneration,
    batch: Option<TileBatch>,
}
impl Surface {
    pub(in super::super) fn new(hwnd: HWND) -> Option<Self> {
        let generation = ProcessGeneration::new(1);
        Some(Self {
            renderer: WindowsRenderer::new(
                Target(NonZeroIsize::new(hwnd as isize)?),
                generation,
                384,
                384,
            )
            .ok()?,
            generation,
            batch: None,
        })
    }
    pub(in super::super) fn update(&mut self, p: &WheelPanel) -> Result<(), String> {
        self.batch = None;
        if let Some((atlas, batch)) = frame(p).map_err(|e| e.to_string())? {
            self.renderer.set_atlas(atlas);
            self.batch = Some(batch);
        }
        Ok(())
    }
    pub(in super::super) fn render(&mut self) {
        let _ = match &self.batch {
            Some(b) => self
                .renderer
                .render_batches(self.generation, b, &SpriteBatch::new()),
            None => self.renderer.render(self.generation),
        };
    }
    pub(in super::super) fn close(&mut self) {
        let _ = self.renderer.close(self.generation);
    }
}

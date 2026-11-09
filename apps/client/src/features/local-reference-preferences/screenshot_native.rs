//! GPU readback for game-only images, native PrintWindow for the complete window.
use super::*;
use oteryn_client::{
    play::screenshot::{CaptureSource, Pixels},
    scene::Scene,
};
use oteryn_renderer::WindowsRenderer;
use std::io;
use windows_sys::Win32::{Graphics::Gdi::*, Storage::Xps::PrintWindow};

impl Settings {
    pub(crate) fn screenshots(
        &self,
        renderer: &mut WindowsRenderer<Arc<Window>>,
        scene: &Scene,
        link: Option<&oteryn_client::play::PlayLink>,
    ) {
        let mut source = NativeCapture {
            renderer,
            scene,
            hwnd: self.parent,
        };
        let mut state = lock(&self.shared);
        match state.process_screenshots(std::time::Instant::now(), link, &mut source) {
            Ok(paths) if !paths.is_empty() => {
                state.feedback = lfmt!(
                    "Saved {} screenshot(s): {}",
                    paths.len(),
                    paths
                        .last()
                        .map_or_else(String::new, |p| p.display().to_string())
                )
            }
            Err(error) => {
                state.cancel_screenshot();
                state.feedback = lfmt!("Screenshot failed: {error}", error = error);
                oteryn_client::win_mutex::show_error(&format!(
                    "Screenshot was not saved: {error}. Press Print Screen to retry."
                ));
            }
            Ok(_) => {}
        }
    }
}

struct NativeCapture<'a> {
    renderer: &'a mut WindowsRenderer<Arc<Window>>,
    scene: &'a Scene,
    hwnd: HWND,
}
impl CaptureSource for NativeCapture<'_> {
    fn capture(&mut self, game_only: bool) -> io::Result<Pixels> {
        if game_only {
            let (width, height, rgba) = self
                .renderer
                .capture_batches(self.scene.tiles(), self.scene.sprites())
                .map_err(io::Error::other)?;
            Pixels::new(width, height, rgba)
        } else {
            unsafe { window_pixels(self.hwnd) }
        }
    }
}

unsafe fn window_pixels(hwnd: HWND) -> io::Result<Pixels> {
    unsafe {
        let mut rect: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return Err(io::Error::last_os_error());
        }
        let (width, height) = (rect.right - rect.left, rect.bottom - rect.top);
        if width <= 0
            || height <= 0
            || width > 16384
            || height > 16384
            || i64::from(width) * i64::from(height) > 32 * 1024 * 1024
        {
            return Err(io::Error::other(tr!("Window is too large to capture")));
        }
        let dc = GetWindowDC(hwnd);
        if dc.is_null() {
            return Err(io::Error::last_os_error());
        }
        let memory = CreateCompatibleDC(dc);
        let bitmap = CreateCompatibleBitmap(dc, width, height);
        if memory.is_null() || bitmap.is_null() {
            if !memory.is_null() {
                DeleteDC(memory);
            }
            if !bitmap.is_null() {
                DeleteObject(bitmap);
            }
            ReleaseDC(hwnd, dc);
            return Err(io::Error::last_os_error());
        }
        let previous = SelectObject(memory, bitmap);
        let rendered = PrintWindow(hwnd, memory, 2);
        SelectObject(memory, previous);
        let result = (|| {
            if rendered == 0 {
                return Err(io::Error::last_os_error());
            }
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
            info.bmiHeader.biWidth = width;
            info.bmiHeader.biHeight = -height;
            info.bmiHeader.biPlanes = 1;
            info.bmiHeader.biBitCount = 32;
            info.bmiHeader.biCompression = BI_RGB;
            let mut bytes = vec![0; width as usize * height as usize * 4];
            if GetDIBits(
                memory,
                bitmap,
                0,
                height as u32,
                bytes.as_mut_ptr().cast(),
                &mut info,
                DIB_RGB_COLORS,
            ) != height
            {
                return Err(io::Error::last_os_error());
            }
            for pixel in bytes.chunks_exact_mut(4) {
                pixel.swap(0, 2);
                pixel[3] = 255;
            }
            Pixels::new(width as u32, height as u32, bytes)
        })();
        DeleteObject(bitmap);
        DeleteDC(memory);
        ReleaseDC(hwnd, dc);
        result
    }
}

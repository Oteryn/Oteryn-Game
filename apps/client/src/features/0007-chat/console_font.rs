//! Owns the transcript font on the same thread as the native chat dialog.
use super::*;
use windows_sys::Win32::Graphics::Gdi::{CreateFontIndirectW, DeleteObject, HFONT, LOGFONTW};

#[derive(Default)]
pub(super) struct ConsoleFont {
    handle: HFONT,
    pixels: u16,
}
impl ConsoleFont {
    pub(super) unsafe fn update(&mut self, transcript: HWND, pixels: u16) {
        unsafe {
            if self.pixels == pixels {
                return;
            }
            let mut description: LOGFONTW = std::mem::zeroed();
            description.lfHeight = -i32::from(pixels);
            for (destination, source) in
                description.lfFaceName.iter_mut().zip(wide(tr!("Consolas")))
            {
                *destination = source;
            }
            let next = CreateFontIndirectW(&description);
            if next.is_null() {
                return;
            }
            SendMessageW(transcript, WM_SETFONT, next as usize, 1);
            // T050: the explicit pixel-sized transcript font overrides common UI typography.
            SetPropW(
                transcript,
                wide(crate::play::FIXED_PIXEL_FONT_PROPERTY).as_ptr(),
                std::ptr::dangling_mut::<core::ffi::c_void>(),
            );
            if !self.handle.is_null() {
                DeleteObject(self.handle);
            }
            self.handle = next;
            self.pixels = pixels;
        }
    }
}
impl Drop for ConsoleFont {
    fn drop(&mut self) {
        unsafe {
            if !self.handle.is_null() {
                DeleteObject(self.handle);
            }
        }
    }
}

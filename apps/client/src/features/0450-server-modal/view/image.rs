//! Bounded inline pixels; no URL, asset lookup, or filesystem access.
#![allow(
    unsafe_code,
    reason = "GDI buffer and device context live for this synchronous paint"
)]
use oteryn_client_ui::server_modal::Image;
use windows_sys::Win32::{Foundation::HWND, Graphics::Gdi::*};
pub(super) unsafe fn paint(hwnd: HWND, image: &Image) {
    let mut pixels = image.rgba.clone();
    for pixel in pixels.as_chunks_mut::<4>().0 {
        let alpha = u16::from(pixel[3]);
        for channel in &mut pixel[..3] {
            *channel = ((u16::from(*channel) * alpha + 240 * (255 - alpha)) / 255) as u8;
        }
        pixel.swap(0, 2);
        pixel[3] = 255;
    }
    unsafe {
        let dc = GetDC(hwnd);
        if dc.is_null() {
            return;
        }
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = i32::from(image.width);
        info.bmiHeader.biHeight = -i32::from(image.height);
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        StretchDIBits(
            dc,
            12,
            204,
            i32::from(image.width),
            i32::from(image.height),
            0,
            0,
            i32::from(image.width),
            i32::from(image.height),
            pixels.as_ptr().cast(),
            &info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
        ReleaseDC(hwnd, dc);
    }
}

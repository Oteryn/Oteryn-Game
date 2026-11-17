//! The podium preview owns its HWNDs and decodes artwork outside the session mutex.
#![allow(unsafe_code, reason = "Window and GDI handles remain on this thread")]
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
};
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "�")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
pub(super) fn start(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        let panel = &mut mailbox.properties.house_podium;
        if !panel.open || !panel.selected || panel.graphic_worker {
            return;
        }
        panel.graphic_worker = true;
        panel.preview_requested = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    if std::thread::Builder::new()
        .name("podium-artwork".into())
        .spawn(move || run(weak))
        .is_err()
    {
        close(&rollback);
    }
}
fn close(weak: &Weak<Mutex<Mailbox>>) {
    if let Some(shared) = weak.upgrade() {
        lock(&shared).properties.house_podium.graphic_worker = false;
    }
}
fn run(weak: Weak<Mutex<Mailbox>>) {
    let mut assets = std::env::var_os(crate::map::ASSET_DIR_ENV)
        .ok_or_else(|| "Set OTERYN_ASSET_DIR to the existing pinned artwork root".to_owned())
        .and_then(|root| super::preview_assets::Assets::load(std::path::Path::new(&root)));
    let mut hwnd: HWND = null_mut();
    let mut label: HWND = null_mut();
    let mut held = None;
    let mut pixels: Vec<Vec<u8>> = Vec::new();
    unsafe {
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            let Some(shared) = weak.upgrade() else {
                break;
            };
            let (open, request, artwork, feedback, key) = {
                let mut mailbox = lock(&shared);
                let panel = &mut mailbox.properties.house_podium;
                (
                    panel.open && panel.selected,
                    std::mem::take(&mut panel.preview_requested),
                    panel.artwork,
                    panel.feedback.clone(),
                    panel.fields[3].clone(),
                )
            };
            if !open {
                break;
            }
            if IsWindow(hwnd) == 0 && request {
                hwnd = CreateWindowExW(
                    0,
                    wide("STATIC").as_ptr(),
                    wide("Podium artwork preview").as_ptr(),
                    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    380,
                    280,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null(),
                );
                if hwnd.is_null() {
                    break;
                }
                label = CreateWindowExW(
                    0,
                    wide("STATIC").as_ptr(),
                    wide("Choose a display and press Preview artwork").as_ptr(),
                    WS_CHILD | WS_VISIBLE,
                    10,
                    10,
                    350,
                    65,
                    hwnd,
                    null_mut(),
                    null_mut(),
                    null(),
                );
                held = None;
            }
            if IsWindow(hwnd) != 0 {
                let signature = (artwork, key.clone(), feedback.clone());
                if held.as_ref() != Some(&signature) {
                    pixels.clear();
                    let caption = if let Some(artwork) = artwork {
                        match &mut assets {
                            Ok(assets) => match assets.render(artwork) {
                                Ok(images) => {
                                    pixels = images;
                                    format!(
                                        "{key}\r\nDraft artwork; no ownership granted. Both outfit variants shown when available."
                                    )
                                }
                                Err(error) => error,
                            },
                            Err(error) => error.clone(),
                        }
                    } else {
                        format!("{key}\r\nChoose a display and press Preview artwork. {feedback}")
                    };
                    for image in &mut pixels {
                        for pixel in image.as_chunks_mut::<4>().0 {
                            pixel.swap(0, 2);
                        }
                    }
                    SetWindowTextW(label, wide(&caption).as_ptr());
                    InvalidateRect(hwnd, null(), 1);
                    held = Some(signature);
                }
                let dc = GetDC(hwnd);
                if !dc.is_null() {
                    let mut info: BITMAPINFO = std::mem::zeroed();
                    info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                    info.bmiHeader.biWidth = 128;
                    info.bmiHeader.biHeight = -128;
                    info.bmiHeader.biPlanes = 1;
                    info.bmiHeader.biBitCount = 32;
                    info.bmiHeader.biCompression = BI_RGB;
                    for (i, image) in pixels.iter().enumerate() {
                        StretchDIBits(
                            dc,
                            10 + i as i32 * 170,
                            90,
                            160,
                            160,
                            0,
                            0,
                            128,
                            128,
                            image.as_ptr().cast(),
                            &info,
                            DIB_RGB_COLORS,
                            SRCCOPY,
                        );
                    }
                    ReleaseDC(hwnd, dc);
                }
            }
            MsgWaitForMultipleObjects(0, null(), 0, 50, QS_ALLINPUT);
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
    }
    close(&weak);
}

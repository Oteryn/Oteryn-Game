//! Read-only native appearance preview. Asset I/O and drawing never hold the mailbox lock.
#![allow(
    unsafe_code,
    reason = "The preview window and GDI context stay on this thread"
)]
use super::preview::PreviewAssets;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
};
use windows_sys::Win32::{Foundation::HWND, Graphics::Gdi::*, UI::WindowsAndMessaging::*};
type WeakMailbox = Weak<Mutex<Mailbox>>;
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
pub(super) fn open(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        let panel = &mut mailbox.properties.appearance;
        if (!panel.open && !panel.memorial.open) || panel.preview_open {
            return;
        }
        panel.preview_open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    if std::thread::Builder::new()
        .name("appearance-preview".into())
        .spawn(move || run(weak))
        .is_err()
    {
        close(&rollback);
    }
}
fn close(weak: &WeakMailbox) {
    if let Some(shared) = weak.upgrade() {
        lock(&shared).properties.appearance.preview_open = false;
    }
}
fn run(weak: WeakMailbox) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Appearance preview").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            480,
            410,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            close(&weak);
            return;
        }
        let label = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Loading artwork...").as_ptr(),
            WS_CHILD | WS_VISIBLE,
            12,
            10,
            440,
            45,
            hwnd,
            null_mut(),
            null_mut(),
            null(),
        );
        let root = std::env::var_os(crate::map::ASSET_DIR_ENV)
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::current_dir().ok());
        let mut assets = root
            .as_ref()
            .ok_or_else(|| "Appearance artwork unavailable".to_owned())
            .and_then(|root| PreviewAssets::load(root));
        let mut held = None;
        let mut pixels: Option<Vec<u8>> = None;
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(shared) = weak.upgrade() else {
                break;
            };
            let signature = {
                let mailbox = lock(&shared);
                let panel = &mailbox.properties.appearance;
                if (!panel.open && !panel.memorial.open) || !panel.selected {
                    break;
                }
                (
                    panel.fields.clone(),
                    panel.memorial.clone(),
                    panel
                        .catalogue
                        .as_ref()
                        .map(|value| (value.context, value.root_revision)),
                )
            };
            if held.as_ref() != Some(&signature) {
                let panel = lock(&shared).properties.appearance.clone();
                let image = match &mut assets {
                    Ok(assets) if panel.memorial.open => assets.render_memorial(&panel.memorial),
                    Ok(assets) => assets.render(&panel),
                    Err(_) => Err("Appearance artwork unavailable".into()),
                };
                let caption = match image {
                    Ok(mut image) => {
                        for pixel in image.rgba.as_chunks_mut::<4>().0 {
                            pixel.swap(0, 2);
                        }
                        pixels = Some(image.rgba);
                        if panel.memorial.open {
                            format!("{} (preview only)", panel.memorial.offer().label)
                        } else {
                            format!(
                                "{}\r\nMount: {}",
                                panel.fields[0],
                                if panel.fields[2].is_empty() {
                                    "None"
                                } else {
                                    &panel.fields[2]
                                }
                            )
                        }
                    }
                    Err(error) => {
                        pixels = None;
                        error
                    }
                };
                SetWindowTextW(label, wide(&caption).as_ptr());
                InvalidateRect(hwnd, null(), 1);
                held = Some(signature);
            }
            if let Some(pixels) = &pixels {
                let dc = GetDC(hwnd);
                if !dc.is_null() {
                    let mut info: BITMAPINFO = std::mem::zeroed();
                    info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                    info.bmiHeader.biWidth = 128;
                    info.bmiHeader.biHeight = -128;
                    info.bmiHeader.biPlanes = 1;
                    info.bmiHeader.biBitCount = 32;
                    info.bmiHeader.biCompression = BI_RGB;
                    StretchDIBits(
                        dc,
                        12,
                        65,
                        288,
                        288,
                        0,
                        0,
                        128,
                        128,
                        pixels.as_ptr().cast(),
                        &info,
                        DIB_RGB_COLORS,
                        SRCCOPY,
                    );
                    ReleaseDC(hwnd, dc);
                }
            }
            MsgWaitForMultipleObjects(0, null(), 0, 50, QS_ALLINPUT);
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak);
    }
}

//! A window-thread-owned gallery over server look ids and the existing pinned art loader.
#![allow(unsafe_code)]
use super::controls::{control, read_text, wide};
use crate::{
    combat_input::PlayCommand,
    play::mailbox::{Mailbox, lock},
};
use oteryn_client_runtime::cyclopedia_character::gallery::Kind;
use oteryn_client_ui::cyclopedia_character::{Intent, request};
use std::{
    ptr::{null, null_mut},
    sync::{Mutex, Weak},
};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::System::SystemServices::SS_BITMAP;
use windows_sys::Win32::UI::Controls::EM_SETLIMITTEXT;
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};
#[path = "gallery_pixels.rs"]
mod pixels;
type Shared = Weak<Mutex<Mailbox>>;
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn proc(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let a = GetWindowLongPtrW(h, GWLP_USERDATA) as *mut Actions;
            if !a.is_null() && (*a).0.len() < 16 {
                (*a).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(h, msg, w, l)
    }
}
pub(in crate::play) fn open(weak: Shared, commands: UnboundedSender<PlayCommand>) {
    let Some(shared) = weak.upgrade() else {
        return;
    };
    let generation = {
        let mut m = lock(&shared);
        if !m.cyclopedia_character.selected || m.cyclopedia_character.gallery.open {
            return;
        }
        m.cyclopedia_character.gallery.open = true;
        m.cyclopedia_character.gallery.page = None;
        m.cyclopedia_character.feedback = None;
        m.cyclopedia_character.profile_generation
    };
    let failed = weak.clone();
    if std::thread::Builder::new()
        .name("character-gallery".into())
        .spawn(move || run(weak, commands, generation))
        .is_err()
    {
        close(&failed, generation);
    }
}
fn run(weak: Shared, commands: UnboundedSender<PlayCommand>, generation: u64) {
    // SAFETY: the HWND, bitmap handles and action storage stay on this thread.
    unsafe {
        let h = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Your appearances").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            820,
            750,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if h.is_null() {
            close(&weak, generation);
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(h, GWLP_USERDATA, (&mut *actions as *mut Actions) as isize);
        SetWindowLongPtrW(h, GWLP_WNDPROC, proc as *const () as isize);
        let filter = control(
            h,
            "COMBOBOX",
            "",
            1,
            [10, 10, 220, 180],
            CBS_DROPDOWNLIST as u32,
        );
        for name in [
            "All appearances",
            "Available outfits",
            "Owned mounts",
            "Familiar catalogue",
        ] {
            SendMessageW(filter, CB_ADDSTRING, 0, wide(name).as_ptr() as isize);
        }
        SendMessageW(filter, CB_SETCURSEL, 0, 0);
        let search = control(
            h,
            "EDIT",
            "",
            2,
            [245, 10, 360, 26],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        SendMessageW(search, EM_SETLIMITTEXT, 128, 0);
        control(h, "BUTTON", "Previous", 3, [10, 650, 130, 28], WS_TABSTOP);
        control(h, "BUTTON", "Next", 4, [155, 650, 130, 28], WS_TABSTOP);
        let heading = control(
            h,
            "STATIC",
            "Loading server appearances",
            5,
            [10, 45, 780, 40],
            0,
        );
        let mut images = Vec::new();
        let mut labels = Vec::new();
        for i in 0..24 {
            let x = 10 + (i % 6) * 130;
            let y = 95 + (i / 6) * 135;
            images.push(control(
                h,
                "STATIC",
                "",
                100 + i as usize,
                [x, y, 96, 96],
                SS_BITMAP,
            ));
            labels.push(control(
                h,
                "STATIC",
                "",
                200 + i as usize,
                [x, y + 97, 125, 35],
                0,
            ));
        }
        let mut art = pixels::Art::load();
        let mut bitmaps = Vec::new();
        let mut painted = None;
        let mut initialized = false;
        loop {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if IsWindow(h) == 0 || commands.is_closed() {
                break;
            }
            let Some(shared) = weak.upgrade() else {
                break;
            };
            let (gallery, pending, feedback) = {
                let mut m = lock(&shared);
                if m.cyclopedia_character_shutdown {
                    break;
                }
                let p = &mut m.cyclopedia_character;
                if p.profile_generation != generation || m_shutdown(p) {
                    break;
                }
                p.gallery.search = read_text(search);
                p.gallery.filter = match SendMessageW(filter, CB_GETCURSEL, 0, 0) {
                    1 => Some(Kind::Outfit),
                    2 => Some(Kind::Mount),
                    3 => Some(Kind::Familiar),
                    _ => None,
                };
                let mut intents: Vec<_> = actions
                    .0
                    .drain(..)
                    .filter_map(|id| match id {
                        3 => Some(Intent::GalleryPrevious),
                        4 => Some(Intent::GalleryNext),
                        _ => None,
                    })
                    .collect();
                if !initialized && !p.pending {
                    intents.push(Intent::Appearances);
                    initialized = true;
                }
                for intent in intents {
                    if let Some(command) = request(p, intent)
                        && commands.send(PlayCommand::Feature(command)).is_err()
                    {
                        p.pending = false;
                        p.gallery.pending_offset = None;
                        p.feedback = Some("The session has ended");
                    }
                }
                (p.gallery.clone(), p.pending, p.feedback)
            };
            let signature = (gallery.page.clone(), gallery.filter, gallery.search.clone());
            if painted.as_ref() != Some(&signature) {
                for image in &images {
                    SendMessageW(*image, STM_SETIMAGE, IMAGE_BITMAP as usize, 0);
                }
                for bitmap in bitmaps.drain(..) {
                    DeleteObject(bitmap);
                }
                let rows = gallery.rows();
                for (i, (image, label)) in images.iter().zip(&labels).enumerate() {
                    let Some(entry) = rows.get(i) else {
                        ShowWindow(*image, SW_HIDE);
                        SetWindowTextW(*label, wide("").as_ptr());
                        continue;
                    };
                    ShowWindow(*image, SW_SHOW);
                    let loaded = art
                        .as_mut()
                        .map_err(|e| e.as_str())
                        .and_then(|a| a.pixels(entry.look_type).map_err(|_| "Preview unavailable"));
                    if let Ok(rgba) = loaded
                        && let Some(bitmap) = bitmap(&rgba)
                    {
                        SendMessageW(*image, STM_SETIMAGE, IMAGE_BITMAP as usize, bitmap as isize);
                        bitmaps.push(bitmap);
                    }
                    let title = entry
                        .key
                        .rsplit('/')
                        .next()
                        .unwrap_or(&entry.key)
                        .replace('_', " ");
                    SetWindowTextW(
                        *label,
                        wide(&format!(
                            "{title}\r\n{:?} · {}",
                            entry.kind, entry.look_type
                        ))
                        .as_ptr(),
                    );
                }
                painted = Some(signature);
            }
            let page = gallery.page.as_ref().map_or(String::new(), |p| {
                format!(
                    "{}–{} of {} · Filters apply to this server page",
                    p.offset + u16::from(!p.entries.is_empty()),
                    usize::from(p.offset) + p.entries.len(),
                    p.total
                )
            });
            let status = if pending {
                "Loading"
            } else {
                feedback.unwrap_or("")
            };
            let art_error = art.as_ref().err().map_or("", String::as_str);
            SetWindowTextW(
                heading,
                wide(&format!("{page}\r\n{status} {art_error}")).as_ptr(),
            );
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        DestroyWindow(h);
        for bitmap in bitmaps {
            DeleteObject(bitmap);
        }
        close(&weak, generation);
    }
}
fn m_shutdown(p: &oteryn_client_ui::cyclopedia_character::CyclopediaCharacterPanel) -> bool {
    !p.selected
}
fn close(weak: &Shared, generation: u64) {
    if let Some(shared) = weak.upgrade() {
        let mut m = lock(&shared);
        if m.cyclopedia_character.profile_generation == generation {
            m.cyclopedia_character.gallery.open = false;
        }
    }
}
unsafe fn bitmap(rgba: &[u8]) -> Option<HBITMAP> {
    unsafe {
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = 96;
        info.bmiHeader.biHeight = -96;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        let mut bits = null_mut();
        let bitmap = CreateDIBSection(null_mut(), &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
        if bitmap.is_null() {
            return None;
        }
        std::ptr::copy_nonoverlapping(rgba.as_ptr(), bits as *mut u8, rgba.len());
        Some(bitmap)
    }
}

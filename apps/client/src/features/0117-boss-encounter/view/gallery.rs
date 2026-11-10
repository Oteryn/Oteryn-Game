//! Native boss gallery. All HWNDs, decoded portraits and GDI handles stay on this thread.
#![allow(
    unsafe_code,
    reason = "Win32 gallery resources are owned by their window thread"
)]
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::boss_encounter::{BossEncounterPanel, Intent, catalogue_cards};
use std::ptr::{null, null_mut};
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};
fn wide(s: &str) -> Vec<u16> {
    s.replace('\0', "�").encode_utf16().chain(Some(0)).collect()
}
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let state = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !state.is_null() && (*state).0.len() < 32 {
                (*state).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
pub(super) fn open(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.boss_encounter.gallery_open {
            return;
        }
        mailbox.boss_encounter.gallery_open = true;
    }
    let weak = std::sync::Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("bosstiary-gallery".into())
        .spawn(move || unsafe {
            let hwnd = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("Cyclopedia — Bosstiary gallery").as_ptr(),
                WS_OVERLAPPEDWINDOW | WS_VISIBLE,
                60,
                60,
                1220,
                900,
                null_mut(),
                null_mut(),
                null_mut(),
                null(),
            );
            if !hwnd.is_null() {
                let mut actions = Box::<Actions>::default();
                SetWindowLongPtrW(
                    hwnd,
                    GWLP_USERDATA,
                    actions.as_mut() as *mut Actions as isize,
                );
                SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
                for (i, name) in [
                    "Browse / search",
                    "Next page",
                    "Category",
                    "Previous boss",
                    "Next boss",
                ]
                .iter()
                .enumerate()
                {
                    CreateWindowExW(
                        0,
                        wide("BUTTON").as_ptr(),
                        wide(name).as_ptr(),
                        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                        10 + i as i32 * 150,
                        10,
                        145,
                        28,
                        hwnd,
                        (100 + i) as _,
                        null_mut(),
                        null(),
                    );
                }
                let edit = CreateWindowExW(
                    0,
                    wide("EDIT").as_ptr(),
                    wide("").as_ptr(),
                    WS_CHILD | WS_VISIBLE | WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
                    770,
                    10,
                    400,
                    28,
                    hwnd,
                    10 as _,
                    null_mut(),
                    null(),
                );
                SendMessageW(
                    edit,
                    windows_sys::Win32::UI::Controls::EM_SETLIMITTEXT,
                    48,
                    0,
                );
                for i in 0..16 {
                    CreateWindowExW(
                        0,
                        wide("BUTTON").as_ptr(),
                        wide("Select boss").as_ptr(),
                        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                        15 + (i % 4) * 295,
                        245 + (i / 4) * 190,
                        280,
                        24,
                        hwnd,
                        (1000 + i) as _,
                        null_mut(),
                        null(),
                    );
                }
                let mut message: MSG = std::mem::zeroed();
                let mut portraits = super::portraits::Portraits::new();
                let mut requested = false;
                while IsWindow(hwnd) != 0 && !commands.is_closed() {
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
                    let mut search = [0u16; 50];
                    let count = GetWindowTextW(edit, search.as_mut_ptr(), 50).max(0) as usize;
                    let search = String::from_utf16_lossy(&search[..count]);
                    let panel = {
                        let mut mailbox = lock(&shared);
                        if !requested && !mailbox.boss_encounter.pending {
                            super::super::input::enqueue(
                                &mut mailbox,
                                &commands,
                                Intent::Catalogue,
                            );
                            requested = true;
                        }
                        for action in actions.0.drain(..) {
                            if mailbox.boss_encounter.pending {
                                continue;
                            }
                            if (1000..1016).contains(&action) {
                                let key = catalogue_cards(&mailbox.boss_encounter)
                                    .get(action - 1000)
                                    .map(|e| e.key.clone());
                                if let Some(key) = key {
                                    mailbox.boss_encounter.fields[0] = key;
                                }
                                continue;
                            }
                            match action {
                                100 | 102 => {
                                    mailbox.boss_encounter.browser_search = search.clone();
                                    if action == 102 {
                                        mailbox.boss_encounter.browser_category =
                                            mailbox.boss_encounter.browser_category.next();
                                    }
                                    super::super::input::enqueue(
                                        &mut mailbox,
                                        &commands,
                                        Intent::Catalogue,
                                    );
                                }
                                101 => super::super::input::enqueue(
                                    &mut mailbox,
                                    &commands,
                                    Intent::CatalogueNext,
                                ),
                                103 | 104 => mailbox.boss_encounter.select_boss(action == 103),
                                _ => {}
                            }
                        }
                        mailbox.boss_encounter.clone()
                    };
                    paint(hwnd, &panel, &mut portraits);
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                if IsWindow(hwnd) != 0 {
                    DestroyWindow(hwnd);
                }
            }
            if let Some(shared) = weak.upgrade() {
                lock(&shared).boss_encounter.gallery_open = false;
            }
        })
        .is_err()
        && let Some(shared) = rollback.upgrade()
    {
        lock(&shared).boss_encounter.gallery_open = false;
    }
}
unsafe fn label(dc: HDC, x: i32, y: i32, s: &str) {
    unsafe {
        let s = wide(s);
        TextOutW(dc, x, y, s.as_ptr(), s.len().saturating_sub(1) as i32);
    }
}
unsafe fn fill(dc: HDC, rect: RECT, color: u32) {
    unsafe {
        let brush = CreateSolidBrush(color);
        if !brush.is_null() {
            FillRect(dc, &rect, brush);
            DeleteObject(brush);
        }
    }
}
unsafe fn paint(
    hwnd: HWND,
    panel: &BossEncounterPanel,
    portraits: &mut super::portraits::Portraits,
) {
    unsafe {
        let dc = GetDC(hwnd);
        if dc.is_null() {
            return;
        }
        let mut rect: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut rect);
        fill(
            dc,
            RECT {
                left: 0,
                top: 45,
                right: rect.right,
                bottom: rect.bottom,
            },
            0x181818,
        );
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, 0xeeeeee);
        label(
            dc,
            12,
            50,
            &format!(
                "{:?} • {} boss points • {}",
                panel.browser_category,
                panel.catalogue.as_ref().map_or(0, |p| p.points),
                if panel.pending {
                    "Waiting for server"
                } else {
                    panel
                        .feedback
                        .unwrap_or("Select a named card for entry and slot actions")
                }
            ),
        );
        let cards = catalogue_cards(panel);
        for i in 0..16 {
            ShowWindow(
                GetDlgItem(hwnd, 1000 + i),
                if (i as usize) < cards.len() {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
        }
        for (i, entry) in cards.iter().enumerate() {
            let x = 10 + (i % 4) as i32 * 295;
            let y = 80 + (i / 4) as i32 * 190;
            fill(
                dc,
                RECT {
                    left: x,
                    top: y,
                    right: x + 290,
                    bottom: y + 160,
                },
                if panel.fields[0] == entry.key {
                    0x484028
                } else {
                    0x303030
                },
            );
            label(
                dc,
                x + 5,
                y + 5,
                &entry.name.chars().take(34).collect::<String>(),
            );
            label(
                dc,
                x + 5,
                y + 27,
                &format!(
                    "{} • {} kills • {} points",
                    entry.category_name(),
                    entry.kills,
                    entry.earned_points()
                ),
            );
            if let Some(image) = portraits.image(entry.look, entry.object) {
                let mut bitmap: BITMAPINFO = std::mem::zeroed();
                bitmap.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
                bitmap.bmiHeader.biWidth = image.width as i32;
                bitmap.bmiHeader.biHeight = -(image.height as i32);
                bitmap.bmiHeader.biPlanes = 1;
                bitmap.bmiHeader.biBitCount = 32;
                bitmap.bmiHeader.biCompression = BI_RGB;
                let scale = image.width.max(image.height).max(1);
                StretchDIBits(
                    dc,
                    x + 5,
                    y + 50,
                    (96 * image.width / scale) as i32,
                    (96 * image.height / scale) as i32,
                    0,
                    0,
                    image.width as i32,
                    image.height as i32,
                    image.bgra.as_ptr().cast(),
                    &bitmap,
                    DIB_RGB_COLORS,
                    SRCCOPY,
                );
            } else {
                label(dc, x + 5, y + 70, "Art unavailable");
            }
            for (stage, name) in ["Prowess", "Expertise", "Mastery"].iter().enumerate() {
                label(
                    dc,
                    x + 108,
                    y + 53 + stage as i32 * 30,
                    &format!(
                        "{name}: {}/{}",
                        entry.kills.min(entry.thresholds[stage]),
                        entry.thresholds[stage]
                    ),
                );
            }
        }
        if cards.is_empty() {
            label(
                dc,
                20,
                100,
                "No matching bosses on this page. Browse or continue to the next page.",
            );
        }
        ReleaseDC(hwnd, dc);
    }
}

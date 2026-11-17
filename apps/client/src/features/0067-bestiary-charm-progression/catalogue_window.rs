//! Creature catalogue and art use the same confirmed server card.
#![allow(
    unsafe_code,
    reason = "Win32 handles and DIB memory stay on their owning thread"
)]
use crate::play::{PlayLink, mailbox::lock};
use std::ptr::{null, null_mut};
use windows_sys::Win32::System::SystemServices::SS_BITMAP;
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};
#[path = "catalogue_pixels.rs"]
mod pixels;
#[derive(Default)]
struct Actions(Vec<usize>);
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub(super) fn open(link: &PlayLink) {
    {
        let mut m = lock(&link.mailbox);
        let p = &mut m.properties.charm_progression;
        if !p.selected || p.catalogue.open {
            return;
        }
        p.catalogue.open = true;
    }
    let weak = std::sync::Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("creature-catalogue".into())
        .spawn(move || {
            // SAFETY: the native window, userdata and bitmaps are confined to this thread.
            unsafe {
                let h = CreateWindowExW(
                    0,
                    wide("STATIC").as_ptr(),
                    wide("Creature catalogue").as_ptr(),
                    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    850,
                    760,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null(),
                );
                if !h.is_null() {
                    let mut actions = Box::<Actions>::default();
                    SetWindowLongPtrW(h, GWLP_USERDATA, (&mut *actions as *mut Actions) as isize);
                    SetWindowLongPtrW(h, GWLP_WNDPROC, procedure as *const () as isize);
                    let image = control(h, "STATIC", "", 90, [700, 15, 96, 96], SS_BITMAP);
                    let art_status = control(h, "STATIC", "", 91, [680, 115, 135, 55], 0);
                    let echo_progress = controls::EchoProgress::new(h);
                    let category = control(
                        h,
                        "COMBOBOX",
                        "",
                        92,
                        [10, 10, 320, 400],
                        CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
                    );
                    let progress_filter = control(
                        h,
                        "COMBOBOX",
                        "",
                        96,
                        [340, 10, 330, 160],
                        CBS_DROPDOWNLIST as u32,
                    );
                    for label in ["All discoveries", "In progress", "Complete"] {
                        SendMessageW(
                            progress_filter,
                            CB_ADDSTRING,
                            0,
                            wide(label).as_ptr() as isize,
                        );
                    }
                    SendMessageW(progress_filter, CB_SETCURSEL, 0, 0);
                    let list = control(
                        h,
                        "LISTBOX",
                        "",
                        93,
                        [10, 55, 320, 540],
                        WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
                    );
                    let detail = control(
                        h,
                        "EDIT",
                        "",
                        94,
                        [340, 180, 475, 430],
                        WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
                    );
                    let status = control(h, "STATIC", "", 95, [10, 650, 800, 55], 0);
                    for (id, label, x) in [
                        (1, "Browse", 10),
                        (2, "Next page", 150),
                        (3, "Details", 290),
                        (4, "More loot", 430),
                    ] {
                        control(h, "BUTTON", label, id, [x, 610, 130, 30], WS_TABSTOP);
                    }
                    let mut art = pixels::Art::load();
                    let mut bitmap_handle: HBITMAP = null_mut();
                    let mut painted = None;
                    let mut page_seen = None;
                    let mut displayed: Vec<u16> = Vec::new();
                    let mut category_seen = None;
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
                        let p = {
                            let mut m = lock(&shared);
                            let p = &mut m.properties.charm_progression;
                            if !p.selected || !p.catalogue.open {
                                break;
                            }
                            let selected = SendMessageW(list, LB_GETCURSEL, 0, 0);
                            if let Ok(index) = usize::try_from(selected)
                                && let Some(race_id) = displayed.get(index)
                                && p.catalogue
                                    .page
                                    .cards
                                    .iter()
                                    .chain(&p.selector.races)
                                    .any(|c| c.race == *race_id)
                                && !p.pending
                            {
                                let race = race_id.to_string();
                                if p.fields[1] != race {
                                    p.fields[1] = race;
                                    p.catalogue.detail.clear();
                                    p.catalogue.detail_card = None;
                                    p.catalogue.next_loot = None;
                                }
                            }
                            use oteryn_client_ui::charm_progression::RaceFilter;
                            p.race_filter = match SendMessageW(progress_filter, CB_GETCURSEL, 0, 0)
                            {
                                1 => RaceFilter::InProgress,
                                2 => RaceFilter::Complete,
                                _ => RaceFilter::All,
                            };
                            let selected_category = SendMessageW(category, CB_GETCURSEL, 0, 0);
                            let mut requested = actions.0.drain(..).find_map(|id| match id {
                                1 => Some(0),
                                2 => Some(1),
                                3 => Some(2),
                                4 => Some(3),
                                _ => None,
                            });
                            if let Ok(index) = u8::try_from(selected_category)
                                && index != p.catalogue.category
                                && !p.pending
                            {
                                p.catalogue.category = index;
                                requested = Some(0);
                            }
                            if !initialized && !p.pending {
                                requested = Some(0);
                                initialized = true;
                            }
                            if let Some(action) = requested {
                                super::super::input::catalogue::submit(&mut m, &commands, action);
                            }
                            m.properties.charm_progression.clone()
                        };
                        echo_progress.refresh(p.catalogue.echo_warden_progress(&p.fields[1]));
                        if category_seen.as_ref() != Some(&p.catalogue.page.categories) {
                            SendMessageW(category, CB_RESETCONTENT, 0, 0);
                            for name in std::iter::once("All categories")
                                .chain(p.catalogue.page.categories.iter().map(String::as_str))
                            {
                                SendMessageW(
                                    category,
                                    CB_ADDSTRING,
                                    0,
                                    wide(name).as_ptr() as isize,
                                );
                            }
                            SendMessageW(category, CB_SETCURSEL, p.catalogue.category.into(), 0);
                            category_seen = Some(p.catalogue.page.categories.clone());
                        }
                        let rows: Vec<_> = p
                            .catalogue_rows()
                            .iter()
                            .map(|card| (card.race, p.catalogue_label(card)))
                            .collect();
                        if page_seen.as_ref() != Some(&rows) {
                            SendMessageW(list, LB_RESETCONTENT, 0, 0);
                            for (_, label) in &rows {
                                SendMessageW(list, LB_ADDSTRING, 0, wide(label).as_ptr() as isize);
                            }
                            if let Some(index) = rows
                                .iter()
                                .position(|(id, _)| id.to_string() == p.fields[1])
                            {
                                SendMessageW(list, LB_SETCURSEL, index, 0);
                            }
                            displayed = rows.iter().map(|(id, _)| *id).collect();
                            page_seen = Some(rows);
                        }
                        let card = p.catalogue.selected_card(&p.fields[1]).or_else(|| {
                            p.selector
                                .races
                                .iter()
                                .find(|c| c.race.to_string() == p.fields[1])
                        });
                        let appearance = card.map(|c| (c.outfit_look_type, c.object_look_type));
                        if painted != Some(appearance) {
                            SendMessageW(image, STM_SETIMAGE, IMAGE_BITMAP as usize, 0);
                            if !bitmap_handle.is_null() {
                                DeleteObject(bitmap_handle);
                                bitmap_handle = null_mut();
                            }
                            let result = appearance
                                .ok_or_else(|| "Select a creature".to_owned())
                                .and_then(|(outfit, object)| {
                                    art.as_mut()
                                        .map_err(|e| e.clone())?
                                        .pixels(object.unwrap_or(outfit), object.is_some())
                                });
                            let note = match result {
                                Ok(rgba) => {
                                    bitmap_handle = bitmap(&rgba);
                                    if bitmap_handle.is_null() {
                                        "Could not create preview"
                                    } else {
                                        SendMessageW(
                                            image,
                                            STM_SETIMAGE,
                                            IMAGE_BITMAP as usize,
                                            bitmap_handle as isize,
                                        );
                                        ""
                                    }
                                }
                                Err(ref e) => {
                                    SetWindowTextW(art_status, wide(e).as_ptr());
                                    painted = Some(appearance);
                                    continue;
                                }
                            };
                            SetWindowTextW(art_status, wide(note).as_ptr());
                            painted = Some(appearance);
                        }
                        SetWindowTextW(
                            detail,
                            wide(&format!(
                                "{}\r\n{}",
                                oteryn_client_ui::charm_progression::bestiary_detail_text(&p),
                                p.catalogue.detail
                            ))
                            .as_ptr(),
                        );
                        SetWindowTextW(
                            status,
                            wide(&format!(
                                "{}{}",
                                if p.pending { "Loading… " } else { "" },
                                p.feedback
                            ))
                            .as_ptr(),
                        );
                        std::thread::sleep(std::time::Duration::from_millis(30));
                    }
                    DestroyWindow(h);
                    if !bitmap_handle.is_null() {
                        DeleteObject(bitmap_handle);
                    }
                }
            }
            if let Some(shared) = weak.upgrade() {
                lock(&shared).properties.charm_progression.catalogue.open = false;
            }
        })
        .is_err()
        && let Some(shared) = failed.upgrade()
    {
        lock(&shared).properties.charm_progression.catalogue.open = false;
    }
}

#[path = "catalogue_controls.rs"]
mod controls;
use controls::{bitmap, control, procedure};

//! Native inventory list. Window handles stay on their creating thread.
#![allow(unsafe_code)]
use crate::play::{PlayLink, feature_0014_container_tree as containers, mailbox::lock};
use std::ptr::{null, null_mut};
use std::sync::Arc;
use std::time::Duration;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: userdata points to the live selection flag on this window's thread.
    unsafe {
        if msg == WM_COMMAND
            && (matches!(w & 0xffff, 102..=104)
                || w & 0xffff == 101 && (w >> 16) & 0xffff == LBN_SELCHANGE as usize)
        {
            let flag = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut u8;
            if !flag.is_null() {
                *flag = match w & 0xffff {
                    102 => 2,
                    103 => 3,
                    104 => 4,
                    _ => 1,
                };
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.inventory.open || mailbox.inventory_shutdown {
            return;
        }
        mailbox.inventory.open = true;
    }
    let commands = link.commands.clone();
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    if std::thread::Builder::new()
        .name("inventory-panel".into())
        .spawn(move || {
            // SAFETY: all window operations, dispatch and userdata access occur on this thread.
            unsafe {
                let hwnd = CreateWindowExW(
                    0,
                    wide("STATIC").as_ptr(),
                    wide(tr!(
                        "Inventory and equipment — drag items to equip or remove"
                    ))
                    .as_ptr(),
                    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    750,
                    520,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null(),
                );
                if !hwnd.is_null() {
                    let mut changed = Box::new(0_u8);
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *changed as *mut u8) as isize);
                    SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
                    let list = CreateWindowExW(
                        0,
                        wide("LISTBOX").as_ptr(),
                        wide("").as_ptr(),
                        WS_CHILD
                            | WS_VISIBLE
                            | WS_BORDER
                            | WS_VSCROLL
                            | WS_TABSTOP
                            | LBS_NOTIFY as u32,
                        10,
                        10,
                        420,
                        280,
                        hwnd,
                        101_usize as _,
                        null_mut(),
                        null(),
                    );
                    CreateWindowExW(
                        0,
                        wide("BUTTON").as_ptr(),
                        wide(tr!("Open container")).as_ptr(),
                        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                        10,
                        364,
                        160,
                        30,
                        hwnd,
                        104_usize as _,
                        null_mut(),
                        null(),
                    );
                    CreateWindowExW(
                        0,
                        wide("BUTTON").as_ptr(),
                        wide(tr!("Use with / Choose target")).as_ptr(),
                        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                        10,
                        300,
                        420,
                        28,
                        hwnd,
                        102_usize as _,
                        null_mut(),
                        null(),
                    );
                    CreateWindowExW(
                        0,
                        wide("BUTTON").as_ptr(),
                        wide(tr!("Use rune")).as_ptr(),
                        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                        10,
                        332,
                        420,
                        28,
                        hwnd,
                        103_usize as _,
                        null_mut(),
                        null(),
                    );
                    let status = CreateWindowExW(
                        0,
                        wide("STATIC").as_ptr(),
                        wide("").as_ptr(),
                        WS_CHILD | WS_VISIBLE,
                        10,
                        402,
                        420,
                        70,
                        hwnd,
                        null_mut(),
                        null_mut(),
                        null(),
                    );
                    let mut equipment =
                        crate::play::feature_0012_item_equip_drop::view::Slots::create(hwnd);
                    let mut held: Option<oteryn_client_ui::inventory::InventoryPanel> = None;
                    let mut status_text = String::new();
                    let mut message: MSG = std::mem::zeroed();
                    while !list.is_null() && !status.is_null() && IsWindow(hwnd) != 0 {
                        while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                            if message.message == WM_KEYDOWN
                                && message.wParam == 27
                                && let Some(mailbox) = weak.upgrade()
                            {
                                lock(&mailbox).world_context.cancel_targeting();
                                continue;
                            }
                            if IsDialogMessageW(hwnd, &message) == 0 {
                                TranslateMessage(&message);
                                DispatchMessageW(&message);
                            }
                        }
                        if IsWindow(hwnd) == 0 {
                            break;
                        }
                        let Some(mailbox) = weak.upgrade() else {
                            break;
                        };
                        let shared = mailbox.clone();
                        let mut mailbox = lock(&shared);
                        if mailbox.inventory_shutdown {
                            break;
                        }
                        let action = std::mem::take(&mut *changed);
                        let mut show_containers = false;
                        let mut selected = false;
                        if action != 0 {
                            // Resolve the displayed row and validate its current handle.
                            // A concurrent removal cannot select another item.
                            let row = SendMessageW(list, LB_GETCURSEL, 0, 0);
                            if let Some(item) = usize::try_from(row)
                                .ok()
                                .and_then(|i| held.as_ref()?.rows().get(i).copied())
                                && mailbox.inventory.select(item.handle)
                            {
                                if action == 4 {
                                    show_containers = containers::input::inventory_native_open(
                                        &mut mailbox,
                                        &commands,
                                        item.handle,
                                    );
                                }
                                selected = mailbox.inventory.select(item.handle);
                            }
                        }
                        if matches!(action, 2 | 3) && selected {
                            drop(mailbox);
                            PlayLink::inventory_use_shared(&shared, &commands, action == 3);
                            mailbox = lock(&shared);
                        }
                        let panel = &mailbox.inventory;
                        let feedback = mailbox
                            .world_context
                            .feedback(std::time::Instant::now())
                            .unwrap_or_default();
                        let text = format!("{}\n{}", panel.text(), feedback);
                        if status_text != text {
                            SetWindowTextW(status, wide(&text).as_ptr());
                            status_text = text;
                        }
                        if held.as_ref() != Some(panel) {
                            SendMessageW(list, LB_RESETCONTENT, 0, 0);
                            for (i, item) in panel.rows().iter().enumerate() {
                                let label = format!(
                                    "{}{} ×{}",
                                    if i == 0 {
                                        tr!("Backpack: ")
                                    } else {
                                        tr!("Item: ")
                                    },
                                    item.item_definition_ref,
                                    item.count
                                );
                                SendMessageW(list, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                                if panel
                                    .selected_item()
                                    .is_some_and(|selected| selected.handle == item.handle)
                                {
                                    SendMessageW(list, LB_SETCURSEL, i, 0);
                                }
                            }
                            held = Some(panel.clone());
                        }
                        super::list_surface(
                            &mut mailbox,
                            list,
                            held.as_ref().map(|p| p.rows()).as_deref().unwrap_or(&[]),
                            oteryn_session::ItemMoveDestination::MainBackpack,
                        );
                        equipment.refresh(&mut mailbox);
                        drop(mailbox);
                        if show_containers {
                            crate::play::feature_0014_container_tree::view::open_shared(
                                weak.clone(),
                                commands.clone(),
                            );
                        }
                        std::thread::sleep(Duration::from_millis(30));
                    }
                    if IsWindow(hwnd) != 0 {
                        DestroyWindow(hwnd);
                    }
                }
            }
            if let Some(mailbox) = weak.upgrade() {
                lock(&mailbox).inventory.open = false;
            }
        })
        .is_err()
        && let Some(mailbox) = failed.upgrade()
    {
        lock(&mailbox).inventory.open = false;
    }
}

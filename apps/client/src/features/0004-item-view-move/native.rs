//! Native inventory list. Window handles stay on their creating thread.
#![allow(unsafe_code)]
use crate::play::{PlayLink, mailbox::lock};
use std::ptr::{null, null_mut};
use std::sync::Arc;
use std::time::Duration;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: userdata points to the live selection flag on this window's thread.
    unsafe {
        if msg == WM_COMMAND && w & 0xffff == 102 {
            let flag = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut u8;
            if !flag.is_null() {
                *flag = 2;
            }
            return 0;
        }
        if msg == WM_COMMAND && w & 0xffff == 101 && (w >> 16) & 0xffff == LBN_SELCHANGE as usize {
            let flag = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut u8;
            if !flag.is_null() {
                *flag = 1;
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
                    wide("Inventory — backpack and items").as_ptr(),
                    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    460,
                    420,
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
                    CreateWindowExW(0, wide("BUTTON").as_ptr(), wide("Open container").as_ptr(), WS_CHILD | WS_VISIBLE | WS_TABSTOP, 10, 295, 160, 30, hwnd, 102_usize as _, null_mut(), null());
                    let status = CreateWindowExW(
                        0,
                        wide("STATIC").as_ptr(),
                        wide("").as_ptr(),
                        WS_CHILD | WS_VISIBLE,
                        10,
                        330,
                        420,
                        70,
                        hwnd,
                        null_mut(),
                        null_mut(),
                        null(),
                    );
                    let mut held: Option<oteryn_client_ui::inventory::InventoryPanel> = None;
                    let mut message: MSG = std::mem::zeroed();
                    while !list.is_null() && !status.is_null() && IsWindow(hwnd) != 0 {
                        while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
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
                        let mut mailbox = lock(&mailbox);
                        if mailbox.inventory_shutdown {
                            break;
                        }
                        let action = std::mem::take(&mut *changed);
                        if action != 0 {
                            // Resolve the row against the displayed snapshot, then validate its handle
                            // against current state. A concurrent removal cannot select another item.
                            let row = SendMessageW(list, LB_GETCURSEL, 0, 0);
                            if let Some(item) = usize::try_from(row)
                                .ok()
                                .and_then(|i| held.as_ref()?.rows().get(i).copied())
                            {
                                if mailbox.inventory.select(item.handle) && action == 2 {
                                    super::super::super::feature_0014_container_tree::input::inventory_native_open(&mut mailbox, &commands, item.handle);
                                }
                            }
                        }
                        let panel = &mailbox.inventory;
                        if held.as_ref() != Some(panel) {
                            SendMessageW(list, LB_RESETCONTENT, 0, 0);
                            for (i, item) in panel.rows().iter().enumerate() {
                                let label = format!(
                                    "{}{} ×{}",
                                    if i == 0 { "Backpack: " } else { "Item: " },
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
                            SetWindowTextW(status, wide(&panel.text()).as_ptr());
                            held = Some(panel.clone());
                        }
                        let show_containers = action == 2;
                        drop(mailbox);
                        if show_containers {
                            super::super::super::feature_0014_container_tree::view::open_shared(weak.clone(), commands.clone());
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

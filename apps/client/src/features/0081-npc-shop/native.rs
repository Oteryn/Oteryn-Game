//! Native NPC shop controls are intentionally limited to the two wire actions.
#![allow(unsafe_code)]
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::npc_shop::{Intent, panel_text};
use std::ptr::{null, null_mut};
use windows_sys::Win32::UI::WindowsAndMessaging::*;
fn wide(value: &str) -> Vec<u16> {
    value
        .replace('\0', "�")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
pub(super) fn open_window(
    link: &PlayLink,
    offer: oteryn_client_runtime::npc_shop::npc_shop::NpcShopIntent,
) {
    if lock(&link.mailbox).npc_shop.open {
        return;
    }
    lock(&link.mailbox).npc_shop.open = true;
    let link = PlayLink {
        commands: link.commands.clone(),
        mailbox: link.mailbox.clone(),
    };
    let _ = std::thread::Builder::new()
        .name("npc-shop-panel".into())
        .spawn(move || unsafe {
            let hwnd = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("NPC shop").as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                460,
                300,
                null_mut(),
                null_mut(),
                null_mut(),
                null(),
            );
            if hwnd.is_null() {
                lock(&link.mailbox).npc_shop.open = false;
                return;
            }
            let list = CreateWindowExW(
                0,
                wide("LISTBOX").as_ptr(),
                null(),
                WS_CHILD | WS_VISIBLE | WS_BORDER,
                10,
                10,
                420,
                120,
                hwnd,
                null_mut(),
                null_mut(),
                null(),
            );
            SendMessageW(
                list,
                LB_ADDSTRING,
                0,
                wide(&format!(
                    "Offer {} — {:?} — {} gold",
                    offer.offer_index, offer.side, offer.expected_unit_price
                ))
                .as_ptr() as isize,
            );
            let look = CreateWindowExW(
                0,
                wide("BUTTON").as_ptr(),
                wide("Look").as_ptr(),
                WS_CHILD | WS_VISIBLE,
                10,
                140,
                120,
                30,
                hwnd,
                1 as _,
                null_mut(),
                null(),
            );
            let sell = CreateWindowExW(
                0,
                wide("BUTTON").as_ptr(),
                wide("Sell all").as_ptr(),
                WS_CHILD | WS_VISIBLE,
                140,
                140,
                120,
                30,
                hwnd,
                2 as _,
                null_mut(),
                null(),
            );
            let status = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide(&panel_text(&link.npc_shop_panel())).as_ptr(),
                WS_CHILD | WS_VISIBLE,
                10,
                180,
                420,
                60,
                hwnd,
                null_mut(),
                null_mut(),
                null(),
            );
            let mut msg: MSG = std::mem::zeroed();
            while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
                if msg.message == WM_COMMAND && msg.hwnd == hwnd {
                    match msg.wParam & 0xffff {
                        1 => link.npc_shop_request(Intent::Look(offer)),
                        2 => link.npc_shop_request(Intent::SellAll(offer)),
                        _ => {}
                    }
                }
                SetWindowTextW(status, wide(&panel_text(&link.npc_shop_panel())).as_ptr());
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
                if IsWindow(hwnd) == 0 {
                    break;
                }
            }
            let _ = (look, sell);
            lock(&link.mailbox).npc_shop.open = false;
        });
}

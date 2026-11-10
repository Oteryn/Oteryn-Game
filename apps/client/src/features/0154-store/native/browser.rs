//! Reference Store layout: category sidebar, product list, preview/detail, purchase and footer.
#![allow(
    unsafe_code,
    reason = "Native handles and callback userdata remain on the window thread"
)]
use super::{StorePanel, store};
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_runtime::cui3::store::browse::Page;
use std::{
    ptr::{null, null_mut},
    sync::Arc,
};
use windows_sys::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
};
#[path = "controls.rs"]
mod controls;
use controls::wide;
#[path = "commands.rs"]
mod commands;
#[path = "layout.rs"]
mod layout;
#[derive(Default)]
struct Actions(Vec<(usize, usize)>);
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND {
            let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !p.is_null() && (*p).0.len() < 32 {
                (*p).0.push((w & 65535, (w >> 16) & 65535));
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut m = lock(&link.mailbox);
        if m.store.open {
            return;
        }
        m.store.open = true;
    }
    link.store_browse("featured", "", 0, 0);
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("store-browser".into())
        .spawn(move || {
            let Some(shared) = weak.upgrade() else {
                return;
            };
            let link = PlayLink {
                owns_session: false,
                mailbox: shared,
                commands,
            };
            unsafe {
                run(&link);
            }
            lock(&link.mailbox).store.open = false;
        })
        .is_err()
        && let Some(shared) = rollback.upgrade()
    {
        lock(&shared).store.open = false;
    }
}
unsafe fn run(link: &PlayLink) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("Store")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1040,
            780,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            actions.as_mut() as *mut Actions as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        let layout::Layout {
            category,
            search,
            sort,
            refresh,
            offers,
            image,
            detail,
            buy,
            previous,
            next,
            name,
            sex,
            identity,
            balance,
            status,
        } = layout::create(hwnd);
        let root = std::env::var_os(crate::map::ASSET_DIR_ENV)
            .map(std::path::PathBuf::from)
            .or_else(|| std::env::current_dir().ok());
        let mut assets = root
            .as_deref()
            .ok_or_else(|| "Artwork unavailable".to_owned())
            .and_then(super::assets::Assets::load);
        let mut held: Option<Page> = None;
        let mut displayed_quote = None;
        let mut selected = usize::MAX;
        let mut signature = None;
        let mut pixels = None;
        let mut message: MSG = std::mem::zeroed();
        while IsWindow(hwnd) != 0 && !link.commands.is_closed() {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            commands::handle(
                link,
                hwnd,
                [category, search, sort, offers, name, sex, identity],
                std::mem::take(&mut actions.0),
                displayed_quote.as_ref(),
            );
            let panel: StorePanel = link.store_panel();
            if held != panel.browser.page {
                SendMessageW(category, LB_RESETCONTENT, 0, 0);
                SendMessageW(offers, LB_RESETCONTENT, 0, 0);
                if let Some(page) = &panel.browser.page {
                    for (i, c) in page.categories.iter().enumerate() {
                        let label =
                            format!("{}{}", "    ".repeat(c.key.matches('.').count()), c.title);
                        SendMessageW(category, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                        if c.key == panel.browser.category {
                            SendMessageW(category, LB_SETCURSEL, i, 0);
                        }
                    }
                    for p in &page.products {
                        let label = format!(
                            "{}  —  {} {}",
                            p.offer.title, p.offer.price, p.offer.currency
                        );
                        SendMessageW(offers, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                    }
                    SetWindowTextW(
                        balance,
                        wide(&format!(
                            "Available: {} coins\r\nTransferable: {}",
                            page.total,
                            if page.transferable_known {
                                page.transferable.to_string()
                            } else {
                                "unavailable".into()
                            }
                        ))
                        .as_ptr(),
                    );
                }
                held = panel.browser.page.clone();
                selected = usize::MAX;
            }
            if selected != panel.browser.selected {
                SendMessageW(offers, LB_SETCURSEL, panel.browser.selected, 0);
                selected = panel.browser.selected;
            }
            let product = panel
                .browser
                .page
                .as_ref()
                .and_then(|p| p.products.get(panel.browser.selected));
            let current =
                product.map(|p| (p.artwork, panel.browser.gender, panel.browser.direction));
            if signature != current {
                signature = current;
                pixels = None;
                if let Some((art, gender, dir)) = current {
                    let rendered = assets
                        .as_mut()
                        .map_err(|e| e.clone())
                        .and_then(|a| a.render(art, gender, dir));
                    match rendered {
                        Ok(mut rgba) => {
                            for p in rgba.as_chunks_mut::<4>().0 {
                                p.swap(0, 2);
                            }
                            pixels = Some(rgba);
                            SetWindowTextW(image, wide("").as_ptr());
                        }
                        Err(e) => {
                            SetWindowTextW(image, wide(&e).as_ptr());
                        }
                    }
                }
            }
            SetWindowTextW(
                detail,
                wide(&product.map_or_else(
                    || "No matching products".into(),
                    |p| {
                        format!(
                            "{}\r\n{} {}\r\n{}\r\nRevision: {}",
                            p.offer.title, p.offer.price, p.offer.currency, p.category, p.revision
                        )
                    },
                ))
                .as_ptr(),
            );
            SetWindowTextW(
                status,
                wide(&format!(
                    "{}\r\n{}",
                    super::panel_text(&panel),
                    store::coin_history_text(&panel.coin_history)
                ))
                .as_ptr(),
            );
            for h in [category, search, sort, refresh, previous, next, buy] {
                EnableWindow(h, i32::from(!panel.pending));
            }
            SetWindowTextW(
                buy,
                wide(if panel.confirmed_purchase.is_some() {
                    "Retry original purchase…"
                } else {
                    "Buy selected product…"
                })
                .as_ptr(),
            );
            EnableWindow(
                previous,
                i32::from(!panel.pending && panel.browser.cursors.len() > 1),
            );
            EnableWindow(
                next,
                i32::from(
                    !panel.pending
                        && panel
                            .browser
                            .page
                            .as_ref()
                            .is_some_and(|p| !p.next.is_empty()),
                ),
            );
            EnableWindow(
                buy,
                i32::from(
                    !panel.pending && (product.is_some() || panel.confirmed_purchase.is_some()),
                ),
            );
            if let Some(pixels) = &pixels {
                controls::paint(image, pixels);
            }
            displayed_quote = panel.catalogue.clone().filter(|quote| {
                panel.confirmed_purchase.is_some()
                    || product.is_some_and(|product| {
                        quote.catalog_revision == product.revision
                            && quote.offers.first() == Some(&product.offer)
                    })
            });
            EnableWindow(buy, i32::from(!panel.pending && displayed_quote.is_some()));
            MsgWaitForMultipleObjects(0, null(), 0, 30, QS_ALLINPUT);
        }
        DestroyWindow(hwnd);
    }
}

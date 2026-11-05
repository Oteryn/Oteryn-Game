//! Native catalogue and confirmed purchase presentation.
use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
use oteryn_client_ui::store::{self, StorePanel};

pub(super) fn purchase(
    panel: &mut StorePanel,
    selection: usize,
    confirmed: bool,
) -> Option<ServerMechanicCommand> {
    if panel.pending || !panel.selected {
        return None;
    }
    panel.selected_offer = selection;
    panel.purchase_confirmed = confirmed;
    if confirmed {
        // A fresh local transaction identity for each explicit confirmation.
        use sha2::{Digest, Sha256};
        use std::sync::atomic::{AtomicU64, Ordering};
        static SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?;
        let mut hash = Sha256::new();
        hash.update(now.as_nanos().to_be_bytes());
        hash.update(std::process::id().to_be_bytes());
        hash.update(SEQUENCE.fetch_add(1, Ordering::Relaxed).to_be_bytes());
        panel
            .purchase_request_id
            .copy_from_slice(&hash.finalize()[..16]);
        panel.purchase_request_id[..6].copy_from_slice(&now.as_millis().to_be_bytes()[10..]);
        panel.purchase_request_id[6] = (panel.purchase_request_id[6] & 15) | 0x70;
        panel.purchase_request_id[8] = (panel.purchase_request_id[8] & 63) | 0x80;
    }
    store::purchase(panel)
}

pub(super) fn panel_text(panel: &StorePanel) -> String {
    if !panel.selected {
        return "Store unavailable in this session".into();
    }
    if panel.pending {
        return "Pending store action…".into();
    }
    if let Some(feedback) = &panel.feedback {
        return feedback.clone();
    }
    if let Some(result) = &panel.purchase {
        return format!("Purchase: {:?}", result.code);
    }
    panel.catalogue.as_ref().map_or_else(
        || "Load the server catalogue".into(),
        |page| {
            format!(
                "Balance: {} coins; transferable: {}",
                page.total_coins, page.transferable_coins
            )
        },
    )
}

#[cfg(windows)]
pub(in crate::play) use windows::open_window;

#[cfg(windows)]
mod windows {
    // Win32 handles and callback userdata stay on the creating thread.
    #![allow(unsafe_code)]
    use super::*;
    use crate::play::{PlayLink, mailbox::lock};
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    fn wide(text: &str) -> Vec<u16> {
        text.replace('\0', "\u{fffd}")
            .encode_utf16()
            .chain(Some(0))
            .collect()
    }

    unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        unsafe {
            if msg == WM_COMMAND {
                let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Vec<usize>;
                if !actions.is_null() && (*actions).len() < 32 {
                    (*actions).push(w & 0xffff);
                }
                return 0;
            }
            DefWindowProcW(hwnd, msg, w, l)
        }
    }

    unsafe fn control(
        parent: HWND,
        class: &str,
        text: &str,
        id: usize,
        y: i32,
        height: i32,
        style: u32,
    ) -> HWND {
        unsafe {
            CreateWindowExW(
                0,
                wide(class).as_ptr(),
                wide(text).as_ptr(),
                WS_CHILD | WS_VISIBLE | style,
                10,
                y,
                440,
                height,
                parent,
                id as _,
                null_mut(),
                null(),
            )
        }
    }

    pub(in crate::play) fn open_window(link: &PlayLink) {
        {
            let mut mailbox = lock(&link.mailbox);
            if mailbox.store.open {
                return;
            }
            mailbox.store.open = true;
        }
        link.discovery_open(285);
        let weak = std::sync::Arc::downgrade(&link.mailbox);
        let rollback = weak.clone();
        let commands = link.commands.clone();
        if std::thread::Builder::new()
            .name("store-panel".into())
            .spawn(move || {
                unsafe {
                    let hwnd = CreateWindowExW(
                        0,
                        wide("STATIC").as_ptr(),
                        wide("Store").as_ptr(),
                        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                        CW_USEDEFAULT,
                        CW_USEDEFAULT,
                        480,
                        420,
                        null_mut(),
                        null_mut(),
                        null_mut(),
                        null(),
                    );
                    if !hwnd.is_null() {
                        let mut actions: Vec<usize> = Vec::new();
                        SetWindowLongPtrW(hwnd, GWLP_USERDATA, &mut actions as *mut _ as isize);
                        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
                        let category = control(
                            hwnd,
                            "EDIT",
                            "featured",
                            1,
                            10,
                            25,
                            WS_BORDER | ES_AUTOHSCROLL as u32,
                        );
                        let refresh = control(hwnd, "BUTTON", "Load catalogue", 2, 45, 30, 0);
                        let offers = control(
                            hwnd,
                            "LISTBOX",
                            "",
                            3,
                            85,
                            135,
                            WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
                        );
                        let buy =
                            control(hwnd, "BUTTON", "Purchase selected offer…", 4, 235, 35, 0);
                        let mut displayed = None;
                        let status = control(hwnd, "STATIC", "", 0, 310, 65, 0);
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
                            let mut mailbox = lock(&shared);
                            for action in std::mem::take(&mut actions) {
                                let panel = &mut mailbox.store;
                                let command = {
                                    match action {
                                        2 if !panel.pending => {
                                            let mut text = [0u16; 256];
                                            let len = GetWindowTextW(
                                                category,
                                                text.as_mut_ptr(),
                                                text.len() as i32,
                                            );
                                            panel.category =
                                                String::from_utf16_lossy(&text[..len as usize]);
                                            store::catalogue(panel)
                                        }
                                        4 if !panel.pending && panel.selected => {
                                            let index = SendMessageW(offers, LB_GETCURSEL, 0, 0);
                                            if let Ok(index) = usize::try_from(index)
                                                && let Some(offer) = panel
                                                    .catalogue
                                                    .as_ref()
                                                    .and_then(|page| page.offers.get(index))
                                            {
                                                let question = format!(
                                                    "Purchase {} for {} {}?",
                                                    offer.title, offer.price, offer.currency
                                                );
                                                // Do not hold the mailbox while the modal dialog pumps messages.
                                                let snapshot = panel.catalogue.clone();
                                                drop(mailbox);
                                                let confirmed = MessageBoxW(
                                                    hwnd,
                                                    wide(&question).as_ptr(),
                                                    wide("Confirm purchase").as_ptr(),
                                                    MB_YESNO | MB_DEFBUTTON2,
                                                ) == IDYES;
                                                mailbox = lock(&shared);
                                                let panel = &mut mailbox.store;
                                                if panel.catalogue == snapshot {
                                                    purchase(panel, index, confirmed)
                                                } else {
                                                    None
                                                }
                                            } else {
                                                panel.feedback = Some(
                                                    "Select an offer from the server catalogue"
                                                        .into(),
                                                );
                                                None
                                            }
                                        }
                                        _ => None,
                                    }
                                };
                                if let Some(command) = command
                                    && commands
                                        .send(crate::combat_input::PlayCommand::ServerMechanic(
                                            command,
                                        ))
                                        .is_err()
                                {
                                    mailbox.store.pending = false;
                                    mailbox.store.feedback = Some("The session has ended".into());
                                }
                            }
                            let panel = &mut mailbox.store;
                            if displayed != panel.catalogue {
                                SendMessageW(offers, LB_RESETCONTENT, 0, 0);
                                if let Some(page) = &panel.catalogue {
                                    for offer in &page.offers {
                                        let text = format!(
                                            "{} — {} {}",
                                            offer.title, offer.price, offer.currency
                                        );
                                        SendMessageW(
                                            offers,
                                            LB_ADDSTRING,
                                            0,
                                            wide(&text).as_ptr() as isize,
                                        );
                                    }
                                }
                                displayed = panel.catalogue.clone();
                            }
                            EnableWindow(refresh, i32::from(panel.selected && !panel.pending));
                            EnableWindow(
                                buy,
                                i32::from(
                                    panel.selected
                                        && !panel.pending
                                        && panel
                                            .catalogue
                                            .as_ref()
                                            .is_some_and(|page| !page.offers.is_empty()),
                                ),
                            );
                            SetWindowTextW(status, wide(&panel_text(panel)).as_ptr());
                            drop(mailbox);
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        if IsWindow(hwnd) != 0 {
                            DestroyWindow(hwnd);
                        }
                    }
                }
                if let Some(shared) = weak.upgrade() {
                    lock(&shared).store.open = false;
                }
            })
            .is_err()
            && let Some(shared) = rollback.upgrade()
        {
            lock(&shared).store.open = false;
        }
    }
}

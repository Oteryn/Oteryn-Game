//! Native catalogue and confirmed purchase presentation.
use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
use oteryn_client_ui::store::{self, StorePanel};

pub(in crate::play) fn purchase(
    panel: &mut StorePanel,
    selection: usize,
    confirmed: bool,
) -> Option<ServerMechanicCommand> {
    if panel.pending || !panel.selected {
        return None;
    }
    panel.selected_offer = selection;
    panel.purchase_confirmed = confirmed;
    if confirmed && panel.confirmed_purchase.is_none() {
        // CLIGAP-W09 accepted transport difference: retries reuse the confirmed
        // Game transaction and quote until a decoded terminal result.
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

// Game identities replace Crystal's modal selection of an owned Hireling.
pub(in crate::play) fn customization(
    panel: &mut StorePanel,
    name: &str,
    sex: &str,
    identity: &str,
) {
    panel.hireling_name = name.into();
    panel.hireling_sex = sex.parse().unwrap_or(0);
    let hex: String = identity.chars().filter(|c| *c != '-').collect();
    panel.hireling_id = (hex.len() == 32 && hex.is_ascii())
        .then(|| {
            let mut id = [0; 16];
            for (i, byte) in id.iter_mut().enumerate() {
                *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
            }
            Some(id)
        })
        .flatten();
}

pub(in crate::play) fn panel_text(panel: &StorePanel) -> String {
    if !panel.selected {
        return "Store unavailable in this session".into();
    }
    if panel.pending {
        return "Pending store action…".into();
    }
    let mut status = if let Some(feedback) = &panel.feedback {
        feedback.clone()
    } else if let Some(result) = &panel.purchase {
        format!("Purchase: {:?}", result.code)
    } else {
        panel.catalogue.as_ref().map_or_else(
            || "Load the server catalogue".into(),
            |page| {
                format!(
                    "Balance: {} coins; transferable: {}",
                    page.total_coins, page.transferable_coins
                )
            },
        )
    };
    if let Some(preview) = store::offer_preview(panel) {
        status.push_str("\r\n");
        status.push_str(&preview);
    }
    status
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
        let coin_shop = lock(&link.mailbox).coin_shop.clone();
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
                        790,
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
                        let next = control(hwnd, "BUTTON", "Next catalogue page", 5, 275, 30, 0);
                        let get_coins = control(hwnd, "BUTTON", "Get coins (sandbox)", 9, 550, 30, 0);
                        control(hwnd, "STATIC", "Hireling name / sex (1 male, 2 female) / identity (UUID)", 0, 310, 35, 0);
                        let name = control(hwnd, "EDIT", "", 6, 350, 25, WS_BORDER);
                        let sex = control(hwnd, "EDIT", "1", 7, 380, 25, WS_BORDER);
                        let identity = control(hwnd, "EDIT", "", 8, 410, 25, WS_BORDER);
                        let search = control(hwnd, "EDIT", "", 10, 590, 25, WS_BORDER | ES_AUTOHSCROLL as u32);
                        control(hwnd, "BUTTON", "Search this category page", 11, 620, 30, 0);
                        control(hwnd, "BUTTON", "Sort: catalogue / title / price", 12, 655, 30, 0);
                        control(hwnd, "BUTTON", "Gift coins (preview only)", 13, 695, 30, 0);
                        let mut displayed_rows = Vec::new();
                        let mut displayed = None;
                        let status = control(hwnd, "STATIC", "", 0, 450, 100, 0);
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
                                if action == 13 {
                                    drop(mailbox);
                                    PlayLink { mailbox: shared.clone(), commands: commands.clone() }.store_gift_open();
                                    mailbox = lock(&shared);
                                    continue;
                                }
                                let panel = &mut mailbox.store;
                                let command = {
                                    match action {
                                        11 if !panel.pending => {
                                            let mut text = [0u16; 257];
                                            let n = GetWindowTextW(search, text.as_mut_ptr(), 257);
                                            panel.search = String::from_utf16_lossy(&text[..n.max(0) as usize]);
                                            None
                                        }
                                        12 if !panel.pending => {
                                            use store::StoreSort;
                                            panel.sort = match panel.sort { StoreSort::Catalogue => StoreSort::Title, StoreSort::Title => StoreSort::Price, StoreSort::Price => StoreSort::Catalogue };
                                            None
                                        }
                                        3 if !panel.pending => {
                                            if displayed == panel.catalogue && displayed_rows == store::visible_offers(panel) {
                                                panel.selected_offer = usize::try_from(SendMessageW(offers, LB_GETCURSEL, 0, 0)).ok()
                                                    .and_then(|index| displayed_rows.get(index).copied()).unwrap_or(usize::MAX);
                                            }
                                            None
                                        }
                                        9 => { if let Some(launcher) = &coin_shop { launcher.start(); } None }
                                        2 | 5 if !panel.pending => {
                                            let mut text = [0u16; 256];
                                            let len = GetWindowTextW(
                                                category,
                                                text.as_mut_ptr(),
                                                text.len() as i32,
                                            );
                                            panel.category =
                                                String::from_utf16_lossy(&text[..len as usize]);
                                            if action == 2 {
                                                panel.catalogue = None;
                                            }
                                            store::catalogue(panel)
                                        }
                                        4 if !panel.pending && panel.selected
                                            && displayed == panel.catalogue && displayed_rows == store::visible_offers(panel) => {
                                            let read = |hwnd| {
                                                let mut text = [0u16; 64];
                                                let len = GetWindowTextW(hwnd, text.as_mut_ptr(), 64);
                                                String::from_utf16_lossy(&text[..len.max(0) as usize])
                                            };
                                            customization(panel, &read(name), &read(sex), &read(identity));
                                            let index = usize::try_from(SendMessageW(offers, LB_GETCURSEL, 0, 0)).ok()
                                                .and_then(|index| displayed_rows.get(index).copied());
                                            if let Some(index) = index
                                                && let Some(offer) = panel
                                                    .catalogue
                                                    .as_ref()
                                                    .and_then(|page| page.offers.get(index))
                                            {
                                                let question = if panel.confirmed_purchase.is_some() {
                                                    "Retry the previously confirmed purchase with the same transaction identity?".into()
                                                } else {
                                                    let mut question = format!("Purchase {} for {} {}?", offer.title, offer.price, offer.currency);
                                                    match offer.offer_key.as_str() {
                                                        "store:hireling:25440" => question.push_str(&format!("\nName: {}; sex: {}", read(name), read(sex))),
                                                        "store:hireling:25438" => question.push_str(&format!("\nHireling: {}; new name: {}", read(identity), read(name))),
                                                        "store:hireling:25437" => question.push_str(&format!("\nChange sex of Hireling: {}", read(identity))),
                                                        _ => {}
                                                    }
                                                    question
                                                };
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
                            let visible = store::visible_offers(panel);
                            if displayed != panel.catalogue || displayed_rows != visible {
                                SendMessageW(offers, LB_RESETCONTENT, 0, 0);
                                if let Some(page) = &panel.catalogue {
                                    for index in &visible {
                                        let offer = &page.offers[*index];
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
                                displayed_rows = visible;
                                panel.selected_offer = usize::MAX;
                                displayed = panel.catalogue.clone();
                            }
                            EnableWindow(next, i32::from(panel.selected && !panel.pending && panel.catalogue.as_ref().is_some_and(|p| !p.next_page_cursor.is_empty())));
                            EnableWindow(get_coins, i32::from(coin_shop.as_ref().is_some_and(|launcher|launcher.can_start())));
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

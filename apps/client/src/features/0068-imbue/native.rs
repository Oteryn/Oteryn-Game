//! Windows controls for the imbuement shrine; HWNDs stay on their creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::imbue::{ImbuePanel, Intent, panel_text, slot_label};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const OFFERS: usize = 101;
const REFRESH: usize = 102;
const APPLY: usize = 103;
const CLEAR: usize = 104;
const SLOTS: usize = 106;
const ITEMS: usize = 107;
const OPEN_BAG: usize = 108;

#[derive(Default)]
struct Actions(Vec<usize>);

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: the creating thread owns the live Actions userdata and all controls.
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            let id = w & 0xffff;
            let notification = (w >> 16) & 0xffff;
            if !actions.is_null()
                && (*actions).0.len() < 32
                && ((matches!(id, OFFERS | SLOTS | ITEMS)
                    && notification == LBN_SELCHANGE as usize)
                    || (matches!(id, REFRESH | APPLY | CLEAR | OPEN_BAG)
                        && notification == BN_CLICKED as usize))
            {
                (*actions).0.push(id);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.imbue.open {
            return;
        }
        mailbox.imbue.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("imbuement-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&failed);
    }
}

fn run(
    weak: Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    commands: tokio::sync::mpsc::UnboundedSender<PlayCommand>,
) {
    // SAFETY: this thread creates, services, and destroys every HWND below.
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("Imbuement Shrine")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            620,
            610,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            close(&weak);
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&mut *actions as *mut Actions) as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
        let items = control(
            hwnd,
            "LISTBOX",
            "",
            ITEMS,
            [10, 10, 420, 65],
            LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL | WS_TABSTOP,
        );
        let open_bag = control(
            hwnd,
            "BUTTON",
            tr!("Open selected bag"),
            OPEN_BAG,
            [440, 10, 150, 30],
            WS_TABSTOP,
        );
        let slots = control(
            hwnd,
            "LISTBOX",
            "",
            SLOTS,
            [10, 85, 180, 230],
            LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL | WS_TABSTOP,
        );
        let offers = control(
            hwnd,
            "LISTBOX",
            "",
            OFFERS,
            [200, 85, 390, 230],
            LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL | WS_TABSTOP,
        );
        let refresh = control(
            hwnd,
            "BUTTON",
            tr!("Refresh"),
            REFRESH,
            [10, 325, 140, 30],
            WS_TABSTOP,
        );
        let apply = control(
            hwnd,
            "BUTTON",
            tr!("Apply selected"),
            APPLY,
            [160, 325, 190, 30],
            WS_TABSTOP,
        );
        let clear = control(
            hwnd,
            "BUTTON",
            tr!("Clear selected slot"),
            CLEAR,
            [360, 325, 230, 30],
            WS_TABSTOP,
        );
        let status = control(
            hwnd,
            "EDIT",
            "",
            105,
            [10, 365, 580, 190],
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
        );
        if [
            items, open_bag, slots, offers, refresh, apply, clear, status,
        ]
        .iter()
        .any(|handle| handle.is_null())
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        let mut held: Option<ImbuePanel> = None;
        let mut carried: Vec<oteryn_client_runtime::depot::ItemEntry> = Vec::new();
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
            if mailbox.imbue_shutdown {
                break;
            }
            if mailbox.imbue.refresh_required && !mailbox.imbue.pending {
                super::super::input::enqueue(&mut mailbox, &commands, Intent::Refresh);
            }
            super::super::input::enqueue_details(&mut mailbox, &commands);
            for action in std::mem::take(&mut actions.0) {
                let intent = match action {
                    ITEMS => {
                        if let Ok(row) = usize::try_from(SendMessageW(items, LB_GETCURSEL, 0, 0))
                            && let Some(entry) = carried.get(row)
                        {
                            super::super::input::select_carried(
                                &mut mailbox,
                                &commands,
                                entry.handle,
                            );
                        }
                        None
                    }
                    OPEN_BAG => {
                        if let Ok(row) = usize::try_from(SendMessageW(items, LB_GETCURSEL, 0, 0))
                            && let Some(entry) = carried.get(row)
                        {
                            let _ = commands.send(PlayCommand::DepotContainer(entry.handle));
                        }
                        None
                    }
                    SLOTS => {
                        if let Ok(slot) = u8::try_from(SendMessageW(slots, LB_GETCURSEL, 0, 0)) {
                            mailbox.imbue.select_slot(slot);
                        }
                        None
                    }
                    OFFERS => {
                        if let Ok(row) = usize::try_from(SendMessageW(offers, LB_GETCURSEL, 0, 0)) {
                            mailbox.imbue.select_offer(row);
                        }
                        None
                    }
                    REFRESH => Some(Intent::Refresh),
                    APPLY => Some(Intent::Apply),
                    CLEAR => Some(Intent::Clear),
                    _ => None,
                };
                if let Some(intent) = intent {
                    super::super::input::enqueue(&mut mailbox, &commands, intent);
                }
            }
            let current = super::super::input::carried_items(&mailbox);
            if current != carried {
                SendMessageW(items, LB_RESETCONTENT, 0, 0);
                for entry in &current {
                    let label = lfmt!(
                        "Carried item {} ×{}",
                        entry.item_definition_ref,
                        entry.count
                    );
                    SendMessageW(items, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                }
                if let Some(selected) = mailbox.imbue.item
                    && let Some(row) = current.iter().position(|entry| entry.handle == selected)
                {
                    SendMessageW(items, LB_SETCURSEL, row, 0);
                }
                carried = current;
            }
            if held.as_ref() != Some(&mailbox.imbue) {
                SendMessageW(slots, LB_RESETCONTENT, 0, 0);
                if let Some(window) = &mailbox.imbue.window {
                    for slot in 0..window.slot_count {
                        SendMessageW(
                            slots,
                            LB_ADDSTRING,
                            0,
                            wide(&slot_label(&mailbox.imbue, slot)).as_ptr() as isize,
                        );
                    }
                }
                SendMessageW(slots, LB_SETCURSEL, usize::from(mailbox.imbue.slot), 0);
                SendMessageW(offers, LB_RESETCONTENT, 0, 0);
                if let Some(window) = &mailbox.imbue.window {
                    for (index, offer) in window.offers.iter().enumerate() {
                        let label = lfmt!(
                            "{index}: family {} tier {} — {} gold",
                            oteryn_client_ui::imbue::family_name(offer.family),
                            offer.tier,
                            offer.gold,
                            index = index
                        );
                        SendMessageW(offers, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                    }
                }
                SendMessageW(offers, LB_SETCURSEL, mailbox.imbue.offer, 0);
                SetWindowTextW(status, wide(&panel_text(&mailbox.imbue)).as_ptr());
                let ready = mailbox.imbue.selected && !mailbox.imbue.pending;
                EnableWindow(items, i32::from(ready));
                EnableWindow(
                    open_bag,
                    i32::from(ready && mailbox.depot.items.containers_selected),
                );
                EnableWindow(slots, i32::from(ready));
                EnableWindow(offers, i32::from(ready));
                EnableWindow(refresh, i32::from(ready));
                EnableWindow(apply, i32::from(mailbox.imbue.can_apply()));
                EnableWindow(clear, i32::from(mailbox.imbue.can_clear()));
                held = Some(mailbox.imbue.clone());
            }
            drop(mailbox);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak);
    }
}

fn close(weak: &Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).imbue.open = false;
    }
}

unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    bounds: [i32; 4],
    style: u32,
) -> HWND {
    // SAFETY: parent is live and the synchronous call copies both UTF-16 strings.
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            bounds[0],
            bounds[1],
            bounds[2],
            bounds[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}

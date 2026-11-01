//! Native Prey window; Win32 controls and their userdata stay on the creating thread.
// The existing native panel boundary requires Win32 FFI; no HWND crosses threads.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::prey::{GridLayout, Intent, PreyPanel, creature_label, panel_text, prey, slot_text, tier_choices};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const OFFER: usize = 5000;
const FIRST: usize = 5020;
const NEXT: usize = 5021;
const REFRESH: usize = 5099;
const CELL_WIDTH: i32 = 92;
const CELL_HEIGHT: i32 = 60;
type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;

#[derive(Default)]
struct Actions(Vec<usize>);

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: both windows point at the same live Actions box owned by this thread.
    unsafe {
        if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && (*actions).0.len() < 32 {
                (*actions).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}").encode_utf16().chain(Some(0)).collect()
}

pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.prey.open { return; }
        mailbox.prey.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new().name("prey-panel".into()).spawn(move || run(weak, commands)).is_err() {
        close(&rollback);
    }
}

struct SlotControls {
    title: HWND,
    creatures: Vec<HWND>,
    tiers: [HWND; 5],
    buttons: [HWND; 4],
}

struct Offers {
    window: HWND,
    rows: Vec<HWND>,
    first: HWND,
    next: HWND,
}

unsafe fn control(parent: HWND, class: &str, text: &str, id: usize, rect: [i32; 4], style: u32) -> HWND {
    // SAFETY: parent is live and every string survives the synchronous API call.
    unsafe {
        CreateWindowExW(0, wide(class).as_ptr(), wide(text).as_ptr(), WS_CHILD | WS_VISIBLE | style,
            rect[0], rect[1], rect[2], rect[3], parent, id as _, null_mut(), null())
    }
}

unsafe fn window(owner: HWND, title: &str, width: i32, height: i32, actions: &mut Actions) -> HWND {
    // SAFETY: HWND and actions are exclusively owned until destruction on this thread.
    unsafe {
        let hwnd = CreateWindowExW(0, wide("STATIC").as_ptr(), wide(title).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT, CW_USEDEFAULT, width, height, owner, null_mut(), null_mut(), null());
        if !hwnd.is_null() {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, actions as *mut Actions as isize);
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
        }
        hwnd
    }
}

unsafe fn slot_controls(parent: HWND, index: usize) -> SlotControls {
    // SAFETY: all controls share the live parent and its owning thread.
    unsafe {
        let x = 12 + index as i32 * 302;
        let base = 1000 + index * 100;
        let title = control(parent, "STATIC", "", base + 90, [x, 46, 290, 44], 0);
        let creatures = (0..prey::MAX_CREATURES).map(|cell| {
            let hwnd = control(parent, "BUTTON", "", base + cell,
                [x + (cell % 3) as i32 * 96, 94 + (cell / 3) as i32 * 64, CELL_WIDTH, CELL_HEIGHT],
                BS_CHECKBOX as u32 | BS_PUSHLIKE as u32 | BS_MULTILINE as u32 | WS_TABSTOP);
            ShowWindow(hwnd, SW_HIDE);
            hwnd
        }).collect();
        let tiers = std::array::from_fn(|tier| control(parent, "BUTTON", &format!("{}★", tier + 1),
            base + 70 + tier, [x + tier as i32 * 57, 290, 56, 28], BS_RADIOBUTTON as u32 | WS_TABSTOP));
        let buttons = std::array::from_fn(|button| control(parent, "BUTTON",
            ["List reroll", "Bonus reroll", "Select", "Pick again…"][button],
            base + 80 + button, [x, 324, 92, 28], WS_TABSTOP));
        SlotControls { title, creatures, tiers, buttons }
    }
}

unsafe fn offered_window(parent: HWND, actions: &mut Actions) -> Offers {
    // SAFETY: all HWNDs and the shared action pointer remain on this thread.
    unsafe {
        let hwnd = window(parent, "Creatures offered today", 640, 590, actions);
        let rows = (0..prey::MAX_OFFERED_PAGE).map(|row| control(hwnd, "BUTTON", "", OFFER + row,
            [10, 10 + row as i32 * 30, 604, 28], WS_TABSTOP)).collect();
        let first = control(hwnd, "BUTTON", "First page", FIRST, [10, 506, 294, 28], WS_TABSTOP);
        let next = control(hwnd, "BUTTON", "Next page", NEXT, [320, 506, 294, 28], WS_TABSTOP);
        Offers { window: hwnd, rows, first, next }
    }
}

unsafe fn render_slots(window: HWND, controls: &[SlotControls], panel: &PreyPanel) {
    // SAFETY: controls are checked live on the window's owning thread.
    unsafe {
        let rows = GridLayout::for_panel(panel).rows as i32;
        let tier_y = 94 + rows * 64;
        SetWindowPos(window, null_mut(), 0, 0, 936, tier_y + 160, SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
        for (index, controls) in controls.iter().enumerate() {
            let slot_id = index as u8 + 1;
            let state = panel.slot(slot_id);
            let x = 12 + index as i32 * 302;
            SetWindowTextW(controls.title, wide(&state.map_or_else(
                || format!("Slot {slot_id} · Loading…"), slot_text)).as_ptr());
            for (cell, &hwnd) in controls.creatures.iter().enumerate() {
                if let Some(key) = state.and_then(|state| state.creatures.get(cell)) {
                    SetWindowTextW(hwnd, wide(&creature_label(key)).as_ptr());
                    ShowWindow(hwnd, SW_SHOW);
                    EnableWindow(hwnd, i32::from(panel.pending.is_none() && state.is_some_and(|state| state.state == 1)));
                    SendMessageW(hwnd, BM_SETCHECK, usize::from(panel.candidate[index] == Some(cell as u8 + 1)), 0);
                } else { ShowWindow(hwnd, SW_HIDE); }
            }
            for (tier, (label, checked, enabled)) in tier_choices(panel, slot_id).into_iter().enumerate() {
                let hwnd = controls.tiers[tier];
                MoveWindow(hwnd, x + tier as i32 * 57, tier_y, 56, 28, 1);
                SetWindowTextW(hwnd, wide(&label).as_ptr());
                SendMessageW(hwnd, BM_SETCHECK, usize::from(checked), 0);
                EnableWindow(hwnd, i32::from(enabled));
            }
            for (button, &hwnd) in controls.buttons.iter().enumerate() {
                MoveWindow(hwnd, x + if button < 3 { button as i32 * 96 } else { 0 },
                    tier_y + if button < 3 { 32 } else { 66 }, if button < 3 { 92 } else { 284 }, 28, 1);
                let enabled = panel.pending.is_none() && panel.unlocked(slot_id)
                    && match button {
                        1 => state.is_some_and(|state| state.state == 2),
                        2 => state.is_some_and(|state| state.state == 1) && panel.candidate[index].is_some(),
                        _ => true,
                    };
                EnableWindow(hwnd, i32::from(enabled));
            }
        }
    }
}

unsafe fn render_offers(offers: &Offers, panel: &PreyPanel) {
    // SAFETY: the offers window is still live and owned by this thread.
    unsafe {
        for (index, &hwnd) in offers.rows.iter().enumerate() {
            if let Some(row) = panel.offered_today.get(index) {
                let occupied = row.occupied_slot != 0 && Some(row.occupied_slot) != panel.offered_slot;
                let suffix = if occupied { format!(" · in slot {}", row.occupied_slot) } else { String::new() };
                SetWindowTextW(hwnd, wide(&format!("{} · {} gold{}", creature_label(&row.creature), row.price_gold, suffix)).as_ptr());
                EnableWindow(hwnd, i32::from(!occupied && panel.pending.is_none()));
                ShowWindow(hwnd, SW_SHOW);
            } else { ShowWindow(hwnd, SW_HIDE); }
        }
        EnableWindow(offers.first, i32::from(panel.pending.is_none()));
        EnableWindow(offers.next, i32::from(panel.pending.is_none() && !panel.offered_next.is_empty()));
    }
}

fn run(weak: WeakMailbox, commands: UnboundedSender<PlayCommand>) {
    // SAFETY: windows, controls, userdata and message dispatch remain on this thread.
    unsafe {
        let mut actions = Box::<Actions>::default();
        let hwnd = window(null_mut(), "Prey", 936, 446, &mut actions);
        if hwnd.is_null() { close(&weak); return; }
        let description = control(hwnd, "STATIC", "", 900, [12, 12, 770, 28], 0);
        let refresh = control(hwnd, "BUTTON", "Refresh", REFRESH, [796, 10, 110, 28], WS_TABSTOP);
        let slots: Vec<_> = (0..3).map(|index| slot_controls(hwnd, index)).collect();
        if description.is_null() || refresh.is_null() || slots.iter().any(|slot|
            slot.title.is_null() || slot.creatures.iter().chain(slot.tiers.iter()).chain(slot.buttons.iter()).any(|h| h.is_null()))
        {
            DestroyWindow(hwnd); close(&weak); return;
        }
        let mut offers: Option<Offers> = None;
        let mut held: Option<PreyPanel> = None;
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                let dialog = offers.as_ref().filter(|offers| IsWindow(offers.window) != 0)
                    .map_or(hwnd, |offers| offers.window);
                if IsDialogMessageW(dialog, &message) == 0 {
                    TranslateMessage(&message); DispatchMessageW(&message);
                }
            }
            if IsWindow(hwnd) == 0 { break; }
            let Some(shared) = weak.upgrade() else { break; };
            let mut mailbox = lock(&shared);
            if mailbox.prey_shutdown { break; }
            if offers.as_ref().is_some_and(|offers| IsWindow(offers.window) == 0) {
                offers = None;
                mailbox.prey.offered_slot = None;
            }
            let mut confirmation = None;
            for action in std::mem::take(&mut actions.0) {
                let intent = if action == REFRESH { Some(Intent::Refresh) }
                else if (OFFER..OFFER + prey::MAX_OFFERED_PAGE).contains(&action) {
                    confirmation = mailbox.prey.confirm_offer(action - OFFER); None
                } else if action == FIRST || action == NEXT {
                    mailbox.prey.offered_slot.map(|slot| Intent::OfferedToday { slot,
                        after: if action == FIRST { String::new() } else { mailbox.prey.offered_next.clone() } })
                } else if (1000..1300).contains(&action) {
                    let slot = ((action - 1000) / 100 + 1) as u8;
                    let control = (action - 1000) % 100;
                    match control {
                        0..=63 => { mailbox.prey.choose_candidate(slot, control as u8 + 1); None }
                        70..=74 => Some(Intent::SetTier { slot, tier: (control - 69) as u8 }),
                        80 => Some(Intent::ListReroll { slot }),
                        81 => Some(Intent::BonusReroll { slot }),
                        82 => Some(Intent::Select { slot }),
                        83 => Some(Intent::OfferedToday { slot, after: String::new() }),
                        _ => None,
                    }
                } else { None };
                if let Some(intent) = intent { super::super::input::enqueue(&mut mailbox, &commands, intent); }
            }
            if mailbox.prey.offered_slot.is_some() && offers.is_none() {
                let created = offered_window(hwnd, &mut actions);
                if created.window.is_null() || created.rows.iter().any(|h| h.is_null())
                    || created.first.is_null() || created.next.is_null()
                {
                    if !created.window.is_null() { DestroyWindow(created.window); }
                    mailbox.prey.offered_slot = None;
                    mailbox.prey.feedback = Some("Could not open the offered-creature list.");
                } else { offers = Some(created); }
            }
            if mailbox.prey.offered_slot.is_none() && let Some(old) = offers.take() {
                DestroyWindow(old.window);
            }
            let panel = &mailbox.prey;
            if held.as_ref() != Some(panel) {
                SetWindowTextW(description, wide(&panel_text(panel)).as_ptr());
                EnableWindow(refresh, i32::from(panel.selected && panel.pending.is_none()));
                render_slots(hwnd, &slots, panel);
                if let Some(offers) = &offers { render_offers(offers, panel); }
                held = Some(panel.clone());
            }
            drop(mailbox);
            if let Some(confirmation) = confirmation {
                let answer = MessageBoxW(hwnd, wide(&format!("Select {} again in slot {} for {} gold?",
                    creature_label(&confirmation.creature), confirmation.slot, confirmation.price_gold)).as_ptr(),
                    wide("Confirm Prey purchase").as_ptr(), MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2);
                if answer == IDYES {
                    super::super::input::enqueue(&mut lock(&shared), &commands, Intent::ConfirmPickAgain(confirmation));
                } else { lock(&shared).prey.confirmation = None; }
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        if let Some(offers) = offers && IsWindow(offers.window) != 0 { DestroyWindow(offers.window); }
        if IsWindow(hwnd) != 0 { DestroyWindow(hwnd); }
        close(&weak);
    }
}

fn close(weak: &WeakMailbox) {
    if let Some(mailbox) = weak.upgrade() { lock(&mailbox).prey.open = false; }
}

#[cfg(test)]
#[path = "native_tests.rs"]
mod tests;

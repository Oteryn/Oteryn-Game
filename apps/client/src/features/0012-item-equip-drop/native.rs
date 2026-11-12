//! Controls and drag surfaces are created and refreshed on the inventory window thread.
#![allow(unsafe_code)] // Win32 controls require the owning thread's raw handles.
use crate::play::mailbox::Mailbox;
use std::ptr::{null, null_mut};
use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
pub(in crate::play) struct Slots {
    controls: Vec<HWND>,
    held: Vec<oteryn_client_ui::equipment::Slot>,
    status: HWND,
    held_status: Option<&'static str>,
}
impl Slots {
    pub(in crate::play) fn create(parent: HWND) -> Self {
        let controls = (0..10)
            .map(|i| {
                // SAFETY: parent belongs to this thread; control strings live through the call.
                unsafe {
                    CreateWindowExW(
                        0,
                        wide("LISTBOX").as_ptr(),
                        wide("").as_ptr(),
                        WS_CHILD | WS_VISIBLE | WS_BORDER | WS_TABSTOP,
                        445,
                        10 + i * 32,
                        270,
                        28,
                        parent,
                        null_mut(),
                        null_mut(),
                        null(),
                    )
                }
            })
            .collect();
        // SAFETY: this status control is owned by the inventory window's thread.
        let status = unsafe {
            CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("").as_ptr(),
                WS_CHILD | WS_VISIBLE,
                445,
                335,
                270,
                45,
                parent,
                null_mut(),
                null_mut(),
                null(),
            )
        };
        Self {
            controls,
            status,
            held_status: None,
            held: vec![],
        }
    }
    pub(in crate::play) fn refresh(&mut self, mailbox: &mut Mailbox) {
        let slots = oteryn_client_ui::equipment::protocol::project(
            mailbox.inventory.inventory.as_ref(),
            mailbox.equipment_selected,
        );
        for (i, (control, slot)) in self.controls.iter().zip(&slots).enumerate() {
            if control.is_null() {
                continue;
            }
            if self.held.get(i) != Some(slot) {
                // SAFETY: control belongs to this thread and label lives through SendMessageW.
                unsafe {
                    SendMessageW(*control, LB_RESETCONTENT, 0, 0);
                    SendMessageW(
                        *control,
                        LB_ADDSTRING,
                        0,
                        wide(&slot.text()).as_ptr() as isize,
                    );
                }
            }
            let items: Vec<_> = slot.drag_item().and(slot.item).into_iter().collect();
            super::super::super::feature_0004_item_view_move::view::list_surface(
                mailbox,
                *control,
                &items,
                slot.destination,
            );
        }
        let movement = &mailbox.item_movement;
        let text = if movement.pending {
            tr!("Moving item…")
        } else if movement.prompt.is_some() {
            tr!("Choose the stack count")
        } else {
            movement
                .feedback
                .unwrap_or(tr!("Drag to equip; drag back to Backpack to unequip"))
        };
        // SAFETY: the status control and label are valid on this thread.
        if !self.status.is_null() && self.held_status != Some(text) {
            unsafe {
                SetWindowTextW(self.status, wide(text).as_ptr());
            }
            self.held_status = Some(text);
        }
        self.held = slots;
    }
}

//! Existing Wheel repaint uses the same confirmed model and draft controls.
use super::*;
impl layout::Controls {
    pub(super) unsafe fn paint(&self, panel: &WheelPanel) {
        unsafe {
            let layout::Controls {
                slots,
                points,
                gems,
                modifiers,
                quality,
                vessel,
                lock_button,
                description,
                ..
            } = *self;

            SendMessageW(slots, LB_RESETCONTENT, 0, 0);
            for slot in 0..36 {
                SendMessageW(
                    slots,
                    LB_ADDSTRING,
                    0,
                    wide(&slot_label(panel, slot)).as_ptr() as isize,
                );
            }
            SendMessageW(slots, LB_SETCURSEL, panel.slot, 0);
            SetWindowTextW(points, wide(&panel.points[panel.slot].to_string()).as_ptr());
            SendMessageW(gems, LB_RESETCONTENT, 0, 0);
            if let Some(atelier) = &panel.confirmed_atelier {
                for (row, gem) in atelier.gems.iter().enumerate() {
                    SendMessageW(
                        gems,
                        LB_ADDSTRING,
                        0,
                        wide(&gem_label(gem)).as_ptr() as isize,
                    );
                    if Some(gem.id) == panel.gem {
                        SendMessageW(gems, LB_SETCURSEL, row, 0);
                    }
                }
            }
            SendMessageW(modifiers, LB_RESETCONTENT, 0, 0);
            for (row, (kind, id)) in panel.modifiers().into_iter().enumerate() {
                SendMessageW(
                    modifiers,
                    LB_ADDSTRING,
                    0,
                    wide(&modifier_label(panel, kind, id)).as_ptr() as isize,
                );
                if panel.modifier == Some((kind, id)) {
                    SendMessageW(modifiers, LB_SETCURSEL, row, 0);
                }
            }
            SendMessageW(quality, CB_SETCURSEL, usize::from(panel.quality), 0);
            SendMessageW(vessel, CB_SETCURSEL, usize::from(panel.vessel_slot), 0);
            SetWindowTextW(
                lock_button,
                wide(if panel.selected_gem().is_some_and(|gem| gem.locked) {
                    tr!("Unlock gem")
                } else {
                    tr!("Lock gem")
                })
                .as_ptr(),
            );
            SetWindowTextW(description, wide(&panel_text(panel)).as_ptr());
        }
    }
}

use super::*;

pub(super) unsafe fn slot_controls(parent: HWND, index: usize) -> SlotControls {
    // SAFETY: all controls share the live parent and its owning thread.
    unsafe {
        let x = 12 + index as i32 * 302;
        let base = 1000 + index * 100;
        let title = control(parent, "STATIC", "", base + 90, [x, 46, 290, 44], 0);
        let creatures = (0..prey::MAX_CREATURES)
            .map(|cell| {
                let hwnd = control(
                    parent,
                    "BUTTON",
                    "",
                    base + cell,
                    [
                        x + (cell % 3) as i32 * 96,
                        94 + (cell / 3) as i32 * 64,
                        CELL_WIDTH,
                        CELL_HEIGHT,
                    ],
                    BS_CHECKBOX as u32 | BS_PUSHLIKE as u32 | BS_MULTILINE as u32 | WS_TABSTOP,
                );
                ShowWindow(hwnd, SW_HIDE);
                hwnd
            })
            .collect();
        let tiers = std::array::from_fn(|tier| {
            control(
                parent,
                "BUTTON",
                &format!("{}★", tier + 1),
                base + 70 + tier,
                [x + tier as i32 * 57, 290, 56, 28],
                BS_RADIOBUTTON as u32 | WS_TABSTOP,
            )
        });
        let buttons = std::array::from_fn(|button| {
            control(
                parent,
                "BUTTON",
                [
                    "List reroll",
                    "Bonus reroll",
                    "Select",
                    "Pick again…",
                    "Manual",
                    "Auto reroll",
                    "Lock bonus",
                    "Select any (5 cards)",
                ][button],
                base + 80 + button,
                [x, 324, 92, 28],
                WS_TABSTOP,
            )
        });
        let creature = control(
            parent,
            "EDIT",
            "oteryn:",
            base + 89,
            [x, 420, 284, 28],
            WS_BORDER | WS_TABSTOP,
        );
        SendMessageW(creature, EM_SETLIMITTEXT, 256, 0);
        SlotControls {
            title,
            creatures,
            tiers,
            buttons,
            creature,
        }
    }
}

pub(super) unsafe fn offered_window(parent: HWND, actions: &mut Actions) -> Offers {
    // SAFETY: all HWNDs and the shared action pointer remain on this thread.
    unsafe {
        let hwnd = window(parent, "Creatures offered today", 640, 590, actions);
        let rows = (0..prey::MAX_OFFERED_PAGE)
            .map(|row| {
                control(
                    hwnd,
                    "BUTTON",
                    "",
                    OFFER + row,
                    [10, 10 + row as i32 * 30, 604, 28],
                    WS_TABSTOP,
                )
            })
            .collect();
        let first = control(
            hwnd,
            "BUTTON",
            "First page",
            FIRST,
            [10, 506, 294, 28],
            WS_TABSTOP,
        );
        let next = control(
            hwnd,
            "BUTTON",
            "Next page",
            NEXT,
            [320, 506, 294, 28],
            WS_TABSTOP,
        );
        Offers {
            window: hwnd,
            rows,
            first,
            next,
        }
    }
}

pub(super) unsafe fn render_slots(window: HWND, controls: &[SlotControls], panel: &PreyPanel) {
    // SAFETY: controls are checked live on the window's owning thread.
    unsafe {
        let rows = GridLayout::for_panel(panel).rows as i32;
        let tier_y = 94 + rows * 64;
        SetWindowPos(
            window,
            null_mut(),
            0,
            0,
            936,
            tier_y + 260,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
        for (index, controls) in controls.iter().enumerate() {
            let slot_id = index as u8 + 1;
            let state = panel.slot(slot_id);
            let x = 12 + index as i32 * 302;
            SetWindowTextW(
                controls.title,
                wide(&state.map_or_else(|| format!("Slot {slot_id} · Loading…"), slot_text))
                    .as_ptr(),
            );
            for (cell, &hwnd) in controls.creatures.iter().enumerate() {
                if let Some(key) = state.and_then(|state| state.creatures.get(cell)) {
                    SetWindowTextW(hwnd, wide(&creature_label(key)).as_ptr());
                    ShowWindow(hwnd, SW_SHOW);
                    EnableWindow(
                        hwnd,
                        i32::from(
                            panel.pending.is_none() && state.is_some_and(|state| state.state == 1),
                        ),
                    );
                    SendMessageW(
                        hwnd,
                        BM_SETCHECK,
                        usize::from(panel.candidate[index] == Some(cell as u8 + 1)),
                        0,
                    );
                } else {
                    ShowWindow(hwnd, SW_HIDE);
                }
            }
            for (tier, (label, checked, enabled)) in
                tier_choices(panel, slot_id).into_iter().enumerate()
            {
                let hwnd = controls.tiers[tier];
                MoveWindow(hwnd, x + tier as i32 * 57, tier_y, 56, 28, 1);
                SetWindowTextW(hwnd, wide(&label).as_ptr());
                SendMessageW(hwnd, BM_SETCHECK, usize::from(checked), 0);
                EnableWindow(hwnd, i32::from(enabled));
            }
            for (button, &hwnd) in controls.buttons.iter().enumerate() {
                let (column, row, width) = match button {
                    0..=2 => (button as i32, 32, 92),
                    3 => (0, 66, 284),
                    4..=6 => (button as i32 - 4, 100, 92),
                    _ => (0, 168, 284),
                };
                MoveWindow(hwnd, x + column * 96, tier_y + row, width, 28, 1);
                let enabled = panel.pending.is_none()
                    && panel.unlocked(slot_id)
                    && match button {
                        1 => state.is_some_and(|state| state.state == 2),
                        7 => state.is_some_and(|state| state.state == 1),
                        2 => {
                            state.is_some_and(|state| state.state == 1)
                                && panel.candidate[index].is_some()
                        }
                        _ => true,
                    };
                EnableWindow(hwnd, i32::from(enabled));
            }
            MoveWindow(controls.creature, x, tier_y + 134, 284, 28, 1);
            EnableWindow(
                controls.creature,
                i32::from(panel.pending.is_none() && panel.unlocked(slot_id)),
            );
        }
    }
}

pub(super) unsafe fn render_offers(offers: &Offers, panel: &PreyPanel) {
    // SAFETY: the offers window is still live and owned by this thread.
    unsafe {
        for (index, &hwnd) in offers.rows.iter().enumerate() {
            if let Some(row) = panel.offered_today.get(index) {
                let occupied =
                    row.occupied_slot != 0 && Some(row.occupied_slot) != panel.offered_slot;
                let suffix = if occupied {
                    format!(" · in slot {}", row.occupied_slot)
                } else {
                    String::new()
                };
                SetWindowTextW(
                    hwnd,
                    wide(&format!(
                        "{} · {} gold{}",
                        creature_label(&row.creature),
                        row.price_gold,
                        suffix
                    ))
                    .as_ptr(),
                );
                EnableWindow(hwnd, i32::from(!occupied && panel.pending.is_none()));
                ShowWindow(hwnd, SW_SHOW);
            } else {
                ShowWindow(hwnd, SW_HIDE);
            }
        }
        EnableWindow(offers.first, i32::from(panel.pending.is_none()));
        EnableWindow(
            offers.next,
            i32::from(panel.pending.is_none() && !panel.offered_next.is_empty()),
        );
    }
}

//! Bounded Win32 qualification through the existing app test target, without a login/server.
use super::*;
use windows_sys::Win32::Foundation::RECT;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{IsWindowEnabled, SetActiveWindow};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// The test keeps the owning thread and action userdata alive until all HWNDs are destroyed,
/// including when an assertion unwinds. It never opens a public gameplay connection.
struct OwnedWindow(HWND);
impl Drop for OwnedWindow {
    fn drop(&mut self) {
        // SAFETY: this guard is created and dropped on the native window's creating thread.
        unsafe {
            if IsWindow(self.0) != 0 { DestroyWindow(self.0); }
        }
    }
}

fn projection(count: u8) -> PreyPanel {
    PreyPanel {
        selected: true,
        confirmed: Some(prey::Reply {
            status: 0, wildcard_balance: 4, list_size: count, expansion_owned: count == 12,
            offered_today: Vec::new(), offered_next: String::new(),
            slots: (1..=3).map(|slot| prey::Slot {
                slot, state: u8::from(slot != 2),
                creatures: if slot == 2 { Vec::new() } else {
                    (1..=count).map(|index| format!("oteryn:creature.fixture_{slot}_{index}")).collect()
                },
                selected: String::new(), bonus: 0, grade: 0, percent: 0,
                hunting_seconds: 0, option: 0, free_reroll_at: 0,
                chosen_tier: if slot == 3 { 5 } else { 2 },
                allowed_tier: if slot == 3 { 2 } else { 3 }, selection_epoch: 0,
            }).collect(),
        }),
        ..PreyPanel::default()
    }
}

fn rect(hwnd: HWND) -> TestResult<RECT> {
    // SAFETY: HWND belongs to a live test-owned parent; RECT is a writable native value.
    unsafe {
        let mut value: RECT = std::mem::zeroed();
        if GetWindowRect(hwnd, &mut value) == 0 { return Err("control rectangle unavailable".into()); }
        Ok(value)
    }
}

fn caption(hwnd: HWND) -> TestResult<String> {
    // SAFETY: the fixed buffer is valid for the synchronous call, including its terminator.
    unsafe {
        let mut buffer = [0_u16; 512];
        let length = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        Ok(String::from_utf16(&buffer[..usize::try_from(length)?])?)
    }
}

#[test]
fn prey_native_controls_match_confirmed_tiers_prices_and_expanded_grid() -> TestResult {
    // SAFETY: this test owns the windows, userdata, controls and synchronous messages on one
    // thread. No HWND or mutable userdata is transferred to the runtime/session thread.
    unsafe {
        let mut actions = Box::<Actions>::default();
        let parent = OwnedWindow(window(null_mut(), "Prey control qualification", 936, 446, &mut actions));
        if parent.0.is_null() { return Err("Prey window creation failed".into()); }
        let slots: Vec<_> = (0..3).map(|index| slot_controls(parent.0, index)).collect();
        if slots.iter().any(|slot| slot.title.is_null()
            || slot.creatures.iter().chain(slot.tiers.iter()).chain(slot.buttons.iter()).any(|hwnd| hwnd.is_null()))
        {
            return Err("Prey control creation failed".into());
        }
        let mut panel = projection(9);
        render_slots(parent.0, &slots, &panel);
        assert_eq!(caption(slots[0].tiers[1])?, "2★");
        assert_eq!(SendMessageW(slots[0].tiers[1], BM_GETCHECK, 0, 0), 1);
        assert_ne!(IsWindowEnabled(slots[0].tiers[2]), 0);
        assert_eq!(IsWindowEnabled(slots[0].tiers[3]), 0);
        assert!(slots[1].tiers.iter().all(|hwnd| IsWindowEnabled(*hwnd) == 0));
        assert_eq!(SendMessageW(slots[2].tiers[4], BM_GETCHECK, 0, 0), 1);
        assert_eq!(IsWindowEnabled(slots[2].tiers[4]), 0, "retained tier after level loss stays grey");
        assert_ne!(IsWindowVisible(slots[0].creatures[8]), 0);
        assert_eq!(IsWindowVisible(slots[0].creatures[9]), 0);
        let original_cell = rect(slots[0].creatures[0])?;
        let original_tiers = rect(slots[0].tiers[0])?;
        let original_buttons = rect(slots[0].buttons[0])?;
        SetActiveWindow(parent.0);
        SendMessageW(slots[0].tiers[2], BM_CLICK, 0, 0);
        assert_eq!(actions.0, vec![1072], "native radio click must reach the owning action queue");
        actions.0.clear();

        panel = projection(12);
        render_slots(parent.0, &slots, &panel);
        assert_ne!(IsWindowVisible(slots[0].creatures[11]), 0);
        assert_eq!(IsWindowVisible(slots[0].creatures[12]), 0);
        let expanded_cell = rect(slots[0].creatures[0])?;
        assert_eq!(expanded_cell.right - expanded_cell.left, original_cell.right - original_cell.left);
        assert_eq!(expanded_cell.bottom - expanded_cell.top, original_cell.bottom - original_cell.top);
        assert_eq!(rect(slots[0].tiers[0])?.top - original_tiers.top, 64);
        assert_eq!(rect(slots[0].buttons[0])?.top - original_buttons.top, 64);
        assert!(rect(slots[0].creatures[11])?.bottom <= rect(slots[0].tiers[0])?.top);
        assert!(rect(slots[0].tiers[0])?.bottom <= rect(slots[0].buttons[0])?.top);

        let offers = offered_window(parent.0, &mut actions);
        let _offers_owner = OwnedWindow(offers.window);
        if offers.window.is_null() || offers.rows.iter().any(|hwnd| hwnd.is_null())
            || offers.first.is_null() || offers.next.is_null()
        {
            return Err("offered-creature window creation failed".into());
        }
        panel.offered_slot = Some(1);
        panel.offered_today = vec![
            prey::OfferedCreature { creature: "oteryn:creature.ant".into(), price_gold: 800, occupied_slot: 0 },
            prey::OfferedCreature { creature: "oteryn:creature.dragon".into(), price_gold: 45000, occupied_slot: 3 },
        ];
        render_offers(&offers, &panel);
        assert!(caption(offers.rows[0])?.contains("800 gold"));
        assert!(caption(offers.rows[1])?.contains("45000 gold · in slot 3"));
        assert_ne!(IsWindowEnabled(offers.rows[0]), 0);
        assert_eq!(IsWindowEnabled(offers.rows[1]), 0);
        assert_eq!(IsWindowVisible(offers.rows[2]), 0);
        assert_eq!(IsWindowEnabled(offers.next), 0);
        panel.offered_next = "oteryn:creature.dragon".into();
        render_offers(&offers, &panel);
        assert_ne!(IsWindowEnabled(offers.next), 0);

        panel.pending = Some(prey::Request::SetTier { slot: 1, tier: 3 });
        render_slots(parent.0, &slots, &panel);
        render_offers(&offers, &panel);
        assert!(slots[0].tiers.iter().all(|hwnd| IsWindowEnabled(*hwnd) == 0));
        assert!(slots[0].buttons.iter().all(|hwnd| IsWindowEnabled(*hwnd) == 0));
        assert_eq!(IsWindowEnabled(offers.rows[0]), 0);
        assert_eq!(IsWindowEnabled(offers.next), 0);
    }
    eprintln!("PREY_DIFF WINDOWS_CONTROLS_COMPLETED");
    Ok(())
}

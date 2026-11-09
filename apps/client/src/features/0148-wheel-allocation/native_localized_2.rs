use super::*;
pub(super) fn run(weak: WeakMailbox, commands: UnboundedSender<PlayCommand>) {
    // SAFETY: every HWND and all window-message handling below stay on this thread.
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("Wheel of Destiny / Gem Atelier")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            990,
            740,
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
        let list_style = LBS_NOTIFY as u32 | WS_VSCROLL | WS_TABSTOP | WS_BORDER;
        let combo_style = CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP;
        let title = control(
            hwnd,
            "STATIC",
            tr!("Edit points and vessels, then Save Wheel. Atelier operations apply immediately."),
            120,
            [10, 10, 760, 22],
            0,
        );
        let presets = control(
            hwnd,
            "BUTTON",
            tr!("Build presets"),
            PRESETS,
            [790, 6, 170, 28],
            WS_TABSTOP,
        );
        let slots = control(hwnd, "LISTBOX", "", SLOTS, [10, 40, 300, 330], list_style);
        let points = control(
            hwnd,
            "EDIT",
            "0",
            POINTS,
            [10, 380, 120, 26],
            ES_NUMBER as u32 | WS_TABSTOP | WS_BORDER,
        );
        let apply = control(
            hwnd,
            "BUTTON",
            tr!("Set slot points"),
            APPLY,
            [140, 380, 170, 28],
            WS_TABSTOP,
        );
        let save = control(
            hwnd,
            "BUTTON",
            tr!("Save Wheel"),
            SAVE,
            [10, 414, 145, 28],
            WS_TABSTOP,
        );
        let reset = control(
            hwnd,
            "BUTTON",
            tr!("Reset all points"),
            RESET,
            [165, 414, 145, 28],
            WS_TABSTOP,
        );
        let gems = control(hwnd, "LISTBOX", "", GEMS, [330, 40, 630, 230], list_style);
        let modifiers = control(
            hwnd,
            "LISTBOX",
            "",
            MODIFIERS,
            [330, 280, 340, 160],
            list_style,
        );
        let quality = control(
            hwnd,
            "COMBOBOX",
            "",
            QUALITY,
            [690, 280, 270, 200],
            combo_style,
        );
        let reveal = control(
            hwnd,
            "BUTTON",
            tr!("Reveal gem"),
            REVEAL,
            [690, 312, 270, 28],
            WS_TABSTOP,
        );
        let lock_button = control(
            hwnd,
            "BUTTON",
            tr!("Lock / Unlock"),
            LOCK,
            [690, 346, 130, 28],
            WS_TABSTOP,
        );
        let switch = control(
            hwnd,
            "BUTTON",
            tr!("Switch domain"),
            SWITCH,
            [830, 346, 130, 28],
            WS_TABSTOP,
        );
        let dismantle = control(
            hwnd,
            "BUTTON",
            tr!("Dismantle selected gem"),
            DISMANTLE,
            [690, 380, 270, 28],
            WS_TABSTOP,
        );
        let grade = control(
            hwnd,
            "BUTTON",
            tr!("Upgrade selected modifier"),
            GRADE,
            [690, 414, 270, 28],
            WS_TABSTOP,
        );
        let domain = control(
            hwnd,
            "COMBOBOX",
            "",
            DOMAIN,
            [10, 455, 300, 200],
            combo_style,
        );
        let socket = control(
            hwnd,
            "BUTTON",
            tr!("Socket selected gem"),
            SOCKET,
            [330, 455, 165, 28],
            WS_TABSTOP,
        );
        let unsocket = control(
            hwnd,
            "BUTTON",
            tr!("Clear chosen vessel"),
            UNSOCKET,
            [505, 455, 165, 28],
            WS_TABSTOP,
        );
        let refresh = control(
            hwnd,
            "BUTTON",
            tr!("Refresh"),
            REFRESH,
            [690, 455, 130, 28],
            WS_TABSTOP,
        );
        let retry = control(
            hwnd,
            "BUTTON",
            tr!("Retry last request"),
            RETRY,
            [830, 455, 130, 28],
            WS_TABSTOP,
        );
        let description = control(
            hwnd,
            "EDIT",
            "",
            119,
            [10, 500, 950, 190],
            ES_MULTILINE as u32
                | ES_READONLY as u32
                | ES_AUTOVSCROLL as u32
                | WS_VSCROLL
                | WS_BORDER
                | WS_TABSTOP,
        );
        if [
            title,
            slots,
            points,
            apply,
            save,
            reset,
            gems,
            modifiers,
            quality,
            reveal,
            lock_button,
            switch,
            dismantle,
            grade,
            presets,
            domain,
            socket,
            unsocket,
            refresh,
            retry,
            description,
        ]
        .iter()
        .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        for name in [tr!("Lesser gem"), tr!("Regular gem"), tr!("Greater gem")] {
            SendMessageW(quality, CB_ADDSTRING, 0, wide(name).as_ptr() as isize);
        }
        for value in 0..4 {
            SendMessageW(
                domain,
                CB_ADDSTRING,
                0,
                wide(&format!("{} vessel", domain_name(value))).as_ptr() as isize,
            );
        }
        let mut held: Option<WheelPanel> = None;
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(shared) = weak.upgrade() else {
                break;
            };
            let mut mailbox = lock(&shared);
            if mailbox.wheel_shutdown {
                break;
            }
            for action in std::mem::take(&mut actions.0) {
                if action == PRESETS {
                    drop(mailbox);
                    super::super::presets::open_shared(&shared, &commands);
                    mailbox = lock(&shared);
                    continue;
                }
                let intent = match action {
                    SLOTS => {
                        if let Ok(slot) = usize::try_from(SendMessageW(slots, LB_GETCURSEL, 0, 0))
                            && slot < 36
                        {
                            mailbox.wheel.slot = slot;
                        }
                        None
                    }
                    GEMS => {
                        let row = usize::try_from(SendMessageW(gems, LB_GETCURSEL, 0, 0)).ok();
                        let gem = row
                            .and_then(|row| mailbox.wheel.confirmed_atelier.as_ref()?.gems.get(row))
                            .map(|gem| gem.id);
                        mailbox.wheel.select_gem(gem);
                        None
                    }
                    MODIFIERS => {
                        let row = usize::try_from(SendMessageW(modifiers, LB_GETCURSEL, 0, 0)).ok();
                        mailbox.wheel.modifier =
                            row.and_then(|row| mailbox.wheel.modifiers().get(row).copied());
                        None
                    }
                    QUALITY => {
                        if let Ok(value @ 0..=2) =
                            u8::try_from(SendMessageW(quality, CB_GETCURSEL, 0, 0))
                        {
                            mailbox.wheel.quality = value;
                        }
                        None
                    }
                    DOMAIN => {
                        if let Ok(value @ 0..=3) =
                            u8::try_from(SendMessageW(domain, CB_GETCURSEL, 0, 0))
                        {
                            mailbox.wheel.domain = value;
                        }
                        None
                    }
                    APPLY => {
                        let mut text = [0_u16; 32];
                        let length = GetWindowTextW(points, text.as_mut_ptr(), text.len() as i32);
                        let value = usize::try_from(length)
                            .ok()
                            .and_then(|length| String::from_utf16(&text[..length]).ok())
                            .and_then(|text| text.trim().parse::<u16>().ok());
                        if let Some(value) = value {
                            let slot = mailbox.wheel.slot;
                            mailbox.wheel.set_points(slot, value);
                        } else {
                            mailbox.wheel.feedback = Some(tr!("Enter a whole number of points"));
                        }
                        None
                    }
                    SOCKET => {
                        mailbox.wheel.socket_selected();
                        None
                    }
                    UNSOCKET => {
                        mailbox.wheel.clear_vessel();
                        None
                    }
                    SAVE => Some(Intent::Save),
                    RESET => Some(Intent::Reset),
                    REFRESH => Some(Intent::Refresh),
                    REVEAL => Some(Intent::Reveal),
                    LOCK => Some(Intent::ToggleLock),
                    SWITCH => Some(Intent::SwitchDomain),
                    DISMANTLE => Some(Intent::Dismantle),
                    GRADE => Some(Intent::GradeUp),
                    RETRY => Some(Intent::Retry),
                    _ => None,
                };
                if let Some(intent) = intent {
                    super::super::super::input::enqueue(&mut mailbox, &commands, intent);
                }
            }
            let panel = &mailbox.wheel;
            if held.as_ref() != Some(panel) {
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
                SendMessageW(domain, CB_SETCURSEL, usize::from(panel.domain), 0);
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
                held = Some(panel.clone());
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak);
    }
}

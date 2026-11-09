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
        let Some(controls) = layout::create(hwnd) else {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        };
        let layout::Controls {
            slots,
            points,
            gems,
            modifiers,
            quality,
            domain,
            ..
        } = controls;
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
                if action == VISUAL_ATELIER {
                    drop(mailbox);
                    super::super::gem_native::open(&shared, &commands);
                    mailbox = lock(&shared);
                    continue;
                }
                if action == WHEEL_DIAGRAM {
                    drop(mailbox);
                    super::super::wheel_native::open(&shared, &commands);
                    mailbox = lock(&shared);
                    continue;
                }
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
                controls.paint(panel);
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

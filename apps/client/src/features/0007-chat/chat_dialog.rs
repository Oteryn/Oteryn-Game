//! Local chat dialog message pump on its owning thread.
use super::*;
#[path = "console_font.rs"]
mod console_font;
#[path = "../local-spell-action-bar/cooldown_native.rs"]
mod cooldown_bar;
pub(super) fn run(weak: WeakMailbox, commands: UnboundedSender<PlayCommand>) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("Local Chat (Ctrl+Shift+C)")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1500,
            686,
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
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        let rooms = control(
            hwnd,
            "LISTBOX",
            "",
            MODES,
            [10, 10, 230, 180],
            LBS_NOTIFY as u32 | WS_BORDER,
        );
        for mode in [
            tr!("Say"),
            tr!("Whisper"),
            tr!("Yell"),
            tr!("Private"),
            tr!("Guild"),
        ] {
            SendMessageW(rooms, LB_ADDSTRING, 0, wide(mode).as_ptr() as isize);
        }
        SendMessageW(rooms, LB_SETCURSEL, 0, 0);
        let message = control(
            hwnd,
            "EDIT",
            "",
            MESSAGE,
            [250, 10, 430, 28],
            WS_BORDER | WS_TABSTOP,
        );
        let recipient = control(
            hwnd,
            "EDIT",
            "",
            RECIPIENT,
            [250, 48, 270, 28],
            WS_BORDER | WS_TABSTOP,
        );
        control(
            hwnd,
            "STATIC",
            tr!("Private recipient"),
            RECIPIENT_LABEL,
            [10, 200, 230, 28],
            0,
        );
        control(
            hwnd,
            "BUTTON",
            tr!("Send"),
            SEND,
            [540, 48, 100, 28],
            WS_TABSTOP,
        );
        let transcript = control(
            hwnd,
            "EDIT",
            "",
            106,
            [250, 86, 430, 380],
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
        );
        let tabs = control(
            hwnd,
            "LISTBOX",
            "",
            TAB,
            [10, 245, 230, 75],
            LBS_NOTIFY as u32 | WS_BORDER,
        );
        for tab in [tr!("All"), tr!("Local"), tr!("Private"), tr!("Guild")] {
            SendMessageW(tabs, LB_ADDSTRING, 0, wide(tab).as_ptr() as isize);
        }
        SendMessageW(tabs, LB_SETCURSEL, 0, 0);
        control(
            hwnd,
            "BUTTON",
            tr!("Toggle messages"),
            FILTER,
            [10, 325, 230, 28],
            WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            tr!("Chat on/off/temporary"),
            CHAT_MODE,
            [10, 360, 230, 28],
            WS_TABSTOP,
        );
        for (id, title, y) in [
            (IGNORE, tr!("Ignore recipient"), 400),
            (ALLOW, tr!("Whitelist recipient"), 435),
            (BLOCK, tr!("Blacklist recipient"), 470),
            (CLEAR_LISTS, tr!("Remove recipient from lists"), 505),
        ] {
            control(hwnd, "BUTTON", title, id, [10, y, 230, 28], WS_TABSTOP);
        }
        let log_path = control(
            hwnd,
            "EDIT",
            "chat.log",
            LOG_PATH,
            [250, 480, 300, 28],
            WS_BORDER | WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            tr!("Save log"),
            SAVE,
            [250, 515, 140, 28],
            WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            tr!("Append log"),
            APPEND,
            [400, 515, 140, 28],
            WS_TABSTOP,
        );
        let mut private = PrivateControls::new(hwnd);
        let mut font = console_font::ConsoleFont::default();
        let mut console = console_controls::Controls::new(hwnd);
        let mut channels = channel_dialog::Controls::default();
        // Reserve an independent strip above the console's existing controls.
        let mut child = GetWindow(hwnd, GW_CHILD);
        while !child.is_null() {
            let mut rect: windows_sys::Win32::Foundation::RECT = std::mem::zeroed();
            GetWindowRect(child, &mut rect);
            let mut point = windows_sys::Win32::Foundation::POINT {
                x: rect.left,
                y: rect.top,
            };
            windows_sys::Win32::Graphics::Gdi::ScreenToClient(hwnd, &mut point);
            MoveWindow(
                child,
                point.x,
                point.y + 56,
                rect.right - rect.left,
                rect.bottom - rect.top,
                1,
            );
            child = GetWindow(child, GW_HWNDNEXT);
        }
        let mut cooldown = cooldown_bar::CooldownBar::new(hwnd);
        let mut mirror = mirror::Mirror::new(hwnd);
        let mut held = String::new();
        let mut held_labels = Vec::new();
        let mut msg: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                mirror.input(&msg, tabs, &weak);
                if keyboard(&msg, &weak, &commands, &mut actions.keyboard_input) {
                    continue;
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(shared) = weak.upgrade() else { break };
            let mut mailbox = lock(&shared);
            cooldown.update(
                crate::play::feature_local_spell_action_bar::view::cooldown_bar_icons(&mailbox),
                mailbox.local_preferences.action_bar.cooldown_bar_enabled,
            );
            if mailbox.chat_public_shutdown || !mailbox.chat.open {
                break;
            }
            if let Some(paste) = mailbox.custom_chat_paste.take() {
                SetWindowTextW(message, wide(&paste).as_ptr());
                SetForegroundWindow(hwnd);
                SetFocus(message);
            }
            mailbox.chat.text = read_text(message);
            crate::play::feature_0233_player_typing::input::composer(
                &mut mailbox,
                &commands,
                GetForegroundWindow() == hwnd && GetFocus() == message,
            );
            if mailbox.chat.guild_open {
                SendMessageW(rooms, LB_SETCURSEL, 4, 0);
                mailbox.chat.guild_open = false;
            }
            let guild_mode = SendMessageW(rooms, LB_GETCURSEL, 0, 0) == 4;
            let enabled = crate::play::feature_0007_chat::input::composer_enabled(
                &mailbox.chat,
                if guild_mode { Mode::Guild } else { Mode::Say },
            );
            EnableWindow(message, i32::from(enabled));
            EnableWindow(GetDlgItem(hwnd, SEND as i32), i32::from(enabled));
            for action in std::mem::take(&mut actions.pending) {
                if console.action(action, &mut mailbox, &commands, transcript) {
                    continue;
                }
                if private.action(action, &mut mailbox, &commands) {
                    continue;
                }
                if channels.action(action, &mut mailbox, &commands) {
                    continue;
                }
                match action {
                    IGNORE | ALLOW | BLOCK | CLEAR_LISTS => {
                        use oteryn_client_ui::chat::CommunicationList;
                        let name = read_text(recipient);
                        let list = match action {
                            ALLOW => CommunicationList::Whitelist,
                            BLOCK => CommunicationList::Blacklist,
                            _ => CommunicationList::Ignore,
                        };
                        console::edit_lists(
                            &mut mailbox,
                            if action == CLEAR_LISTS {
                                None
                            } else {
                                Some(list)
                            },
                            &name,
                            action != CLEAR_LISTS,
                        );
                        continue;
                    }
                    SAVE | APPEND => {
                        let path = read_text(log_path);
                        drop(mailbox);
                        let _ = console::save_log(
                            &shared,
                            std::path::Path::new(&path),
                            action == APPEND,
                        );
                        mailbox = lock(&shared);
                        continue;
                    }
                    TAB => {
                        mailbox.chat.console.select_index(
                            usize::try_from(SendMessageW(tabs, LB_GETCURSEL, 0, 0)).unwrap_or(0),
                        );
                        continue;
                    }
                    FILTER => {
                        console::toggle_filter(&mut mailbox);
                        continue;
                    }
                    CHAT_MODE => {
                        console::cycle_mode(&mut mailbox);
                        continue;
                    }
                    _ => {}
                }
                if action != SEND {
                    continue;
                }
                let mode = match SendMessageW(rooms, LB_GETCURSEL, 0, 0) {
                    1 => Mode::Whisper,
                    2 => Mode::Yell,
                    3 => Mode::Private,
                    4 => Mode::Guild,
                    _ => Mode::Say,
                };
                mailbox.chat.text = read_text(message);
                crate::play::feature_0007_chat::input::enqueue(
                    &mut mailbox,
                    &commands,
                    mode,
                    &read_text(recipient),
                );
                if mailbox.chat.pending {
                    SetWindowTextW(message, wide("").as_ptr());
                }
            }
            // Opened channel tabs follow the console state after the fixed tabs.
            let labels = mailbox.chat.console.tab_labels();
            if console::update_tab_labels(&mut held_labels, &labels) {
                SendMessageW(tabs, LB_RESETCONTENT, 0, 0);
                for label in &labels {
                    SendMessageW(tabs, LB_ADDSTRING, 0, wide(label).as_ptr() as isize);
                }
            }
            SendMessageW(tabs, LB_SETCURSEL, mailbox.chat.console.tab_index(), 0);
            console.tick(&mut mailbox, &commands, transcript);
            channels.tick(&mut mailbox, hwnd);
            font.update(transcript, mailbox.chat.console.display.font_px);
            private.render(&mailbox.chat_private);
            mirror.render(&mailbox.chat, transcript);
            let text = format!(
                "{}\n{}",
                panel_text(&mailbox.chat),
                oteryn_client_ui::player_typing::panel_text(&mailbox.player_typing)
            );
            if held != text {
                SetWindowTextW(transcript, wide(&text).as_ptr());
                held = text;
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            let _ = DestroyWindow(hwnd);
        }
        if let Some(shared) = weak.upgrade() {
            crate::play::feature_0233_player_typing::input::composer(
                &mut lock(&shared),
                &commands,
                false,
            );
        }
        close(&weak);
    }
}

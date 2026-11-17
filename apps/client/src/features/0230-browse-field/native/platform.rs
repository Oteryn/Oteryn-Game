#[allow(unsafe_code)]
mod platform {
    use super::{enabled, item_command, result_text, submit};
    use crate::combat_input::PlayCommand;
    use crate::play::{PlayLink, mailbox::lock};
    use oteryn_client_ui::browse_field::BrowseFieldPanel;
    use std::ptr::{null, null_mut};
    use std::sync::{Arc, Weak};
    use std::time::Duration;
    use tokio::sync::mpsc::UnboundedSender;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    const OPEN: usize = 103;
    const SEEK: usize = 104;
    type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;
    #[derive(Default)]
    struct Actions(Vec<usize>);
    unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        unsafe {
            if msg == WM_COMMAND {
                let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
                let id = w & 0xffff;
                let note = (w >> 16) & 0xffff;
                if !actions.is_null()
                    && matches!(id, OPEN | SEEK | 105..=110)
                    && note == BN_CLICKED as usize
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
            let mut m = lock(&link.mailbox);
            if m.browse_field.open {
                return;
            }
            m.browse_field.open = true;
        }
        let weak = Arc::downgrade(&link.mailbox);
        let rollback = weak.clone();
        let commands = link.commands.clone();
        if std::thread::Builder::new()
            .name("browse-field-panel".into())
            .spawn(move || run(weak, commands))
            .is_err()
        {
            close(&rollback);
        }
    }
    fn run(weak: WeakMailbox, commands: UnboundedSender<PlayCommand>) {
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("Browse field").as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                920,
                600,
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
            let fields = [
                ("Tile X", "0"),
                ("Tile Y", "0"),
                ("Floor", "0"),
                ("View id", "0"),
                ("Page index", "0"),
                ("Field item row", "0"),
                ("Backpack item row", "0"),
            ];
            let edits = std::array::from_fn::<_, 7, _>(|i| {
                let x = 10 + i as i32 * 118;
                control(hwnd, "STATIC", fields[i].0, 120 + i, [x, 10, 110, 24], 0);
                control(
                    hwnd,
                    "EDIT",
                    fields[i].1,
                    130 + i,
                    [x, 40, 110, 24],
                    WS_BORDER | WS_TABSTOP,
                )
            });
            let buttons = [
                ("Open tile", OPEN),
                ("Seek page", SEEK),
                ("Take to backpack", 105),
                ("Drop here", 106),
                ("Open container", 107),
                ("Up", 108),
                ("Use selected", 109),
                ("Use backpack item", 110),
            ]
            .map(|(label, id)| {
                control(
                    hwnd,
                    "BUTTON",
                    label,
                    id,
                    [
                        10 + ((id - OPEN) % 4) as i32 * 225,
                        80 + ((id - OPEN) / 4) as i32 * 35,
                        215,
                        30,
                    ],
                    WS_TABSTOP,
                )
            });
            let description = control(
                hwnd,
                "EDIT",
                "",
                112,
                [10, 160, 885, 385],
                ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
            );
            let mut held: Option<BrowseFieldPanel> = None;
            let mut message: MSG = std::mem::zeroed();
            loop {
                while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                if IsWindow(hwnd) == 0 {
                    break;
                }
                let Some(shared) = weak.upgrade() else { break };
                let mut mailbox = lock(&shared);
                if commands.is_closed() || !mailbox.browse_field.open {
                    break;
                }
                for action in std::mem::take(&mut actions.0) {
                    let values = edits.map(|edit| read_text(edit));
                    let command = if matches!(action, OPEN | SEEK) {
                        submit(
                            &mut mailbox.browse_field,
                            action == SEEK,
                            std::array::from_fn(|i| values[i].as_str()),
                        )
                        .map(PlayCommand::ServerMechanic)
                    } else if let Some(displayed) = &held {
                        // Retain the handles that were displayed if a push arrived before redraw.
                        let mut panel = displayed.clone();
                        let command = item_command(&mut panel, action, &values[5], &values[6]);
                        if command.is_some() && enabled(&mailbox.browse_field) {
                            mailbox.browse_field.pending =
                                !matches!(&command, Some(PlayCommand::WorldContext(_)));
                            mailbox.browse_field.feedback = None;
                            command
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    if let Some(command) = command
                        && commands.send(command).is_err()
                    {
                        mailbox.browse_field.pending = false;
                        mailbox.browse_field.feedback = Some("The session has ended".into());
                    }
                }
                if held.as_ref() != Some(&mailbox.browse_field) {
                    let panel = &mailbox.browse_field;
                    if held.is_none() {
                        for (edit, value) in edits.iter().zip([
                            panel.x.to_string(),
                            panel.y.to_string(),
                            panel.floor.to_string(),
                            panel.view_id.to_string(),
                            panel.index.to_string(),
                        ]) {
                            SetWindowTextW(*edit, wide(&value).as_ptr());
                        }
                    }
                    for control in edits.into_iter().chain(buttons) {
                        windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                            control,
                            i32::from(enabled(panel)),
                        );
                    }
                    SetWindowTextW(
                        description,
                        wide(&result_text(panel).replace('\n', "\r\n")).as_ptr(),
                    );
                    held = Some(mailbox.browse_field.clone());
                }
                drop(mailbox);
                std::thread::sleep(Duration::from_millis(30));
            }
            if IsWindow(hwnd) != 0 {
                let _ = DestroyWindow(hwnd);
            }
            close(&weak);
        }
    }
    fn close(weak: &WeakMailbox) {
        if let Some(m) = weak.upgrade() {
            lock(&m).browse_field.open = false;
        }
    }
    unsafe fn control(
        parent: HWND,
        class: &str,
        text: &str,
        id: usize,
        b: [i32; 4],
        style: u32,
    ) -> HWND {
        unsafe {
            CreateWindowExW(
                0,
                wide(class).as_ptr(),
                wide(text).as_ptr(),
                WS_CHILD | WS_VISIBLE | style,
                b[0],
                b[1],
                b[2],
                b[3],
                parent,
                id as _,
                null_mut(),
                null(),
            )
        }
    }
    unsafe fn read_text(hwnd: HWND) -> String {
        unsafe {
            let length = GetWindowTextLengthW(hwnd);
            let mut value = vec![0; usize::try_from(length).unwrap_or(0) + 1];
            let read = GetWindowTextW(hwnd, value.as_mut_ptr(), length + 1);
            String::from_utf16_lossy(&value[..usize::try_from(read).unwrap_or(0)])
        }
    }
}

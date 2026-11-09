//! Native boss difficulty controls and platform-independent form validation.
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_client_ui::boss_difficulty::{BossDifficultyPanel, Intent, request};

pub(super) fn enabled(panel: &BossDifficultyPanel) -> bool {
    panel.selected && !panel.pending
}

pub(super) fn submit(
    panel: &mut BossDifficultyPanel,
    intent: Intent,
    boss: &str,
    context: &str,
    difficulty: &str,
) -> Option<FeatureCommand> {
    if !enabled(panel) {
        return None;
    }
    if matches!(intent, Intent::Open) {
        panel.boss_key = boss.trim().to_owned();
    } else {
        let text = context.trim();
        let parsed = (|| {
            if text.len() != 32 || !text.is_ascii() {
                return None;
            }
            let mut id = [0; 16];
            for (slot, pair) in id.iter_mut().zip(text.as_bytes().as_chunks::<2>().0) {
                *slot = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
            }
            (id != [0; 16]).then_some(id)
        })();
        let Some(id) = parsed else {
            panel.feedback = Some(tr!("Enter a nonzero context as 32 hex digits"));
            return None;
        };
        if !matches!(intent, Intent::Cancel) {
            let Some(value) = difficulty.trim().parse::<u16>().ok().filter(|d| *d <= 25) else {
                panel.feedback = Some(tr!("Enter a difficulty from 0 to 25"));
                return None;
            };
            panel.difficulty = value;
        }
        panel.context = Some(id);
    }
    request(panel, intent)
}

#[cfg(windows)]
pub(in crate::play) use platform::open_window;

#[cfg(windows)]
// Win32 controls are owned and accessed exclusively by the panel thread.
#[allow(unsafe_code)]
mod platform {
    use super::{enabled, submit};
    use crate::combat_input::PlayCommand;
    use crate::play::{PlayLink, mailbox::lock};
    use oteryn_client_ui::boss_difficulty::{BossDifficultyPanel, Intent, panel_text};
    use std::ptr::{null, null_mut};
    use std::sync::{Arc, Weak};
    use std::time::Duration;
    use tokio::sync::mpsc::UnboundedSender;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    const BOSS: usize = 101;
    const CONTEXT: usize = 102;
    const DIFFICULTY: usize = 107;
    const OPEN: usize = 103;
    const PREVIEW: usize = 104;
    const START: usize = 105;
    const CANCEL: usize = 106;
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
                    && matches!(id, OPEN | PREVIEW | START | CANCEL)
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
            if m.boss_difficulty.open {
                return;
            }
            m.boss_difficulty.open = true;
        }
        let weak = Arc::downgrade(&link.mailbox);
        let rollback = weak.clone();
        let commands = link.commands.clone();
        if std::thread::Builder::new()
            .name("boss-difficulty-panel".into())
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
                wide(tr!("Boss difficulty")).as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                650,
                480,
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
            control(hwnd, "STATIC", tr!("Boss id"), 110, [10, 10, 190, 24], 0);
            let boss = control(
                hwnd,
                "EDIT",
                "",
                BOSS,
                [205, 10, 400, 24],
                WS_BORDER | WS_TABSTOP,
            );
            control(
                hwnd,
                "STATIC",
                tr!("Server context (automatic)"),
                111,
                [10, 45, 190, 24],
                0,
            );
            let context = control(
                hwnd,
                "EDIT",
                "",
                CONTEXT,
                [205, 45, 400, 24],
                WS_BORDER | ES_READONLY as u32,
            );
            control(
                hwnd,
                "STATIC",
                tr!("Difficulty (0–25)"),
                113,
                [10, 80, 190, 24],
                0,
            );
            let difficulty = control(
                hwnd,
                "COMBOBOX",
                "",
                DIFFICULTY,
                [205, 80, 160, 240],
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP,
            );
            for level in 0..=25 {
                SendMessageW(
                    difficulty,
                    CB_ADDSTRING,
                    0,
                    wide(&level.to_string()).as_ptr() as isize,
                );
            }
            SendMessageW(difficulty, CB_SETCURSEL, 0, 0);
            let buttons = [
                (tr!("Open"), OPEN, Intent::Open),
                (tr!("Preview"), PREVIEW, Intent::Preview),
                (tr!("Start"), START, Intent::Start),
                (tr!("Cancel"), CANCEL, Intent::Cancel),
            ]
            .map(|(label, id, intent)| {
                (
                    control(
                        hwnd,
                        "BUTTON",
                        label,
                        id,
                        [10 + (id as i32 - OPEN as i32) * 145, 115, 135, 30],
                        WS_TABSTOP,
                    ),
                    intent,
                )
            });
            let description = control(
                hwnd,
                "EDIT",
                "",
                112,
                [10, 155, 595, 265],
                ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
            );
            let mut held: Option<BossDifficultyPanel> = None;
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
                if commands.is_closed() {
                    break;
                }
                for action in std::mem::take(&mut actions.0) {
                    let intent = match action {
                        OPEN => Intent::Open,
                        PREVIEW => Intent::Preview,
                        START => Intent::Start,
                        CANCEL => Intent::Cancel,
                        _ => continue,
                    };
                    if let Some(command) = submit(
                        &mut mailbox.boss_difficulty,
                        intent,
                        &read_text(boss),
                        &read_text(context),
                        &read_text(difficulty),
                    ) && commands.send(PlayCommand::Feature(command)).is_err()
                    {
                        mailbox.boss_difficulty.pending = false;
                        mailbox.boss_difficulty.feedback = Some(tr!("The session has ended"));
                    }
                }
                if held.as_ref() != Some(&mailbox.boss_difficulty) {
                    let panel = &mailbox.boss_difficulty;
                    if held.as_ref().map(|p| p.context) != Some(panel.context) {
                        let value = panel
                            .context
                            .map(|id| {
                                id.iter()
                                    .map(|byte| format!("{byte:02x}"))
                                    .collect::<String>()
                            })
                            .unwrap_or_default();
                        SetWindowTextW(context, wide(&value).as_ptr());
                    }
                    if held.is_none() {
                        SetWindowTextW(boss, wide(&panel.boss_key).as_ptr());
                        SendMessageW(difficulty, CB_SETCURSEL, usize::from(panel.difficulty), 0);
                    }
                    if let Some(oteryn_client_runtime::boss_difficulty::Reply::Window {
                        selected_difficulty,
                        ..
                    }) = &panel.reply
                        && held.as_ref().and_then(|p| p.reply.as_ref()) != panel.reply.as_ref()
                    {
                        SendMessageW(
                            difficulty,
                            CB_SETCURSEL,
                            usize::from(*selected_difficulty),
                            0,
                        );
                    }
                    for (button, intent) in &buttons {
                        windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                            *button,
                            i32::from(
                                enabled(panel)
                                    && (matches!(intent, Intent::Open) || panel.context.is_some()),
                            ),
                        );
                    }
                    for edit in [boss, context, difficulty] {
                        windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                            edit,
                            i32::from(panel.selected && !panel.pending),
                        );
                    }
                    SetWindowTextW(
                        description,
                        wide(&format!(
                            "{}\r\n{}",
                            panel_text(&mailbox.boss_difficulty).replace('\n', "\r\n"),
                            if mailbox.boss_difficulty.pending {
                                tr!("Waiting for the server")
                            } else {
                                ""
                            }
                        ))
                        .as_ptr(),
                    );
                    held = Some(mailbox.boss_difficulty.clone());
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
            lock(&m).boss_difficulty.open = false;
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

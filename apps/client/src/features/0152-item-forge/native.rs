//! Native presentation of the existing feature request and reducer state.
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::item_forge::ItemForgePanel;

pub(in crate::play) const CONTROLS: &[(&str, &str, &str)] = &[
    (
        "EDIT",
        "Operation (8 current dust; 0 history; 1 fusion; 2 convergence fusion; 3 transfer; 4 convergence transfer; 5 slivers; 6 cores; 7 dust limit)",
        "0",
    ),
    ("EDIT", "First item handle", "0"),
    ("EDIT", "Second item handle", "0"),
    (
        "EDIT",
        "Occurrence (32 hex digits; required for mutations)",
        "00000000000000000000000000000000",
    ),
    ("EDIT", "Boost (0 or 1)", "0"),
    ("EDIT", "Protect (0 or 1)", "0"),
    ("EDIT", "History limit (0–50)", "20"),
    ("BUTTON", "Submit", ""),
];
pub(in crate::play) struct Inputs(pub Vec<String>);
impl Default for Inputs {
    fn default() -> Self {
        Self(CONTROLS.iter().map(|c| c.2.to_owned()).collect())
    }
}
pub(in crate::play) fn enabled(panel: &ItemForgePanel) -> bool {
    panel.selected && !panel.pending
}
pub(in crate::play) fn presentation(panel: &ItemForgePanel) -> String {
    let mut text = oteryn_client_ui::item_forge::panel_text(panel);
    if panel.pending {
        text.push_str(tr!("\nWaiting for server…"));
    }
    if panel.decoded.is_none() && !panel.response.is_empty() {
        match serde_json::from_slice::<serde_json::Value>(&panel.response) {
            Ok(value) => text.push_str(&lfmt!("\nResult: {value:#}", value = value)),
            Err(_) => text.push_str(tr!("\nThe server returned invalid forge results")),
        }
    }
    text
}
pub(in crate::play) fn submit(link: &PlayLink, inputs: &Inputs) {
    dispatch(link, inputs, false);
}
pub(in crate::play) fn dispatch(link: &PlayLink, inputs: &Inputs, detail: bool) {
    let mut mailbox = lock(&link.mailbox);
    let panel = &mut mailbox.item_forge;
    if !enabled(panel) {
        return;
    }
    let parsed = (|| {
        let _ = detail;
        use oteryn_client_runtime::cui3::item_forge::Operation;
        let chosen = inputs.0.first()?.trim().parse::<u8>().ok()?;
        panel.query_current_dust = chosen == 8;
        if panel.query_current_dust {
            return Some(());
        }
        let operation = match chosen {
            0 => Operation::QueryHistory,
            1 => Operation::Fusion,
            2 => Operation::ConvergenceFusion,
            3 => Operation::Transfer,
            4 => Operation::ConvergenceTransfer,
            5 => Operation::DustToSlivers,
            6 => Operation::SliversToCores,
            7 => Operation::DustLimit,
            _ => return None,
        };
        let first = inputs.0.get(1)?.trim().parse().ok()?;
        let second = inputs.0.get(2)?.trim().parse().ok()?;
        let hex = inputs.0.get(3)?.trim();
        if hex.len() != 32 || !hex.is_ascii() {
            return None;
        }
        let mut occurrence = [0; 16];
        for (i, byte) in occurrence.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
        }
        let boolean = |index: usize| match inputs.0.get(index)?.trim() {
            "0" => Some(false),
            "1" => Some(true),
            _ => None,
        };
        let boost = boolean(4)?;
        let protect = boolean(5)?;
        let limit = inputs.0.get(6)?.trim().parse().ok()?;
        panel.operation = operation;
        panel.first_item = first;
        panel.second_item = second;
        panel.occurrence = occurrence;
        panel.boost = boost;
        panel.protect = protect;
        panel.history_limit = limit;
        Some(())
    })();
    if parsed.is_none() {
        panel.feedback = Some(tr!("Enter valid values for the labelled controls").into());
        return;
    }
    let command = oteryn_client_ui::item_forge::control_request(panel);
    if let Some(command) = command
        && link
            .commands
            .send(crate::combat_input::PlayCommand::ServerMechanic(command))
            .is_err()
    {
        panel.pending = false;
        panel.feedback = Some(tr!("The session has ended").into());
    }
}

#[cfg(windows)]
pub(in crate::play) use windows::open_window;
#[cfg(windows)]
mod windows {
    #![allow(unsafe_code)]
    use super::*;
    use std::ptr::{null, null_mut};
    use std::sync::Arc;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    #[derive(Default)]
    struct Actions(Vec<usize>);
    unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        // SAFETY: userdata points to this thread's Actions box until destruction.
        unsafe {
            if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
                let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
                if !actions.is_null() && (*actions).0.len() < 16 {
                    (*actions).0.push(w & 0xffff);
                }
                return 0;
            }
            DefWindowProcW(hwnd, msg, w, l)
        }
    }
    fn wide(s: &str) -> Vec<u16> {
        s.replace('\0', "�").encode_utf16().chain(Some(0)).collect()
    }
    pub(in crate::play) fn open_window(link: &PlayLink) {
        {
            let mut mailbox = lock(&link.mailbox);
            if mailbox.item_forge.open {
                return;
            }
            mailbox.item_forge.open = true;
        }
        let weak = Arc::downgrade(&link.mailbox);
        let rollback = weak.clone();
        let commands = link.commands.clone();
        if std::thread::Builder::new()
            .name("item_forge-panel".into())
            .spawn(move || {
                run(weak, commands);
            })
            .is_err()
            && let Some(shared) = rollback.upgrade()
        {
            lock(&shared).item_forge.open = false;
        }
    }
    fn run(
        weak: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
        commands: tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    ) {
        // SAFETY: every HWND, message and userdata access stays on this creating thread.
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("item-forge").as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                900,
                800,
                null_mut(),
                null_mut(),
                null_mut(),
                null(),
            );
            if !hwnd.is_null() {
                let mut actions = Box::<Actions>::default();
                SetWindowLongPtrW(
                    hwnd,
                    GWLP_USERDATA,
                    (&mut *actions as *mut Actions) as isize,
                );
                SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
                let mut handles = Vec::new();
                let mut labels = Vec::new();
                for (index, (class, label, value)) in CONTROLS.iter().enumerate() {
                    let label = oteryn_client_ui::localization::tr(label);
                    let y = 10 + index as i32 * 65;
                    labels.push(control(
                        hwnd,
                        "STATIC",
                        label,
                        200 + index,
                        [10, y, 860, 22],
                        0,
                    ));
                    handles.push(control(
                        hwnd,
                        class,
                        if *class == "BUTTON" { label } else { value },
                        100 + index,
                        [10, y + 24, 860, 28],
                        WS_TABSTOP | WS_BORDER,
                    ));
                }
                let status = control(
                    hwnd,
                    "EDIT",
                    "",
                    300,
                    [10, 540, 860, 210],
                    ES_MULTILINE as u32
                        | ES_READONLY as u32
                        | ES_AUTOVSCROLL as u32
                        | WS_VSCROLL
                        | WS_BORDER
                        | WS_TABSTOP,
                );
                if !status.is_null() && handles.iter().chain(labels.iter()).all(|h| !h.is_null()) {
                    let mut held = None;
                    loop {
                        let mut message: MSG = std::mem::zeroed();
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
                        if lock(&shared).quest_shutdown {
                            break;
                        }
                        let link = PlayLink {
                            mailbox: shared.clone(),
                            commands: commands.clone(),
                        };
                        for id in std::mem::take(&mut actions.0) {
                            if id < 100
                                || id >= 100 + CONTROLS.len()
                                || CONTROLS[id - 100].0 != "BUTTON"
                            {
                                continue;
                            }
                            let inputs = Inputs(
                                handles
                                    .iter()
                                    .map(|handle| {
                                        let mut buffer = [0u16; 256];
                                        let len = GetWindowTextW(
                                            *handle,
                                            buffer.as_mut_ptr(),
                                            buffer.len() as i32,
                                        );
                                        String::from_utf16_lossy(&buffer[..len.max(0) as usize])
                                    })
                                    .collect(),
                            );
                            if id == 100 + CONTROLS.len() - 1 && CONTROLS.len() == 5 {
                                dispatch(&link, &inputs, true);
                            } else {
                                submit(&link, &inputs);
                            }
                        }
                        let panel = lock(&shared).item_forge.clone();
                        if !panel.open {
                            break;
                        }
                        if held.as_ref() != Some(&panel) {
                            SetWindowTextW(
                                status,
                                wide(&presentation(&panel).replace('\n', "\r\n")).as_ptr(),
                            );
                            for handle in &handles {
                                EnableWindow(*handle, i32::from(enabled(&panel)));
                            }
                            held = Some(panel);
                        }
                        std::thread::sleep(std::time::Duration::from_millis(30));
                    }
                }
                if IsWindow(hwnd) != 0 {
                    DestroyWindow(hwnd);
                }
            }
            if let Some(shared) = weak.upgrade() {
                lock(&shared).item_forge.open = false;
            }
        }
    }
    unsafe fn control(
        parent: HWND,
        class: &str,
        text: &str,
        id: usize,
        bounds: [i32; 4],
        style: u32,
    ) -> HWND {
        // SAFETY: parent belongs to this thread; strings survive the synchronous call.
        unsafe {
            CreateWindowExW(
                0,
                wide(class).as_ptr(),
                wide(text).as_ptr(),
                WS_CHILD | WS_VISIBLE | style,
                bounds[0],
                bounds[1],
                bounds[2],
                bounds[3],
                parent,
                id as _,
                null_mut(),
                null(),
            )
        }
    }
}

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
        if mailbox.depot_search.open {
            return;
        }
        mailbox.depot_search.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("depot_search-panel".into())
        .spawn(move || {
            run(weak, commands);
        })
        .is_err()
        && let Some(shared) = rollback.upgrade()
    {
        lock(&shared).depot_search.open = false;
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
            wide("depot-search").as_ptr(),
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
                let y = 10 + index.min(5) as i32 * 55;
                let x = if index >= 5 {
                    10 + (index - 5) as i32 * 215
                } else {
                    10
                };
                let width = if index >= 5 { 205 } else { 860 };
                labels.push(control(
                    hwnd,
                    "STATIC",
                    label,
                    200 + index,
                    [x, y, width, 22],
                    0,
                ));
                handles.push(control(
                    hwnd,
                    class,
                    if *class == "BUTTON" { label } else { value },
                    100 + index,
                    [x, y + 24, width, 28],
                    WS_TABSTOP | WS_BORDER,
                ));
            }
            let results = control(
                hwnd,
                "LISTBOX",
                "",
                400,
                [10, 350, 860, 175],
                LBS_NOTIFY as u32 | WS_VSCROLL | WS_BORDER | WS_TABSTOP,
            );
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
            if !results.is_null()
                && !status.is_null()
                && handles.iter().chain(labels.iter()).all(|h| !h.is_null())
            {
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
                    for id in std::mem::take(&mut actions.0) {
                        if id < 100
                            || id >= 100 + CONTROLS.len()
                            || CONTROLS[id - 100].0 != "BUTTON"
                        {
                            continue;
                        }
                        let mut inputs = Inputs(
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
                        if let Ok(row) = usize::try_from(SendMessageW(results, LB_GETCURSEL, 0, 0))
                        {
                            inputs.0[2] = row.to_string();
                            SetWindowTextW(handles[2], wide(&inputs.0[2]).as_ptr());
                        }
                        activate_displayed(
                            &shared,
                            &commands,
                            &inputs,
                            id - 100,
                            held.as_ref()
                                .and_then(|p: &DepotSearchPanel| p.reply.as_ref()),
                        );
                    }
                    let panel = lock(&shared).depot_search.clone();
                    if held.as_ref() != Some(&panel) {
                        let selected = SendMessageW(results, LB_GETCURSEL, 0, 0);
                        SendMessageW(results, LB_RESETCONTENT, 0, 0);
                        if let Some(reply) = &panel.reply {
                            for (index, row) in reply.results.iter().enumerate() {
                                let tiers = row
                                    .tiers
                                    .iter()
                                    .map(|t| lfmt!("tier {}: {}", t.tier, t.count))
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                let text = lfmt!(
                                    "{} | Item {} | Depot {} | Stash {} | Inbox {} | {}",
                                    index,
                                    row.client_id,
                                    row.depot,
                                    row.stash,
                                    row.inbox,
                                    tiers
                                );
                                SendMessageW(
                                    results,
                                    LB_ADDSTRING,
                                    0,
                                    wide(&text).as_ptr() as isize,
                                );
                            }
                            if !reply.results.is_empty() {
                                SendMessageW(
                                    results,
                                    LB_SETCURSEL,
                                    usize::try_from(selected)
                                        .unwrap_or(0)
                                        .min(reply.results.len() - 1),
                                    0,
                                );
                            }
                        }
                        EnableWindow(results, i32::from(enabled(&panel)));
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
            lock(&shared).depot_search.open = false;
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

//! Native container browser; window handles and userdata remain on the creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::container_tree::ContainerPanel;
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
    time::Duration,
};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::*,
};

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: userdata is the live action queue owned by this window's thread.
    unsafe {
        if msg == WM_COMMAND {
            let id = (w & 0xffff) as u16;
            let notification = (w >> 16) & 0xffff;
            if ((101..=102).contains(&id) && notification == LBN_SELCHANGE as usize)
                || ((103..=106).contains(&id) && notification == BN_CLICKED as usize)
            {
                let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Vec<u16>;
                if !actions.is_null() {
                    (*actions).push(id);
                }
                return 0;
            }
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}

pub(in crate::play) fn open_window(link: &PlayLink) {
    open_shared(Arc::downgrade(&link.mailbox), link.commands.clone());
}

pub(in crate::play) fn open_shared(
    weak: Weak<Mutex<Mailbox>>,
    commands: UnboundedSender<PlayCommand>,
) {
    {
        let Some(mailbox) = weak.upgrade() else {
            return;
        };
        let mut mailbox = lock(&mailbox);
        if mailbox.inventory_shutdown || mailbox.containers.open {
            return;
        }
        mailbox.containers.open = true;
    }
    let failed = weak.clone();
    if std::thread::Builder::new()
        .name("container-browser".into())
        .spawn(move || {
            // SAFETY: creation, dispatch, userdata and destruction are confined to this thread.
            unsafe {
                let hwnd = CreateWindowExW(
                    0,
                    wide("STATIC").as_ptr(),
                    wide("Containers — carried and ground").as_ptr(),
                    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    520,
                    480,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null(),
                );
                if !hwnd.is_null() {
                    let mut actions: Box<Vec<u16>> = Box::default();
                    SetWindowLongPtrW(
                        hwnd,
                        GWLP_USERDATA,
                        (&mut *actions as *mut Vec<u16>) as isize,
                    );
                    SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
                    let list = |id: usize, y, height| {
                        CreateWindowExW(
                            0,
                            wide("LISTBOX").as_ptr(),
                            wide("").as_ptr(),
                            WS_CHILD
                                | WS_VISIBLE
                                | WS_BORDER
                                | WS_VSCROLL
                                | WS_TABSTOP
                                | LBS_NOTIFY as u32,
                            10,
                            y,
                            480,
                            height,
                            hwnd,
                            id as _,
                            null_mut(),
                            null(),
                        )
                    };
                    let views = list(101, 10, 80);
                    let entries = list(102, 100, 220);
                    for (id, label, x) in [
                        (103_usize, "Open selected", 10),
                        (104, "Up", 175),
                        (105, "Close view", 340),
                    ] {
                        CreateWindowExW(
                            0,
                            wide("BUTTON").as_ptr(),
                            wide(label).as_ptr(),
                            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                            x,
                            330,
                            150,
                            30,
                            hwnd,
                            id as _,
                            null_mut(),
                            null(),
                        );
                    }
                    let destination = CreateWindowExW(
                        0,
                        wide("EDIT").as_ptr(),
                        wide("Target view ID").as_ptr(),
                        WS_CHILD | WS_VISIBLE | WS_BORDER | WS_TABSTOP,
                        10,
                        370,
                        235,
                        28,
                        hwnd,
                        null_mut(),
                        null_mut(),
                        null(),
                    );
                    CreateWindowExW(
                        0,
                        wide("BUTTON").as_ptr(),
                        wide("Move all to view").as_ptr(),
                        WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                        255,
                        370,
                        235,
                        28,
                        hwnd,
                        106 as _,
                        null_mut(),
                        null(),
                    );
                    let status = CreateWindowExW(
                        0,
                        wide("STATIC").as_ptr(),
                        wide("").as_ptr(),
                        WS_CHILD | WS_VISIBLE,
                        10,
                        405,
                        480,
                        35,
                        hwnd,
                        null_mut(),
                        null_mut(),
                        null(),
                    );
                    let mut held: Option<ContainerPanel> = None;
                    let mut message: MSG = std::mem::zeroed();
                    while !views.is_null()
                        && !entries.is_null()
                        && !status.is_null()
                        && IsWindow(hwnd) != 0
                    {
                        while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                            if IsDialogMessageW(hwnd, &message) == 0 {
                                TranslateMessage(&message);
                                DispatchMessageW(&message);
                            }
                        }
                        if IsWindow(hwnd) == 0 {
                            break;
                        }
                        let Some(mailbox) = weak.upgrade() else {
                            break;
                        };
                        let mut mailbox = lock(&mailbox);
                        if mailbox.inventory_shutdown {
                            break;
                        }
                        for action in actions.drain(..) {
                            if action == 106 {
                                let mut text = [0u16; 20];
                                let len = GetWindowTextW(destination, text.as_mut_ptr(), 20).max(0)
                                    as usize;
                                if let Ok(id) =
                                    String::from_utf16_lossy(&text[..len]).trim().parse::<u8>()
                                {
                                    super::super::input::bulk::begin(&mut mailbox, &commands, id);
                                }
                                continue;
                            }
                            let panel = &mut mailbox.containers;
                            let intent = match action {
                                101 => {
                                    let row = SendMessageW(views, LB_GETCURSEL, 0, 0);
                                    if let Some(view) = usize::try_from(row)
                                        .ok()
                                        .and_then(|i| held.as_ref()?.views.as_ref()?.views.get(i))
                                    {
                                        panel.activate(view.view_id);
                                    }
                                    None
                                }
                                102 => {
                                    let row = SendMessageW(entries, LB_GETCURSEL, 0, 0);
                                    if let Some((id, item)) =
                                        usize::try_from(row).ok().and_then(|i| {
                                            Some((
                                                held.as_ref()?.active?,
                                                held.as_ref()?.rows().get(i)?,
                                            ))
                                        })
                                        && panel.active == Some(id)
                                    {
                                        panel.select(item.handle);
                                    }
                                    None
                                }
                                103 => panel.open_selected(),
                                104 => panel.up(),
                                105 => panel.close(),
                                _ => None,
                            };
                            if let Some(intent) = intent
                                && commands.send(PlayCommand::ContainerView(intent)).is_err()
                            {
                                panel.end();
                            }
                        }
                        let panel = &mut mailbox.containers;
                        if held.as_ref() != Some(panel) {
                            SendMessageW(views, LB_RESETCONTENT, 0, 0);
                            if let Some(state) = &panel.views {
                                for (i, view) in state.views.iter().enumerate() {
                                    SendMessageW(
                                        views,
                                        LB_ADDSTRING,
                                        0,
                                        wide(&format!(
                                            "View {}: container {}",
                                            view.view_id, view.container_handle
                                        ))
                                        .as_ptr() as isize,
                                    );
                                    if panel.active == Some(view.view_id) {
                                        SendMessageW(views, LB_SETCURSEL, i, 0);
                                    }
                                }
                            }
                            SendMessageW(entries, LB_RESETCONTENT, 0, 0);
                            for (i, item) in panel.rows().iter().enumerate() {
                                SendMessageW(
                                    entries,
                                    LB_ADDSTRING,
                                    0,
                                    wide(&format!(
                                        "Item {} ×{}",
                                        item.item_definition_ref, item.count
                                    ))
                                    .as_ptr() as isize,
                                );
                                if panel
                                    .selected_item()
                                    .is_some_and(|s| s.handle == item.handle)
                                {
                                    SendMessageW(entries, LB_SETCURSEL, i, 0);
                                }
                            }
                            SetWindowTextW(
                                status,
                                wide(
                                    &oteryn_client_ui::house_container_capacity::view::panel_text(
                                        panel,
                                    ),
                                )
                                .as_ptr(),
                            );
                            held = Some(panel.clone());
                        }
                        SetWindowTextW(
                            status,
                            wide(&format!(
                                "{}\n{}",
                                oteryn_client_ui::house_container_capacity::view::panel_text(
                                    &mailbox.containers
                                ),
                                mailbox.item_movement.feedback.unwrap_or_default()
                            ))
                            .as_ptr(),
                        );
                        if let Some(panel) = &held
                            && let Some(view) = panel.current()
                        {
                            super::super::super::feature_0004_item_view_move::view::list_surface(
                                &mut mailbox,
                                entries,
                                panel.rows(),
                                oteryn_session::ItemMoveDestination::Container(
                                    view.container_handle,
                                ),
                            );
                            super::super::super::feature_0004_item_view_move::view::view_surfaces(
                                &mut mailbox,
                                views,
                                panel,
                            );
                        } else {
                            super::super::super::feature_0004_item_view_move::view::clear_surfaces(
                                &mut mailbox,
                                &[entries, views],
                            );
                        }
                        drop(mailbox);
                        std::thread::sleep(Duration::from_millis(30));
                    }
                    if IsWindow(hwnd) != 0 {
                        DestroyWindow(hwnd);
                    }
                }
            }
            if let Some(mailbox) = weak.upgrade() {
                lock(&mailbox).containers.open = false;
            }
        })
        .is_err()
        && let Some(mailbox) = failed.upgrade()
    {
        lock(&mailbox).containers.open = false;
    }
}

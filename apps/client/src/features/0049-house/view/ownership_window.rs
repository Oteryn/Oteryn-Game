//! The dialog reviews a frozen selection without holding the mailbox during the modal.
#![allow(
    unsafe_code,
    reason = "Window handles and boxed userdata belong to this UI thread"
)]
use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
};
#[path = "ownership_controls.rs"]
mod controls;
use super::super::input::ownership as input;
use controls::{read, set, wide};
type Shared = Weak<Mutex<Mailbox>>;
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn procedure(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND {
            let id = w & 0xffff;
            let notification = (w >> 16) & 0xffff;
            if notification == BN_CLICKED as usize
                || (id == 50 && notification == CBN_SELCHANGE as usize)
            {
                let actions = GetWindowLongPtrW(h, GWLP_USERDATA) as *mut Actions;
                if !actions.is_null() && (*actions).0.len() < 32 {
                    (*actions).0.push(id);
                }
                return 0;
            }
        }
        DefWindowProcW(h, msg, w, l)
    }
}
pub(in crate::play) fn open(link: &PlayLink) {
    let weak = Arc::downgrade(&link.mailbox);
    open_from_mailbox(&mut lock(&link.mailbox), weak, &link.commands);
}
pub(in crate::play) fn open_from_mailbox(
    mailbox: &mut Mailbox,
    weak: Shared,
    commands: &UnboundedSender<PlayCommand>,
) {
    let state = &mut mailbox.properties.house.ownership;
    if !state.model.available() || state.open {
        return;
    }
    state.open = true;
    state.context = weak.clone();
    input::page(mailbox, commands, 0);
    let rollback = weak.clone();
    let commands = commands.clone();
    if std::thread::Builder::new()
        .name("house-ownership".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&rollback);
    }
}
fn close(weak: &Shared) {
    if let Some(shared) = weak.upgrade() {
        lock(&shared).properties.house.ownership.open = false;
    }
}
fn run(weak: Shared, commands: UnboundedSender<PlayCommand>) {
    unsafe {
        let h = CreateWindowExW(
            WS_EX_CONTROLPARENT,
            wide("STATIC").as_ptr(),
            wide("House ownership and move-out").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            850,
            735,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if h.is_null() {
            close(&weak);
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(h, GWLP_USERDATA, (&mut *actions as *mut Actions) as isize);
        SetWindowLongPtrW(h, GWLP_WNDPROC, procedure as *const () as isize);
        let Some(controls::Widgets {
            choice,
            info,
            edits,
            navigation,
            buttons,
        }) = controls::build(h)
        else {
            DestroyWindow(h);
            close(&weak);
            return;
        };
        let mut rows = Vec::<(String, u64)>::new();
        let mut fields: [String; 3] = Default::default();
        let mut shown = String::new();
        loop {
            let mut message: MSG = std::mem::zeroed();
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(h, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            if IsWindow(h) == 0 || commands.is_closed() {
                break;
            }
            let Some(shared) = weak.upgrade() else {
                break;
            };
            let state = lock(&shared).properties.house.ownership.clone();
            if !state.open || !state.model.available() {
                break;
            }
            let fresh = state
                .model
                .data()
                .map(|d| {
                    d.rows
                        .iter()
                        .map(|r| (r.house.clone(), r.revision))
                        .collect()
                })
                .unwrap_or_default();
            if rows != fresh {
                rows = fresh;
                SendMessageW(choice, CB_RESETCONTENT, 0, 0);
                if let Some(data) = state.model.data() {
                    for row in &data.rows {
                        SendMessageW(choice, CB_ADDSTRING, 0, wide(&row.name).as_ptr() as LPARAM);
                    }
                }
                SendMessageW(choice, CB_SETCURSEL, state.model.index(), 0);
            }
            if fields != state.fields {
                fields = state.fields.clone();
                for (edit, field) in edits.iter().zip(&fields) {
                    set(*edit, field);
                }
            }
            let text = oteryn_client_ui::house::ownership_text(&state.model);
            if text != shown {
                set(info, &text);
                shown = text;
            }
            let ready = !state.model.pending();
            let allowed = state.model.row().map_or(0, |r| r.allowed);
            EnableWindow(choice, i32::from(ready));
            for (i, edit) in edits.iter().enumerate() {
                EnableWindow(
                    *edit,
                    i32::from(
                        ready && (allowed & (1 << 3) != 0 || (i == 1 && allowed & (1 << 1) != 0)),
                    ),
                );
            }
            for (i, button) in buttons.iter().enumerate() {
                EnableWindow(*button, i32::from(ready && allowed & (1 << (i + 1)) != 0));
            }
            EnableWindow(navigation[0], i32::from(ready));
            EnableWindow(navigation[1], i32::from(ready && !state.history.is_empty()));
            EnableWindow(
                navigation[2],
                i32::from(ready && state.model.data().is_some_and(|d| d.has_more)),
            );
            let values = [read(edits[0], 29), read(edits[1], 40), read(edits[2], 15)];
            if ready && values != state.fields {
                let mut mailbox = lock(&shared);
                if mailbox.properties.house.ownership.fields == state.fields {
                    let _ = input::edit_fields(&mut mailbox, values);
                }
            }
            let events = std::mem::take(&mut actions.0);
            for id in events {
                match id {
                    50 => {
                        if let Ok(index) = usize::try_from(SendMessageW(choice, CB_GETCURSEL, 0, 0))
                        {
                            let _ = lock(&shared).properties.house.ownership.select(index);
                        }
                    }
                    100..=102 => input::page(
                        &mut lock(&shared),
                        &commands,
                        match id {
                            101 => -1,
                            102 => 1,
                            _ => 0,
                        },
                    ),
                    201..=206 => {
                        let values = [read(edits[0], 29), read(edits[1], 40), read(edits[2], 15)];
                        let review = input::review(&mut lock(&shared), (id - 200) as u8, values);
                        match review {
                            Ok(review) => {
                                let accepted = MessageBoxW(
                                    h,
                                    wide(review.text()).as_ptr(),
                                    wide("Confirm house action").as_ptr(),
                                    MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
                                ) == IDYES;
                                input::confirm(&mut lock(&shared), &commands, review, accepted);
                            }
                            Err(error) => {
                                lock(&shared).properties.house.ownership.model.feedback = error
                            }
                        }
                    }
                    _ => {}
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(35));
        }
        SetWindowLongPtrW(h, GWLP_USERDATA, 0);
        if IsWindow(h) != 0 {
            DestroyWindow(h);
        }
        close(&weak);
    }
}

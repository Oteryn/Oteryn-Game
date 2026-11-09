//! Window-thread callbacks emit bounded layout actions; the play view owns preferences.
use super::*;
use oteryn_client::play::{HudAction, HudDock, HudPanel};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use windows_sys::Win32::{
    Foundation::*,
    UI::{Shell::DefSubclassProc, WindowsAndMessaging::*},
};
pub(super) type Actions = Rc<RefCell<Vec<HudAction>>>;
pub(super) struct State {
    pub(super) id: HudPanel,
    pub(super) root: HWND,
    pub(super) edit: HWND,
    pub(super) bars: [HWND; 2],
    pub(super) show_bars: Cell<bool>,
    pub(super) dragging: Cell<bool>,
    pub(super) actions: Actions,
}
fn emit(state: &State, action: HudAction) {
    let mut events = state.actions.borrow_mut();
    if events.len() < 32 {
        events.push(action);
    }
}
pub(super) unsafe fn resize(hwnd: HWND, state: &State) {
    unsafe {
        let mut rect: RECT = std::mem::zeroed();
        if GetClientRect(hwnd, &mut rect) == 0 {
            return;
        }
        let top = if state.show_bars.get() { 28 } else { 0 };
        MoveWindow(
            state.edit,
            0,
            top,
            rect.right.max(0),
            (rect.bottom - top).max(0),
            1,
        );
        for (index, bar) in state.bars.iter().enumerate() {
            MoveWindow(
                *bar,
                4,
                2 + index as i32 * 12,
                (rect.right - 8).max(0),
                9,
                1,
            );
        }
    }
}
pub(super) unsafe extern "system" fn procedure(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    unsafe {
        let state = &*(data as *const State);
        match msg {
            WM_CLOSE => {
                emit(state, HudAction::Visible(state.id, false));
                ShowWindow(hwnd, SW_HIDE);
                return 0;
            }
            WM_ENTERSIZEMOVE => state.dragging.set(true),
            WM_EXITSIZEMOVE => {
                state.dragging.set(false);
                let mut rect: RECT = std::mem::zeroed();
                let mut parent: WINDOWINFO = std::mem::zeroed();
                parent.cbSize = std::mem::size_of::<WINDOWINFO>() as u32;
                if GetWindowRect(hwnd, &mut rect) != 0
                    && GetWindowInfo(state.root, &mut parent) != 0
                {
                    let bounds = [
                        rect.left - parent.rcClient.left,
                        rect.top - parent.rcClient.top,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                    ]
                    .map(|value| u16::try_from(value.clamp(0, 4096)).unwrap_or(0));
                    emit(state, HudAction::Dragged(state.id, bounds));
                }
            }
            WM_SIZE => resize(hwnd, state),
            WM_GETMINMAXINFO => {
                let result = DefSubclassProc(hwnd, msg, w, l);
                if l != 0 {
                    let limits = &mut *(l as *mut MINMAXINFO);
                    limits.ptMinTrackSize.x = 160;
                    limits.ptMinTrackSize.y = 64;
                }
                return result;
            }
            WM_CONTEXTMENU => {
                let menu = CreatePopupMenu();
                if !menu.is_null() {
                    for (index, label) in [
                        "Dock left",
                        "Dock right",
                        "Dock top",
                        "Float",
                        "Move earlier",
                        "Move later",
                        "Hide",
                        "Reset HUD layout",
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        AppendMenuW(menu, MF_STRING, 31000 + index, wide(label).as_ptr());
                    }
                    let mut rect: RECT = std::mem::zeroed();
                    GetWindowRect(hwnd, &mut rect);
                    let x = if l == -1 {
                        rect.left + 12
                    } else {
                        i32::from(l as u16 as i16)
                    };
                    let y = if l == -1 {
                        rect.top + 12
                    } else {
                        i32::from((l as u32 >> 16) as u16 as i16)
                    };
                    let action = TrackPopupMenu(
                        menu,
                        TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                        x,
                        y,
                        0,
                        hwnd,
                        std::ptr::null(),
                    );
                    DestroyMenu(menu);
                    let action = match action {
                        31000 => Some(HudAction::Dock(state.id, HudDock::Left)),
                        31001 => Some(HudAction::Dock(state.id, HudDock::Right)),
                        31002 => Some(HudAction::Dock(state.id, HudDock::Top)),
                        31003 => Some(HudAction::Dock(state.id, HudDock::Floating)),
                        31004 => Some(HudAction::Earlier(state.id)),
                        31005 => Some(HudAction::Later(state.id)),
                        31006 => Some(HudAction::Visible(state.id, false)),
                        31007 => Some(HudAction::Reset),
                        _ => None,
                    };
                    if let Some(action) = action {
                        emit(state, action);
                    }
                }
                return 0;
            }
            _ => {}
        }
        DefSubclassProc(hwnd, msg, w, l)
    }
}

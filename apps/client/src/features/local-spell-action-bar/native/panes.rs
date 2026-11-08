//! HUD panes and controls live on the winit shell thread.
use super::*;
use events::{Actions, State};
use oteryn_client::play::HudPanel;
use std::{
    cell::{Cell, RefCell},
    ptr::{null, null_mut},
};
use windows_sys::Win32::{
    Foundation::*,
    UI::{
        Controls::*,
        Shell::{RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};
pub(super) struct Pane {
    frame: HWND,
    state: Box<State>,
    text: RefCell<String>,
}
impl Pane {
    pub(super) fn new(root: HWND, id: HudPanel, actions: Actions) -> Option<Self> {
        unsafe {
            let frame = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide(id.title()).as_ptr(),
                WS_CHILD | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME | WS_CLIPCHILDREN,
                8,
                8,
                240,
                100,
                root,
                null_mut(),
                null_mut(),
                null(),
            );
            if frame.is_null() {
                return None;
            }
            let edit = CreateWindowExW(
                0,
                wide("EDIT").as_ptr(),
                wide("").as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
                0,
                0,
                220,
                60,
                frame,
                null_mut(),
                null_mut(),
                null(),
            );
            let bars = [0, 1].map(|_| {
                CreateWindowExW(
                    0,
                    wide("msctls_progress32").as_ptr(),
                    wide("").as_ptr(),
                    WS_CHILD,
                    0,
                    0,
                    220,
                    9,
                    frame,
                    null_mut(),
                    null_mut(),
                    null(),
                )
            });
            if edit.is_null() || bars.iter().any(|bar| bar.is_null()) {
                DestroyWindow(frame);
                return None;
            }
            for bar in bars {
                SendMessageW(bar, PBM_SETRANGE32, 0, 10000);
            }
            let state = Box::new(State {
                id,
                root,
                edit,
                bars,
                actions,
                dragging: Cell::new(false),
                show_bars: Cell::new(false),
            });
            if SetWindowSubclass(
                frame,
                Some(events::procedure),
                1,
                (&*state as *const State) as usize,
            ) == 0
            {
                DestroyWindow(frame);
                return None;
            }
            Some(Self {
                frame,
                state,
                text: Default::default(),
            })
        }
    }
    pub(super) fn update(
        &self,
        text: &str,
        rect: Option<[u16; 4]>,
        vitals: Option<&oteryn_client_runtime::spell::ActorVitals>,
        bars: bool,
    ) {
        unsafe {
            let Some(rect) = rect else {
                ShowWindow(self.frame, SW_HIDE);
                return;
            };
            let show_bars = bars && self.state.id == HudPanel::Resources && vitals.is_some();
            self.state.show_bars.set(show_bars);
            for (index, bar) in self.state.bars.iter().enumerate() {
                ShowWindow(*bar, if show_bars { SW_SHOW } else { SW_HIDE });
                if let Some(vitals) = vitals {
                    let (value, max) = if index == 0 {
                        (vitals.health, vitals.max_health)
                    } else {
                        (vitals.mana, vitals.max_mana)
                    };
                    let fill = if max == 0 {
                        0
                    } else {
                        u64::from(value.min(max)) * 10000 / u64::from(max)
                    };
                    SendMessageW(*bar, PBM_SETPOS, usize::try_from(fill).unwrap_or(0), 0);
                }
            }
            if !self.state.dragging.get() {
                MoveWindow(
                    self.frame,
                    i32::from(rect[0]),
                    i32::from(rect[1]),
                    i32::from(rect[2]),
                    i32::from(rect[3]),
                    1,
                );
            }
            events::resize(self.frame, &self.state);
            let mut previous = self.text.borrow_mut();
            if previous.as_str() != text {
                let first = SendMessageW(self.state.edit, EM_GETFIRSTVISIBLELINE, 0, 0);
                SetWindowTextW(self.state.edit, wide(text).as_ptr());
                SendMessageW(self.state.edit, EM_LINESCROLL, 0, first);
                text.clone_into(&mut previous);
            }
            ShowWindow(self.frame, SW_SHOW);
        }
    }
    pub(super) fn hide(&self) {
        unsafe {
            ShowWindow(self.frame, SW_HIDE);
            SetWindowTextW(self.state.edit, wide("").as_ptr());
        }
        self.text.borrow_mut().clear();
    }
}
impl Drop for Pane {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(self.frame) != 0 {
                RemoveWindowSubclass(self.frame, Some(events::procedure), 1);
                DestroyWindow(self.frame);
            }
        }
    }
}

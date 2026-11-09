//! Docked action rows. HWNDs and pointer tracking stay on the shell thread.
#![allow(unsafe_code)] // Child HWNDs are created and destroyed on this thread.
use oteryn_client::play::{BarDock, BarGesture, BarLayout, SpellAim};
use windows_sys::Win32::{
    Foundation::{HWND, POINT, RECT},
    UI::{
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON, VK_RBUTTON},
        WindowsAndMessaging::*,
    },
};
#[path = "tooltips_native.rs"]
mod tips;
const ROW_SLOTS: usize = 50;
const ROWS: usize = 3;
const TOOLS: usize = 7;
pub(super) struct SpellRow {
    tips: tips::Tips,
    buttons: Vec<HWND>,
    tools: Vec<HWND>,
    pressed: Vec<bool>,
    parent: HWND,
    drag: Option<usize>,
    mouse_down: bool,
    right_down: bool,
}
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
unsafe fn button(parent: HWND) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide("BUTTON").as_ptr(),
            wide("").as_ptr(),
            WS_CHILD | BS_MULTILINE as u32,
            0,
            0,
            1,
            1,
            parent,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null(),
        )
    }
}
impl SpellRow {
    pub(super) fn new(parent: HWND) -> Option<Self> {
        let mut row = Self {
            tips: tips::Tips::new(parent, ROWS * ROW_SLOTS),
            buttons: Vec::new(),
            tools: Vec::new(),
            pressed: vec![false; ROWS * (ROW_SLOTS + TOOLS)],
            parent,
            drag: None,
            mouse_down: false,
            right_down: false,
        };
        for _ in 0..ROWS * ROW_SLOTS {
            let handle = unsafe { button(parent) };
            if handle.is_null() {
                return None;
            }
            row.buttons.push(handle);
        }
        for _ in 0..ROWS * TOOLS {
            let handle = unsafe { button(parent) };
            if handle.is_null() {
                return None;
            }
            row.tools.push(handle);
        }
        Some(row)
    }
    pub(super) fn update(
        &mut self,
        labels: Option<&[String]>,
        layout: &BarLayout,
        tooltips: &[String],
    ) -> Vec<BarGesture> {
        self.tips.update(self.parent, &self.buttons, tooltips);
        let mut gestures = Vec::new();
        let mut visible = [false; ROWS * ROW_SLOTS];
        unsafe {
            let mut rect: RECT = std::mem::zeroed();
            GetClientRect(self.parent, &mut rect);
            for (bar, row) in layout.rows.iter().enumerate() {
                let horizontal = row.dock == BarDock::Bottom;
                let count = if horizontal {
                    ((rect.right - 180) / 62).clamp(1, 12)
                } else {
                    ((rect.bottom - 180) / 54).clamp(1, 12)
                } as usize;
                let count = count.min(usize::from(row.count));
                let offset = usize::from(row.offset).min(ROW_SLOTS - count);
                let width = if horizontal {
                    (rect.right - 180).max(62) / count as i32
                } else {
                    78
                };
                for local in 0..ROW_SLOTS {
                    let slot = bar * ROW_SLOTS + local;
                    let handle = self.buttons[slot];
                    let label = labels
                        .and_then(|labels| labels.get(slot))
                        .filter(|_| row.enabled && (offset..offset + count).contains(&local));
                    visible[slot] = label.is_some();
                    ShowWindow(handle, if label.is_some() { SW_SHOWNA } else { SW_HIDE });
                    let display = local.saturating_sub(offset) as i32;
                    let (x, y) = match row.dock {
                        BarDock::Bottom => (
                            8 + display * width,
                            (rect.bottom - 88 - bar as i32 * 56).max(0),
                        ),
                        BarDock::Left => (8 + bar as i32 * 80, 94 + display * 54),
                        BarDock::Right => (
                            (rect.right - 260 - bar as i32 * 80).max(0),
                            94 + display * 54,
                        ),
                    };
                    MoveWindow(handle, x, y, width - 2, 52, 1);
                    if let Some(label) = label {
                        SetWindowTextW(handle, wide(label).as_ptr());
                    }
                    let down = label.is_some()
                        && SendMessageW(handle, BM_GETSTATE, 0, 0) & BST_PUSHED as isize != 0;
                    if down && !self.pressed[slot] {
                        if row.locked {
                            gestures.push(BarGesture::Cast { slot });
                        } else {
                            self.drag = Some(slot);
                        }
                    }
                    self.pressed[slot] = down;
                }
                for tool in 0..TOOLS {
                    let handle = self.tools[bar * TOOLS + tool];
                    ShowWindow(handle, if labels.is_some() { SW_SHOWNA } else { SW_HIDE });
                    let text = match tool {
                        0 => format!("{}:{}", bar + 1, if row.enabled { "On" } else { "Off" }),
                        1 => match row.dock {
                            BarDock::Bottom => "Bottom",
                            BarDock::Left => "Left",
                            BarDock::Right => "Right",
                        }
                        .into(),
                        2 => if row.locked { "Locked" } else { "Edit/drag" }.into(),
                        3 => "<".into(),
                        4 => ">".into(),
                        5 => format!("{} −", row.count),
                        _ => "+".into(),
                    };
                    let columns = (rect.right / 310).max(1) as usize;
                    MoveWindow(
                        handle,
                        8 + (bar % columns) as i32 * 310 + tool as i32 * 43,
                        (rect.bottom - 28 - (bar / columns) as i32 * 26).max(0),
                        42,
                        24,
                        1,
                    );
                    SetWindowTextW(handle, wide(&text).as_ptr());
                    let down = labels.is_some()
                        && SendMessageW(handle, BM_GETSTATE, 0, 0) & BST_PUSHED as isize != 0;
                    let index = ROWS * ROW_SLOTS + bar * TOOLS + tool;
                    if down && !self.pressed[index] {
                        gestures.push(match tool {
                            0 => BarGesture::Configure {
                                bar,
                                dock: row.dock,
                                enabled: !row.enabled,
                                locked: row.locked,
                            },
                            1 => BarGesture::Configure {
                                bar,
                                dock: match row.dock {
                                    BarDock::Bottom => BarDock::Left,
                                    BarDock::Left => BarDock::Right,
                                    BarDock::Right => BarDock::Bottom,
                                },
                                enabled: row.enabled,
                                locked: row.locked,
                            },
                            2 => BarGesture::Configure {
                                bar,
                                dock: row.dock,
                                enabled: row.enabled,
                                locked: !row.locked,
                            },
                            3 => BarGesture::Scroll { bar, by: -1 },
                            4 => BarGesture::Scroll { bar, by: 1 },
                            5 => BarGesture::Count {
                                bar,
                                count: row.count.saturating_sub(1).max(1),
                            },
                            _ => BarGesture::Count {
                                bar,
                                count: row.count.saturating_add(1).min(50),
                            },
                        });
                    }
                    self.pressed[index] = down;
                }
            }
            let right_down = GetAsyncKeyState(i32::from(VK_RBUTTON)) < 0;
            if right_down && !self.right_down {
                let mut cursor = POINT { x: 0, y: 0 };
                if GetCursorPos(&mut cursor) != 0 {
                    for (slot, button) in self
                        .buttons
                        .iter()
                        .enumerate()
                        .filter(|(slot, _)| visible[*slot])
                    {
                        let mut bounds: RECT = std::mem::zeroed();
                        GetWindowRect(*button, &mut bounds);
                        if cursor.x >= bounds.left
                            && cursor.x < bounds.right
                            && cursor.y >= bounds.top
                            && cursor.y < bounds.bottom
                        {
                            let menu = CreatePopupMenu();
                            if !menu.is_null() {
                                AppendMenuW(
                                    menu,
                                    MF_STRING,
                                    1,
                                    wide("Assign Multiaction").as_ptr(),
                                );
                                for (id, label) in [
                                    (2, "Spell: in facing direction"),
                                    (3, "Spell: towards cursor"),
                                    (4, "Spell: at target"),
                                ] {
                                    AppendMenuW(menu, MF_STRING, id, wide(label).as_ptr());
                                }
                                let selected = TrackPopupMenu(
                                    menu,
                                    TPM_RETURNCMD | TPM_RIGHTBUTTON,
                                    cursor.x,
                                    cursor.y,
                                    0,
                                    self.parent,
                                    std::ptr::null(),
                                );
                                DestroyMenu(menu);
                                match selected {
                                    1 => gestures.push(BarGesture::AssignMultiaction { slot }),
                                    2..=4 => gestures.push(BarGesture::Aim {
                                        slot,
                                        aim: match selected {
                                            2 => SpellAim::Facing,
                                            3 => SpellAim::Cursor,
                                            _ => SpellAim::Target,
                                        },
                                    }),
                                    _ => {}
                                }
                            }
                            break;
                        }
                    }
                }
            }
            self.right_down = right_down;
            let down = GetAsyncKeyState(i32::from(VK_LBUTTON)) < 0;
            if self.mouse_down
                && !down
                && let Some(from) = self.drag.take()
            {
                let mut cursor = POINT { x: 0, y: 0 };
                if GetCursorPos(&mut cursor) != 0 {
                    for (to, handle) in self
                        .buttons
                        .iter()
                        .enumerate()
                        .filter(|(slot, _)| visible[*slot])
                    {
                        let mut target: RECT = std::mem::zeroed();
                        GetWindowRect(*handle, &mut target);
                        if (target.left..target.right).contains(&cursor.x)
                            && (target.top..target.bottom).contains(&cursor.y)
                        {
                            gestures.push(BarGesture::Swap { from, to });
                            break;
                        }
                    }
                }
            }
            self.mouse_down = down;
            if labels.is_none() {
                self.drag = None;
            }
        }
        gestures
    }
}
impl Drop for SpellRow {
    fn drop(&mut self) {
        for button in self.buttons.iter().chain(&self.tools) {
            unsafe {
                DestroyWindow(*button);
            }
        }
    }
}

//! Proficiency controls beside the resource/status pane, owned by the shell thread.
#![allow(
    unsafe_code,
    reason = "Child HWNDs and callback state are owned by one UI thread"
)]
use oteryn_client::play::PlayLink;
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::*,
    UI::{
        Controls::*,
        Shell::{DefSubclassProc, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};
#[derive(Default)]
struct Actions(Vec<usize>);
pub(super) struct ProficiencyHud {
    root: HWND,
    bar: HWND,
    label: HWND,
    icon: HWND,
    actions: Box<Actions>,
    displayed: (u32, Option<u32>),
}
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
unsafe extern "system" fn root_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && w >> 16 == BN_CLICKED as usize {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && (*actions).0.len() < 16 {
                (*actions).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
unsafe extern "system" fn bar_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    unsafe {
        if msg == WM_LBUTTONUP || (msg == WM_KEYUP && matches!(w, 13 | 32)) {
            let actions = &mut *(data as *mut Actions);
            if actions.0.len() < 16 {
                actions.0.push(1);
            }
            return 0;
        }
        DefSubclassProc(hwnd, msg, w, l)
    }
}
impl ProficiencyHud {
    pub(super) fn new(parent: HWND) -> Option<Self> {
        unsafe {
            let root = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("").as_ptr(),
                WS_CHILD | WS_CLIPCHILDREN,
                0,
                0,
                280,
                58,
                parent,
                null_mut(),
                null_mut(),
                null(),
            );
            if root.is_null() {
                return None;
            }
            let mut hud = Self {
                root,
                bar: null_mut(),
                label: null_mut(),
                icon: null_mut(),
                actions: Box::default(),
                displayed: (0, None),
            };
            SetWindowLongPtrW(
                root,
                GWLP_USERDATA,
                (&mut *hud.actions as *mut Actions) as isize,
            );
            SetWindowLongPtrW(root, GWLP_WNDPROC, root_proc as *const () as isize);
            hud.bar = CreateWindowExW(
                0,
                wide("msctls_progress32").as_ptr(),
                wide(tr!("Weapon proficiency progress — click to open")).as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                0,
                28,
                220,
                24,
                root,
                null_mut(),
                null_mut(),
                null(),
            );
            hud.label = CreateWindowExW(
                0,
                wide("BUTTON").as_ptr(),
                wide("").as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                0,
                0,
                220,
                26,
                root,
                1usize as *mut _,
                null_mut(),
                null(),
            );
            hud.icon = CreateWindowExW(
                0,
                wide("BUTTON").as_ptr(),
                wide("↑").as_ptr(),
                WS_CHILD | WS_TABSTOP,
                224,
                0,
                52,
                52,
                root,
                2usize as *mut _,
                null_mut(),
                null(),
            );
            if [hud.bar, hud.label, hud.icon]
                .iter()
                .any(|hwnd| hwnd.is_null())
            {
                return None;
            }
            SendMessageW(hud.bar, PBM_SETRANGE32, 0, 100);
            if SetWindowSubclass(
                hud.bar,
                Some(bar_proc),
                1,
                (&mut *hud.actions as *mut Actions) as usize,
            ) == 0
            {
                return None;
            }
            Some(hud)
        }
    }
    pub(super) fn update(&mut self, link: &PlayLink, rect: Option<[u16; 4]>) {
        let panel = link.weapon_proficiency_hud();
        let displayed = panel.as_ref().map_or((0, None), |panel| {
            (
                panel.definition,
                panel
                    .advancement
                    .as_ref()
                    .map(|(definition, _)| *definition),
            )
        });
        let actions = std::mem::take(&mut self.actions.0);
        if self.displayed == displayed && panel.as_ref().is_some_and(|panel| !panel.pending) {
            for action in actions {
                link.weapon_proficiency_open_track(action == 2);
            }
        }
        link.weapon_proficiency_tick(std::time::Instant::now());
        self.displayed = displayed;
        unsafe {
            let Some(([x, y, w, h], panel)) = rect.zip(panel) else {
                ShowWindow(self.root, SW_HIDE);
                return;
            };
            let width = i32::from(w).max(160);
            let mut bounds: RECT = std::mem::zeroed();
            GetClientRect(GetParent(self.root), &mut bounds);
            let below = i32::from(y) + i32::from(h) + 2;
            let top = if below + 56 <= bounds.bottom {
                below
            } else {
                (i32::from(y) - 58).max(0)
            };
            MoveWindow(
                self.root,
                i32::from(x).min((bounds.right - width).max(0)),
                top,
                width,
                56,
                1,
            );
            MoveWindow(self.label, 0, 0, width - 56, 26, 1);
            MoveWindow(self.bar, 0, 28, width - 56, 24, 1);
            MoveWindow(self.icon, width - 52, 0, 52, 52, 1);
            let percent = panel.percent;
            SetWindowTextW(
                self.label,
                wide(&lfmt!("{} — {}%", panel.name, percent)).as_ptr(),
            );
            SendMessageW(self.bar, PBM_SETPOS, percent as usize, 0);
            if let Some((_, name)) = &panel.advancement {
                SetWindowTextW(self.icon, wide(&lfmt!("↑\n{}", name)).as_ptr());
            }
            ShowWindow(
                self.icon,
                if displayed.1.is_some() {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
            ShowWindow(self.root, SW_SHOW);
        }
    }
    pub(super) fn hide(&mut self) {
        unsafe {
            ShowWindow(self.root, SW_HIDE);
        }
        self.actions.0.clear();
        self.displayed = (0, None);
    }
}
impl Drop for ProficiencyHud {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.root);
        }
    }
}

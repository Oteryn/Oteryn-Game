//! Same-process native windows keep their thread and become game-window children.
#![allow(
    unsafe_code,
    reason = "Window hooks and subclass userdata belong to the owning GUI thread"
)]
use super::{Dock, Layout};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{Mutex, OnceLock},
};
use windows_sys::Win32::{
    Foundation::*,
    System::Threading::GetCurrentProcessId,
    UI::{
        Shell::{DefSubclassProc, GetWindowSubclass, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};
use winit::{
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};
#[path = "events.rs"]
mod events;
struct Host {
    root: usize,
    size: [i32; 2],
    layout: Layout,
    windows: BTreeMap<usize, String>,
    moving: BTreeSet<usize>,
    hooks: BTreeMap<u32, usize>,
}
fn path() -> Option<std::path::PathBuf> {
    crate::play::PreferenceStore::path()
        .map(|path| path.with_file_name("client-sidebar-layout.json"))
}
fn state() -> &'static Mutex<Host> {
    static HOST: OnceLock<Mutex<Host>> = OnceLock::new();
    HOST.get_or_init(|| {
        Mutex::new(Host {
            root: 0,
            size: [1, 1],
            layout: path()
                .and_then(|path| Layout::load(&path).ok())
                .unwrap_or_default(),
            windows: BTreeMap::new(),
            moving: BTreeSet::new(),
            hooks: BTreeMap::new(),
        })
    })
}
fn lock() -> std::sync::MutexGuard<'static, Host> {
    state()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
fn save(host: &Host) {
    if let Some(path) = path()
        && let Err(error) = host.layout.save(&path)
    {
        eprintln!("Could not save sidebar layout: {error}");
    }
}
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
unsafe fn rect(hwnd: HWND) -> [i32; 4] {
    unsafe {
        let mut rect: RECT = std::mem::zeroed();
        GetWindowRect(hwnd, &mut rect);
        [
            rect.left,
            rect.top,
            rect.right - rect.left,
            rect.bottom - rect.top,
        ]
    }
}
pub fn update(window: &Window) {
    let Ok(handle) = window.window_handle() else {
        return;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };
    let root = handle.hwnd.get() as HWND;
    unsafe {
        {
            let mut host = lock();
            host.root = root as usize;
            host.size = [
                window.inner_size().width as i32,
                window.inner_size().height as i32,
            ];
        }
        EnumWindows(Some(discover), root as isize);
        let placements = {
            let mut host = lock();
            host.windows.retain(|hwnd, _| IsWindow(*hwnd as HWND) != 0);
            let visible: Vec<_> = host
                .windows
                .iter()
                .filter(|(hwnd, _)| IsWindowVisible(**hwnd as HWND) != 0)
                .map(|(_, id)| id.clone())
                .collect();
            host.windows
                .iter()
                .filter(|(hwnd, _)| !host.moving.contains(hwnd))
                .filter_map(|(hwnd, id)| {
                    host.layout
                        .rect(id, host.size, &visible)
                        .map(|rect| (*hwnd, rect))
                })
                .collect::<Vec<_>>()
        };
        for (hwnd, [x, y, w, h]) in placements {
            SetWindowPos(
                hwnd as HWND,
                std::ptr::null_mut(),
                x,
                y,
                w,
                h,
                SWP_ASYNCWINDOWPOS | SWP_NOACTIVATE | SWP_NOZORDER,
            );
        }
    }
}
unsafe extern "system" fn discover(hwnd: HWND, root: LPARAM) -> i32 {
    unsafe {
        if hwnd as isize == root || GetWindow(hwnd, GW_OWNER) == root as HWND {
            return 1;
        }
        let mut process = 0;
        let thread = GetWindowThreadProcessId(hwnd, &mut process);
        if process != GetCurrentProcessId() || GetWindowTextLengthW(hwnd) == 0 {
            return 1;
        }
        let mut host = lock();
        if let std::collections::btree_map::Entry::Vacant(entry) = host.hooks.entry(thread) {
            let hook = SetWindowsHookExW(WH_CALLWNDPROC, Some(hook), std::ptr::null_mut(), thread);
            if !hook.is_null() {
                entry.insert(hook as usize);
            }
        }
        drop(host);
        PostMessageW(hwnd, WM_NULL, 0, 0);
        1
    }
}
unsafe extern "system" fn hook(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if code >= 0 && l != 0 {
            let event = &*(l as *const CWPSTRUCT);
            if event.message == WM_NULL {
                attach(event.hwnd);
            }
        }
        CallNextHookEx(std::ptr::null_mut(), code, w, l)
    }
}
unsafe fn attach(hwnd: HWND) {
    unsafe {
        let root = lock().root as HWND;
        if root.is_null() || hwnd == root || GetAncestor(hwnd, GA_ROOT) != hwnd {
            return;
        }
        let mut existing = 0;
        if GetWindowSubclass(hwnd, Some(events::procedure), 0x033, &mut existing) != 0 {
            return;
        }
        let mut text = vec![0_u16; 257];
        let count = GetWindowTextW(hwnd, text.as_mut_ptr(), 257).max(0) as usize;
        let id = String::from_utf16_lossy(&text[..count]);
        if id.is_empty()
            || id.contains("Login")
            || id.contains("Character selection")
            || id.contains("Settings")
        {
            return;
        }
        let original = rect(hwnd);
        let mut info: WINDOWINFO = std::mem::zeroed();
        info.cbSize = std::mem::size_of::<WINDOWINFO>() as u32;
        GetWindowInfo(root, &mut info);
        let mut data = Box::new(events::Pane {
            id: id.clone(),
            base: [1, 1],
            children: Vec::new(),
        });
        let mut client: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut client);
        data.base = [client.right.max(1), client.bottom.max(1)];
        EnumChildWindows(
            hwnd,
            Some(events::child),
            data.as_mut() as *mut events::Pane as isize,
        );
        if SetWindowSubclass(
            hwnd,
            Some(events::procedure),
            0x033,
            data.as_mut() as *mut events::Pane as usize,
        ) == 0
        {
            return;
        }
        let _ = Box::into_raw(data);
        SetWindowLongPtrW(
            hwnd,
            GWL_STYLE,
            ((GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 & !WS_POPUP)
                | WS_CHILD
                | WS_THICKFRAME
                | WS_CAPTION
                | WS_SYSMENU) as isize,
        );
        SetParent(hwnd, root);
        SetWindowPos(
            hwnd,
            std::ptr::null_mut(),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED,
        );
        let mut host = lock();
        host.layout.open(
            &id,
            [
                original[0]
                    .saturating_sub(info.rcClient.left)
                    .clamp(0, 4096),
                original[1].saturating_sub(info.rcClient.top).clamp(0, 4096),
                original[2].clamp(200, 4096),
                original[3].clamp(100, 4096),
            ],
        );
        host.windows.insert(hwnd as usize, id);
        save(&host);
    }
}

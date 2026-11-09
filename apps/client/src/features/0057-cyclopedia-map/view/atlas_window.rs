//! One UI thread owns the HWND, userdata and GDI resources; producers publish immutable drawings.
#![allow(
    unsafe_code,
    reason = "Win32 window and painting resources are confined to this UI thread"
)]
use oteryn_client_ui::cyclopedia_map::{
    Panel,
    atlas_view::{Drawing, project},
};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
};
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};
type Commands = tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>;
type Key = (u64, bool, Vec<(u16, u16, i8)>);
struct Frame {
    key: Key,
    version: u64,
    drawing: Arc<Drawing>,
    owner: Weak<()>,
    commands: Commands,
}
static ACTIVE: Mutex<Weak<Mutex<Frame>>> = Mutex::new(Weak::new());
fn lock<T>(value: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
fn key(panel: &Panel) -> Key {
    (
        panel.atlas.revision,
        panel.hide_pois,
        panel
            .map
            .as_ref()
            .map_or_else(Vec::new, |map| map.visible.clone()),
    )
}
fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
pub(super) fn refresh(panel: &Panel) {
    let Some(shared) = lock(&ACTIVE).upgrade() else {
        return;
    };
    let mut frame = lock(&shared);
    let key = key(panel);
    if Weak::ptr_eq(&frame.owner, &Arc::downgrade(&panel.atlas.lifetime)) && frame.key != key {
        frame.key = key;
        frame.drawing = Arc::new(project(panel));
        frame.version = frame.version.wrapping_add(1);
    }
}
pub(super) fn open(panel: &Panel, commands: &Commands) {
    let mut active = lock(&ACTIVE);
    if let Some(shared) = active.upgrade() {
        let mut frame = lock(&shared);
        frame.key = key(panel);
        frame.drawing = Arc::new(project(panel));
        frame.owner = Arc::downgrade(&panel.atlas.lifetime);
        frame.commands = commands.clone();
        frame.version = frame.version.wrapping_add(1);
        return;
    }
    let shared = Arc::new(Mutex::new(Frame {
        key: key(panel),
        version: 1,
        drawing: Arc::new(project(panel)),
        owner: Arc::downgrade(&panel.atlas.lifetime),
        commands: commands.clone(),
    }));
    *active = Arc::downgrade(&shared);
    let _ = std::thread::Builder::new()
        .name("World-atlas-map".into())
        .spawn(move || run(shared));
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if message == WM_PAINT {
            let shared = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Arc<Mutex<Frame>>;
            if !shared.is_null() {
                let drawing = lock(&**shared).drawing.clone();
                paint(hwnd, &drawing);
                return 0;
            }
        }
        DefWindowProcW(hwnd, message, w, l)
    }
}
unsafe fn fill(dc: HDC, rect: RECT, color: u32) {
    unsafe {
        let brush = CreateSolidBrush(color);
        if !brush.is_null() {
            FillRect(dc, &rect, brush);
            DeleteObject(brush as _);
        }
    }
}
unsafe fn paint(hwnd: HWND, drawing: &Drawing) {
    unsafe {
        let mut paint: PAINTSTRUCT = std::mem::zeroed();
        let dc = BeginPaint(hwnd, &mut paint);
        let mut bounds: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut bounds);
        fill(dc, bounds, 0x20180f);
        let colors = [0x78634c, 0x706c45, 0x687748, 0x83754f, 0x6d6253, 0x7b6b4b];
        let brushes = colors.map(|color| CreateSolidBrush(color));
        for rect in &drawing.rects {
            let [x, y, width, height] = rect.bounds;
            let brush = brushes[usize::from(rect.color)];
            if !brush.is_null() {
                FillRect(
                    dc,
                    &RECT {
                        left: x,
                        top: y,
                        right: x + width,
                        bottom: y + height,
                    },
                    brush,
                );
            }
        }
        for brush in brushes {
            if !brush.is_null() {
                DeleteObject(brush as _);
            }
        }
        for &[x, y] in &drawing.points {
            fill(
                dc,
                RECT {
                    left: x - 3,
                    top: y - 3,
                    right: x + 4,
                    bottom: y + 4,
                },
                0x5eddf3,
            );
        }
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, 0xe7ecee);
        let caption = wide(&drawing.caption);
        TextOutW(dc, 12, 12, caption.as_ptr(), (caption.len() - 1) as i32);
        let legend = wide(
            "Area footprints · gold: last confirmed visible POIs · floor/region filters in World atlas",
        );
        TextOutW(dc, 12, 488, legend.as_ptr(), (legend.len() - 1) as i32);
        EndPaint(hwnd, &paint);
    }
}
fn run(shared: Arc<Mutex<Frame>>) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Oteryn world atlas").as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            680,
            570,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            return;
        }
        let userdata = Box::new(shared);
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&*userdata as *const Arc<Mutex<Frame>>) as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
        let mut version = 0;
        let mut message: MSG = std::mem::zeroed();
        while IsWindow(hwnd) != 0 {
            let current = lock(&userdata);
            if current.owner.upgrade().is_none() || current.commands.is_closed() {
                break;
            }
            let changed = version != current.version;
            version = current.version;
            drop(current);
            if changed {
                InvalidateRect(hwnd, null(), 0);
            }
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            MsgWaitForMultipleObjects(0, null(), 0, 50, QS_ALLINPUT);
        }
        if IsWindow(hwnd) != 0 {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            DestroyWindow(hwnd);
        }
    }
}

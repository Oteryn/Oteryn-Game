//! Entrance markers use only the server's catalogue, with no inferred terrain.
#![allow(
    unsafe_code,
    reason = "All HWNDs and messages are confined to the map UI thread"
)]
use oteryn_client_ui::house::MapHouse;
use std::ptr::{null, null_mut};
use std::sync::{
    Arc, Weak,
    atomic::{AtomicBool, Ordering},
};
use windows_sys::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
fn wide(value: &str) -> Vec<u16> {
    value
        .replace('\0', "�")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
pub(super) fn open(
    mailbox: &mut crate::play::mailbox::Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    houses: Vec<MapHouse>,
) {
    let running = mailbox.properties.house.map_running.clone();
    if running.swap(true, Ordering::AcqRel) {
        return;
    }
    let weak = Arc::downgrade(&running);
    let context = mailbox.properties.house.ownership.context.clone();
    let commands = commands.clone();
    if std::thread::Builder::new()
        .name("House-entrances".into())
        .spawn(move || run(houses, commands, weak, context))
        .is_err()
    {
        running.store(false, Ordering::Release);
    }
}
fn run(
    houses: Vec<MapHouse>,
    commands: tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    running: Weak<AtomicBool>,
    context: Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
) {
    unsafe {
        let window = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("House entrances — north up").as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            680,
            560,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if !window.is_null() {
            let mut shown = houses;
            let mut labels = draw_labels(window, &shown);
            let mut message: MSG = std::mem::zeroed();
            while IsWindow(window) != 0
                && !commands.is_closed()
                && running
                    .upgrade()
                    .is_some_and(|flag| flag.load(Ordering::Acquire))
            {
                while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                let Some(shared) = context.upgrade() else {
                    break;
                };
                let fresh =
                    super::super::state::map_projection(&crate::play::mailbox::lock(&shared));
                if fresh != shown {
                    for child in labels.drain(..) {
                        if !child.is_null() {
                            DestroyWindow(child);
                        }
                    }
                    shown = fresh;
                    labels = draw_labels(window, &shown);
                }
                MsgWaitForMultipleObjects(0, null(), 0, 100, QS_ALLINPUT);
            }
            if IsWindow(window) != 0 {
                DestroyWindow(window);
            }
        }
    }
    if let Some(running) = running.upgrade() {
        running.store(false, Ordering::Release);
    }
}
unsafe fn draw_labels(window: HWND, houses: &[MapHouse]) -> Vec<HWND> {
    unsafe {
        let mut labels = Vec::new();
        let caption = houses.first().map_or_else(
            || "No house entrance on this page".into(),
            |h| {
                format!(
                    "Floor {} · selected entrance ◆ · other entrances ⌂ · page fitted to view",
                    h.position.2
                )
            },
        );
        labels.push(label(window, &caption, [12, 12, 640, 30]));
        for house in houses {
            let (x, y, floor) = house.position;
            let text = format!(
                "{} {}\r\n{x},{y},{floor}",
                if house.selected { "◆" } else { "⌂" },
                house.label
            );
            labels.push(label(
                window,
                &text,
                [house.pixel.0, house.pixel.1, 220, 48],
            ));
        }
        labels.push(label(window,"Entrances on the selected floor. Query another page or row in Houses to change the map.",[12,440,640,48]));
        labels
    }
}
unsafe fn label(parent: HWND, text: &str, [x, y, width, height]: [i32; 4]) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE,
            x,
            y,
            width,
            height,
            parent,
            null_mut(),
            null_mut(),
            null(),
        )
    }
}

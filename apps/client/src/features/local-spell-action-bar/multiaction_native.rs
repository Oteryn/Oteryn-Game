//! Three-slot assignment editor, including drag from the live spell/object list.
#![allow(
    unsafe_code,
    reason = "The editor owns its Win32 controls on one dialog thread"
)]
use super::{BarAction, ItemUseMode, SpellAim, assignments};
use crate::play::{PlayLink, mailbox::lock};
use std::{
    ptr::{null, null_mut},
    sync::Arc,
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::ScreenToClient,
    UI::{
        Input::KeyboardAndMouse::{GetAsyncKeyState, VK_LBUTTON},
        WindowsAndMessaging::*,
    },
};
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    rect: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            rect[0],
            rect[1],
            rect[2],
            rect[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
#[derive(Default)]
struct Pending {
    actions: Vec<usize>,
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && w >> 16 == BN_CLICKED as usize {
            let pending = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Pending;
            if !pending.is_null() && (*pending).actions.len() < 16 {
                (*pending).actions.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
unsafe fn choose_aim(parent: HWND, mut action: BarAction, point: POINT) -> Option<BarAction> {
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return None;
        }
        let spell_labels = SpellAim::ALL.map(|aim| aim.option().label());
        let labels: &[&str] = match action {
            BarAction::Spell { .. } => &spell_labels,
            _ => &[
                "Use at cursor position",
                "Use on self",
                "Use on current target",
                "Use with crosshair",
            ],
        };
        if matches!(action, BarAction::Spell { .. }) {
            AppendMenuW(
                menu,
                MF_STRING | MF_GRAYED,
                0,
                wide(oteryn_client_ui::spell_action_bar::TARGET_OPTIONS_LABEL).as_ptr(),
            );
        }
        for (index, label) in labels.iter().enumerate() {
            AppendMenuW(menu, MF_STRING, index + 1, wide(label).as_ptr());
        }
        let choice = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            parent,
            null(),
        );
        DestroyMenu(menu);
        if choice == 0 {
            return None;
        }
        match &mut action {
            BarAction::Spell { aim, .. } => *aim = SpellAim::ALL[choice as usize - 1],
            BarAction::Use { mode, .. } => {
                *mode = match choice {
                    1 => ItemUseMode::CursorPosition,
                    2 => ItemUseMode::SelfActor,
                    3 => ItemUseMode::Target,
                    _ => ItemUseMode::Cursor,
                }
            }
            _ => {}
        }
        Some(action)
    }
}
pub(super) fn open(link: &PlayLink, slot: usize) {
    if link.uses_shared_frontend() {
        return;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let _ = std::thread::Builder::new()
        .name("multiaction-editor".into())
        .spawn(move || run(weak, slot));
}
fn run(weak: std::sync::Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>, slot: usize) {
    unsafe {
        let Some(shared) = weak.upgrade() else {
            return;
        };
        let (choices, known, mut assigned) = {
            let mailbox = lock(&shared);
            let spells = oteryn_client_ui::spell_action_bar::dialog_spells(
                mailbox.spell_catalog.catalog.entries(),
                false,
            );
            let mut known: Vec<bool> = spells.iter().map(|entry| entry.learned).collect();
            let mut choices = spells
                .into_iter()
                .map(|entry| {
                    (
                        entry.name.clone(),
                        BarAction::Spell {
                            name: entry.name.clone(),
                            words: entry.words.clone(),
                            aim: SpellAim::Target,
                        },
                    )
                })
                .collect::<Vec<_>>();
            let mut definitions = std::collections::BTreeSet::new();
            for item in super::item_actions::visible_items(&mailbox) {
                if definitions.insert(item.item_definition_ref) {
                    known.push(true);
                    choices.push((
                        format!("Object {}", item.item_definition_ref),
                        BarAction::Use {
                            definition: item.item_definition_ref,
                            mode: ItemUseMode::CursorPosition,
                        },
                    ));
                }
            }
            let mut assigned: [Option<BarAction>; 3] = std::array::from_fn(|_| None);
            if let Some(BarAction::Multi { actions }) = mailbox.bar_actions.saved.0.get(&slot) {
                for (target, action) in assigned.iter_mut().zip(actions) {
                    *target = Some(action.clone());
                }
            }
            (choices, known, assigned)
        };
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Assign Multiaction").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            680,
            480,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            return;
        }
        let mut pending = Box::<Pending>::default();
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&mut *pending as *mut Pending) as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        let list = control(
            hwnd,
            "LISTBOX",
            "",
            100,
            [10, 35, 300, 350],
            LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL,
        );
        control(
            hwnd,
            "STATIC",
            "Drag a spell or object to I, II or III.\nOr select one and click a slot. Clear removes it.",
            0,
            [320, 10, 340, 45],
            0,
        );
        let slots: [HWND; 3] = std::array::from_fn(|i| {
            control(
                hwnd,
                "BUTTON",
                "",
                110 + i,
                [320, 70 + i as i32 * 80, 255, 65],
                BS_MULTILINE as u32,
            )
        });
        let clears: [HWND; 3] = std::array::from_fn(|i| {
            control(
                hwnd,
                "BUTTON",
                "Clear",
                120 + i,
                [580, 70 + i as i32 * 80, 65, 65],
                0,
            )
        });
        let save = control(hwnd, "BUTTON", "Save", 130, [320, 325, 325, 30], 0);
        let known_only = control(
            hwnd,
            "BUTTON",
            oteryn_client_ui::spell_action_bar::SHOW_ONLY_KNOWN_LABEL,
            140,
            [10, 392, 300, 24],
            BS_AUTOCHECKBOX as u32,
        );
        let feedback = control(
            hwnd,
            "STATIC",
            "First action off cooldown will execute.",
            0,
            [320, 365, 325, 60],
            0,
        );
        if [list, save, feedback, known_only]
            .into_iter()
            .chain(slots)
            .chain(clears)
            .any(|handle| handle.is_null())
        {
            DestroyWindow(hwnd);
            return;
        }
        let mut shown: Vec<usize> = (0..choices.len()).collect();
        for (label, _) in &choices {
            SendMessageW(list, LB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        let mut down = false;
        let mut drag = None;
        let mut message: MSG = std::mem::zeroed();
        while IsWindow(hwnd) != 0 {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            if lock(&shared).world_input_blocked() {
                break;
            }
            let held = GetAsyncKeyState(i32::from(VK_LBUTTON)) < 0;
            let mut cursor = POINT { x: 0, y: 0 };
            GetCursorPos(&mut cursor);
            let mut point = cursor;
            ScreenToClient(list, &mut point);
            let mut bounds: RECT = std::mem::zeroed();
            GetClientRect(list, &mut bounds);
            if held
                && !down
                && point.x >= 0
                && point.y >= 0
                && point.x < bounds.right
                && point.y < bounds.bottom
            {
                let index = SendMessageW(
                    list,
                    LB_ITEMFROMPOINT,
                    0,
                    ((point.y as u32) << 16 | point.x as u32) as isize,
                );
                if (index >> 16) == 0 {
                    drag = shown
                        .get((index & 0xffff) as usize)
                        .and_then(|&choice| choices.get(choice))
                        .map(|(_, action)| action.clone());
                }
            }
            if down
                && !held
                && let Some(action) = drag.take()
            {
                for (index, handle) in slots.iter().enumerate() {
                    GetWindowRect(*handle, &mut bounds);
                    if cursor.x >= bounds.left
                        && cursor.x < bounds.right
                        && cursor.y >= bounds.top
                        && cursor.y < bounds.bottom
                    {
                        assigned[index] = choose_aim(hwnd, action.clone(), cursor);
                    }
                }
            }
            down = held;
            for id in std::mem::take(&mut pending.actions) {
                match id {
                    110..=112 => {
                        let index = SendMessageW(list, LB_GETCURSEL, 0, 0);
                        if let Ok(index) = usize::try_from(index)
                            && let Some((_, action)) =
                                shown.get(index).and_then(|&choice| choices.get(choice))
                        {
                            assigned[id - 110] = choose_aim(hwnd, action.clone(), cursor);
                        }
                    }
                    120..=122 => assigned[id - 120] = None,
                    140 => {
                        let only_known = SendMessageW(known_only, BM_GETCHECK, 0, 0) == 1;
                        shown = (0..choices.len())
                            .filter(|&i| !only_known || known[i])
                            .collect();
                        SendMessageW(list, LB_RESETCONTENT, 0, 0);
                        for &choice in &shown {
                            SendMessageW(
                                list,
                                LB_ADDSTRING,
                                0,
                                wide(&choices[choice].0).as_ptr() as isize,
                            );
                        }
                    }
                    130 => {
                        let actions = assigned.iter().flatten().cloned().collect();
                        let mut mailbox = lock(&shared);
                        let saved = assignments::set_action(
                            &mut mailbox,
                            slot,
                            BarAction::Multi { actions },
                        );
                        SetWindowTextW(
                            feedback,
                            wide(if saved {
                                "Multiaction saved"
                            } else {
                                "Assign at least one spell or object"
                            })
                            .as_ptr(),
                        );
                    }
                    _ => {}
                }
            }
            let mailbox = lock(&shared);
            for (index, handle) in slots.iter().enumerate() {
                let label = assigned[index].as_ref().map_or_else(
                    || "Empty".into(),
                    |action| assignments::label(&mailbox, action),
                );
                SetWindowTextW(
                    *handle,
                    wide(&format!("{}: {label}", ["I", "II", "III"][index])).as_ptr(),
                );
            }
            drop(mailbox);
            std::thread::park_timeout(Duration::from_millis(40));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
    }
}

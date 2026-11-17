//! Available and displayed shortcut lists on the native settings page.
use super::*;
use oteryn_client::play::ShortcutEdit;
pub(crate) struct ShortcutControls {
    labels: [HWND; 2],
    lists: [HWND; 2],
    buttons: [HWND; 4],
    keys: [Vec<&'static str>; 2],
}
impl ShortcutControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let labels = [
                control(
                    hwnd,
                    "STATIC",
                    "Available shortcuts",
                    0,
                    [10, 60, 340, 24],
                    0,
                ),
                control(
                    hwnd,
                    "STATIC",
                    "Displayed shortcuts (top to bottom)",
                    0,
                    [380, 60, 340, 24],
                    0,
                ),
            ];
            let lists = [
                control(
                    hwnd,
                    "LISTBOX",
                    "",
                    0,
                    [10, 90, 340, 240],
                    WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
                ),
                control(
                    hwnd,
                    "LISTBOX",
                    "",
                    0,
                    [380, 90, 340, 240],
                    WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
                ),
            ];
            let buttons = [
                ("Add →", 360, 10),
                ("Remove", 361, 380),
                ("Move up", 362, 495),
                ("Move down", 363, 610),
            ]
            .map(|(label, id, x)| control(hwnd, "BUTTON", label, id, [x, 345, 110, 28], 0));
            if labels
                .iter()
                .chain(lists.iter())
                .chain(buttons.iter())
                .any(|h| h.is_null())
            {
                return None;
            }
            Some(Self {
                labels,
                lists,
                buttons,
                keys: Default::default(),
            })
        }
    }
    pub(crate) unsafe fn show(&self, show: bool) {
        unsafe {
            for h in self
                .labels
                .iter()
                .chain(self.lists.iter())
                .chain(self.buttons.iter())
            {
                ShowWindow(*h, if show { SW_SHOW } else { SW_HIDE });
            }
        }
    }
    pub(crate) unsafe fn handle(&self, id: usize, state: &mut Shared) -> bool {
        unsafe {
            let edit = match id {
                360 => ShortcutEdit::Add,
                361 => ShortcutEdit::Remove,
                362 => ShortcutEdit::Up,
                363 => ShortcutEdit::Down,
                _ => return false,
            };
            let list = usize::from(id != 360);
            let selected = usize::try_from(SendMessageW(self.lists[list], LB_GETCURSEL, 0, 0))
                .ok()
                .and_then(|i| self.keys[list].get(i));
            if let Some(key) = selected {
                state.edit_shortcut(key, edit);
            } else {
                state.feedback = "Select a shortcut first".into();
            }
            true
        }
    }
    pub(crate) unsafe fn refresh(&mut self, state: &Shared) {
        unsafe {
            for index in 0..2 {
                let selected = usize::try_from(SendMessageW(self.lists[index], LB_GETCURSEL, 0, 0))
                    .ok()
                    .and_then(|i| self.keys[index].get(i))
                    .copied();
                SendMessageW(self.lists[index], LB_RESETCONTENT, 0, 0);
                let choices = state.shortcut_choices(index == 1);
                self.keys[index].clear();
                for (id, label) in choices {
                    self.keys[index].push(id);
                    SendMessageW(
                        self.lists[index],
                        LB_ADDSTRING,
                        0,
                        wide(label).as_ptr() as isize,
                    );
                }
                if let Some(i) =
                    selected.and_then(|key| self.keys[index].iter().position(|id| *id == key))
                {
                    SendMessageW(self.lists[index], LB_SETCURSEL, i, 0);
                }
            }
        }
    }
}
